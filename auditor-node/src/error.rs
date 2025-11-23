//! 審計節點統一錯誤類型定義
//!
//! 本模塊定義了審計節點運行過程中可能遇到的所有錯誤類型，
//! 使用 thiserror crate 提供良好的錯誤鏈和上下文信息。

use thiserror::Error;

/// 審計節點錯誤類型
///
/// 涵蓋所有子系統的錯誤情況：
/// - Sui 區塊鏈交互
/// - Walrus 存儲節點通信
/// - 默克爾證明驗證
/// - PQC 簽名操作
/// - Seal 加密
/// - 配置管理
#[derive(Error, Debug)]
pub enum AuditorError {
    /// Sui 客戶端錯誤
    ///
    /// 當與 Sui 區塊鏈交互失敗時返回此錯誤
    #[error("Sui client error: {0}")]
    SuiClient(String),

    /// 存儲節點不可達
    ///
    /// 當無法連接到 Walrus 存儲節點時返回此錯誤
    #[error("Storage node unreachable: {0}")]
    StorageNodeUnreachable(String),

    /// 默克爾證明驗證失敗
    ///
    /// 當 sliver 的默克爾證明無法通過驗證時返回此錯誤
    /// 這表示存儲節點可能返回了損壞或篡改的數據
    #[error("Merkle proof verification failed")]
    MerkleVerificationFailed,

    /// 無效的 Sliver 數據
    ///
    /// 當 sliver 數據格式不正確或無法解析時返回此錯誤
    #[error("Invalid sliver data: {0}")]
    InvalidSliver(String),

    /// PQC 簽名錯誤
    ///
    /// 當 Dilithium3 簽名生成或驗證失敗時返回此錯誤
    #[error("PQC signature error: {0}")]
    PqcSignature(String),

    /// 配置錯誤
    ///
    /// 當配置文件格式錯誤或缺少必要參數時返回此錯誤
    #[error("Configuration error: {0}")]
    Config(String),

    /// Seal 加密錯誤
    ///
    /// 當加密審計報告或設置訪問策略失敗時返回此錯誤
    #[error("Seal encryption error: {0}")]
    SealEncryption(String),

    /// HTTP 請求錯誤
    ///
    /// 當向存儲節點發送 HTTP 請求失敗時返回此錯誤
    #[error("HTTP request error: {0}")]
    HttpRequest(#[from] reqwest::Error),

    /// 序列化/反序列化錯誤
    ///
    /// 當 JSON 或 Bincode 序列化失敗時返回此錯誤
    #[error("Serialization error: {0}")]
    Serialization(String),

    /// I/O 錯誤
    ///
    /// 當文件操作或網絡 I/O 失敗時返回此錯誤
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// 密鑰庫錯誤
    ///
    /// 當無法加載或保存 PQC 密鑰時返回此錯誤
    #[error("Keystore error: {0}")]
    Keystore(String),

    /// 通用錯誤
    ///
    /// 用於包裝其他未分類的錯誤
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

/// Result 類型別名
///
/// 使用統一的錯誤類型簡化函數簽名
pub type Result<T> = std::result::Result<T, AuditorError>;

/// 從 JSON 錯誤轉換
impl From<serde_json::Error> for AuditorError {
    fn from(err: serde_json::Error) -> Self {
        AuditorError::Serialization(err.to_string())
    }
}

/// 從 Bincode 錯誤轉換
impl From<bincode::Error> for AuditorError {
    fn from(err: bincode::Error) -> Self {
        AuditorError::Serialization(err.to_string())
    }
}

/// 從 PQC 錯誤轉換
impl From<pqc_signer::error::PqcError> for AuditorError {
    fn from(err: pqc_signer::error::PqcError) -> Self {
        AuditorError::PqcSignature(err.to_string())
    }
}
