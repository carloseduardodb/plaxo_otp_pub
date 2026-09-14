//! Vault encryption.
//!
//! Payload format (v2), base64-encoded:
//!
//! ```text
//! "PLX2" | salt (16 bytes) | nonce (12 bytes) | AES-256-GCM ciphertext
//! ```
//!
//! The key is derived with Argon2id from the master password plus the
//! per-vault random salt stored in the payload itself.
//!
//! Vaults written by versions <= 1.3.3 have no header: they are a bare
//! `nonce | ciphertext` whose key was a single SHA-256 pass over the password
//! and a salt compiled into the binary. Those are still readable so existing
//! vaults can be migrated, but they are never written again — see
//! [`VaultKey::legacy_key`].

use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Key, Nonce,
};
use argon2::{Algorithm, Argon2, Params, Version};
use base64::{engine::general_purpose, Engine as _};
use rand::RngCore;
use sha2::{Digest, Sha256};
use zeroize::{Zeroize, ZeroizeOnDrop};

use crate::types::{AppError, Result};

pub const SALT_LEN: usize = 16;
const NONCE_LEN: usize = 12;
const MAGIC: &[u8; 4] = b"PLX2";
const HEADER_LEN: usize = MAGIC.len() + SALT_LEN;

/// Argon2id cost. 64 MiB / 3 passes / 4 lanes is the OWASP baseline and takes
/// roughly 100 ms on a current laptop — unnoticeable when unlocking once per
/// session, but it puts offline guessing about seven orders of magnitude
/// further away than the old single SHA-256 pass.
const ARGON_MEMORY_KIB: u32 = 65_536;
const ARGON_ITERATIONS: u32 = 3;
const ARGON_PARALLELISM: u32 = 4;

/// The salt baked into versions <= 1.3.3. Only ever used to read old vaults.
const LEGACY_SALT: &[u8] = b"plaxo-otp-salt-2024";

/// A derived vault key, bound to the salt it was derived with.
///
/// Both keys are wiped from memory on drop. The master password itself is
/// never stored: it is consumed by [`VaultKey::derive`] and dropped there.
#[derive(Clone, Zeroize, ZeroizeOnDrop)]
pub struct VaultKey {
    key: [u8; 32],
    /// Key for the pre-v2 format, kept only so an old vault can be read once
    /// and immediately rewritten in v2.
    legacy_key: [u8; 32],
    #[zeroize(skip)]
    salt: [u8; SALT_LEN],
}

impl std::fmt::Debug for VaultKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Never let key material reach a log line.
        f.debug_struct("VaultKey").finish_non_exhaustive()
    }
}

impl VaultKey {
    /// Derive a key for an existing vault whose salt is already known.
    pub fn derive(password: &str, salt: [u8; SALT_LEN]) -> Result<Self> {
        let params = Params::new(
            ARGON_MEMORY_KIB,
            ARGON_ITERATIONS,
            ARGON_PARALLELISM,
            Some(32),
        )
        .map_err(|e| AppError::Encryption(format!("Invalid Argon2 parameters: {e}")))?;

        let argon = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);

        let mut key = [0u8; 32];
        argon
            .hash_password_into(password.as_bytes(), &salt, &mut key)
            .map_err(|e| AppError::Encryption(format!("Key derivation failed: {e}")))?;

        Ok(Self {
            key,
            legacy_key: legacy_derive_key(password),
            salt,
        })
    }

    /// Derive a key for a brand-new vault, generating a fresh random salt.
    pub fn new_vault(password: &str) -> Result<Self> {
        let mut salt = [0u8; SALT_LEN];
        rand::thread_rng().fill_bytes(&mut salt);
        Self::derive(password, salt)
    }

    pub fn salt(&self) -> [u8; SALT_LEN] {
        self.salt
    }
}

/// The pre-v2 derivation: one SHA-256 pass with a global constant salt.
///
/// Retained only to decrypt vaults written by older versions. Never use it to
/// encrypt anything.
fn legacy_derive_key(password: &str) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(password.as_bytes());
    hasher.update(LEGACY_SALT);
    hasher.finalize().into()
}

/// Read the salt out of a stored payload.
///
/// Returns `Ok(None)` when the payload is in the pre-v2 format, which carries
/// no salt — the caller should then pick a fresh salt and migrate.
pub fn payload_salt(payload: &str) -> Result<Option<[u8; SALT_LEN]>> {
    let raw = general_purpose::STANDARD
        .decode(payload.trim())
        .map_err(|_| AppError::Encryption("Invalid base64 data".to_string()))?;

    if raw.len() >= HEADER_LEN && &raw[..MAGIC.len()] == MAGIC {
        let mut salt = [0u8; SALT_LEN];
        salt.copy_from_slice(&raw[MAGIC.len()..HEADER_LEN]);
        Ok(Some(salt))
    } else {
        Ok(None)
    }
}

/// True when the payload still uses the pre-v2 format and should be migrated.
pub fn is_legacy_payload(payload: &str) -> bool {
    matches!(payload_salt(payload), Ok(None))
}

/// Encrypt to the v2 format. Always writes a fresh random nonce.
pub fn encrypt_data(data: &str, vault_key: &VaultKey) -> Result<String> {
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&vault_key.key));

    let mut nonce_bytes = [0u8; NONCE_LEN];
    rand::thread_rng().fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);

    let ciphertext = cipher
        .encrypt(nonce, data.as_bytes())
        .map_err(|_| AppError::Encryption("Failed to encrypt data".to_string()))?;

    let mut out = Vec::with_capacity(HEADER_LEN + NONCE_LEN + ciphertext.len());
    out.extend_from_slice(MAGIC);
    out.extend_from_slice(&vault_key.salt);
    out.extend_from_slice(&nonce_bytes);
    out.extend_from_slice(&ciphertext);

    Ok(general_purpose::STANDARD.encode(out))
}

/// Decrypt either format.
///
/// A v2 payload is decrypted with the Argon2id key; a pre-v2 payload falls
/// back to the legacy key so the vault can be read once and migrated. A v2
/// payload whose salt does not match this key is rejected rather than being
/// silently attempted, since that means the key belongs to a different vault.
pub fn decrypt_data(encrypted_data: &str, vault_key: &VaultKey) -> Result<String> {
    let raw = general_purpose::STANDARD
        .decode(encrypted_data.trim())
        .map_err(|_| AppError::Encryption("Invalid base64 data".to_string()))?;

    let (key, payload) = if raw.len() >= HEADER_LEN && &raw[..MAGIC.len()] == MAGIC {
        if raw[MAGIC.len()..HEADER_LEN] != vault_key.salt {
            return Err(AppError::Encryption(
                "Payload belongs to a different vault".to_string(),
            ));
        }
        (&vault_key.key, &raw[HEADER_LEN..])
    } else {
        (&vault_key.legacy_key, &raw[..])
    };

    if payload.len() < NONCE_LEN {
        return Err(AppError::Encryption("Data too short".to_string()));
    }

    let (nonce_bytes, ciphertext) = payload.split_at(NONCE_LEN);
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(key));

    let plaintext = cipher
        .decrypt(Nonce::from_slice(nonce_bytes), ciphertext)
        .map_err(|_| AppError::InvalidMasterPassword)?;

    String::from_utf8(plaintext)
        .map_err(|_| AppError::Encryption("Invalid UTF-8 data".to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key_for(password: &str) -> VaultKey {
        VaultKey::new_vault(password).unwrap()
    }

    #[test]
    fn encrypt_decrypt_roundtrip() {
        let vk = key_for("correct horse battery staple");
        let encrypted = encrypt_data("test data", &vk).unwrap();
        assert_eq!(decrypt_data(&encrypted, &vk).unwrap(), "test data");
    }

    #[test]
    fn wrong_password_is_rejected() {
        let vk = key_for("password1");
        let encrypted = encrypt_data("test data", &vk).unwrap();

        // Same salt, different password: the vault must not open.
        let wrong = VaultKey::derive("password2", vk.salt()).unwrap();
        assert!(decrypt_data(&encrypted, &wrong).is_err());
    }

    #[test]
    fn each_vault_gets_a_distinct_salt() {
        let a = key_for("same password");
        let b = key_for("same password");
        assert_ne!(a.salt(), b.salt(), "salt must not be reused across vaults");
        assert_ne!(a.key, b.key, "same password must not yield the same key");
    }

    #[test]
    fn nonce_is_not_reused() {
        let vk = key_for("password");
        let first = encrypt_data("same plaintext", &vk).unwrap();
        let second = encrypt_data("same plaintext", &vk).unwrap();
        assert_ne!(first, second, "identical plaintext must not produce identical output");
    }

    #[test]
    fn output_carries_the_v2_header() {
        let vk = key_for("password");
        let encrypted = encrypt_data("data", &vk).unwrap();
        let raw = general_purpose::STANDARD.decode(&encrypted).unwrap();

        assert_eq!(&raw[..4], MAGIC);
        assert_eq!(&raw[4..HEADER_LEN], &vk.salt());
        assert!(!is_legacy_payload(&encrypted));
        assert_eq!(payload_salt(&encrypted).unwrap(), Some(vk.salt()));
    }

    /// Builds a payload exactly as versions <= 1.3.3 wrote it.
    fn legacy_encrypt(data: &str, password: &str) -> String {
        let key = legacy_derive_key(password);
        let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&key));
        let mut nonce_bytes = [0u8; NONCE_LEN];
        rand::thread_rng().fill_bytes(&mut nonce_bytes);
        let ciphertext = cipher
            .encrypt(Nonce::from_slice(&nonce_bytes), data.as_bytes())
            .unwrap();
        let mut out = nonce_bytes.to_vec();
        out.extend_from_slice(&ciphertext);
        general_purpose::STANDARD.encode(out)
    }

    #[test]
    fn legacy_vault_still_opens() {
        let legacy = legacy_encrypt("my secrets", "hunter2");
        assert!(is_legacy_payload(&legacy));
        assert_eq!(payload_salt(&legacy).unwrap(), None);

        // Salt is irrelevant to reading a legacy payload, so any fresh key works.
        let vk = VaultKey::new_vault("hunter2").unwrap();
        assert_eq!(decrypt_data(&legacy, &vk).unwrap(), "my secrets");
    }

    #[test]
    fn legacy_vault_rejects_wrong_password() {
        let legacy = legacy_encrypt("my secrets", "hunter2");
        let vk = VaultKey::new_vault("wrong").unwrap();
        assert!(decrypt_data(&legacy, &vk).is_err());
    }

    #[test]
    fn migrated_vault_is_no_longer_legacy() {
        let legacy = legacy_encrypt("my secrets", "hunter2");
        let vk = VaultKey::new_vault("hunter2").unwrap();

        let plaintext = decrypt_data(&legacy, &vk).unwrap();
        let migrated = encrypt_data(&plaintext, &vk).unwrap();

        assert!(!is_legacy_payload(&migrated));
        assert_eq!(decrypt_data(&migrated, &vk).unwrap(), "my secrets");
    }

    #[test]
    fn payload_from_another_vault_is_rejected() {
        let mine = key_for("password");
        let theirs = key_for("password");
        let encrypted = encrypt_data("data", &theirs).unwrap();
        assert!(decrypt_data(&encrypted, &mine).is_err());
    }
}
