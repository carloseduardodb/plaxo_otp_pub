use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OtpApp {
    pub id: String,
    pub name: String,
    pub secret: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct QrData {
    pub name: String,
    pub secret: String,
}

#[derive(Debug, Error, Serialize)]
pub enum AppError {
    #[error("Encryption error: {0}")]
    Encryption(String),
    
    #[error("IO error: {0}")]
    Io(String),
    
    #[error("Serialization error: {0}")]
    Serialization(String),
    
    #[error("Invalid secret key: {0}")]
    InvalidSecret(String),
    
    #[error("Master password not set")]
    NoMasterPassword,
    
    #[error("Invalid master password")]
    InvalidMasterPassword,
    
    #[error("App not found")]
    AppNotFound,
    
    #[error("Google Drive error: {0}")]
    GoogleDrive(String),
    
    #[error("QR code error: {0}")]
    QrCode(String),
}

impl From<std::io::Error> for AppError {
    fn from(error: std::io::Error) -> Self {
        AppError::Io(error.to_string())
    }
}

impl From<serde_json::Error> for AppError {
    fn from(error: serde_json::Error) -> Self {
        AppError::Serialization(error.to_string())
    }
}

impl From<AppError> for String {
    fn from(error: AppError) -> Self {
        error.to_string()
    }
}

pub type Result<T> = std::result::Result<T, AppError>;
