use std::collections::HashMap;
use std::io::prelude::*;
use std::net::TcpListener;

use auto_launch::AutoLaunchBuilder;
use tauri::{AppHandle, ClipboardManager, Runtime};

use crate::crypto::{self, VaultKey};
use crate::google_drive::GoogleDriveClient;
use crate::otp::OtpGenerator;
use crate::qr::QrCodeReader;
use crate::state::AppState;
use crate::storage::Storage;
use crate::sync::SyncManager;
use crate::types::{OtpApp, AppError, Result};

#[tauri::command]
pub fn has_master_password(state: tauri::State<AppState>) -> bool {
    if state.has_master_password() {
        return true;
    }
    
    // Check if encrypted data file exists
    let storage = Storage::new();
    storage.has_apps_file()
}

#[tauri::command]
pub fn verify_master_password(password: String, state: tauri::State<AppState>) -> bool {
    match unlock_vault(&password, &state) {
        Ok(()) => true,
        Err(e) => {
            // Deliberately coarse: the caller only learns that the vault did
            // not open, never why.
            tracing::warn!("Unlock failed: {}", e);
            false
        }
    }
}

/// Unlock the vault, migrating it off the pre-v2 format on the way in.
fn unlock_vault(password: &str, state: &AppState) -> Result<()> {
    let storage = Storage::new();

    let Some(payload) = storage.read_apps_payload()? else {
        // No vault yet — this password establishes one, with a fresh salt.
        state.set_encryption_key(VaultKey::new_vault(password)?);
        state.set_apps(Vec::new());
        tracing::info!("New vault created");
        return Ok(());
    };

    let was_legacy = crypto::is_legacy_payload(&payload);

    // A v2 vault carries its own salt. A legacy one has none, so it gets a
    // fresh random salt that the rewrite below will persist.
    let key = match crypto::payload_salt(&payload)? {
        Some(salt) => VaultKey::derive(password, salt)?,
        None => VaultKey::new_vault(password)?,
    };

    // Decryption is the password check: AES-GCM authentication fails on a
    // wrong key, so a bad password never gets past this line.
    let apps = storage.load_apps(&key)?;

    state.set_encryption_key(key);
    state.set_apps(apps);

    if was_legacy {
        migrate_vault(state, &storage)?;
    }

    tracing::info!("Vault unlocked: {} apps", state.get_apps().len());
    Ok(())
}

/// Rewrite a pre-v2 vault under Argon2id.
///
/// `save_apps` keeps a `.backup` of the old file, so a failure part-way
/// through still leaves a readable vault — legacy payloads stay decryptable.
fn migrate_vault(state: &AppState, storage: &Storage) -> Result<()> {
    let key = state.get_encryption_key().ok_or(AppError::NoMasterPassword)?;

    tracing::info!("Legacy vault detected, migrating to Argon2id...");
    storage.save_apps(&state.get_apps(), &key)?;

    // The Google auth blob was written with the same old key. Rewrite it too,
    // otherwise the next sync would fail to read it.
    if storage.google_auth_needs_migration() {
        match storage.load_google_auth(&key) {
            Ok(auth_json) => storage.save_google_auth(&auth_json, &key)?,
            Err(e) => tracing::warn!("Could not migrate Google auth, re-link needed: {}", e),
        }
    }

    tracing::info!("Vault migration complete");
    Ok(())
}

#[tauri::command]
pub fn get_apps(state: tauri::State<AppState>) -> Vec<OtpApp> {
    state.get_apps()
}

#[tauri::command]
pub fn add_app(name: String, secret: String, state: tauri::State<AppState>) -> Result<()> {
    tracing::info!("Adding app: {}", name);
    
    if name.trim().is_empty() {
        return Err(AppError::InvalidSecret("App name cannot be empty".to_string()));
    }
    
    if secret.trim().is_empty() {
        return Err(AppError::InvalidSecret("Secret cannot be empty".to_string()));
    }
    
    // Validate secret first
    let otp_generator = OtpGenerator::new();
    otp_generator.validate_secret(&secret)?;
    
    // Test code generation
    let test_code = otp_generator.generate_code(&secret)?;
    tracing::info!("Test code generated: {}", test_code);
    
    let id = uuid::Uuid::new_v4().to_string();
    let app = OtpApp { 
        id: id.clone(), 
        name: name.trim().to_string(), 
        secret: secret.trim().to_uppercase() 
    };
    
    state.add_app(app);
    tracing::info!("App added with ID: {}, Total apps: {}", id, state.get_apps().len());
    
    // Save encrypted data
    let key = state.get_encryption_key()
        .ok_or(AppError::NoMasterPassword)?;
    
    let storage = Storage::new();
    storage.save_apps(&state.get_apps(), &key)?;
    tracing::info!("Apps saved to disk successfully");
    
    Ok(())
}

#[tauri::command]
pub fn edit_app_name(id: String, new_name: String, state: tauri::State<AppState>) -> Result<()> {
    if !state.update_app_name(&id, new_name) {
        return Err(AppError::AppNotFound);
    }
    
    // Save encrypted data
    let key = state.get_encryption_key()
        .ok_or(AppError::NoMasterPassword)?;
    
    let storage = Storage::new();
    storage.save_apps(&state.get_apps(), &key)?;
    
    Ok(())
}

#[tauri::command]
pub fn delete_app(app_id: String, state: tauri::State<AppState>) -> Result<()> {
    tracing::info!("Deleting app with ID: {}", app_id);
    
    if !state.remove_app(&app_id) {
        return Err(AppError::AppNotFound);
    }
    
    tracing::info!("Apps after deletion: {}", state.get_apps().len());
    
    // Save encrypted data
    let key = state.get_encryption_key()
        .ok_or(AppError::NoMasterPassword)?;
    
    let storage = Storage::new();
    storage.save_apps(&state.get_apps(), &key)?;
    
    Ok(())
}

#[tauri::command]
pub fn generate_otp(app_id: String, state: tauri::State<AppState>) -> Result<String> {
    let app = state.get_app_by_id(&app_id)
        .ok_or(AppError::AppNotFound)?;
    
    let otp_generator = OtpGenerator::new();
    otp_generator.generate_code(&app.secret)
}

#[tauri::command]
pub fn copy_to_clipboard<R: Runtime>(app: AppHandle<R>, text: String) -> Result<()> {
    app.clipboard_manager()
        .write_text(text)
        .map_err(|e| AppError::Io(e.to_string()))
}

#[tauri::command]
pub fn import_2fas_file(file_content: String, state: tauri::State<AppState>) -> Result<usize> {
    use serde_json::Value;
    
    tracing::info!("Starting 2FAS import...");
    
    let json: Value = serde_json::from_str(&file_content)?;
    
    let services = json.get("services")
        .and_then(|s| s.as_array())
        .ok_or_else(|| AppError::Serialization("Invalid 2FAS file format".to_string()))?;
    
    let mut imported_count = 0;
    let otp_generator = OtpGenerator::new();
    
    for service in services {
        if let (Some(name), Some(secret)) = (
            service.get("name").and_then(|n| n.as_str()),
            service.get("secret").and_then(|s| s.as_str())
        ) {
            // Validate secret
            if otp_generator.validate_secret(secret).is_ok() {
                let app = OtpApp {
                    id: uuid::Uuid::new_v4().to_string(),
                    name: name.to_string(),
                    secret: secret.to_string(),
                };
                state.add_app(app);
                imported_count += 1;
                tracing::info!("Imported: {}", name);
            } else {
                tracing::warn!("Skipped invalid secret for: {}", name);
            }
        } else {
            tracing::warn!("Skipped service - missing required fields: {:?}", service);
        }
    }
    
    tracing::info!("Total imported: {}, Total in memory: {}", imported_count, state.get_apps().len());
    
    // Save encrypted data
    let key = state.get_encryption_key()
        .ok_or(AppError::NoMasterPassword)?;
    
    let storage = Storage::new();
    storage.save_apps(&state.get_apps(), &key)?;
    
    Ok(imported_count)
}

#[tauri::command]
pub fn decode_qr_from_image(image_data: Vec<u8>) -> Result<crate::types::QrData> {
    tracing::info!("Decoding QR from image, size: {} bytes", image_data.len());
    
    if image_data.is_empty() {
        return Err(AppError::QrCode("Empty image data".to_string()));
    }
    
    let qr_reader = QrCodeReader::new();
    let result = qr_reader.decode_from_image(&image_data)?;
    
    tracing::info!("QR decoded successfully: {} - {}", result.name, result.secret);
    Ok(result)
}

#[tauri::command]
pub fn decode_qr_from_clipboard() -> Result<crate::types::QrData> {
    use arboard::Clipboard;
    
    tracing::info!("Reading image from clipboard...");
    
    let mut clipboard = Clipboard::new()
        .map_err(|e| AppError::QrCode(format!("Erro ao acessar clipboard: {}", e)))?;
    
    let img = clipboard.get_image()
        .map_err(|e| AppError::QrCode(format!("Nenhuma imagem no clipboard: {}", e)))?;
    
    // Convert RGBA to PNG bytes
    use image::{ImageBuffer, RgbaImage};
    let rgba_img: RgbaImage = ImageBuffer::from_raw(img.width as u32, img.height as u32, img.bytes.into_owned())
        .ok_or_else(|| AppError::QrCode("Erro ao processar imagem".to_string()))?;
    
    let mut png_bytes = Vec::new();
    rgba_img.write_to(&mut std::io::Cursor::new(&mut png_bytes), image::ImageOutputFormat::Png)
        .map_err(|e| AppError::QrCode(format!("Erro ao converter imagem: {}", e)))?;
    
    tracing::info!("Image found in clipboard, size: {} bytes", png_bytes.len());
    
    let qr_reader = QrCodeReader::new();
    let result = qr_reader.decode_from_image(&png_bytes)?;
    
    tracing::info!("QR decoded successfully from clipboard: {} - {}", result.name, result.secret);
    Ok(result)
}

#[tauri::command]
pub fn set_autostart(enabled: bool) -> Result<()> {
    let exe_path = std::env::current_exe()
        .map_err(AppError::from)?;
    let exe_str = exe_path.to_str()
        .ok_or_else(|| AppError::Io("Invalid executable path".to_string()))?;
    
    let auto_launch = AutoLaunchBuilder::new()
        .set_app_name("Plaxo OTP")
        .set_app_path(exe_str)
        .build()
        .map_err(|e| AppError::Io(e.to_string()))?;

    if enabled {
        auto_launch.enable()
            .map_err(|e| AppError::Io(e.to_string()))?;
        tracing::info!("Autostart enabled");
    } else {
        auto_launch.disable()
            .map_err(|e| AppError::Io(e.to_string()))?;
        tracing::info!("Autostart disabled");
    }

    Ok(())
}

#[tauri::command]
pub fn get_autostart_status() -> Result<bool> {
    let exe_path = std::env::current_exe()
        .map_err(AppError::from)?;
    let exe_str = exe_path.to_str()
        .ok_or_else(|| AppError::Io("Invalid executable path".to_string()))?;
    
    let auto_launch = AutoLaunchBuilder::new()
        .set_app_name("Plaxo OTP")
        .set_app_path(exe_str)
        .build()
        .map_err(|e| AppError::Io(e.to_string()))?;

    auto_launch.is_enabled()
        .map_err(|e| AppError::Io(e.to_string()))
}

#[tauri::command]
pub fn reset_master_password(state: tauri::State<AppState>) -> Result<()> {
    tracing::info!("Resetting master password and data...");
    
    // Clear state in memory
    state.clear_all();
    
    // Remove files from disk
    let storage = Storage::new();
    storage.reset_all_data()?;
    
    tracing::info!("Reset completed successfully");
    Ok(())
}

// Google Drive commands

/// Whether this build was compiled with Google credentials.
#[tauri::command]
pub fn google_drive_is_configured() -> bool {
    GoogleDriveClient::is_configured()
}

/// Parse the query string out of the first line of an HTTP request.
///
/// The redirect carries both `code` and `state`, so splitting on whitespace
/// alone is not enough — each parameter has to be separated and percent-decoded.
fn parse_callback_query(request: &str) -> HashMap<String, String> {
    let mut params = HashMap::new();

    // "GET /?code=...&state=... HTTP/1.1"
    let Some(target) = request
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
    else {
        return params;
    };

    let Some((_, query)) = target.split_once('?') else {
        return params;
    };

    for pair in query.split('&') {
        if let Some((raw_key, raw_value)) = pair.split_once('=') {
            let key = urlencoding::decode(raw_key).unwrap_or_default().into_owned();
            let value = urlencoding::decode(raw_value)
                .unwrap_or_default()
                .into_owned();
            params.insert(key, value);
        }
    }

    params
}

fn open_in_browser(url: &str) -> Result<()> {
    #[cfg(target_os = "windows")]
    let mut command = {
        let mut c = std::process::Command::new("cmd");
        c.args(["/C", "start", "", url]);
        c
    };

    #[cfg(target_os = "macos")]
    let mut command = {
        let mut c = std::process::Command::new("open");
        c.arg(url);
        c
    };

    #[cfg(target_os = "linux")]
    let mut command = {
        let mut c = std::process::Command::new("xdg-open");
        c.arg(url);
        c
    };

    command
        .spawn()
        .map_err(|_| AppError::GoogleDrive("Failed to open browser".to_string()))?;
    Ok(())
}

#[tauri::command]
pub async fn google_drive_auth_flow(state: tauri::State<'_, AppState>) -> Result<()> {
    let client = GoogleDriveClient::new();

    // The verifier and CSRF nonce live only for this attempt and never leave
    // the process.
    let session = client.begin_auth()?;

    let listener = TcpListener::bind("127.0.0.1:8080")
        .map_err(|_| AppError::GoogleDrive("Failed to start local server".to_string()))?;

    listener
        .set_nonblocking(true)
        .map_err(|_| AppError::GoogleDrive("Failed to set non-blocking".to_string()))?;

    open_in_browser(&session.url)?;

    let start_time = std::time::Instant::now();
    let timeout = std::time::Duration::from_secs(60);

    loop {
        if start_time.elapsed() > timeout {
            return Err(AppError::GoogleDrive("Authentication timeout".to_string()));
        }

        match listener.accept() {
            Ok((mut stream, _)) => {
                let mut buffer = [0u8; 4096];
                let Ok(read) = stream.read(&mut buffer) else {
                    continue;
                };

                let request = String::from_utf8_lossy(&buffer[..read]);
                let params = parse_callback_query(&request);

                // The browser also asks for /favicon.ico and similar; keep
                // waiting for the actual redirect instead of failing.
                if params.is_empty() {
                    let _ = stream.write_all(
                        b"HTTP/1.1 404 Not Found\r\nConnection: close\r\n\r\n",
                    );
                    continue;
                }

                if let Some(error) = params.get("error") {
                    let _ = stream.write_all(
                        b"HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nConnection: close\r\n\r\n\
                          <html><body><h1>Authorization denied</h1>\
                          <p>You can close this window.</p></body></html>",
                    );
                    return Err(AppError::GoogleDrive(format!(
                        "Authorization denied: {}",
                        error
                    )));
                }

                // Reject a redirect this process did not initiate.
                if params.get("state").map(String::as_str) != Some(session.state.as_str()) {
                    let _ = stream.write_all(
                        b"HTTP/1.1 400 Bad Request\r\nConnection: close\r\n\r\n",
                    );
                    return Err(AppError::GoogleDrive(
                        "State mismatch on OAuth callback — request rejected".to_string(),
                    ));
                }

                let Some(code) = params.get("code") else {
                    let _ = stream.write_all(
                        b"HTTP/1.1 400 Bad Request\r\nConnection: close\r\n\r\n",
                    );
                    continue;
                };

                let _ = stream.write_all(
                    b"HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nConnection: close\r\n\r\n\
                      <html><body><h1>Authorization completed!</h1>\
                      <p>You can close this window.</p></body></html>",
                );

                let auth = client.exchange_code(code, &session.verifier).await?;

                let key = state
                    .get_encryption_key()
                    .ok_or(AppError::NoMasterPassword)?;

                let sync_manager = SyncManager::new();
                sync_manager.save_google_auth(&auth, &key).await?;

                state.set_google_auth(Some(auth));

                tracing::info!("Authentication successful");
                return Ok(());
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(std::time::Duration::from_millis(100));
                continue;
            }
            Err(_) => continue,
        }
    }
}

#[tauri::command]
pub async fn sync_with_google_drive(state: tauri::State<'_, AppState>) -> Result<()> {
    let apps = state.get_apps();
    let key = state.get_encryption_key()
        .ok_or(AppError::NoMasterPassword)?;
    let auth = state.get_google_auth()
        .ok_or_else(|| AppError::GoogleDrive("Google Drive not authenticated".to_string()))?;
    
    let sync_manager = SyncManager::new();
    sync_manager.sync_to_google_drive(&apps, &key, &auth).await?;
    Ok(())
}

#[tauri::command]
pub async fn restore_from_google_drive(state: tauri::State<'_, AppState>) -> Result<usize> {
    let key = state.get_encryption_key()
        .ok_or(AppError::NoMasterPassword)?;
    let auth = state.get_google_auth()
        .ok_or_else(|| AppError::GoogleDrive("Google Drive not authenticated".to_string()))?;
    
    let sync_manager = SyncManager::new();
    let cloud_apps = sync_manager.sync_from_google_drive(&key, &auth).await?;
    let count = cloud_apps.len();
    
    state.set_apps(cloud_apps);
    
    // Save locally too
    let storage = Storage::new();
    storage.save_apps(&state.get_apps(), &key)?;
    
    Ok(count)
}

#[tauri::command]
pub async fn check_google_auth(state: tauri::State<'_, AppState>) -> Result<bool> {
    // Check if already syncing
    if state.is_syncing() {
        tracing::info!("Sync already in progress, skipping...");
        return Ok(false);
    }
    
    state.set_syncing(true);
    
    let key = state.get_encryption_key()
        .ok_or(AppError::NoMasterPassword)?;
    
    let result = {
        let sync_manager = SyncManager::new();
        match sync_manager.load_google_auth(&key).await {
            Ok(auth) => {
                state.set_google_auth(Some(auth.clone()));
                
                // Initial sync - check if there's data in the cloud
                tracing::info!("Checking cloud data...");
                match sync_manager.sync_from_google_drive(&key, &auth).await {
                    Ok(cloud_apps) => {
                        if !cloud_apps.is_empty() {
                            tracing::info!("Found {} apps in cloud, syncing...", cloud_apps.len());
                            let mut current_apps = state.get_apps();
                            
                            // Merge: keep local apps and add new ones from cloud
                            for cloud_app in cloud_apps {
                                if !current_apps.iter().any(|local_app| local_app.id == cloud_app.id) {
                                    current_apps.push(cloud_app);
                                }
                            }
                            
                            state.set_apps(current_apps);
                            
                            // Save locally
                            let storage = Storage::new();
                            storage.save_apps(&state.get_apps(), &key)?;
                            tracing::info!("Initial sync completed!");
                        } else {
                            tracing::info!("No cloud data, uploading local data...");
                            let apps = state.get_apps();
                            if !apps.is_empty() {
                                sync_manager.sync_to_google_drive(&apps, &key, &auth).await?;
                            }
                        }
                    }
                    Err(e) => {
                        tracing::warn!("Error checking cloud: {}", e);
                        // Don't fail if can't access cloud
                    }
                }
                
                Ok(true)
            }
            Err(_) => Ok(false)
        }
    };
    
    // Release the lock
    state.set_syncing(false);
    
    result
}

#[tauri::command]
pub async fn clear_google_auth(state: tauri::State<'_, AppState>) -> Result<()> {
    // Clear from memory
    state.set_google_auth(None);
    
    // Remove file from disk
    let storage = Storage::new();
    storage.clear_google_auth()?;
    
    tracing::info!("Google Drive authentication removed");
    Ok(())
}
