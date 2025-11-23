//! Sui 區塊鏈客戶端模塊
//!
//! 負責與 Sui 區塊鏈交互:
//! - 查詢 Walrus Blob 對象元數據
//! - 提交審計報告交易
//! - 查詢審計配置
//! - 管理審計員聲譽
//!
//! # 架構說明
//!
//! 本模塊封裝了與兩個智能合約包的交互:
//! 1. `audit_system` - 核心審計邏輯和審計員管理
//! 2. `access_policy` - 審計報告訪問控制
//!
//! # 安全性
//!
//! - 所有交易都需要簽名者私鑰
//! - 使用 Sui SDK 的事務塊 API 構造交易
//! - 支持 gas budget 配置

use crate::error::{AuditorError, Result};
use crate::types::{BlobMetadata, ObjectID as LocalObjectID};
use tracing::{debug, info, warn};

// 條件編譯：只在啟用 sui-sdk feature 時導入
#[cfg(feature = "sui-sdk")]
use {
    sui_sdk::{
        rpc_types::{SuiObjectDataOptions, SuiTransactionBlockResponseOptions},
        types::{
            base_types::{ObjectID, SuiAddress},
            programmable_transaction_builder::ProgrammableTransactionBuilder,
            quorum_driver_types::ExecuteTransactionRequestType,
            transaction::{Argument, CallArg, Command, Transaction, TransactionData},
            Identifier,
        },
        SuiClient, SuiClientBuilder,
    },
    std::str::FromStr,
};

// 未啟用 sui-sdk 時的類型別名
#[cfg(not(feature = "sui-sdk"))]
type SuiAddress = String;

#[cfg(not(feature = "sui-sdk"))]
type ObjectID = String;

// BlobMetadata 已移至 types.rs，統一使用 types::BlobMetadata

/// 審計系統客戶端
///
/// 封裝與 Sui 區塊鏈上審計系統合約的所有交互
pub struct AuditSystemClient {
    #[cfg(feature = "sui-sdk")]
    sui_client: SuiClient,

    #[cfg(not(feature = "sui-sdk"))]
    rpc_url: String,

    /// 審計系統合約的 Package ID
    audit_package_id: String,

    /// 訪問策略合約的 Package ID
    access_package_id: String,

    /// AuditorRegistry 共享對象的 ID
    registry_id: String,

    /// AuditConfig 共享對象的 ID
    audit_config_id: String,

    /// RewardPool 共享對象的 ID（可選）
    reward_pool_id: Option<String>,

    /// Gas budget (默認 10M MIST = 0.01 SUI)
    gas_budget: u64,
}

impl AuditSystemClient {
    /// 創建新的審計系統客戶端
    ///
    /// # 參數
    /// - `rpc_url`: Sui RPC 端點 URL
    /// - `audit_package_id`: audit_system 合約包 ID
    /// - `access_package_id`: access_policy 合約包 ID
    /// - `registry_id`: AuditorRegistry 共享對象 ID
    /// - `audit_config_id`: AuditConfig 共享對象 ID
    ///
    /// # 錯誤
    /// - 如果無法連接到 Sui RPC 節點
    /// - 如果 Package ID 格式不正確
    #[cfg(feature = "sui-sdk")]
    pub async fn new(
        rpc_url: &str,
        audit_package_id: &str,
        access_package_id: &str,
        registry_id: &str,
        audit_config_id: &str,
    ) -> Result<Self> {
        info!("Connecting to Sui RPC at {}", rpc_url);

        let sui_client = SuiClientBuilder::default()
            .build(rpc_url)
            .await
            .map_err(|e| AuditorError::SuiClient(format!("Failed to build Sui client: {}", e)))?;

        // 驗證連接
        let chain_id = sui_client
            .read_api()
            .get_chain_identifier()
            .await
            .map_err(|e| AuditorError::SuiClient(format!("Failed to get chain ID: {}", e)))?;

        info!("Connected to Sui chain: {}", chain_id);

        Ok(Self {
            sui_client,
            audit_package_id: audit_package_id.to_string(),
            access_package_id: access_package_id.to_string(),
            registry_id: registry_id.to_string(),
            audit_config_id: audit_config_id.to_string(),
            reward_pool_id: None,
            gas_budget: 10_000_000, // 0.01 SUI
        })
    }

    /// 創建新的審計系統客戶端（無 Sui SDK 版本）
    #[cfg(not(feature = "sui-sdk"))]
    pub async fn new(
        rpc_url: &str,
        audit_package_id: &str,
        access_package_id: &str,
        registry_id: &str,
        audit_config_id: &str,
    ) -> Result<Self> {
        info!("Creating AuditSystemClient (sui-sdk feature disabled)");
        warn!("Sui SDK is disabled - all blockchain operations will fail");

        Ok(Self {
            rpc_url: rpc_url.to_string(),
            audit_package_id: audit_package_id.to_string(),
            access_package_id: access_package_id.to_string(),
            registry_id: registry_id.to_string(),
            audit_config_id: audit_config_id.to_string(),
            reward_pool_id: None,
            gas_budget: 10_000_000,
        })
    }

    /// 設置 Gas Budget
    pub fn set_gas_budget(&mut self, budget: u64) {
        self.gas_budget = budget;
    }

    /// 設置 RewardPool ID
    pub fn set_reward_pool_id(&mut self, pool_id: String) {
        self.reward_pool_id = Some(pool_id);
    }

    // ============ Blob 元數據查詢 ============

    /// 讀取 Walrus Blob 對象的元數據
    ///
    /// # 參數
    /// - `blob_object_id`: Walrus Blob 對象的 ID（注意不是 blob_id u256）
    ///
    /// # 返回
    /// - `BlobMetadata`: Blob 的元數據，包括默克爾根、大小、epoch 等
    ///
    /// # 實現邏輯
    /// 1. 調用 Sui RPC 獲取對象數據
    /// 2. 解析對象的 Move 結構
    /// 3. 提取所需字段並轉換為 BlobMetadata
    #[cfg(feature = "sui-sdk")]
    pub async fn get_blob_metadata(&self, blob_object_id: &str) -> Result<BlobMetadata> {
        info!("Fetching blob metadata for object {}", blob_object_id);

        let object_id = ObjectID::from_str(blob_object_id)
            .map_err(|e| AuditorError::SuiClient(format!("Invalid object ID: {}", e)))?;

        // 獲取對象數據（包含內容）
        let object_response = self
            .sui_client
            .read_api()
            .get_object_with_options(
                object_id,
                SuiObjectDataOptions::new()
                    .with_content()
                    .with_owner()
                    .with_type(),
            )
            .await
            .map_err(|e| AuditorError::SuiClient(format!("Failed to fetch object: {}", e)))?;

        let object_data = object_response
            .data
            .ok_or_else(|| AuditorError::SuiClient("Object not found".to_string()))?;

        debug!("Blob object type: {:?}", object_data.object_type);

        // 解析對象內容
        // 注意: 實際實現需要根據 Walrus Blob 對象的 Move 結構解析
        // 這裡提供簡化版本，實際需要使用 sui_sdk 的 MoveStruct 解析

        // TODO: 實際解析 Walrus Blob Move 結構
        // 參考: contracts/walrus/sources/system/blob.move

        warn!("Using mock blob metadata - actual parsing not yet implemented");

        Ok(BlobMetadata {
            merkle_root: vec![0u8; 32],
            size: 1024,
            encoded_size: 5120,
            epoch: 1,
            storage_nodes: vec![],
            erasure_k: 10,
            erasure_n: 15,
        })
    }

    #[cfg(not(feature = "sui-sdk"))]
    pub async fn get_blob_metadata(&self, blob_object_id: &str) -> Result<BlobMetadata> {
        warn!("get_blob_metadata called without sui-sdk feature");
        Err(AuditorError::SuiClient(
            "Sui SDK not enabled - cannot query blob metadata".to_string(),
        ))
    }

    // ============ 審計記錄提交 ============

    /// 提交審計記錄到鏈上
    ///
    /// 調用 `audit_core::submit_audit_record` 函數
    ///
    /// # 參數
    /// - `signer`: 簽名者地址（審計員地址）
    /// - `blob_id`: Blob ID (u256 as bytes)
    /// - `blob_object_id`: Blob 對象 ID
    /// - `challenge_epoch`: 執行審計的 epoch
    /// - `total_challenges`: 總挑戰次數
    /// - `successful_verifications`: 成功驗證次數
    /// - `integrity_hash`: 完整性哈希
    /// - `pqc_signature`: PQC 簽名
    /// - `pqc_algorithm`: PQC 算法類型 (1=Falcon512, 2=Dilithium2, 3=Dilithium3)
    #[cfg(feature = "sui-sdk")]
    pub async fn submit_audit_record(
        &self,
        signer: SuiAddress,
        blob_id: Vec<u8>,           // u256 as 32 bytes
        blob_object_id: ObjectID,
        challenge_epoch: u32,
        total_challenges: u16,
        successful_verifications: u16,
        integrity_hash: Vec<u8>,
        pqc_signature: Vec<u8>,
        pqc_algorithm: u8,
    ) -> Result<String> {
        info!(
            "Submitting audit record for blob {:?}, epoch {}",
            blob_object_id, challenge_epoch
        );

        // 構造可編程交易塊
        let mut ptb = ProgrammableTransactionBuilder::new();

        // 準備參數
        let config_arg = ptb.obj(CallArg::Object(
            ObjectID::from_str(&self.audit_config_id)
                .map_err(|e| AuditorError::SuiClient(format!("Invalid config ID: {}", e)))?
                .into(),
        ))?;

        // blob_id (u256)
        let blob_id_arg = ptb.pure(blob_id)?;

        // blob_object_id (ID)
        let blob_obj_arg = ptb.pure(blob_object_id)?;

        // challenge_epoch (u32)
        let epoch_arg = ptb.pure(challenge_epoch)?;

        // total_challenges (u16)
        let total_arg = ptb.pure(total_challenges)?;

        // successful_verifications (u16)
        let success_arg = ptb.pure(successful_verifications)?;

        // integrity_hash (vector<u8>)
        let hash_arg = ptb.pure(integrity_hash)?;

        // pqc_signature (vector<u8>)
        let sig_arg = ptb.pure(pqc_signature)?;

        // pqc_algorithm (u8)
        let algo_arg = ptb.pure(pqc_algorithm)?;

        // Clock object (0x6)
        let clock_arg = ptb.obj(CallArg::CLOCK)?;

        // 構造 Move 調用
        let package_id = ObjectID::from_str(&self.audit_package_id)
            .map_err(|e| AuditorError::SuiClient(format!("Invalid package ID: {}", e)))?;

        ptb.command(Command::move_call(
            package_id,
            Identifier::new("audit_core").map_err(|e| {
                AuditorError::SuiClient(format!("Invalid module name: {}", e))
            })?,
            Identifier::new("submit_audit_record").map_err(|e| {
                AuditorError::SuiClient(format!("Invalid function name: {}", e))
            })?,
            vec![], // 無類型參數
            vec![
                config_arg,
                blob_id_arg,
                blob_obj_arg,
                epoch_arg,
                total_arg,
                success_arg,
                hash_arg,
                sig_arg,
                algo_arg,
                clock_arg,
            ],
        ));

        // 完成交易塊構建
        let pt = ptb.finish();

        // TODO: 實際簽名和提交交易
        // 這需要審計員的私鑰，應該從配置或密鑰庫中獲取

        warn!("Transaction building complete but not submitted - signer integration needed");

        Ok("0x0000000000000000000000000000000000000000000000000000000000000000".to_string())
    }

    #[cfg(not(feature = "sui-sdk"))]
    pub async fn submit_audit_record(
        &self,
        _signer: SuiAddress,
        _blob_id: Vec<u8>,
        _blob_object_id: LocalObjectID,
        _challenge_epoch: u32,
        _total_challenges: u16,
        _successful_verifications: u16,
        _integrity_hash: Vec<u8>,
        _pqc_signature: Vec<u8>,
        _pqc_algorithm: u8,
    ) -> Result<String> {
        Err(AuditorError::SuiClient(
            "Sui SDK not enabled - cannot submit audit record".to_string(),
        ))
    }

    // ============ 審計報告元數據提交 ============

    /// 提交審計報告元數據
    ///
    /// 調用 `auditor_registry::submit_audit_report_metadata`
    ///
    /// # 參數
    /// - `encrypted_report_blob_id`: Walrus 上加密報告的 Blob ID
    /// - `audit_record_ids`: 報告涵蓋的審計記錄 ID 列表
    /// - `pqc_signature`: 對報告的 PQC 簽名
    #[cfg(feature = "sui-sdk")]
    pub async fn submit_report_metadata(
        &self,
        _signer: SuiAddress,
        encrypted_report_blob_id: ObjectID,
        audit_record_ids: Vec<ObjectID>,
        pqc_signature: Vec<u8>,
    ) -> Result<String> {
        info!(
            "Submitting audit report metadata for {} audit records",
            audit_record_ids.len()
        );

        // TODO: 實現與 submit_audit_record 類似的交易構造邏輯

        warn!("submit_report_metadata not fully implemented");

        Ok("0x0000000000000000000000000000000000000000000000000000000000000000".to_string())
    }

    #[cfg(not(feature = "sui-sdk"))]
    pub async fn submit_report_metadata(
        &self,
        _signer: SuiAddress,
        _encrypted_report_blob_id: LocalObjectID,
        _audit_record_ids: Vec<LocalObjectID>,
        _pqc_signature: Vec<u8>,
    ) -> Result<String> {
        Err(AuditorError::SuiClient(
            "Sui SDK not enabled - cannot submit report metadata".to_string(),
        ))
    }

    // ============ 訪問策略管理 ============

    /// 為審計報告設置訪問策略
    ///
    /// 調用 `report_access::create_policy`
    #[cfg(feature = "sui-sdk")]
    pub async fn set_report_access_policy(
        &self,
        _signer: SuiAddress,
        report_blob_id: u64,
        audit_record_id: ObjectID,
        authorized_readers: Vec<SuiAddress>,
        validity_days: u64,
    ) -> Result<String> {
        info!(
            "Setting access policy for report {} with {} authorized readers",
            report_blob_id,
            authorized_readers.len()
        );

        // TODO: 構造調用 access_policy::report_access::create_policy 的交易

        warn!("set_report_access_policy not fully implemented");

        Ok("0x0000000000000000000000000000000000000000000000000000000000000000".to_string())
    }

    #[cfg(not(feature = "sui-sdk"))]
    pub async fn set_report_access_policy(
        &self,
        _signer: SuiAddress,
        _report_blob_id: u64,
        _audit_record_id: LocalObjectID,
        _authorized_readers: Vec<SuiAddress>,
        _validity_days: u64,
    ) -> Result<String> {
        Err(AuditorError::SuiClient(
            "Sui SDK not enabled - cannot set access policy".to_string(),
        ))
    }

    // ============ 審計員管理 ============

    /// 查詢審計員的聲譽分數
    ///
    /// 調用只讀函數 `auditor_registry::get_auditor_reputation`
    #[cfg(feature = "sui-sdk")]
    pub async fn get_auditor_reputation(&self, auditor: SuiAddress) -> Result<u64> {
        info!("Querying reputation for auditor {}", auditor);

        // TODO: 使用 devInspectTransactionBlock 調用只讀函數

        warn!("get_auditor_reputation not fully implemented");

        Ok(0)
    }

    #[cfg(not(feature = "sui-sdk"))]
    pub async fn get_auditor_reputation(&self, _auditor: SuiAddress) -> Result<u64> {
        Err(AuditorError::SuiClient(
            "Sui SDK not enabled - cannot query reputation".to_string(),
        ))
    }

    /// 檢查審計員是否已註冊
    #[cfg(feature = "sui-sdk")]
    pub async fn is_auditor_registered(&self, auditor: SuiAddress) -> Result<bool> {
        info!("Checking if auditor {} is registered", auditor);

        // TODO: 調用 auditor_registry::is_auditor_registered

        warn!("is_auditor_registered not fully implemented");

        Ok(false)
    }

    #[cfg(not(feature = "sui-sdk"))]
    pub async fn is_auditor_registered(&self, _auditor: SuiAddress) -> Result<bool> {
        Err(AuditorError::SuiClient(
            "Sui SDK not enabled - cannot check registration".to_string(),
        ))
    }

    // ============ 獎勵管理 ============

    /// 領取審計獎勵
    ///
    /// 當審計員發現作弊節點時，可以領取獎勵
    #[cfg(feature = "sui-sdk")]
    pub async fn claim_audit_reward(
        &self,
        _signer: SuiAddress,
        reward_claim_id: ObjectID,
    ) -> Result<String> {
        info!("Claiming audit reward with claim ID {}", reward_claim_id);

        if self.reward_pool_id.is_none() {
            return Err(AuditorError::Config(
                "RewardPool ID not configured".to_string(),
            ));
        }

        // TODO: 調用 incentives::claim_audit_reward

        warn!("claim_audit_reward not fully implemented");

        Ok("0x0000000000000000000000000000000000000000000000000000000000000000".to_string())
    }

    #[cfg(not(feature = "sui-sdk"))]
    pub async fn claim_audit_reward(
        &self,
        _signer: SuiAddress,
        _reward_claim_id: LocalObjectID,
    ) -> Result<String> {
        Err(AuditorError::SuiClient(
            "Sui SDK not enabled - cannot claim reward".to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[cfg(not(feature = "sui-sdk"))]
    async fn test_client_creation_without_sdk() {
        let client = AuditSystemClient::new(
            "https://fullnode.testnet.sui.io:443",
            "0x1234",
            "0x5678",
            "0xabcd",
            "0xef01",
        )
        .await;

        assert!(client.is_ok());
    }
}
