use std::fs;
use std::path::{Path, PathBuf};

use crate::crypto::{decrypt_data, encrypt_data, VaultKey};
use crate::types::{OtpApp, AppError, Result};

const DATA_DIR: &str = ".plaxo-otp";

/// Restrict a path to the owning user (0600 for files, 0700 for directories).
///
/// Without this the vault inherits the process umask and typically lands
/// world-readable, letting any local account copy the encrypted file and
/// attack it offline at leisure.
///
/// No-op on Windows, where access is governed by ACLs inherited from the
/// user profile directory rather than by Unix mode bits.
#[cfg(unix)]
fn restrict_to_owner(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;

    let is_dir = path.metadata()?.is_dir();
    let mode = if is_dir { 0o700 } else { 0o600 };
    fs::set_permissions(path, fs::Permissions::from_mode(mode))?;
    Ok(())
}

#[cfg(not(unix))]
fn restrict_to_owner(_path: &Path) -> Result<()> {
    Ok(())
}
const APPS_FILE: &str = "apps.enc";
const GOOGLE_AUTH_FILE: &str = "google_auth.enc";

pub struct Storage {
    /// Directory holding the vault. `None` means the real per-user location.
    ///
    /// Tests must override this: without it they would read and write the
    /// developer's own vault.
    root: Option<PathBuf>,
}

impl Storage {
    pub fn new() -> Self {
        Self { root: None }
    }

    /// Storage rooted at an explicit directory, for tests.
    #[cfg(test)]
    pub fn with_root(root: PathBuf) -> Self {
        Self { root: Some(root) }
    }

    fn get_data_dir(&self) -> Result<PathBuf> {
        let mut path = match &self.root {
            Some(root) => root.clone(),
            None => dirs::home_dir()
                .or_else(|| std::env::current_dir().ok())
                .ok_or_else(|| AppError::Io("Could not determine home directory".to_string()))?,
        };

        path.push(DATA_DIR);
        fs::create_dir_all(&path)?;
        restrict_to_owner(&path)?;
        Ok(path)
    }

    fn get_apps_file_path(&self) -> Result<PathBuf> {
        let mut path = self.get_data_dir()?;
        path.push(APPS_FILE);
        Ok(path)
    }

    fn get_google_auth_file_path(&self) -> Result<PathBuf> {
        let mut path = self.get_data_dir()?;
        path.push(GOOGLE_AUTH_FILE);
        Ok(path)
    }

    /// Read the stored vault without decrypting it.
    ///
    /// The unlock path needs the salt out of the payload before it can derive
    /// a key, and needs to know whether the payload predates the v2 format.
    /// Returns `Ok(None)` when no vault exists yet.
    pub fn read_apps_payload(&self) -> Result<Option<String>> {
        let file_path = self.get_apps_file_path()?;
        if !file_path.exists() {
            return Ok(None);
        }
        Ok(Some(fs::read_to_string(&file_path)?))
    }

    /// Whether the stored Google auth blob is still in the pre-v2 format.
    pub fn google_auth_needs_migration(&self) -> bool {
        let Ok(file_path) = self.get_google_auth_file_path() else {
            return false;
        };
        match fs::read_to_string(&file_path) {
            Ok(payload) => crate::crypto::is_legacy_payload(&payload),
            Err(_) => false,
        }
    }

    pub fn save_apps(&self, apps: &[OtpApp], key: &VaultKey) -> Result<()> {
        tracing::info!("Starting save of {} apps", apps.len());
        
        let json = serde_json::to_string(apps)
            .map_err(|e| {
                tracing::error!("Failed to serialize apps: {}", e);
                e
            })?;
        
        tracing::debug!("Serialized {} bytes of JSON", json.len());
        
        let encrypted = encrypt_data(&json, key)
            .map_err(|e| {
                tracing::error!("Failed to encrypt data: {}", e);
                e
            })?;
        
        tracing::debug!("Encrypted {} bytes", encrypted.len());
        
        let file_path = self.get_apps_file_path()?;
        tracing::debug!("Target file: {:?}", file_path);
        
        // Secure write with temporary file
        let temp_path = format!("{}.tmp", file_path.to_string_lossy());
        
        // Backup current file if it exists
        if file_path.exists() {
            let backup_path = format!("{}.backup", file_path.to_string_lossy());
            fs::copy(&file_path, &backup_path)
                .map_err(|e| {
                    tracing::warn!("Failed to create backup: {}", e);
                    e
                })?;
            restrict_to_owner(Path::new(&backup_path))?;
            tracing::debug!("Backup created");
        }
        
        // Write to temporary file, restricting it before the data lands so the
        // secrets are never briefly readable by others.
        fs::write(&temp_path, &encrypted)
            .map_err(|e| {
                tracing::error!("Failed to write temp file: {}", e);
                e
            })?;
        
        restrict_to_owner(Path::new(&temp_path))?;

        tracing::debug!("Temp file written");
        
        // Atomic move to final file
        fs::rename(&temp_path, &file_path)
            .map_err(|e| {
                tracing::error!("Failed to rename temp file: {}", e);
                e
            })?;
        
        restrict_to_owner(&file_path)?;

        tracing::info!("Successfully saved {} apps to {:?}", apps.len(), file_path);
        Ok(())
    }

    pub fn load_apps(&self, key: &VaultKey) -> Result<Vec<OtpApp>> {
        let file_path = self.get_apps_file_path()?;
        
        if !file_path.exists() {
            return Ok(Vec::new());
        }
        
        // Try to load main file
        match self.try_load_file(&file_path, key) {
            Ok(apps) => {
                tracing::info!("Loaded {} apps from storage", apps.len());
                Ok(apps)
            }
            Err(e) => {
                tracing::warn!("Failed to load main file: {}", e);
                
                // Try to load from backup
                let backup_path = format!("{}.backup", file_path.to_string_lossy());
                if std::path::Path::new(&backup_path).exists() {
                    tracing::info!("Attempting to load from backup...");
                    match self.try_load_file(&PathBuf::from(&backup_path), key) {
                        Ok(apps) => {
                            tracing::info!("Backup loaded successfully, restoring main file...");
                            // Restore main file from backup
                            if let Err(restore_err) = fs::copy(&backup_path, &file_path) {
                                tracing::warn!("Could not restore main file: {}", restore_err);
                            }
                            Ok(apps)
                        }
                        Err(backup_err) => {
                            tracing::error!("Backup also corrupted: {}", backup_err);
                            Err(e) // Return original error
                        }
                    }
                } else {
                    Err(e)
                }
            }
        }
    }

    fn try_load_file(&self, file_path: &PathBuf, key: &VaultKey) -> Result<Vec<OtpApp>> {
        // Vaults written before permissions were enforced are still group- and
        // world-readable. Tighten them on the way in, so an existing install is
        // fixed at the next unlock rather than at the next write.
        let _ = restrict_to_owner(file_path);

        let encrypted_data = fs::read_to_string(file_path)?;
        let decrypted = decrypt_data(&encrypted_data, key)?;
        let apps: Vec<OtpApp> = serde_json::from_str(&decrypted)?;
        Ok(apps)
    }

    pub fn save_google_auth(&self, auth_data: &str, key: &VaultKey) -> Result<()> {
        let encrypted = encrypt_data(auth_data, key)?;
        let file_path = self.get_google_auth_file_path()?;
        fs::write(&file_path, &encrypted)?;
        restrict_to_owner(&file_path)?;
        tracing::info!("Saved Google auth to storage");
        Ok(())
    }

    pub fn load_google_auth(&self, key: &VaultKey) -> Result<String> {
        let file_path = self.get_google_auth_file_path()?;
        
        if !file_path.exists() {
            return Err(AppError::GoogleDrive("Auth not found".to_string()));
        }
        
        let encrypted_data = fs::read_to_string(&file_path)?;
        let decrypted = decrypt_data(&encrypted_data, key)?;
        tracing::info!("Loaded Google auth from storage");
        Ok(decrypted)
    }

    pub fn clear_google_auth(&self) -> Result<()> {
        let file_path = self.get_google_auth_file_path()?;
        if file_path.exists() {
            fs::remove_file(&file_path)?;
            tracing::info!("Cleared Google auth from storage");
        }
        Ok(())
    }

    pub fn has_apps_file(&self) -> bool {
        self.get_apps_file_path()
            .map(|path| path.exists())
            .unwrap_or(false)
    }

    pub fn reset_all_data(&self) -> Result<()> {
        let data_dir = self.get_data_dir()?;
        
        if data_dir.exists() {
            fs::remove_dir_all(&data_dir)?;
            tracing::info!("Reset all data");
        }
        
        Ok(())
    }
}

impl Default for Storage {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::VaultKey;
    use tempfile::TempDir;

    #[test]
    fn save_and_load_roundtrip() {
        // Rooted at a temp dir — never the developer's real vault.
        let temp_dir = TempDir::new().unwrap();
        let storage = Storage::with_root(temp_dir.path().to_path_buf());
        let key = VaultKey::new_vault("test_password").unwrap();

        let apps = vec![OtpApp {
            id: "1".to_string(),
            name: "Test App".to_string(),
            secret: "JBSWY3DPEHPK3PXP".to_string(),
        }];

        storage.save_apps(&apps, &key).unwrap();
        let loaded = storage.load_apps(&key).unwrap();

        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].name, "Test App");
        assert_eq!(loaded[0].secret, "JBSWY3DPEHPK3PXP");
    }

    #[test]
    fn writes_land_inside_the_configured_root() {
        let temp_dir = TempDir::new().unwrap();
        let storage = Storage::with_root(temp_dir.path().to_path_buf());
        let key = VaultKey::new_vault("pw").unwrap();

        storage.save_apps(&[], &key).unwrap();

        assert!(
            temp_dir.path().join(DATA_DIR).join(APPS_FILE).exists(),
            "vault must be written under the test root"
        );
    }

    #[test]
    fn stored_vault_uses_the_current_format() {
        let temp_dir = TempDir::new().unwrap();
        let storage = Storage::with_root(temp_dir.path().to_path_buf());
        let key = VaultKey::new_vault("pw").unwrap();

        storage.save_apps(&[], &key).unwrap();

        let payload = storage.read_apps_payload().unwrap().unwrap();
        assert!(!crate::crypto::is_legacy_payload(&payload));
    }

    #[cfg(unix)]
    #[test]
    fn vault_is_not_readable_by_others() {
        use std::os::unix::fs::PermissionsExt;

        let temp_dir = TempDir::new().unwrap();
        let storage = Storage::with_root(temp_dir.path().to_path_buf());
        let key = VaultKey::new_vault("pw").unwrap();

        storage
            .save_apps(
                &[OtpApp {
                    id: "1".into(),
                    name: "App".into(),
                    secret: "JBSWY3DPEHPK3PXP".into(),
                }],
                &key,
            )
            .unwrap();
        // Saving twice also exercises the backup copy.
        storage.save_apps(&[], &key).unwrap();

        let dir = temp_dir.path().join(DATA_DIR);
        let vault = dir.join(APPS_FILE);
        let backup = dir.join(format!("{APPS_FILE}.backup"));

        for path in [&vault, &backup] {
            let mode = path.metadata().unwrap().permissions().mode() & 0o777;
            assert_eq!(
                mode, 0o600,
                "{path:?} must be owner-only, found {mode:o}"
            );
        }

        let dir_mode = dir.metadata().unwrap().permissions().mode() & 0o777;
        assert_eq!(dir_mode, 0o700, "data dir must be owner-only");
    }

    #[cfg(unix)]
    #[test]
    fn loading_tightens_a_legacy_loose_vault() {
        use std::os::unix::fs::PermissionsExt;

        let temp_dir = TempDir::new().unwrap();
        let storage = Storage::with_root(temp_dir.path().to_path_buf());
        let key = VaultKey::new_vault("pw").unwrap();
        storage.save_apps(&[], &key).unwrap();

        let vault = temp_dir.path().join(DATA_DIR).join(APPS_FILE);
        // Simulate a vault written by an older build, under the default umask.
        fs::set_permissions(&vault, fs::Permissions::from_mode(0o664)).unwrap();

        storage.load_apps(&key).unwrap();

        let mode = vault.metadata().unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600, "reading must tighten an over-permissive vault");
    }

    #[test]
    fn wrong_password_cannot_load() {
        let temp_dir = TempDir::new().unwrap();
        let storage = Storage::with_root(temp_dir.path().to_path_buf());

        let key = VaultKey::new_vault("right").unwrap();
        storage
            .save_apps(
                &[OtpApp {
                    id: "1".into(),
                    name: "App".into(),
                    secret: "JBSWY3DPEHPK3PXP".into(),
                }],
                &key,
            )
            .unwrap();

        let salt = crate::crypto::payload_salt(&storage.read_apps_payload().unwrap().unwrap())
            .unwrap()
            .unwrap();
        let wrong = VaultKey::derive("wrong", salt).unwrap();

        assert!(storage.load_apps(&wrong).is_err());
    }
}
