/// Error type definitions
use thiserror::Error;

#[derive(Error, Debug)]
pub enum PqcError {
    #[error("Signing failed: {0}")]
    SigningError(String),

    #[error("Verification failed: {0}")]
    VerificationError(String),

    #[error("Key generation failed: {0}")]
    KeyGenerationError(String),

    #[error("Encoding error: {0}")]
    EncodingError(String),

    #[error("Unsupported algorithm: {0}")]
    UnsupportedAlgorithm(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, PqcError>;
