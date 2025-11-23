//! Sliver 解析與驗證模塊
//!
//! 負責處理 Walrus 的 Sliver 數據結構和驗證邏輯。
//!
//! # Erasure Coding 基本原理
//!
//! Walrus 使用 Reed-Solomon 糾刪碼將 Blob 編碼為多個 Slivers：
//!
//! 1. **編碼過程**：
//!    - 將原始 Blob 分割為 k 個數據片段
//!    - 使用 Reed-Solomon 算法生成 n-k 個冗餘片段
//!    - 總共產生 n 個 Slivers（n > k）
//!    - 只需任意 k 個 Slivers 即可恢復原始 Blob
//!
//! 2. **冗餘度**：
//!    - 冗餘因子 = n/k（例如 15/10 = 1.5x）
//!    - 可容忍 n-k 個 Slivers 丟失或損壞
//!    - 更高的冗餘度提供更強的容錯能力
//!
//! 3. **安全保證**：
//!    - 每個 Sliver 的哈希值存儲在默克爾樹中
//!    - 默克爾根固定在 Sui 區塊鏈上
//!    - 存儲節點無法偽造 Sliver 數據
//!
//! # 審計流程
//!
//! 1. 審計員隨機選擇 Sliver 索引
//! 2. 向存儲節點發送挑戰（請求該 Sliver）
//! 3. 存儲節點返回 Sliver 數據 + 默克爾證明
//! 4. 審計員驗證：
//!    - 計算 sliver_hash = SHA3-256(sliver_data)
//!    - 使用默克爾證明驗證 sliver_hash 在默克爾樹中
//!    - 驗證計算的默克爾根與鏈上記錄的根匹配
//! 5. 如果驗證通過，證明該 Sliver 完整且未被篡改
//!
//! # 與默克爾樹驗證的關係
//!
//! - Sliver 驗證依賴於默克爾證明
//! - 默克爾樹的葉子節點 = SHA3-256(sliver_data)
//! - 默克爾根存儲在 Sui 區塊鏈的 Blob 對象中
//! - 這提供了從鏈上到存儲層的完整信任鏈

use crate::crypto::merkle::{MerkleProof, MerkleRoot};
use crate::error::{AuditorError, Result};
use serde::{Deserialize, Serialize};
use sha3::{Digest, Sha3_256};
use tracing::{debug, error, info, warn};

/// Sliver 元數據（從 BlobMetadata 中提取）
///
/// 包含驗證 Sliver 所需的上下文信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SliverMetadata {
    /// 該 Blob 的默克爾根（存儲在 Sui 區塊鏈上）
    pub merkle_root: MerkleRoot,

    /// Blob 的總 Sliver 數量
    /// 對於 (k=10, n=15) 的編碼，total_slivers = 15
    pub total_slivers: u64,

    /// Erasure coding 參數 (k, n)
    /// - k = 原始數據片段數（恢復所需的最小片段數）
    /// - n = 總編碼片段數（n > k，提供冗餘）
    /// 例如：(10, 15) 表示 10 個數據片段 + 5 個冗餘片段
    pub erasure_params: (usize, usize),
}

impl SliverMetadata {
    /// 創建新的 Sliver 元數據
    pub fn new(merkle_root: MerkleRoot, total_slivers: u64, k: usize, n: usize) -> Result<Self> {
        validate_erasure_params(k, n)?;

        if total_slivers as usize != n {
            error!(
                "Total slivers ({}) does not match erasure coding n parameter ({})",
                total_slivers, n
            );
            return Err(AuditorError::InvalidSliver(format!(
                "total_slivers ({}) must equal n ({})",
                total_slivers, n
            )));
        }

        Ok(Self {
            merkle_root,
            total_slivers,
            erasure_params: (k, n),
        })
    }

    /// 獲取數據片段數（k）
    pub fn k(&self) -> usize {
        self.erasure_params.0
    }

    /// 獲取總片段數（n）
    pub fn n(&self) -> usize {
        self.erasure_params.1
    }

    /// 獲取冗餘片段數（n - k）
    pub fn redundancy_count(&self) -> usize {
        self.n() - self.k()
    }

    /// 獲取冗餘因子（n / k）
    pub fn redundancy_factor(&self) -> f64 {
        self.n() as f64 / self.k() as f64
    }

    /// 檢查給定的 Sliver 索引是否有效
    pub fn is_valid_index(&self, index: u64) -> bool {
        index < self.total_slivers
    }
}

/// Sliver 數據（從存儲節點接收）
///
/// 代表 Blob 的一個編碼片段
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Sliver {
    /// Sliver 索引（0 到 n-1）
    pub index: u64,

    /// Sliver 原始字節數據（經過 Erasure Coding 編碼的片段）
    pub data: Vec<u8>,
}

impl Sliver {
    /// 創建新的 Sliver
    pub fn new(index: u64, data: Vec<u8>) -> Self {
        Self { index, data }
    }

    /// 驗證 Sliver 的完整性
    ///
    /// # 驗證邏輯
    ///
    /// 1. 檢查 Sliver 索引是否在有效範圍內
    /// 2. 計算 sliver_hash = SHA3-256(sliver.data)
    /// 3. 使用 merkle_proof 驗證 sliver_hash 確實在默克爾樹中
    /// 4. 驗證計算出的默克爾根與 metadata.merkle_root 匹配
    ///
    /// # 參數
    /// - `metadata`: Blob 的元數據（包含默克爾根和 Erasure Coding 參數）
    /// - `merkle_proof`: 從存儲節點獲取的默克爾證明
    ///
    /// # 返回
    /// - `Ok(true)`: 驗證通過，Sliver 完整且未被篡改
    /// - `Ok(false)`: 驗證失敗，Sliver 可能損壞或被篡改
    /// - `Err(_)`: 驗證過程中出現錯誤
    ///
    /// # 示例
    /// ```no_run
    /// # use auditor_node::crypto::sliver::{Sliver, SliverMetadata};
    /// # use auditor_node::crypto::merkle::MerkleProof;
    /// # fn example(sliver: Sliver, metadata: SliverMetadata, proof: MerkleProof) -> Result<(), Box<dyn std::error::Error>> {
    /// let is_valid = sliver.verify(&metadata, &proof)?;
    /// if is_valid {
    ///     println!("Sliver verification passed!");
    /// } else {
    ///     println!("Sliver verification failed!");
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn verify(&self, metadata: &SliverMetadata, merkle_proof: &MerkleProof) -> Result<bool> {
        debug!(
            "Verifying sliver {} with {} bytes (erasure params: {:?})",
            self.index,
            self.data.len(),
            metadata.erasure_params
        );

        // 1. 檢查索引有效性
        if !metadata.is_valid_index(self.index) {
            error!(
                "Sliver index {} out of range (total slivers: {})",
                self.index, metadata.total_slivers
            );
            return Err(AuditorError::InvalidSliver(format!(
                "index {} >= total_slivers {}",
                self.index, metadata.total_slivers
            )));
        }

        // 2. 檢查數據非空
        if self.data.is_empty() {
            error!("Sliver {} has empty data", self.index);
            return Err(AuditorError::InvalidSliver(
                "empty sliver data".to_string(),
            ));
        }

        // 3. 計算 Sliver 的哈希值
        let sliver_hash = self.compute_hash();
        debug!(
            "Sliver {} hash: {:02x?}",
            self.index,
            &sliver_hash[..8] // 只打印前 8 字節
        );

        // 4. 使用默克爾證明驗證
        // 注意：MerkleProof.verify() 內部會計算哈希並驗證整個路徑
        let verified = merkle_proof.verify(&self.data, &metadata.merkle_root);

        if verified {
            info!(
                "Sliver {} verification PASSED (data size: {} bytes)",
                self.index,
                self.data.len()
            );
        } else {
            warn!(
                "Sliver {} verification FAILED (expected root: {:02x?})",
                self.index,
                &metadata.merkle_root[..8]
            );
        }

        Ok(verified)
    }

    /// 計算 Sliver 數據的 SHA3-256 哈希
    ///
    /// 這個哈希值應該是默克爾樹的葉子節點
    pub fn compute_hash(&self) -> [u8; 32] {
        let mut hasher = Sha3_256::new();
        hasher.update(&self.data);
        hasher.finalize().into()
    }

    /// 從存儲節點響應中解析 Sliver
    ///
    /// # 參數
    /// - `index`: Sliver 索引
    /// - `data`: Sliver 原始字節數據
    ///
    /// # 返回
    /// - `Ok(Sliver)`: 成功解析
    /// - `Err(AuditorError)`: 數據無效
    pub fn from_response_bytes(index: u64, data: Vec<u8>) -> Result<Self> {
        if data.is_empty() {
            error!("Received empty sliver data for index {}", index);
            return Err(AuditorError::InvalidSliver(
                "empty sliver data".to_string(),
            ));
        }

        debug!(
            "Parsed sliver {} from response ({} bytes)",
            index,
            data.len()
        );

        Ok(Self { index, data })
    }

    /// 獲取 Sliver 數據的長度
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// 檢查 Sliver 數據是否為空
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }
}

/// Erasure Coding 參數驗證
///
/// 檢查 (k, n) 參數是否符合以下規則：
/// - k >= 1（至少有一個原始片段）
/// - n > k（必須有冗餘編碼，否則無法提供容錯）
/// - n <= 1000（合理的上限，避免過度冗餘）
/// - k <= n（邏輯一致性）
///
/// # 參數
/// - `k`: 數據片段數（恢復所需的最小片段數）
/// - `n`: 總片段數（包含數據片段 + 冗餘片段）
///
/// # 示例
/// ```
/// use auditor_node::crypto::sliver::validate_erasure_params;
///
/// // 有效參數
/// assert!(validate_erasure_params(10, 15).is_ok());
///
/// // 無效參數
/// assert!(validate_erasure_params(0, 10).is_err());  // k 不能為 0
/// assert!(validate_erasure_params(10, 5).is_err());  // n 必須大於 k
/// assert!(validate_erasure_params(10, 2000).is_err()); // n 過大
/// ```
pub fn validate_erasure_params(k: usize, n: usize) -> Result<()> {
    if k == 0 {
        error!("Erasure coding k parameter cannot be 0");
        return Err(AuditorError::InvalidSliver(
            "k must be at least 1".to_string(),
        ));
    }

    if n <= k {
        error!(
            "Erasure coding n ({}) must be greater than k ({})",
            n, k
        );
        return Err(AuditorError::InvalidSliver(format!(
            "n ({}) must be > k ({})",
            n, k
        )));
    }

    if n > 1000 {
        error!("Erasure coding n parameter ({}) exceeds maximum (1000)", n);
        return Err(AuditorError::InvalidSliver(format!(
            "n ({}) exceeds maximum (1000)",
            n
        )));
    }

    debug!("Erasure coding parameters validated: k={}, n={}", k, n);
    Ok(())
}

/// 計算建議的挑戰數量
///
/// 基於 Erasure Coding 參數和期望的置信度，計算審計時應該挑戰的 Sliver 數量。
///
/// # 統計原理
///
/// 假設存儲節點損壞了 p% 的 Slivers：
/// - 單次挑戰檢測到損壞的概率 = p
/// - c 次挑戰都未檢測到的概率 = (1-p)^c
/// - 檢測到至少一次損壞的概率（置信度）= 1 - (1-p)^c
///
/// 例如：
/// - p = 10%（10% Sliver 損壞）
/// - c = 20 次挑戰
/// - 置信度 = 1 - 0.9^20 ≈ 87.8%
///
/// # 參數
/// - `total_slivers`: 總 Sliver 數量
/// - `confidence_level`: 期望的置信度（0.0 到 1.0）
/// - `assumed_corruption_rate`: 假設的損壞率（0.0 到 1.0）
///
/// # 返回
/// 建議的挑戰次數
pub fn calculate_challenge_count(
    total_slivers: u64,
    confidence_level: f64,
    assumed_corruption_rate: f64,
) -> u64 {
    if assumed_corruption_rate <= 0.0 || assumed_corruption_rate >= 1.0 {
        warn!("Invalid corruption rate {}, using default 0.1", assumed_corruption_rate);
        return (total_slivers as f64 * 0.1).ceil() as u64;
    }

    if confidence_level <= 0.0 || confidence_level >= 1.0 {
        warn!("Invalid confidence level {}, using default 0.95", confidence_level);
        return (total_slivers as f64 * 0.1).ceil() as u64;
    }

    // c = log(1 - confidence) / log(1 - p)
    let challenge_count =
        (1.0 - confidence_level).ln() / (1.0 - assumed_corruption_rate).ln();

    let result = challenge_count.ceil() as u64;

    // 限制在合理範圍內
    let min_challenges = 1;
    let max_challenges = total_slivers;

    result.max(min_challenges).min(max_challenges)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_erasure_params_validation() {
        // 有效參數
        assert!(validate_erasure_params(5, 10).is_ok());
        assert!(validate_erasure_params(10, 15).is_ok());
        assert!(validate_erasure_params(1, 2).is_ok());

        // 無效參數
        assert!(validate_erasure_params(0, 10).is_err()); // k = 0
        assert!(validate_erasure_params(10, 5).is_err()); // n < k
        assert!(validate_erasure_params(10, 10).is_err()); // n = k (無冗餘)
        assert!(validate_erasure_params(10, 2000).is_err()); // n 過大
    }

    #[test]
    fn test_sliver_metadata_creation() {
        let merkle_root = [0u8; 32];
        let metadata = SliverMetadata::new(merkle_root, 15, 10, 15);
        assert!(metadata.is_ok());

        let metadata = metadata.unwrap();
        assert_eq!(metadata.k(), 10);
        assert_eq!(metadata.n(), 15);
        assert_eq!(metadata.redundancy_count(), 5);
        assert_eq!(metadata.redundancy_factor(), 1.5);
    }

    #[test]
    fn test_sliver_metadata_invalid_params() {
        let merkle_root = [0u8; 32];

        // total_slivers 不匹配 n
        let result = SliverMetadata::new(merkle_root, 10, 10, 15);
        assert!(result.is_err());

        // 無效的 erasure params
        let result = SliverMetadata::new(merkle_root, 10, 10, 10);
        assert!(result.is_err());
    }

    #[test]
    fn test_sliver_index_validation() {
        let merkle_root = [0u8; 32];
        let metadata = SliverMetadata::new(merkle_root, 15, 10, 15).unwrap();

        assert!(metadata.is_valid_index(0));
        assert!(metadata.is_valid_index(14));
        assert!(!metadata.is_valid_index(15));
        assert!(!metadata.is_valid_index(100));
    }

    #[test]
    fn test_sliver_from_response() {
        let sliver = Sliver::from_response_bytes(0, vec![1, 2, 3, 4]);
        assert!(sliver.is_ok());

        let sliver = sliver.unwrap();
        assert_eq!(sliver.index, 0);
        assert_eq!(sliver.len(), 4);
        assert!(!sliver.is_empty());
    }

    #[test]
    fn test_sliver_from_empty_response() {
        let result = Sliver::from_response_bytes(0, vec![]);
        assert!(result.is_err());
    }

    #[test]
    fn test_sliver_hash_computation() {
        let sliver = Sliver::new(0, vec![1, 2, 3, 4]);
        let hash = sliver.compute_hash();

        // SHA3-256 總是產生 32 字節
        assert_eq!(hash.len(), 32);

        // 相同數據應該產生相同哈希
        let sliver2 = Sliver::new(0, vec![1, 2, 3, 4]);
        let hash2 = sliver2.compute_hash();
        assert_eq!(hash, hash2);

        // 不同數據應該產生不同哈希
        let sliver3 = Sliver::new(0, vec![5, 6, 7, 8]);
        let hash3 = sliver3.compute_hash();
        assert_ne!(hash, hash3);
    }

    #[test]
    fn test_calculate_challenge_count() {
        // 測試基本計算
        let count = calculate_challenge_count(100, 0.95, 0.1);
        assert!(count > 0);
        assert!(count <= 100);

        // 更高的置信度需要更多挑戰
        let count_high = calculate_challenge_count(100, 0.99, 0.1);
        let count_low = calculate_challenge_count(100, 0.90, 0.1);
        assert!(count_high > count_low);

        // 更高的損壞率需要更少挑戰（更容易檢測到）
        let count_high_corruption = calculate_challenge_count(100, 0.95, 0.2);
        let count_low_corruption = calculate_challenge_count(100, 0.95, 0.05);
        assert!(count_high_corruption < count_low_corruption);
    }

    #[test]
    fn test_calculate_challenge_count_edge_cases() {
        // 無效參數應該返回默認值
        let count = calculate_challenge_count(100, 1.5, 0.1); // 無效置信度
        assert!(count > 0);

        let count = calculate_challenge_count(100, 0.95, -0.1); // 無效損壞率
        assert!(count > 0);
    }

    #[test]
    fn test_sliver_verify_invalid_index() {
        let merkle_root = [0u8; 32];
        let metadata = SliverMetadata::new(merkle_root, 10, 5, 10).unwrap();

        // 索引超出範圍
        let sliver = Sliver::new(15, vec![1, 2, 3, 4]);
        let proof = MerkleProof::new(vec![], 15);

        let result = sliver.verify(&metadata, &proof);
        assert!(result.is_err());
    }

    #[test]
    fn test_sliver_verify_empty_data() {
        let merkle_root = [0u8; 32];
        let metadata = SliverMetadata::new(merkle_root, 10, 5, 10).unwrap();

        // 空數據
        let sliver = Sliver::new(0, vec![]);
        let proof = MerkleProof::new(vec![], 0);

        let result = sliver.verify(&metadata, &proof);
        assert!(result.is_err());
    }
}
