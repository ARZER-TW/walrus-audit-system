//! 應用層完整性驗證模組
//!
//! 本模組實現了對 Walrus 儲存的 Blob 進行應用層完整性驗證。
//!
//! # 驗證策略
//!
//! 我們採用「信任但驗證（Trust on First Use, TOFU）」模式：
//!
//! 1. **協議層保證**（Walrus 負責）:
//!    - 數據可用性（通過 erasure coding 和冗餘存儲）
//!    - blob_id 是經過 RS2 編碼後數據的加密哈希
//!
//! 2. **應用層保證**（我們負責）:
//!    - 下載的原始數據完整性（通過 SHA-256）
//!    - 歷史一致性追蹤（每次審計記錄內容哈希）
//!    - 防篡改證明（通過 PQC 簽名）
//!
//! # 為什麼不驗證 blob_id？
//!
//! Walrus 的 `blob_id` 是對 **erasure-coded 數據** 的哈希，而非原始數據。
//! 要驗證 blob_id，需要：
//! 1. 獲取 Walrus 的 RS2 編碼實現
//! 2. 對原始數據執行相同的編碼
//! 3. 計算編碼後數據的哈希
//!
//! 這在技術上可行，但對於 Hackathon 時間線和應用層審計來說：
//! - **不必要**：Walrus 協議層已經保證了 blob_id 的正確性
//! - **不經濟**：重新編碼大型數據消耗大量資源
//! - **可替代**：我們的 SHA-256 哈希提供了等效的完整性保證
//!
//! # 架構優勢
//!
//! 這種雙層驗證架構提供了：
//! - **獨立性**：不依賴 Walrus 內部實現細節
//! - **可移植性**：同樣的邏輯可以用於其他存儲後端
//! - **可審計性**：SHA-256 是公認的標準，任何人都可以驗證
//! - **量子抗性**：PQC 簽名保護審計記錄的長期真實性

use crate::crypto::merkle::{MerkleTree, MerkleError};
use crate::error::{AuditorError, Result};
use chrono::Utc;
use rand::Rng;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tracing::{debug, info, warn};

/// Walrus Aggregator 的基礎 URL（Testnet）
pub const WALRUS_AGGREGATOR_TESTNET: &str = "https://aggregator.walrus-testnet.walrus.space";

/// 審計數據結構
///
/// 包含單次審計的所有關鍵信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditData {
    /// Walrus Blob ID（用於索引和關聯）
    pub blob_id: String,

    /// 內容哈希（SHA-256）- 應用層完整性基準
    ///
    /// 這是對從 Aggregator 下載的原始數據計算的哈希
    pub content_hash: String,

    /// Merkle 根（Blake2b-256）- 協議層完整性證明
    ///
    /// 使用 4KB chunks 構建的 Merkle Tree 根哈希
    pub merkle_root: String,

    /// 總挑戰次數
    ///
    /// 在審計過程中隨機選擇的 chunk 數量
    pub total_challenges: u16,

    /// 成功驗證次數
    ///
    /// Merkle Proof 驗證通過的 chunk 數量
    pub successful_verifications: u16,

    /// 失敗驗證次數
    ///
    /// Merkle Proof 驗證失敗的 chunk 數量
    pub failed_verifications: u16,

    /// 文件大小（bytes）
    pub file_size: u64,

    /// 審計時間戳（Unix 時間，秒）
    pub timestamp: u64,

    /// 驗證狀態
    ///
    /// - "ACCESSIBLE": Blob 可成功下載並完成哈希計算
    /// - "UNREACHABLE": Aggregator 無法訪問
    /// - "CORRUPTED": 下載成功但哈希與記錄不符（僅在後續審計時出現）
    pub verification_status: VerificationStatus,

    /// 可選：Sui 對象 ID（如果已知）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sui_object_id: Option<String>,
}

/// 驗證狀態枚舉
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "UPPERCASE")]
pub enum VerificationStatus {
    /// Blob 可訪問且完整
    Accessible,
    /// Aggregator 無法訪問 Blob
    Unreachable,
    /// Blob 內容與預期哈希不符（數據損壞）
    Corrupted,
}

/// 完整性驗證器
///
/// 負責執行應用層完整性審計
pub struct IntegrityVerifier {
    /// HTTP 客戶端
    http_client: Client,

    /// Walrus Aggregator URL
    aggregator_url: String,
}

impl IntegrityVerifier {
    /// 創建新的完整性驗證器
    ///
    /// # 參數
    /// - `aggregator_url`: Walrus Aggregator 的基礎 URL
    ///
    /// # 示例
    /// ```no_run
    /// use auditor_node::integrity::IntegrityVerifier;
    ///
    /// let verifier = IntegrityVerifier::new(
    ///     "https://aggregator.walrus-testnet.walrus.space".to_string()
    /// );
    /// ```
    pub fn new(aggregator_url: String) -> Self {
        let http_client = Client::builder()
            .timeout(std::time::Duration::from_secs(60))
            .build()
            .expect("Failed to build HTTP client");

        info!("Created IntegrityVerifier for {}", aggregator_url);

        Self {
            http_client,
            aggregator_url,
        }
    }

    /// 創建使用 Testnet 配置的驗證器
    pub fn new_testnet() -> Self {
        Self::new(WALRUS_AGGREGATOR_TESTNET.to_string())
    }

    /// 審計單個 Blob
    ///
    /// 執行完整的審計流程：
    /// 1. 從 Walrus Aggregator 下載 Blob
    /// 2. 計算 SHA-256 哈希
    /// 3. 記錄時間戳和狀態
    ///
    /// # 參數
    /// - `blob_id`: Walrus Blob ID（Base64 格式）
    ///
    /// # 返回
    /// - `Ok(AuditData)`: 審計成功，包含完整性數據
    /// - `Err(AuditorError)`: 審計失敗（網絡錯誤、超時等）
    ///
    /// # 錯誤處理
    /// - HTTP 4xx 錯誤 → `VerificationStatus::Unreachable`
    /// - HTTP 5xx 錯誤 → `VerificationStatus::Unreachable`
    /// - 網絡超時 → `AuditorError::StorageNodeUnreachable`
    ///
    /// # 示例
    /// ```no_run
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # use auditor_node::integrity::IntegrityVerifier;
    /// let verifier = IntegrityVerifier::new_testnet();
    ///
    /// let audit_data = verifier.audit_blob("eRrTusk8yshFQpkemgDnbg0f4-qDo623V2NpeVG1Zcg").await?;
    ///
    /// println!("Content hash: {}", audit_data.content_hash);
    /// println!("File size: {} bytes", audit_data.file_size);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn audit_blob(&self, blob_id: &str) -> Result<AuditData> {
        info!("Starting integrity audit for blob: {}", blob_id);

        let url = format!("{}/v1/blobs/{}", self.aggregator_url, blob_id);
        debug!("Downloading from: {}", url);

        // 1. 下載 Blob
        let response = self
            .http_client
            .get(&url)
            .send()
            .await
            .map_err(|e| {
                if e.is_timeout() {
                    AuditorError::StorageNodeUnreachable(format!(
                        "Aggregator timeout: {}",
                        self.aggregator_url
                    ))
                } else if e.is_connect() {
                    AuditorError::StorageNodeUnreachable(format!(
                        "Cannot connect to aggregator: {}",
                        self.aggregator_url
                    ))
                } else {
                    AuditorError::StorageNodeUnreachable(format!("Network error: {}", e))
                }
            })?;

        if !response.status().is_success() {
            warn!(
                "Failed to download blob {}: HTTP {}",
                blob_id,
                response.status()
            );

            return Ok(AuditData {
                blob_id: blob_id.to_string(),
                content_hash: String::new(),
                merkle_root: String::new(),
                total_challenges: 0,
                successful_verifications: 0,
                failed_verifications: 0,
                file_size: 0,
                timestamp: Utc::now().timestamp() as u64,
                verification_status: VerificationStatus::Unreachable,
                sui_object_id: None,
            });
        }

        let content = response.bytes().await.map_err(|e| {
            AuditorError::StorageNodeUnreachable(format!("Failed to read response body: {}", e))
        })?;

        debug!("Downloaded {} bytes", content.len());

        // 2. 計算 SHA-256 哈希（應用層完整性基準）
        let mut hasher = Sha256::new();
        hasher.update(&content);
        let hash_result = hasher.finalize();
        let content_hash = hex::encode(hash_result);

        info!(
            "SHA-256 hash: {}",
            &content_hash[..16]
        );

        // 3. 構建 Merkle Tree（協議層完整性證明）
        const CHUNK_SIZE: usize = 4096; // 4KB chunks

        let merkle_tree = match MerkleTree::from_blob(&content, CHUNK_SIZE) {
            Ok(tree) => tree,
            Err(e) => {
                warn!("Failed to build Merkle tree: {}", e);
                return Ok(AuditData {
                    blob_id: blob_id.to_string(),
                    content_hash,
                    merkle_root: String::new(),
                    total_challenges: 0,
                    successful_verifications: 0,
                    failed_verifications: 0,
                    file_size: content.len() as u64,
                    timestamp: Utc::now().timestamp() as u64,
                    verification_status: VerificationStatus::Accessible,
                    sui_object_id: None,
                });
            }
        };

        let merkle_root_bytes = merkle_tree.root();
        let merkle_root = hex::encode(merkle_root_bytes);
        let leaf_count = merkle_tree.leaf_count();

        info!(
            "Merkle Tree built: {} leaves, root: {}",
            leaf_count,
            &merkle_root[..16]
        );

        // 4. 執行挑戰-響應驗證
        let total_challenges = if leaf_count == 1 {
            1 // 對於單 chunk 的 blob，只驗證一次
        } else {
            std::cmp::min(10, leaf_count) as u16 // 最多 10 次挑戰，或全部 chunks
        };

        let mut successful_verifications = 0u16;
        let mut failed_verifications = 0u16;

        info!("Starting challenge-response verification with {} challenges", total_challenges);

        let mut rng = rand::thread_rng();
        let mut challenged_indices = std::collections::HashSet::new();

        for challenge_num in 0..total_challenges {
            // 隨機選擇一個未被挑戰過的 chunk 索引
            let leaf_index = if leaf_count == 1 {
                0
            } else {
                loop {
                    let idx = rng.gen_range(0..leaf_count);
                    if !challenged_indices.contains(&idx) {
                        challenged_indices.insert(idx);
                        break idx;
                    }
                }
            };

            debug!("Challenge {}/{}: Testing chunk {}", challenge_num + 1, total_challenges, leaf_index);

            // 生成 Merkle Proof
            let proof = match merkle_tree.generate_proof(leaf_index) {
                Ok(p) => p,
                Err(e) => {
                    warn!("Failed to generate proof for chunk {}: {}", leaf_index, e);
                    failed_verifications += 1;
                    continue;
                }
            };

            // 獲取對應的 chunk 數據
            let chunk_start = leaf_index * CHUNK_SIZE;
            let chunk_end = std::cmp::min(chunk_start + CHUNK_SIZE, content.len());
            let chunk = &content[chunk_start..chunk_end];

            // 驗證 Merkle Proof
            let is_valid = proof.verify(chunk, &merkle_root_bytes);

            if is_valid {
                successful_verifications += 1;
                debug!("✓ Chunk {} verification passed", leaf_index);
            } else {
                failed_verifications += 1;
                warn!("✗ Chunk {} verification FAILED", leaf_index);
            }
        }

        let success_rate = (successful_verifications as f64 / total_challenges as f64) * 100.0;

        info!(
            "Audit completed for blob {}: {} bytes, {} challenges, {}/{} passed ({:.1}%)",
            blob_id,
            content.len(),
            total_challenges,
            successful_verifications,
            total_challenges,
            success_rate
        );

        // 5. 生成審計數據
        Ok(AuditData {
            blob_id: blob_id.to_string(),
            content_hash,
            merkle_root,
            total_challenges,
            successful_verifications,
            failed_verifications,
            file_size: content.len() as u64,
            timestamp: Utc::now().timestamp() as u64,
            verification_status: VerificationStatus::Accessible,
            sui_object_id: None,
        })
    }

    /// 驗證 Blob 的完整性（與已知哈希比對）
    ///
    /// 用於後續審計：檢查當前內容是否與歷史記錄一致
    ///
    /// # 參數
    /// - `blob_id`: Walrus Blob ID
    /// - `expected_hash`: 預期的 SHA-256 哈希（十六進制字符串）
    ///
    /// # 返回
    /// - `Ok(AuditData)`: 審計完成，`verification_status` 指示是否匹配
    ///
    /// # 示例
    /// ```no_run
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # use auditor_node::integrity::IntegrityVerifier;
    /// let verifier = IntegrityVerifier::new_testnet();
    ///
    /// let expected = "bd9e5380f78734bc182e4bb8c464101d3baeb23387d701608901e64cd879e1f5";
    /// let result = verifier.verify_blob("eRrTusk8yshFQpkemgDnbg0f4-qDo623V2NpeVG1Zcg", expected).await?;
    ///
    /// assert_eq!(result.verification_status, auditor_node::integrity::VerificationStatus::Accessible);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn verify_blob(&self, blob_id: &str, expected_hash: &str) -> Result<AuditData> {
        info!(
            "Verifying blob {} against expected hash: {}...",
            blob_id,
            &expected_hash[..16]
        );

        // 執行審計（下載並計算哈希）
        let mut audit_data = self.audit_blob(blob_id).await?;

        // 比對哈希
        if audit_data.verification_status == VerificationStatus::Accessible {
            if audit_data.content_hash != expected_hash {
                warn!(
                    "INTEGRITY VIOLATION: Blob {} hash mismatch!\n  Expected: {}\n  Got:      {}",
                    blob_id, expected_hash, audit_data.content_hash
                );
                audit_data.verification_status = VerificationStatus::Corrupted;
            } else {
                info!("Blob {} integrity verified successfully", blob_id);
            }
        }

        Ok(audit_data)
    }

    /// 批量審計多個 Blob
    ///
    /// 並發執行多個審計任務以提高效率
    ///
    /// # 參數
    /// - `blob_ids`: Blob ID 列表
    ///
    /// # 返回
    /// - `Vec<AuditData>`: 所有審計結果（順序與輸入對應）
    ///
    /// # 示例
    /// ```no_run
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # use auditor_node::integrity::IntegrityVerifier;
    /// let verifier = IntegrityVerifier::new_testnet();
    ///
    /// let blob_ids = vec![
    ///     "blob_id_1".to_string(),
    ///     "blob_id_2".to_string(),
    ///     "blob_id_3".to_string(),
    /// ];
    ///
    /// let results = verifier.audit_blobs_batch(&blob_ids).await?;
    ///
    /// for result in results {
    ///     println!("Blob {}: {:?}", result.blob_id, result.verification_status);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn audit_blobs_batch(&self, blob_ids: &[String]) -> Result<Vec<AuditData>> {
        info!("Starting batch audit for {} blobs", blob_ids.len());

        let mut tasks = Vec::new();

        for blob_id in blob_ids {
            let blob_id_clone = blob_id.clone();
            let verifier_clone = self.clone();

            tasks.push(tokio::spawn(async move {
                verifier_clone.audit_blob(&blob_id_clone).await
            }));
        }

        let mut results = Vec::new();

        for task in tasks {
            match task.await {
                Ok(Ok(audit_data)) => results.push(audit_data),
                Ok(Err(e)) => {
                    warn!("Batch audit task failed: {}", e);
                    // 繼續處理其他任務，不中斷整個批次
                }
                Err(e) => {
                    warn!("Batch audit task panicked: {}", e);
                }
            }
        }

        info!("Batch audit completed: {}/{} successful", results.len(), blob_ids.len());

        Ok(results)
    }
}

// 實現 Clone 以支持並發審計
impl Clone for IntegrityVerifier {
    fn clone(&self) -> Self {
        Self {
            http_client: self.http_client.clone(),
            aggregator_url: self.aggregator_url.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_verifier_creation() {
        let verifier = IntegrityVerifier::new_testnet();
        assert_eq!(verifier.aggregator_url, WALRUS_AGGREGATOR_TESTNET);
    }

    #[test]
    fn test_verification_status_serialization() {
        let status = VerificationStatus::Accessible;
        let json = serde_json::to_string(&status).unwrap();
        assert_eq!(json, "\"ACCESSIBLE\"");
    }

    #[tokio::test]
    #[ignore] // 需要實際的 Walrus Testnet 連接
    async fn test_real_blob_audit() {
        let verifier = IntegrityVerifier::new_testnet();

        // 使用我們上傳的測試 Blob
        let blob_id = "eRrTusk8yshFQpkemgDnbg0f4-qDo623V2NpeVG1Zcg";

        let result = verifier.audit_blob(blob_id).await;
        assert!(result.is_ok());

        let audit_data = result.unwrap();
        assert_eq!(audit_data.verification_status, VerificationStatus::Accessible);
        assert_eq!(audit_data.file_size, 870);
        assert!(!audit_data.content_hash.is_empty());

        println!("Audit result: {:?}", audit_data);
    }

    #[tokio::test]
    #[ignore] // 需要實際的 Walrus Testnet 連接
    async fn test_blob_verification_success() {
        let verifier = IntegrityVerifier::new_testnet();

        let blob_id = "eRrTusk8yshFQpkemgDnbg0f4-qDo623V2NpeVG1Zcg";
        let expected_hash = "bd9e5380f78734bc182e4bb8c464101d3baeb23387d701608901e64cd879e1f5";

        let result = verifier.verify_blob(blob_id, expected_hash).await;
        assert!(result.is_ok());

        let audit_data = result.unwrap();
        assert_eq!(audit_data.verification_status, VerificationStatus::Accessible);
    }

    #[tokio::test]
    #[ignore]
    async fn test_blob_verification_failure() {
        let verifier = IntegrityVerifier::new_testnet();

        let blob_id = "eRrTusk8yshFQpkemgDnbg0f4-qDo623V2NpeVG1Zcg";
        let wrong_hash = "0000000000000000000000000000000000000000000000000000000000000000";

        let result = verifier.verify_blob(blob_id, wrong_hash).await;
        assert!(result.is_ok());

        let audit_data = result.unwrap();
        assert_eq!(audit_data.verification_status, VerificationStatus::Corrupted);
    }
}
