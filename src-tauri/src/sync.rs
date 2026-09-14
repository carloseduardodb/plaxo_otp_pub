use crate::crypto::{decrypt_data, encrypt_data, VaultKey};
use crate::google_drive::{GoogleDriveAuth, GoogleDriveClient};
use crate::storage::Storage;
use crate::types::{OtpApp, AppError, Result};

pub struct SyncManager {
    storage: Storage,
    client: GoogleDriveClient,
}

impl SyncManager {
    pub fn new() -> Self {
        Self {
            storage: Storage::new(),
            client: GoogleDriveClient::new(),
        }
    }



    pub async fn sync_to_google_drive(
        &self,
        apps: &[OtpApp],
        key: &VaultKey,
        auth: &GoogleDriveAuth,
    ) -> Result<()> {
        let json = serde_json::to_string(apps)?;
        let encrypted = encrypt_data(&json, key)?;
        
        let filename = "plaxo-otp-backup.enc";
        
        tracing::info!("Starting sync with Google Drive... (Apps: {})", apps.len());
        
        match self.client.find_file(auth, filename).await? {
            Some(file_id) => {
                tracing::info!("Updating existing file in Google Drive...");
                self.client.update_file(auth, &file_id, encrypted.as_bytes()).await?;
                tracing::info!("File updated successfully in Google Drive!");
            }
            None => {
                tracing::info!("Creating new file in Google Drive...");
                let file_id = self.client.upload_file(auth, filename, encrypted.as_bytes()).await?;
                tracing::info!("File created successfully in Google Drive! ID: {}", file_id);
            }
        }
        
        tracing::info!("Sync completed! File: {}", filename);
        Ok(())
    }

    pub async fn sync_from_google_drive(
        &self,
        key: &VaultKey,
        auth: &GoogleDriveAuth,
    ) -> Result<Vec<OtpApp>> {
        let filename = "plaxo-otp-backup.enc";
        
        match self.client.find_file(auth, filename).await? {
            Some(file_id) => {
                let encrypted_data = self.client.download_file(auth, &file_id).await?;
                
                let encrypted_str = String::from_utf8(encrypted_data)
                    .map_err(|_| AppError::GoogleDrive("Corrupted data in Google Drive".to_string()))?;
                
                let decrypted = decrypt_data(&encrypted_str, key)?;
                let apps: Vec<OtpApp> = serde_json::from_str(&decrypted)?;
                
                Ok(apps)
            }
            None => Ok(Vec::new()),
        }
    }

    pub async fn save_google_auth(&self, auth: &GoogleDriveAuth, key: &VaultKey) -> Result<()> {
        let json = serde_json::to_string(auth)?;
        self.storage.save_google_auth(&json, key)?;
        Ok(())
    }

    pub async fn load_google_auth(&self, key: &VaultKey) -> Result<GoogleDriveAuth> {
        let json = self.storage.load_google_auth(key)?;
        let mut auth: GoogleDriveAuth = serde_json::from_str(&json)?;
        
        // Check if token needs refresh
        let now = chrono::Utc::now().timestamp() as u64;
        if now >= auth.expires_at {
            tracing::info!("Token expired, refreshing...");
            auth = self.client.refresh_token(&auth.refresh_token).await?;
            
            // Save the new token
            self.save_google_auth(&auth, key).await?;
        }
        
        Ok(auth)
    }
}

impl Default for SyncManager {
    fn default() -> Self {
        Self::new()
    }
}
