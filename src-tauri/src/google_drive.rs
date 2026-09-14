//! Google Drive backup sync.
//!
//! OAuth uses the installed-app loopback flow with PKCE (RFC 7636). The
//! authorization code is bound to a one-time verifier that never leaves this
//! process, so intercepting the redirect is not enough to redeem it.
//!
//! Credentials are supplied at build time and are never committed:
//!
//! ```sh
//! PLAXO_GOOGLE_CLIENT_ID=...apps.googleusercontent.com cargo tauri build
//! ```
//!
//! A build without them still compiles; Drive sync simply reports that it is
//! not configured. See `docs/google-drive-setup.md`.

use std::collections::HashMap;

use base64::{engine::general_purpose, Engine as _};
use rand::RngCore;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::types::{AppError, Result};

/// Injected by the build; absent in an unconfigured checkout.
const CLIENT_ID: Option<&str> = option_env!("PLAXO_GOOGLE_CLIENT_ID");

/// Optional. Google issues a secret for "Desktop app" clients and expects it
/// on the token endpoint, but it is explicitly *not* confidential — it ships
/// inside every binary. PKCE is what actually protects the exchange. Clients
/// created without a secret work too, so this stays optional and, either way,
/// out of the repository.
const CLIENT_SECRET: Option<&str> = option_env!("PLAXO_GOOGLE_CLIENT_SECRET");

const REDIRECT_URI: &str = "http://localhost:8080";
const SCOPE: &str = "https://www.googleapis.com/auth/drive.file";

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GoogleDriveAuth {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_at: u64,
}

#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: String,
    refresh_token: Option<String>,
    expires_in: u64,
}

#[derive(Debug, Deserialize)]
struct FileResponse {
    id: String,
}

#[derive(Debug, Deserialize)]
struct FilesResponse {
    files: Vec<FileResponse>,
}

/// One in-flight authorization attempt.
///
/// `verifier` and `state` are single-use and must not outlive the attempt.
pub struct AuthSession {
    pub url: String,
    pub verifier: String,
    pub state: String,
}

pub struct GoogleDriveClient {
    client: Client,
}

/// URL-safe base64 without padding, as PKCE requires.
fn b64url(bytes: &[u8]) -> String {
    general_purpose::URL_SAFE_NO_PAD.encode(bytes)
}

/// 32 random bytes, base64url-encoded — a 43-character verifier, the length
/// RFC 7636 recommends.
fn random_token() -> String {
    let mut bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut bytes);
    b64url(&bytes)
}

impl GoogleDriveClient {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
        }
    }

    /// Whether this build carries Google credentials.
    pub fn is_configured() -> bool {
        CLIENT_ID.map(|id| !id.trim().is_empty()).unwrap_or(false)
    }

    fn client_id() -> Result<&'static str> {
        CLIENT_ID.filter(|id| !id.trim().is_empty()).ok_or_else(|| {
            AppError::GoogleDrive(
                "Google Drive sync is not configured in this build. Rebuild with \
                 PLAXO_GOOGLE_CLIENT_ID set — see docs/google-drive-setup.md."
                    .to_string(),
            )
        })
    }

    fn client_secret() -> Option<&'static str> {
        CLIENT_SECRET.filter(|s| !s.trim().is_empty())
    }

    /// Start an authorization attempt: build the consent URL and the secrets
    /// that bind it to this process.
    pub fn begin_auth(&self) -> Result<AuthSession> {
        let client_id = Self::client_id()?;

        let verifier = random_token();
        let challenge = b64url(&Sha256::digest(verifier.as_bytes()));
        let state = random_token();

        let url = format!(
            "https://accounts.google.com/o/oauth2/v2/auth\
             ?client_id={}\
             &redirect_uri={}\
             &response_type=code\
             &scope={}\
             &access_type=offline\
             &prompt=consent\
             &code_challenge={}\
             &code_challenge_method=S256\
             &state={}",
            urlencoding::encode(client_id),
            urlencoding::encode(REDIRECT_URI),
            urlencoding::encode(SCOPE),
            urlencoding::encode(&challenge),
            urlencoding::encode(&state),
        );

        Ok(AuthSession {
            url,
            verifier,
            state,
        })
    }

    /// Redeem an authorization code. `verifier` must be the one from the
    /// [`AuthSession`] that produced the code.
    pub async fn exchange_code(&self, code: &str, verifier: &str) -> Result<GoogleDriveAuth> {
        let client_id = Self::client_id()?;

        let mut params = HashMap::new();
        params.insert("client_id", client_id);
        params.insert("code", code);
        params.insert("code_verifier", verifier);
        params.insert("grant_type", "authorization_code");
        params.insert("redirect_uri", REDIRECT_URI);
        if let Some(secret) = Self::client_secret() {
            params.insert("client_secret", secret);
        }

        let response = self
            .client
            .post("https://oauth2.googleapis.com/token")
            .form(&params)
            .send()
            .await
            .map_err(|e| AppError::GoogleDrive(format!("Request failed: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(AppError::GoogleDrive(format!(
                "HTTP {}: {}",
                status, error_text
            )));
        }

        let token_response: TokenResponse = response
            .json()
            .await
            .map_err(|e| AppError::GoogleDrive(format!("Invalid response: {}", e)))?;

        Ok(GoogleDriveAuth {
            access_token: token_response.access_token,
            refresh_token: token_response.refresh_token.unwrap_or_default(),
            expires_at: chrono::Utc::now().timestamp() as u64 + token_response.expires_in,
        })
    }

    pub async fn refresh_token(&self, refresh_token: &str) -> Result<GoogleDriveAuth> {
        let client_id = Self::client_id()?;

        let mut params = HashMap::new();
        params.insert("client_id", client_id);
        params.insert("refresh_token", refresh_token);
        params.insert("grant_type", "refresh_token");
        if let Some(secret) = Self::client_secret() {
            params.insert("client_secret", secret);
        }

        let response = self
            .client
            .post("https://oauth2.googleapis.com/token")
            .form(&params)
            .send()
            .await
            .map_err(|e| AppError::GoogleDrive(format!("Refresh request failed: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(AppError::GoogleDrive(format!(
                "Refresh failed HTTP {}: {}",
                status, error_text
            )));
        }

        let token_response: TokenResponse = response
            .json()
            .await
            .map_err(|e| AppError::GoogleDrive(format!("Invalid refresh response: {}", e)))?;

        Ok(GoogleDriveAuth {
            access_token: token_response.access_token,
            refresh_token: refresh_token.to_string(),
            expires_at: chrono::Utc::now().timestamp() as u64 + token_response.expires_in,
        })
    }

    pub async fn find_file(&self, auth: &GoogleDriveAuth, filename: &str) -> Result<Option<String>> {
        tracing::info!("Searching for file: {}", filename);
        
        let response = self.client
            .get("https://www.googleapis.com/drive/v3/files")
            .bearer_auth(&auth.access_token)
            .query(&[("q", format!("name='{}'", filename))])
            .send()
            .await
            .map_err(|e| AppError::GoogleDrive(format!("Search request failed: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(AppError::GoogleDrive(format!("Search failed HTTP {}: {}", status, error_text)));
        }

        let files_response: FilesResponse = response
            .json()
            .await
            .map_err(|e| AppError::GoogleDrive(format!("Invalid search response: {}", e)))?;

        if let Some(file) = files_response.files.first() {
            tracing::info!("File found! ID: {}", file.id);
            Ok(Some(file.id.clone()))
        } else {
            tracing::info!("File not found");
            Ok(None)
        }
    }

    pub async fn upload_file(&self, auth: &GoogleDriveAuth, filename: &str, content: &[u8]) -> Result<String> {
        tracing::info!("Uploading file: {} ({} bytes)", filename, content.len());
        
        let metadata = serde_json::json!({
            "name": filename
        });

        let form = reqwest::multipart::Form::new()
            .part("metadata", reqwest::multipart::Part::text(metadata.to_string())
                .mime_str("application/json")
                .map_err(|e| AppError::GoogleDrive(format!("Failed to create metadata: {}", e)))?)
            .part("media", reqwest::multipart::Part::bytes(content.to_vec())
                .file_name(filename.to_string())
                .mime_str("text/plain")
                .map_err(|e| AppError::GoogleDrive(format!("Failed to create media: {}", e)))?);

        let response = self.client
            .post("https://www.googleapis.com/upload/drive/v3/files?uploadType=multipart")
            .bearer_auth(&auth.access_token)
            .multipart(form)
            .send()
            .await
            .map_err(|e| AppError::GoogleDrive(format!("Upload request failed: {}", e)))?;

        let status = response.status();
        let response_text = response.text().await
            .map_err(|e| AppError::GoogleDrive(format!("Failed to read response: {}", e)))?;
        
        if !status.is_success() {
            return Err(AppError::GoogleDrive(format!("Upload failed HTTP {}: {}", status, response_text)));
        }

        let file_response: FileResponse = serde_json::from_str(&response_text)
            .map_err(|e| AppError::GoogleDrive(format!("Invalid upload response: {} - Response: {}", e, response_text)))?;

        tracing::info!("Upload completed! File ID: {}", file_response.id);
        Ok(file_response.id)
    }

    pub async fn update_file(&self, auth: &GoogleDriveAuth, file_id: &str, content: &[u8]) -> Result<()> {
        tracing::info!("Updating file ID: {} ({} bytes)", file_id, content.len());
        
        let response = self.client
            .patch(&format!("https://www.googleapis.com/upload/drive/v3/files/{}?uploadType=media", file_id))
            .bearer_auth(&auth.access_token)
            .body(content.to_vec())
            .send()
            .await
            .map_err(|e| AppError::GoogleDrive(format!("Update request failed: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(AppError::GoogleDrive(format!("Update failed HTTP {}: {}", status, error_text)));
        }

        tracing::info!("File updated successfully!");
        Ok(())
    }

    pub async fn download_file(&self, auth: &GoogleDriveAuth, file_id: &str) -> Result<Vec<u8>> {
        tracing::info!("Downloading file ID: {}", file_id);
        
        let response = self.client
            .get(&format!("https://www.googleapis.com/drive/v3/files/{}?alt=media", file_id))
            .bearer_auth(&auth.access_token)
            .send()
            .await
            .map_err(|e| AppError::GoogleDrive(format!("Download request failed: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(AppError::GoogleDrive(format!("Download failed HTTP {}: {}", status, error_text)));
        }

        let bytes = response.bytes().await
            .map_err(|e| AppError::GoogleDrive(format!("Failed to read download: {}", e)))?
            .to_vec();

        tracing::info!("Downloaded {} bytes", bytes.len());
        Ok(bytes)
    }
}

impl Default for GoogleDriveClient {
    fn default() -> Self {
        Self::new()
    }
}
