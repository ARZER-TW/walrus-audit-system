/// 錯誤類型定義
use thiserror::Error;

#[derive(Error, Debug)]
pub enum PqcError {
    #[error("簽名失敗: {0}")]
    SigningError(String),

    #[error("驗證失敗: {0}")]
    VerificationError(String),

    #[error("密鑰生成失敗: {0}")]
    KeyGenerationError(String),

    #[error("編碼錯誤: {0}")]
    EncodingError(String),

    #[error("不支持的算法: {0}")]
    UnsupportedAlgorithm(String),

    #[error("IO 錯誤: {0}")]
    IoError(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, PqcError>;
