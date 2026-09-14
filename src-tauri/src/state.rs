use std::sync::{Arc, RwLock};

use crate::crypto::VaultKey;
use crate::google_drive::GoogleDriveAuth;
use crate::types::OtpApp;

#[derive(Debug)]
pub struct AppState {
    pub apps: Arc<RwLock<Vec<OtpApp>>>,
    /// Whether the vault is unlocked. The master password itself is never
    /// retained — only the key derived from it.
    pub unlocked: Arc<RwLock<bool>>,
    pub encryption_key: Arc<RwLock<Option<VaultKey>>>,
    pub google_auth: Arc<RwLock<Option<GoogleDriveAuth>>>,
    pub syncing: Arc<RwLock<bool>>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            apps: Arc::new(RwLock::new(Vec::new())),
            unlocked: Arc::new(RwLock::new(false)),
            encryption_key: Arc::new(RwLock::new(None)),
            google_auth: Arc::new(RwLock::new(None)),
            syncing: Arc::new(RwLock::new(false)),
        }
    }

    pub fn get_apps(&self) -> Vec<OtpApp> {
        self.apps.read().unwrap().clone()
    }

    pub fn get_app_by_id(&self, id: &str) -> Option<OtpApp> {
        self.apps.read().unwrap()
            .iter()
            .find(|app| app.id == id)
            .cloned()
    }

    pub fn set_apps(&self, apps: Vec<OtpApp>) {
        let mut apps_guard = self.apps.write().unwrap();
        *apps_guard = apps;
    }

    pub fn add_app(&self, app: OtpApp) {
        let mut apps_guard = self.apps.write().unwrap();
        apps_guard.push(app);
    }

    pub fn remove_app(&self, id: &str) -> bool {
        let mut apps_guard = self.apps.write().unwrap();
        let before_len = apps_guard.len();
        apps_guard.retain(|app| app.id != id);
        apps_guard.len() != before_len
    }

    pub fn update_app_name(&self, id: &str, new_name: String) -> bool {
        let mut apps_guard = self.apps.write().unwrap();
        if let Some(app) = apps_guard.iter_mut().find(|a| a.id == id) {
            app.name = new_name;
            true
        } else {
            false
        }
    }

    pub fn get_encryption_key(&self) -> Option<VaultKey> {
        self.encryption_key.read().unwrap().clone()
    }

    /// Store the derived key and mark the vault unlocked.
    pub fn set_encryption_key(&self, key: VaultKey) {
        let mut key_guard = self.encryption_key.write().unwrap();
        *key_guard = Some(key);
        let mut unlocked_guard = self.unlocked.write().unwrap();
        *unlocked_guard = true;
    }

    pub fn has_master_password(&self) -> bool {
        *self.unlocked.read().unwrap()
    }

    pub fn get_google_auth(&self) -> Option<GoogleDriveAuth> {
        self.google_auth.read().unwrap().clone()
    }

    pub fn set_google_auth(&self, auth: Option<GoogleDriveAuth>) {
        let mut auth_guard = self.google_auth.write().unwrap();
        *auth_guard = auth;
    }

    pub fn is_syncing(&self) -> bool {
        *self.syncing.read().unwrap()
    }

    pub fn set_syncing(&self, syncing: bool) {
        let mut syncing_guard = self.syncing.write().unwrap();
        *syncing_guard = syncing;
    }

    pub fn clear_all(&self) {
        let mut apps_guard = self.apps.write().unwrap();
        let mut unlocked_guard = self.unlocked.write().unwrap();
        let mut key_guard = self.encryption_key.write().unwrap();
        let mut auth_guard = self.google_auth.write().unwrap();

        apps_guard.clear();
        *unlocked_guard = false;
        // Dropping the VaultKey zeroizes the key material.
        *key_guard = None;
        *auth_guard = None;
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}
