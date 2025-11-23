//! 與 Walrus 兼容的默克爾樹驗證模塊
//!
//! # 關於默克爾樹在 Walrus 中的應用
//!
//! Walrus 使用默克爾樹來組織 blob 的 slivers（碎片）:
//! - 每個 sliver 是葉子節點
//! - 默克爾根存儲在區塊鏈上,作為數據完整性的承諾
//! - 審計時,存儲節點提供 sliver 數據 + 默克爾證明
//! - 審計節點驗證證明,無需下載完整 blob
//!
//! # 哈希函數選擇
//!
//! 使用 **Blake2b-256** 與 Walrus 保持一致:
//! - 快速且安全的現代哈希算法
//! - 256 位輸出,與區塊鏈哈希標準對齊
//! - 葉子節點使用 LEAF_PREFIX=[0]，內部節點使用 INNER_PREFIX=[1]
//! - 參考 Walrus 官方 merkle.rs 實現
//!
//! # 驗證算法
//!
//! 默克爾證明驗證步驟:
//! 1. 計算葉子哈希: `current = H(leaf_data)`
//! 2. 遍歷證明路徑,逐層向上計算:
//!    - 如果當前索引第 i 位是 0,當前節點在左邊: `current = H(current || sibling)`
//!    - 如果是 1,當前節點在右邊: `current = H(sibling || current)`
//!    - 索引右移一位: `index >>= 1`
//! 3. 比較最終計算出的根與提供的根是否相等

use serde::{Deserialize, Serialize};
use fastcrypto::hash::{Blake2b256, HashFunction};

/// Walrus 官方默克爾樹哈希前綴常量
///
/// 根據 Walrus 官方 merkle.rs 定義：
/// - LEAF_PREFIX: 用於葉子節點哈希
/// - INNER_PREFIX: 用於內部節點哈希
///
/// 這些前綴確保不同層級的節點哈希無法互相碰撞
const LEAF_PREFIX: [u8; 1] = [0];
const INNER_PREFIX: [u8; 1] = [1];

/// 默克爾證明路徑
///
/// # 示例
///
/// ```
/// use auditor_node::crypto::merkle::{MerkleProof, hash_leaf};
///
/// let proof = MerkleProof {
///     path: vec![
///         [1u8; 32],  // 第一層兄弟節點
///         [2u8; 32],  // 第二層兄弟節點
///     ],
///     leaf_index: 0,
/// };
///
/// let leaf_data = b"slicer data from storage node";
/// let leaf_hash = hash_leaf(leaf_data);
/// // ... 驗證邏輯
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MerkleProof {
    /// 從葉子到根的證明路徑（兄弟節點哈希）
    ///
    /// path[0] 是最底層（葉子的兄弟節點）
    /// path[n-1] 是最頂層（根的子節點的兄弟）
    pub path: Vec<[u8; 32]>,

    /// 葉子索引（用於確定左右位置）
    ///
    /// 索引從 0 開始,用二進制表示路徑:
    /// - 第 i 位為 0: 當前節點在左邊
    /// - 第 i 位為 1: 當前節點在右邊
    pub leaf_index: u64,
}

/// 默克爾根類型別名
pub type MerkleRoot = [u8; 32];

impl MerkleProof {
    /// 驗證葉子節點確實屬於該默克爾樹
    ///
    /// # 參數
    /// - `leaf_data`: 原始葉子數據（例如 Walrus sliver 數據）
    /// - `root`: 聲稱的默克爾根（從區塊鏈獲取）
    ///
    /// # 返回
    /// - `true`: 驗證通過,葉子數據確實屬於該樹
    /// - `false`: 驗證失敗,數據被篡改或證明無效
    ///
    /// # 示例
    ///
    /// ```
    /// use auditor_node::crypto::merkle::{MerkleProof, hash_leaf, hash_node};
    ///
    /// // 構建一個簡單的默克爾樹（4 個葉子）
    /// let leaves = vec![b"leaf0", b"leaf1", b"leaf2", b"leaf3"];
    /// let leaf_hashes: Vec<[u8; 32]> = leaves.iter().map(|l| hash_leaf(l)).collect();
    ///
    /// // 構建樹: (((L0, L1), (L2, L3)))
    /// let node01 = hash_node(&leaf_hashes[0], &leaf_hashes[1]);
    /// let node23 = hash_node(&leaf_hashes[2], &leaf_hashes[3]);
    /// let root = hash_node(&node01, &node23);
    ///
    /// // 證明 leaf0 屬於該樹
    /// let proof = MerkleProof {
    ///     path: vec![leaf_hashes[1], node23],  // 兄弟節點: L1 和 (L2,L3)
    ///     leaf_index: 0,  // leaf0 的索引
    /// };
    ///
    /// assert!(proof.verify(b"leaf0", &root));
    /// assert!(!proof.verify(b"wrong_data", &root));
    /// ```
    pub fn verify(&self, leaf_data: &[u8], root: &MerkleRoot) -> bool {
        // 1. 計算葉子節點哈希
        let mut current_hash = hash_leaf(leaf_data);

        // 2. 使用證明路徑逐層向上計算
        let mut index = self.leaf_index;

        for sibling in &self.path {
            // 根據索引的二進制位確定當前節點位置
            if index & 1 == 0 {
                // 當前節點在左邊
                current_hash = hash_node(&current_hash, sibling);
            } else {
                // 當前節點在右邊
                current_hash = hash_node(sibling, &current_hash);
            }

            // 移到父節點層
            index >>= 1;
        }

        // 3. 比較計算出的根與提供的根
        &current_hash == root
    }

    /// 從字節反序列化證明
    ///
    /// # 格式
    /// 使用 `bincode` 序列化格式,與 Rust 生態兼容
    ///
    /// # 錯誤
    /// - 如果字節格式無效,返回 `Deserialization` 錯誤
    ///
    /// # 示例
    ///
    /// ```
    /// use auditor_node::crypto::merkle::MerkleProof;
    ///
    /// let original = MerkleProof {
    ///     path: vec![[1u8; 32], [2u8; 32]],
    ///     leaf_index: 5,
    /// };
    ///
    /// let bytes = original.to_bytes();
    /// let restored = MerkleProof::from_bytes(&bytes).unwrap();
    ///
    /// assert_eq!(original, restored);
    /// ```
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, MerkleError> {
        bincode::deserialize(bytes).map_err(|e| {
            MerkleError::Deserialization(format!("Failed to deserialize MerkleProof: {}", e))
        })
    }

    /// 序列化證明為字節
    ///
    /// # 返回
    /// 二進制表示的證明數據,可以通過 `from_bytes` 恢復
    ///
    /// # 示例
    ///
    /// ```
    /// use auditor_node::crypto::merkle::MerkleProof;
    ///
    /// let proof = MerkleProof {
    ///     path: vec![[0xAAu8; 32]],
    ///     leaf_index: 3,
    /// };
    ///
    /// let bytes = proof.to_bytes();
    /// assert!(bytes.len() > 0);
    /// ```
    pub fn to_bytes(&self) -> Vec<u8> {
        bincode::serialize(self).expect("MerkleProof serialization should never fail")
    }

    /// 創建新的默克爾證明
    pub fn new(path: Vec<[u8; 32]>, leaf_index: u64) -> Self {
        Self { path, leaf_index }
    }

    /// 獲取證明深度（樹的高度）
    pub fn depth(&self) -> usize {
        self.path.len()
    }
}

/// 計算單個內部節點的哈希（Walrus 使用的方式）
///
/// # 參數
/// - `left`: 左子節點哈希
/// - `right`: 右子節點哈希
///
/// # 返回
/// `H(left || right)` 的結果
///
/// # 示例
///
/// ```
/// use auditor_node::crypto::merkle::hash_node;
///
/// let left = [0x01u8; 32];
/// let right = [0x02u8; 32];
/// let parent = hash_node(&left, &right);
///
/// // 驗證哈希長度
/// assert_eq!(parent.len(), 32);
/// ```
pub fn hash_node(left: &[u8; 32], right: &[u8; 32]) -> [u8; 32] {
    let mut hasher = Blake2b256::default();
    hasher.update(&INNER_PREFIX);
    hasher.update(left);
    hasher.update(right);
    hasher.finalize().digest
}

/// 計算葉子節點的哈希
///
/// # 參數
/// - `data`: 葉子節點的原始數據（例如 Walrus sliver 數據）
///
/// # 返回
/// `H(data)` 的結果
///
/// # 示例
///
/// ```
/// use auditor_node::crypto::merkle::hash_leaf;
///
/// let sliver_data = b"This is a storage sliver from Walrus node";
/// let leaf_hash = hash_leaf(sliver_data);
///
/// // 驗證哈希長度
/// assert_eq!(leaf_hash.len(), 32);
///
/// // 相同數據產生相同哈希
/// let hash2 = hash_leaf(sliver_data);
/// assert_eq!(leaf_hash, hash2);
/// ```
pub fn hash_leaf(data: &[u8]) -> [u8; 32] {
    let mut hasher = Blake2b256::default();
    hasher.update(&LEAF_PREFIX);
    hasher.update(data);
    hasher.finalize().digest
}

/// 默克爾樹相關錯誤
#[derive(Debug, thiserror::Error)]
pub enum MerkleError {
    /// 反序列化失敗
    #[error("Deserialization failed: {0}")]
    Deserialization(String),

    /// 證明格式無效
    #[error("Invalid proof format")]
    InvalidProof,

    /// 證明驗證失敗
    #[error("Proof verification failed")]
    VerificationFailed,

    /// 空數據無法構建樹
    #[error("Cannot build tree from empty data")]
    EmptyData,

    /// 無效的葉子索引
    #[error("Invalid leaf index: {index} (total leaves: {total})")]
    InvalidLeafIndex { index: usize, total: usize },
}

/// Merkle Tree 構建器
///
/// 用於從原始 blob 數據構建完整的 Merkle Tree
/// 並生成任意葉子的 Merkle Proof
#[derive(Debug, Clone)]
pub struct MerkleTree {
    /// 所有層級的節點（從底層葉子到根）
    /// layers[0] = 葉子層, layers[n-1] = 根
    layers: Vec<Vec<[u8; 32]>>,

    /// Merkle 根
    root: [u8; 32],

    /// 葉子總數
    leaf_count: usize,
}

impl MerkleTree {
    /// 從 blob 數據構建 Merkle Tree
    ///
    /// # 參數
    /// - `blob_data`: 原始 blob 數據
    /// - `chunk_size`: 切片大小（bytes），通常是 4096
    ///
    /// # 返回
    /// - `Ok(MerkleTree)`: 構建成功
    /// - `Err(MerkleError::EmptyData)`: 數據為空
    ///
    /// # 示例
    ///
    /// ```
    /// use auditor_node::crypto::merkle::MerkleTree;
    ///
    /// let blob_data = b"Hello Walrus!".repeat(100); // 1300 bytes
    /// let tree = MerkleTree::from_blob(&blob_data, 4096).unwrap();
    ///
    /// // 獲取 Merkle 根
    /// let root = tree.root();
    /// println!("Merkle Root: {:?}", hex::encode(root));
    ///
    /// // 生成葉子 0 的證明
    /// let proof = tree.generate_proof(0).unwrap();
    /// ```
    pub fn from_blob(blob_data: &[u8], chunk_size: usize) -> Result<Self, MerkleError> {
        if blob_data.is_empty() {
            return Err(MerkleError::EmptyData);
        }

        // 步驟 1: 將 blob 切成 chunks
        let chunks: Vec<&[u8]> = blob_data
            .chunks(chunk_size)
            .collect();

        let leaf_count = chunks.len();

        // 步驟 2: 計算葉子哈希
        let mut current_layer: Vec<[u8; 32]> = chunks
            .iter()
            .map(|chunk| hash_leaf(chunk))
            .collect();

        let mut layers = vec![current_layer.clone()];

        // 步驟 3: 逐層構建樹，直到根節點
        while current_layer.len() > 1 {
            let mut next_layer = Vec::new();

            // 如果當前層有奇數個節點，複製最後一個節點
            let mut i = 0;
            while i < current_layer.len() {
                if i + 1 < current_layer.len() {
                    // 正常配對
                    let left = current_layer[i];
                    let right = current_layer[i + 1];
                    next_layer.push(hash_node(&left, &right));
                    i += 2;
                } else {
                    // 奇數節點：與自己配對
                    let node = current_layer[i];
                    next_layer.push(hash_node(&node, &node));
                    i += 1;
                }
            }

            layers.push(next_layer.clone());
            current_layer = next_layer;
        }

        let root = current_layer[0];

        Ok(MerkleTree {
            layers,
            root,
            leaf_count,
        })
    }

    /// 獲取 Merkle 根
    pub fn root(&self) -> [u8; 32] {
        self.root
    }

    /// 獲取葉子總數
    pub fn leaf_count(&self) -> usize {
        self.leaf_count
    }

    /// 生成指定葉子的 Merkle Proof
    ///
    /// # 參數
    /// - `leaf_index`: 葉子索引（從 0 開始）
    ///
    /// # 返回
    /// - `Ok(MerkleProof)`: 證明生成成功
    /// - `Err(MerkleError::InvalidLeafIndex)`: 索引超出範圍
    ///
    /// # 示例
    ///
    /// ```
    /// use auditor_node::crypto::merkle::MerkleTree;
    ///
    /// let blob_data = b"Test data for Merkle Tree";
    /// let tree = MerkleTree::from_blob(blob_data, 4096).unwrap();
    ///
    /// // 生成葉子 0 的證明
    /// let proof = tree.generate_proof(0).unwrap();
    ///
    /// // 驗證證明
    /// let root = tree.root();
    /// assert!(proof.verify(blob_data, &root));
    /// ```
    pub fn generate_proof(&self, leaf_index: usize) -> Result<MerkleProof, MerkleError> {
        if leaf_index >= self.leaf_count {
            return Err(MerkleError::InvalidLeafIndex {
                index: leaf_index,
                total: self.leaf_count,
            });
        }

        let mut path = Vec::new();
        let mut current_index = leaf_index;

        // 從葉子層向上遍歷到根
        for layer in &self.layers[..self.layers.len() - 1] {
            // 計算兄弟節點索引
            let sibling_index = if current_index % 2 == 0 {
                current_index + 1
            } else {
                current_index - 1
            };

            // 如果兄弟節點存在，加入路徑
            if sibling_index < layer.len() {
                path.push(layer[sibling_index]);
            } else {
                // 如果是最後一個奇數節點，兄弟節點是自己
                path.push(layer[current_index]);
            }

            // 移動到父節點
            current_index /= 2;
        }

        Ok(MerkleProof {
            path,
            leaf_index: leaf_index as u64,
        })
    }

    /// 獲取所有葉子的哈希
    pub fn leaf_hashes(&self) -> &[[u8; 32]] {
        &self.layers[0]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 測試基本的哈希函數
    #[test]
    fn test_hash_functions() {
        let data = b"test data";
        let hash1 = hash_leaf(data);
        let hash2 = hash_leaf(data);

        // 相同輸入產生相同輸出
        assert_eq!(hash1, hash2);

        // 哈希長度為 32 字節
        assert_eq!(hash1.len(), 32);

        // 不同輸入產生不同輸出
        let hash3 = hash_leaf(b"different data");
        assert_ne!(hash1, hash3);
    }

    /// 測試節點哈希的順序敏感性
    #[test]
    fn test_hash_node_order_matters() {
        let left = [0x01u8; 32];
        let right = [0x02u8; 32];

        let hash_lr = hash_node(&left, &right);
        let hash_rl = hash_node(&right, &left);

        // H(left, right) ≠ H(right, left)
        assert_ne!(hash_lr, hash_rl);
    }

    /// 測試單葉子樹的驗證（邊界情況）
    #[test]
    fn test_single_leaf_tree() {
        let leaf_data = b"single leaf";
        let root = hash_leaf(leaf_data);

        // 空路徑表示單節點樹
        let proof = MerkleProof {
            path: vec![],
            leaf_index: 0,
        };

        assert!(proof.verify(leaf_data, &root));
        assert!(!proof.verify(b"wrong data", &root));
    }

    /// 測試兩葉子樹的驗證
    #[test]
    fn test_two_leaf_tree() {
        // 構建樹: (leaf0, leaf1)
        let leaf0 = b"leaf 0";
        let leaf1 = b"leaf 1";

        let hash0 = hash_leaf(leaf0);
        let hash1 = hash_leaf(leaf1);
        let root = hash_node(&hash0, &hash1);

        // 驗證 leaf0
        let proof0 = MerkleProof {
            path: vec![hash1], // 兄弟節點是 leaf1
            leaf_index: 0,
        };
        assert!(proof0.verify(leaf0, &root));

        // 驗證 leaf1
        let proof1 = MerkleProof {
            path: vec![hash0], // 兄弟節點是 leaf0
            leaf_index: 1,
        };
        assert!(proof1.verify(leaf1, &root));

        // 錯誤的數據應該驗證失敗
        assert!(!proof0.verify(b"wrong", &root));
    }

    /// 測試四葉子樹的完整驗證
    #[test]
    fn test_four_leaf_tree() {
        // 構建樹:
        //       root
        //      /    \
        //   node01  node23
        //   /  \    /  \
        //  L0  L1  L2  L3

        let leaves = [b"leaf0", b"leaf1", b"leaf2", b"leaf3"];
        let leaf_hashes: Vec<[u8; 32]> = leaves.iter().map(|l| hash_leaf(*l)).collect();

        let node01 = hash_node(&leaf_hashes[0], &leaf_hashes[1]);
        let node23 = hash_node(&leaf_hashes[2], &leaf_hashes[3]);
        let root = hash_node(&node01, &node23);

        // 驗證 leaf0 (索引 0 = 0b00)
        let proof0 = MerkleProof {
            path: vec![leaf_hashes[1], node23],
            leaf_index: 0,
        };
        assert!(proof0.verify(leaves[0], &root));

        // 驗證 leaf1 (索引 1 = 0b01)
        let proof1 = MerkleProof {
            path: vec![leaf_hashes[0], node23],
            leaf_index: 1,
        };
        assert!(proof1.verify(leaves[1], &root));

        // 驗證 leaf2 (索引 2 = 0b10)
        let proof2 = MerkleProof {
            path: vec![leaf_hashes[3], node01],
            leaf_index: 2,
        };
        assert!(proof2.verify(leaves[2], &root));

        // 驗證 leaf3 (索引 3 = 0b11)
        let proof3 = MerkleProof {
            path: vec![leaf_hashes[2], node01],
            leaf_index: 3,
        };
        assert!(proof3.verify(leaves[3], &root));
    }

    /// 測試篡改葉子數據導致驗證失敗
    #[test]
    fn test_tampered_leaf_fails() {
        let leaves = [b"leaf0", b"leaf1"];
        let hashes: Vec<[u8; 32]> = leaves.iter().map(|l| hash_leaf(*l)).collect();
        let root = hash_node(&hashes[0], &hashes[1]);

        let proof = MerkleProof {
            path: vec![hashes[1]],
            leaf_index: 0,
        };

        // 原始數據驗證成功
        assert!(proof.verify(leaves[0], &root));

        // 篡改數據驗證失敗
        assert!(!proof.verify(b"tampered", &root));
    }

    /// 測試篡改證明路徑導致驗證失敗
    #[test]
    fn test_tampered_proof_fails() {
        let leaves = [b"leaf0", b"leaf1"];
        let hashes: Vec<[u8; 32]> = leaves.iter().map(|l| hash_leaf(*l)).collect();
        let root = hash_node(&hashes[0], &hashes[1]);

        // 使用錯誤的兄弟節點哈希
        let bad_proof = MerkleProof {
            path: vec![[0xFFu8; 32]], // 錯誤的哈希
            leaf_index: 0,
        };

        assert!(!bad_proof.verify(leaves[0], &root));
    }

    /// 測試錯誤的索引導致驗證失敗
    #[test]
    fn test_wrong_index_fails() {
        let leaves = [b"leaf0", b"leaf1"];
        let hashes: Vec<[u8; 32]> = leaves.iter().map(|l| hash_leaf(*l)).collect();
        let root = hash_node(&hashes[0], &hashes[1]);

        // 使用錯誤的索引
        let bad_proof = MerkleProof {
            path: vec![hashes[0]], // 正確的兄弟節點
            leaf_index: 1,         // 但索引錯誤
        };

        assert!(!bad_proof.verify(leaves[0], &root));
    }

    /// 測試序列化和反序列化
    #[test]
    fn test_serialization() {
        let original = MerkleProof {
            path: vec![[1u8; 32], [2u8; 32], [3u8; 32]],
            leaf_index: 42,
        };

        let bytes = original.to_bytes();
        let restored = MerkleProof::from_bytes(&bytes).unwrap();

        assert_eq!(original, restored);
    }

    /// 測試無效字節的反序列化失敗
    #[test]
    fn test_invalid_deserialization() {
        let invalid_bytes = vec![0xFF, 0xFF, 0xFF];
        let result = MerkleProof::from_bytes(&invalid_bytes);

        assert!(result.is_err());
        match result {
            Err(MerkleError::Deserialization(_)) => {}
            _ => panic!("Expected Deserialization error"),
        }
    }

    /// 測試證明深度
    #[test]
    fn test_proof_depth() {
        let proof = MerkleProof {
            path: vec![[0u8; 32], [1u8; 32], [2u8; 32]],
            leaf_index: 0,
        };

        assert_eq!(proof.depth(), 3);
    }

    /// 測試八葉子樹（更深的樹）
    #[test]
    fn test_eight_leaf_tree() {
        // 構建 8 葉子的默克爾樹
        let leaves: Vec<&[u8]> = vec![
            b"leaf0", b"leaf1", b"leaf2", b"leaf3", b"leaf4", b"leaf5", b"leaf6", b"leaf7",
        ];
        let leaf_hashes: Vec<[u8; 32]> = leaves.iter().map(|l| hash_leaf(*l)).collect();

        // 構建第二層（4 個節點）
        let layer1 = vec![
            hash_node(&leaf_hashes[0], &leaf_hashes[1]),
            hash_node(&leaf_hashes[2], &leaf_hashes[3]),
            hash_node(&leaf_hashes[4], &leaf_hashes[5]),
            hash_node(&leaf_hashes[6], &leaf_hashes[7]),
        ];

        // 構建第三層（2 個節點）
        let layer2 = vec![hash_node(&layer1[0], &layer1[1]), hash_node(&layer1[2], &layer1[3])];

        // 構建根
        let root = hash_node(&layer2[0], &layer2[1]);

        // 驗證 leaf5 (索引 5 = 0b101)
        let proof5 = MerkleProof {
            path: vec![leaf_hashes[4], layer1[3], layer2[0]],
            leaf_index: 5,
        };
        assert!(proof5.verify(leaves[5], &root));
        println!("✓ Verified leaf5 in 8-leaf tree");

        // 驗證其他葉子
        let proof0 = MerkleProof {
            path: vec![leaf_hashes[1], layer1[1], layer2[1]],
            leaf_index: 0,
        };
        assert!(proof0.verify(leaves[0], &root));

        let proof7 = MerkleProof {
            path: vec![leaf_hashes[6], layer1[2], layer2[0]],
            leaf_index: 7,
        };
        assert!(proof7.verify(leaves[7], &root));
    }

    /// 測試空數據的哈希
    #[test]
    fn test_empty_data() {
        let empty = b"";
        let hash = hash_leaf(empty);

        // 空數據也應該產生有效的哈希
        assert_eq!(hash.len(), 32);

        // 可以驗證空數據
        let root = hash;
        let proof = MerkleProof {
            path: vec![],
            leaf_index: 0,
        };
        assert!(proof.verify(empty, &root));
    }

    // ==================== MerkleTree 構建測試 ====================

    /// 測試從小型 blob 構建 Merkle Tree
    #[test]
    fn test_merkle_tree_small_blob() {
        // 870 bytes blob（小於 4KB，只會產生 1 個 chunk）
        let blob_data = b"Hello Walrus!".repeat(67); // ~870 bytes

        let tree = MerkleTree::from_blob(&blob_data, 4096).unwrap();

        // 驗證基本屬性
        assert_eq!(tree.leaf_count(), 1);
        assert_eq!(tree.leaf_hashes().len(), 1);

        // 單葉子樹的根應該等於葉子的哈希
        let expected_root = hash_leaf(&blob_data);
        assert_eq!(tree.root(), expected_root);

        // 生成並驗證證明
        let proof = tree.generate_proof(0).unwrap();
        assert!(proof.path.is_empty()); // 單葉子樹的路徑應該為空
        assert!(proof.verify(&blob_data, &tree.root()));
    }

    /// 測試從中型 blob 構建 Merkle Tree（多個 chunks）
    #[test]
    fn test_merkle_tree_multiple_chunks() {
        // 10KB blob，會產生 3 個 chunks (4KB + 4KB + 2KB)
        let blob_data = b"X".repeat(10240);

        let tree = MerkleTree::from_blob(&blob_data, 4096).unwrap();

        // 驗證基本屬性
        assert_eq!(tree.leaf_count(), 3); // ceil(10240 / 4096) = 3
        assert_eq!(tree.leaf_hashes().len(), 3);

        // 測試每個葉子的證明
        for i in 0..3 {
            let chunk_start = i * 4096;
            let chunk_end = std::cmp::min(chunk_start + 4096, blob_data.len());
            let chunk = &blob_data[chunk_start..chunk_end];

            let proof = tree.generate_proof(i).unwrap();
            assert!(proof.verify(chunk, &tree.root()), "Proof for chunk {} failed", i);
        }
    }

    /// 測試 4KB 邊界情況
    #[test]
    fn test_merkle_tree_exact_4kb() {
        // 恰好 4KB
        let blob_data = b"A".repeat(4096);

        let tree = MerkleTree::from_blob(&blob_data, 4096).unwrap();

        assert_eq!(tree.leaf_count(), 1);

        let proof = tree.generate_proof(0).unwrap();
        assert!(proof.verify(&blob_data, &tree.root()));
    }

    /// 測試超過 100KB 的大型 blob
    #[test]
    fn test_merkle_tree_large_blob() {
        // 130KB blob，會產生 32 個 chunks
        let blob_data = b"Y".repeat(131072);

        let tree = MerkleTree::from_blob(&blob_data, 4096).unwrap();

        assert_eq!(tree.leaf_count(), 32); // ceil(131072 / 4096) = 32

        // 測試隨機幾個葉子的證明
        for &i in &[0, 5, 15, 31] {
            let chunk_start = i * 4096;
            let chunk_end = std::cmp::min(chunk_start + 4096, blob_data.len());
            let chunk = &blob_data[chunk_start..chunk_end];

            let proof = tree.generate_proof(i).unwrap();
            assert!(proof.verify(chunk, &tree.root()), "Proof for chunk {} failed", i);
        }
    }

    /// 測試空數據錯誤
    #[test]
    fn test_merkle_tree_empty_data() {
        let result = MerkleTree::from_blob(&[], 4096);

        assert!(result.is_err());
        match result {
            Err(MerkleError::EmptyData) => {}
            _ => panic!("Expected EmptyData error"),
        }
    }

    /// 測試無效的葉子索引
    #[test]
    fn test_merkle_tree_invalid_index() {
        let blob_data = b"Test".repeat(100);
        let tree = MerkleTree::from_blob(&blob_data, 4096).unwrap();

        let result = tree.generate_proof(999);

        assert!(result.is_err());
        match result {
            Err(MerkleError::InvalidLeafIndex { index, total }) => {
                assert_eq!(index, 999);
                assert_eq!(total, tree.leaf_count());
            }
            _ => panic!("Expected InvalidLeafIndex error"),
        }
    }

    /// 測試不同 chunk size
    #[test]
    fn test_merkle_tree_different_chunk_sizes() {
        let blob_data = b"Z".repeat(10000);

        // 測試 1KB chunks
        let tree_1kb = MerkleTree::from_blob(&blob_data, 1024).unwrap();
        assert_eq!(tree_1kb.leaf_count(), 10); // ceil(10000 / 1024) = 10

        // 測試 2KB chunks
        let tree_2kb = MerkleTree::from_blob(&blob_data, 2048).unwrap();
        assert_eq!(tree_2kb.leaf_count(), 5); // ceil(10000 / 2048) = 5

        // 不同 chunk size 會產生不同的 Merkle 根
        assert_ne!(tree_1kb.root(), tree_2kb.root());
    }

    /// 測試真實場景：從 Walrus 下載的 blob
    #[test]
    fn test_merkle_tree_realistic_scenario() {
        // 模擬 Walrus blob: 65KB
        let blob_data = b"Walrus storage integrity audit test data\n".repeat(1587); // ~65KB

        let tree = MerkleTree::from_blob(&blob_data, 4096).unwrap();

        // 驗證葉子數量
        let expected_leaves = (blob_data.len() + 4095) / 4096; // ceil division
        assert_eq!(tree.leaf_count(), expected_leaves);

        // 模擬審計挑戰：隨機選擇 10 個葉子進行驗證
        let challenge_indices = [0, 3, 7, 10, 12, 14, 15];

        let mut successful_verifications = 0;
        for &index in &challenge_indices {
            if index >= tree.leaf_count() {
                continue;
            }

            let chunk_start = index * 4096;
            let chunk_end = std::cmp::min(chunk_start + 4096, blob_data.len());
            let chunk = &blob_data[chunk_start..chunk_end];

            let proof = tree.generate_proof(index).unwrap();
            if proof.verify(chunk, &tree.root()) {
                successful_verifications += 1;
            }
        }

        // 所有挑戰都應該驗證成功
        assert_eq!(successful_verifications, challenge_indices.len());
    }
}
