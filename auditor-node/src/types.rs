//! 共享數據類型定義
//!
//! 本模塊定義審計節點中各個子系統共享的數據結構

use serde::{Deserialize, Serialize};

// 暫時使用 String 代替 ObjectID（實際部署時啟用 Sui SDK）
#[cfg(feature = "sui-sdk")]
use sui_types::base_types::ObjectID;

#[cfg(not(feature = "sui-sdk"))]
pub type ObjectID = String;

/// Walrus Blob 元數據
///
/// 從 Sui 區塊鏈查詢 Walrus Blob 對象得到的元數據
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlobMetadata {
    /// Blob 對象 ID（Sui 對象 ID）
    pub blob_object_id: ObjectID,

    /// Blob ID（u256，Blake2b-256 哈希）
    pub blob_id: String,

    /// 默克爾根哈希
    pub merkle_root: Vec<u8>,

    /// Blob 大小（字節）
    pub blob_size: u64,

    /// Erasure coding 參數 - 數據 slivers 數量
    pub encoding_k: u16,

    /// Erasure coding 參數 - 總 slivers 數量（包含冗餘）
    pub encoding_n: u16,

    /// Blob 開始 epoch
    pub start_epoch: u32,

    /// Blob 結束 epoch（過期時間）
    pub end_epoch: u32,

    /// Blob 所有者地址
    pub owner: String,
}

/// 存儲節點信息
///
/// Walrus 存儲節點的網絡信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageNodeInfo {
    /// 節點 ID
    pub node_id: u16,

    /// 節點 API 端點（HTTP）
    pub api_endpoint: String,

    /// 節點公鑰（用於驗證簽名）
    pub public_key: Vec<u8>,

    /// 節點是否在線
    pub is_online: bool,
}

/// 審計挑戰
///
/// 對存儲節點發起的單次挑戰
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditChallenge {
    /// Sliver 索引（0 到 n-1）
    pub sliver_index: u16,

    /// 目標 Shard ID（存儲節點 ID）
    pub shard_id: u16,

    /// 挑戰類型（1=完整 sliver, 2=recovery symbol）
    pub challenge_type: u8,

    /// 挑戰時間戳
    pub timestamp: u64,
}

/// 審計響應
///
/// 存儲節點返回的挑戰響應
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditResponse {
    /// 對應的挑戰
    pub challenge: AuditChallenge,

    /// Sliver 數據
    pub sliver_data: Vec<u8>,

    /// 默克爾證明路徑
    pub merkle_proof: Vec<[u8; 32]>,

    /// 響應時間戳
    pub timestamp: u64,

    /// 存儲節點簽名（可選）
    pub signature: Option<Vec<u8>>,
}

/// 審計結果
///
/// 單次挑戰的驗證結果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChallengeResult {
    /// 挑戰信息
    pub challenge: AuditChallenge,

    /// 是否驗證成功
    pub verified: bool,

    /// 默克爾證明是否有效
    pub merkle_proof_valid: bool,

    /// 響應數據哈希
    pub response_hash: Vec<u8>,

    /// 失敗原因（如有）
    pub failure_reason: Option<String>,
}

/// 審計報告
///
/// 完整的審計報告（提交到鏈上前的完整版本）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditReport {
    /// 審計的 Blob ID
    pub blob_id: String,

    /// Blob 對象 ID
    pub blob_object_id: ObjectID,

    /// 審計員地址
    pub auditor: String,

    /// 審計執行時間戳
    pub timestamp: u64,

    /// 執行審計時的 epoch
    pub challenge_epoch: u32,

    /// 所有挑戰結果
    pub challenge_results: Vec<ChallengeResult>,

    /// 總挑戰次數
    pub total_challenges: u16,

    /// 成功驗證次數
    pub successful_verifications: u16,

    /// 失敗驗證次數
    pub failed_verifications: u16,

    /// 完整性哈希（所有挑戰的聚合哈希）
    pub integrity_hash: Vec<u8>,

    /// PQC 簽名
    pub pqc_signature: Vec<u8>,

    /// PQC 算法（1=Falcon512, 2=Dilithium2, 3=Dilithium3）
    pub pqc_algorithm: u8,

    /// 審計是否通過
    pub is_valid: bool,

    /// 失敗原因（如有）
    pub failure_reason: Option<String>,
}

/// 配置結構（將在 config.rs 中使用）
///
/// 審計節點運行時配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditorConfig {
    /// Sui RPC 端點
    pub sui_rpc_url: String,

    /// Walrus 聚合器 API 端點
    pub walrus_aggregator_url: String,

    /// 審計員私鑰路徑
    pub auditor_private_key_path: String,

    /// PQC 密鑰庫路徑
    pub pqc_keystore_path: String,

    /// 最小挑戰次數
    pub min_challenges: u16,

    /// 最大挑戰次數
    pub max_challenges: u16,

    /// 審計間隔（秒）
    pub audit_interval_secs: u64,

    /// HTTP 請求超時（秒）
    pub http_timeout_secs: u64,

    /// 是否啟用 Seal 加密
    pub enable_seal_encryption: bool,

    /// Seal API 端點（可選）
    pub seal_api_url: Option<String>,

    /// Audit System 合約 Package ID
    pub audit_system_package_id: Option<String>,

    /// Access Policy 合約 Package ID
    pub access_policy_package_id: Option<String>,

    /// Auditor Registry Object ID
    pub auditor_registry_id: Option<String>,

    /// Incentives Object ID
    pub incentives_id: Option<String>,
}

impl Default for AuditorConfig {
    fn default() -> Self {
        Self {
            sui_rpc_url: std::env::var("SUI_RPC_URL")
                .unwrap_or_else(|_| "https://fullnode.testnet.sui.io:443".to_string()),
            walrus_aggregator_url: std::env::var("WALRUS_AGGREGATOR_URL")
                .unwrap_or_else(|_| "https://aggregator.walrus-testnet.walrus.space".to_string()),
            auditor_private_key_path: std::env::var("AUDITOR_PRIVATE_KEY_PATH")
                .unwrap_or_else(|_| "./keys/auditor.key".to_string()),
            pqc_keystore_path: std::env::var("PQC_KEYSTORE_PATH")
                .unwrap_or_else(|_| "./keys/pqc_keystore".to_string()),
            min_challenges: std::env::var("MIN_CHALLENGES")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(10),
            max_challenges: std::env::var("MAX_CHALLENGES")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(100),
            audit_interval_secs: std::env::var("AUDIT_INTERVAL_SECS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(3600),
            http_timeout_secs: std::env::var("HTTP_TIMEOUT_SECS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(30),
            enable_seal_encryption: std::env::var("ENABLE_SEAL_ENCRYPTION")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(false),
            seal_api_url: std::env::var("SEAL_API_URL").ok(),
            audit_system_package_id: std::env::var("AUDIT_SYSTEM_PACKAGE_ID").ok(),
            access_policy_package_id: std::env::var("ACCESS_POLICY_PACKAGE_ID").ok(),
            auditor_registry_id: std::env::var("AUDITOR_REGISTRY_ID").ok(),
            incentives_id: std::env::var("INCENTIVES_ID").ok(),
        }
    }
}
