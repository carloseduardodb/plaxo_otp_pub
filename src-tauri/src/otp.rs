use totp_rs::{Algorithm, TOTP, Secret};

use crate::types::{AppError, Result};

pub struct OtpGenerator;

impl OtpGenerator {
    pub fn new() -> Self {
        Self
    }

    pub fn generate_code(&self, secret: &str) -> Result<String> {
        let clean_secret = self.clean_secret(secret);
        
        if clean_secret.is_empty() {
            return Err(AppError::InvalidSecret("Empty secret".to_string()));
        }
        
        let secret_bytes = Secret::Encoded(clean_secret.clone())
            .to_bytes()
            .map_err(|e| AppError::InvalidSecret(format!("Invalid Base32 secret: {}", e)))?;
        
        // Try to create TOTP with validation
        let totp = match TOTP::new(Algorithm::SHA1, 6, 1, 30, secret_bytes.clone()) {
            Ok(t) => t,
            Err(_) => {
                // If it fails due to size validation, use unchecked version
                TOTP::new_unchecked(Algorithm::SHA1, 6, 1, 30, secret_bytes)
            }
        };
        
        let code = totp
            .generate_current()
            .map_err(|e| AppError::InvalidSecret(format!("Failed to generate code: {}", e)))?;
        
        Ok(code)
    }

    fn clean_secret(&self, secret: &str) -> String {
        secret
            .replace(' ', "")
            .replace('-', "")
            .replace('=', "")
            .to_uppercase()
    }

    pub fn validate_secret(&self, secret: &str) -> Result<()> {
        let clean_secret = self.clean_secret(secret);
        
        if clean_secret.is_empty() {
            return Err(AppError::InvalidSecret("Chave secreta vazia".to_string()));
        }
        
        if clean_secret.len() < 16 {
            return Err(AppError::InvalidSecret("Chave secreta muito curta (mínimo 16 caracteres)".to_string()));
        }
        
        // Validate Base32 characters
        let invalid_chars: Vec<char> = clean_secret
            .chars()
            .filter(|c| !"ABCDEFGHIJKLMNOPQRSTUVWXYZ234567".contains(*c))
            .collect();
        
        if !invalid_chars.is_empty() {
            return Err(AppError::InvalidSecret(
                format!("Caracteres inválidos na chave: {:?}. Use apenas A-Z e 2-7", invalid_chars)
            ));
        }
        
        // Try to decode
        Secret::Encoded(clean_secret.clone())
            .to_bytes()
            .map_err(|e| AppError::InvalidSecret(format!("Chave Base32 inválida: {}", e)))?;
        
        Ok(())
    }
}

impl Default for OtpGenerator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_code() {
        let generator = OtpGenerator::new();
        let secret = "JBSWY3DPEHPK3PXP";
        
        let code = generator.generate_code(secret).unwrap();
        assert_eq!(code.len(), 6);
        assert!(code.chars().all(|c| c.is_ascii_digit()));
    }

    #[test]
    fn test_clean_secret() {
        let generator = OtpGenerator::new();
        
        assert_eq!(generator.clean_secret("JBSWY3DP EHPK3PXP"), "JBSWY3DPEHPK3PXP");
        assert_eq!(generator.clean_secret("jbswy3dp-ehpk3pxp"), "JBSWY3DPEHPK3PXP");
        assert_eq!(generator.clean_secret("JBSWY3DPEHPK3PXP="), "JBSWY3DPEHPK3PXP");
    }

    #[test]
    fn test_validate_secret() {
        let generator = OtpGenerator::new();
        
        assert!(generator.validate_secret("JBSWY3DPEHPK3PXP").is_ok());
        assert!(generator.validate_secret("").is_err());
        assert!(generator.validate_secret("INVALID@SECRET").is_err());
    }
}
