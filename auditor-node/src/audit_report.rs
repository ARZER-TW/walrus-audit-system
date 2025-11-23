//! 審計報告生成模組
//!
//! 本模組整合完整性驗證和 PQC 簽名，生成最終的審計報告。
//!
//! # 報告結構
//!
//! 一個完整的審計報告包含：
//! 1. **審計數據** (`AuditData`): 包含 blob_id, content_hash, file_size 等
//! 2. **PQC 簽名** (`Dilithium3`): 對審計數據的量子安全簽名
//! 3. **元數據**: 審計員地址、簽名算法、時間戳等
//!
//! # 簽名流程
//!
//! ```text
//! AuditData
//!     ↓
//! JSON 序列化
//!     ↓
//! SHA-256 哈希（可選，用於摘要）
//!     ↓
//! Dilithium3 簽名
//!     ↓
//! SignedAuditReport
//! ```
//!
//! # 為什麼使用 PQC 簽名？
//!
//! - **長期真實性保證**: 審計報告可能需要保存數年甚至數十年
//! - **量子抗性**: Dilithium3 (NIST FIPS 204) 可抵抗量子計算機攻擊
//! - **法律效力**: 符合未來監管要求的數位簽名標準
//! - **防篡改**: 任何對報告的修改都會使簽名失效
//!
//! # 與 Sui 鏈上簽名的關係
//!
//! - **Sui 鏈上**: 使用 ECDSA（Ed25519）進行交易簽名
//! - **應用層**: 使用 Dilithium3 簽名審計報告本身
//! - **雙重保護**: 鏈上記錄（當前安全）+ PQC 簽名（長期安全）

use crate::error::{AuditorError, Result};
use crate::integrity::{AuditData, VerificationStatus};
use base64::{engine::general_purpose, Engine as _};
use chrono::Utc;
use pqc_signer::dilithium::Dilithium3Signer;
use pqc_signer::traits::Signer;
use serde::{Deserialize, Serialize};
use tracing::{debug, info};

/// PQC 算法類型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum PqcAlgorithm {
    /// Dilithium3 (NIST FIPS 204 Level 3)
    Dilithium3,
    /// Falcon512 (NIST FIPS 205 Level 1)
    #[allow(dead_code)]
    Falcon512,
}

impl PqcAlgorithm {
    pub fn as_str(&self) -> &str {
        match self {
            PqcAlgorithm::Dilithium3 => "Dilithium3",
            PqcAlgorithm::Falcon512 => "Falcon512",
        }
    }
}

/// 簽名的審計報告
///
/// 包含審計數據和對應的 PQC 簽名
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignedAuditReport {
    /// 審計數據
    pub audit_data: AuditData,

    /// PQC 簽名（Base64 編碼）
    ///
    /// 對 `audit_data` 的 JSON 序列化結果進行簽名
    pub signature: String,

    /// 簽名算法
    pub algorithm: PqcAlgorithm,

    /// 審計員公鑰（Base64 編碼）
    pub auditor_public_key: String,

    /// 報告生成時間戳（Unix 時間，秒）
    pub report_timestamp: u64,

    /// 可選：審計員 Sui 地址
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auditor_sui_address: Option<String>,
}

impl SignedAuditReport {
    /// 驗證報告簽名
    ///
    /// # 返回
    /// - `Ok(true)`: 簽名有效
    /// - `Ok(false)`: 簽名無效
    /// - `Err(_)`: 驗證過程中出錯
    pub fn verify_signature(&self) -> Result<bool> {
        // 重新序列化審計數據
        let audit_json = serde_json::to_vec(&self.audit_data)
            .map_err(|e| AuditorError::Serialization(format!("Failed to serialize audit data: {}", e)))?;

        // 解碼簽名
        let signature_bytes = general_purpose::STANDARD.decode(&self.signature)
            .map_err(|e| AuditorError::Serialization(format!("Failed to decode signature: {}", e)))?;

        // 解碼公鑰
        let public_key_bytes = general_purpose::STANDARD.decode(&self.auditor_public_key)
            .map_err(|e| AuditorError::Serialization(format!("Failed to decode public key: {}", e)))?;

        match self.algorithm {
            PqcAlgorithm::Dilithium3 => {
                // 使用公鑰創建驗證器（僅用於驗證，無簽名能力）
                let verifier = Dilithium3Signer::from_public_key_only(&public_key_bytes)?;
                verifier.verify(&audit_json, &signature_bytes)
                    .map_err(|e| AuditorError::PqcSignature(e.to_string()))
            }
            PqcAlgorithm::Falcon512 => {
                Err(AuditorError::PqcSignature("Falcon512 not yet implemented".to_string()))
            }
        }
    }

    /// 將報告序列化為 JSON
    pub fn to_json(&self) -> Result<String> {
        serde_json::to_string_pretty(self)
            .map_err(|e| AuditorError::Serialization(format!("Failed to serialize report: {}", e)))
    }

    /// 從 JSON 反序列化報告
    pub fn from_json(json: &str) -> Result<Self> {
        serde_json::from_str(json)
            .map_err(|e| AuditorError::Serialization(format!("Failed to deserialize report: {}", e)))
    }
}

/// 審計報告生成器
///
/// 負責整合完整性驗證和 PQC 簽名
pub struct AuditReportGenerator {
    /// PQC 簽名器
    signer: Dilithium3Signer,

    /// 審計員 Sui 地址（可選）
    auditor_address: Option<String>,
}

impl AuditReportGenerator {
    /// 創建新的報告生成器
    ///
    /// # 參數
    /// - `signer`: Dilithium3 簽名器（包含密鑰對）
    /// - `auditor_address`: 審計員的 Sui 地址（可選）
    pub fn new(signer: Dilithium3Signer, auditor_address: Option<String>) -> Self {
        info!("Created AuditReportGenerator with algorithm: Dilithium3");
        Self {
            signer,
            auditor_address,
        }
    }

    /// 從密鑰庫加載生成器
    ///
    /// # 參數
    /// - `keystore_path`: 密鑰庫文件路徑
    /// - `auditor_address`: 審計員的 Sui 地址（可選）
    ///
    /// # 示例
    /// ```no_run
    /// use auditor_node::audit_report::AuditReportGenerator;
    ///
    /// let generator = AuditReportGenerator::from_keystore(
    ///     "/path/to/pqc_keystore.json",
    ///     Some("0x1234...".to_string())
    /// ).expect("Failed to load keystore");
    /// ```
    pub fn from_keystore(keystore_path: &str, auditor_address: Option<String>) -> Result<Self> {
        info!("Loading PQC keystore from: {}", keystore_path);

        // 讀取密鑰庫文件
        let keystore_data = std::fs::read_to_string(keystore_path)
            .map_err(|e| AuditorError::Keystore(format!("Failed to read keystore file: {}", e)))?;

        #[derive(Deserialize)]
        struct KeystoreData {
            public_key: String,
            secret_key: String,
        }

        let keystore: KeystoreData = serde_json::from_str(&keystore_data)
            .map_err(|e| AuditorError::Keystore(format!("Failed to parse keystore JSON: {}", e)))?;

        // 解碼密鑰
        let public_key_bytes = general_purpose::STANDARD.decode(&keystore.public_key)
            .map_err(|e| AuditorError::Keystore(format!("Failed to decode public key: {}", e)))?;
        let secret_key_bytes = general_purpose::STANDARD.decode(&keystore.secret_key)
            .map_err(|e| AuditorError::Keystore(format!("Failed to decode secret key: {}", e)))?;

        // 從字節創建簽名器
        let signer = Dilithium3Signer::from_bytes(&public_key_bytes, &secret_key_bytes)?;

        Ok(Self::new(signer, auditor_address))
    }

    /// 生成新的密鑰對並創建生成器
    ///
    /// # 參數
    /// - `keystore_path`: 保存密鑰的路徑
    /// - `auditor_address`: 審計員的 Sui 地址（可選）
    pub fn generate_new(keystore_path: &str, auditor_address: Option<String>) -> Result<Self> {
        info!("Generating new PQC keypair and saving to: {}", keystore_path);

        // 生成密鑰對
        let mut signer = Dilithium3Signer::new();
        signer.generate_keypair()?;

        // 將密鑰編碼為 Base64 並保存到 JSON 文件
        #[derive(Serialize)]
        struct KeystoreData {
            public_key: String,
            secret_key: String,
            algorithm: String,
        }

        let keystore = KeystoreData {
            public_key: general_purpose::STANDARD.encode(signer.public_key()),
            secret_key: general_purpose::STANDARD.encode(signer.secret_key()),
            algorithm: "Dilithium3".to_string(),
        };

        let keystore_json = serde_json::to_string_pretty(&keystore)
            .map_err(|e| AuditorError::Keystore(format!("Failed to serialize keystore: {}", e)))?;

        std::fs::write(keystore_path, keystore_json)
            .map_err(|e| AuditorError::Keystore(format!("Failed to write keystore file: {}", e)))?;

        info!("Keypair generated and saved successfully");

        Ok(Self::new(signer, auditor_address))
    }

    /// 生成簽名的審計報告
    ///
    /// # 參數
    /// - `audit_data`: 完整性驗證的審計數據
    ///
    /// # 返回
    /// - `Ok(SignedAuditReport)`: 包含 PQC 簽名的完整報告
    ///
    /// # 示例
    /// ```no_run
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// use auditor_node::integrity::{IntegrityVerifier, AuditData};
    /// use auditor_node::audit_report::AuditReportGenerator;
    ///
    /// // 1. 執行完整性審計
    /// let verifier = IntegrityVerifier::new_testnet();
    /// let audit_data = verifier.audit_blob("blob_id_here").await?;
    ///
    /// // 2. 生成簽名的報告
    /// let generator = AuditReportGenerator::from_keystore(
    ///     "pqc_keystore.json",
    ///     Some("0x1234...".to_string())
    /// )?;
    ///
    /// let signed_report = generator.generate_report(audit_data)?;
    ///
    /// // 3. 輸出 JSON
    /// println!("{}", signed_report.to_json()?);
    /// # Ok(())
    /// # }
    /// ```
    pub fn generate_report(&self, audit_data: AuditData) -> Result<SignedAuditReport> {
        info!(
            "Generating signed audit report for blob: {}",
            audit_data.blob_id
        );

        // 1. 序列化審計數據
        let audit_json = serde_json::to_vec(&audit_data)
            .map_err(|e| AuditorError::Serialization(format!("Failed to serialize audit data: {}", e)))?;

        debug!("Audit data serialized: {} bytes", audit_json.len());

        // 2. 使用 Dilithium3 簽名
        let signature_bytes = self.signer.sign(&audit_json)?;

        debug!("Signature generated: {} bytes", signature_bytes.len());

        // 3. 編碼為 Base64
        let signature_base64 = general_purpose::STANDARD.encode(&signature_bytes);
        let public_key_base64 = general_purpose::STANDARD.encode(self.signer.public_key());

        // 4. 構造簽名報告
        let report = SignedAuditReport {
            audit_data,
            signature: signature_base64,
            algorithm: PqcAlgorithm::Dilithium3,
            auditor_public_key: public_key_base64,
            report_timestamp: Utc::now().timestamp() as u64,
            auditor_sui_address: self.auditor_address.clone(),
        };

        info!(
            "Report generated successfully: {} (status: {:?})",
            report.audit_data.blob_id, report.audit_data.verification_status
        );

        Ok(report)
    }

    /// 生成報告並驗證簽名（自檢）
    ///
    /// 用於測試簽名流程的正確性
    pub fn generate_and_verify(&self, audit_data: AuditData) -> Result<SignedAuditReport> {
        let report = self.generate_report(audit_data)?;

        // 立即驗證簽名
        let is_valid = report.verify_signature()?;

        if is_valid {
            info!("Self-verification passed ✓");
        } else {
            return Err(AuditorError::PqcSignature(
                "Self-verification failed immediately after signing".to_string(),
            ));
        }

        Ok(report)
    }

    /// 批量生成報告
    ///
    /// # 參數
    /// - `audit_data_list`: 多個審計數據
    ///
    /// # 返回
    /// - `Vec<SignedAuditReport>`: 簽名的報告列表
    pub fn generate_batch_reports(
        &self,
        audit_data_list: Vec<AuditData>,
    ) -> Result<Vec<SignedAuditReport>> {
        info!("Generating {} signed reports", audit_data_list.len());

        let mut reports = Vec::new();

        for audit_data in audit_data_list {
            let report = self.generate_report(audit_data)?;
            reports.push(report);
        }

        info!("Batch report generation completed: {} reports", reports.len());

        Ok(reports)
    }

    /// 獲取公鑰（Base64 編碼）
    pub fn public_key_base64(&self) -> String {
        general_purpose::STANDARD.encode(self.signer.public_key())
    }
}

/// 報告統計信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportStatistics {
    /// 總審計次數
    pub total_audits: usize,

    /// 成功訪問的 Blob 數量
    pub accessible_count: usize,

    /// 無法訪問的 Blob 數量
    pub unreachable_count: usize,

    /// 檢測到損壞的 Blob 數量
    pub corrupted_count: usize,

    /// 平均文件大小（bytes）
    pub average_file_size: u64,

    /// 總數據量（bytes）
    pub total_data_size: u64,
}

impl ReportStatistics {
    /// 從報告列表計算統計信息
    pub fn from_reports(reports: &[SignedAuditReport]) -> Self {
        let total_audits = reports.len();
        let mut accessible_count = 0;
        let mut unreachable_count = 0;
        let mut corrupted_count = 0;
        let mut total_data_size = 0u64;

        for report in reports {
            match report.audit_data.verification_status {
                VerificationStatus::Accessible => accessible_count += 1,
                VerificationStatus::Unreachable => unreachable_count += 1,
                VerificationStatus::Corrupted => corrupted_count += 1,
            }

            total_data_size += report.audit_data.file_size;
        }

        let average_file_size = if total_audits > 0 {
            total_data_size / total_audits as u64
        } else {
            0
        };

        Self {
            total_audits,
            accessible_count,
            unreachable_count,
            corrupted_count,
            average_file_size,
            total_data_size,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::integrity::VerificationStatus;

    #[test]
    fn test_report_generator_creation() {
        let mut signer = Dilithium3Signer::new();
        signer.generate_keypair().unwrap();

        let generator = AuditReportGenerator::new(signer, Some("0x1234".to_string()));
        assert_eq!(generator.auditor_address, Some("0x1234".to_string()));
    }

    #[test]
    fn test_report_generation_and_verification() {
        // 創建簽名器
        let mut signer = Dilithium3Signer::new();
        signer.generate_keypair().unwrap();

        let generator = AuditReportGenerator::new(signer, None);

        // 創建測試審計數據
        let audit_data = AuditData {
            blob_id: "test_blob_id".to_string(),
            content_hash: "abcdef1234567890".to_string(),
            merkle_root: "0000000000000000000000000000000000000000000000000000000000000000".to_string(),
            total_challenges: 10,
            successful_verifications: 10,
            failed_verifications: 0,
            file_size: 1024,
            timestamp: 1234567890,
            verification_status: VerificationStatus::Accessible,
            sui_object_id: None,
        };

        // 生成報告
        let report = generator.generate_report(audit_data.clone()).unwrap();

        // 驗證基本字段
        assert_eq!(report.audit_data.blob_id, "test_blob_id");
        assert_eq!(report.algorithm, PqcAlgorithm::Dilithium3);
        assert!(!report.signature.is_empty());
        assert!(!report.auditor_public_key.is_empty());

        // 驗證簽名
        let is_valid = report.verify_signature().unwrap();
        assert!(is_valid, "Signature verification should pass");
    }

    #[test]
    fn test_report_json_serialization() {
        let mut signer = Dilithium3Signer::new();
        signer.generate_keypair().unwrap();

        let generator = AuditReportGenerator::new(signer, None);

        let audit_data = AuditData {
            blob_id: "test_blob".to_string(),
            content_hash: "hash123".to_string(),
            merkle_root: "1111111111111111111111111111111111111111111111111111111111111111".to_string(),
            total_challenges: 5,
            successful_verifications: 5,
            failed_verifications: 0,
            file_size: 500,
            timestamp: 9999999,
            verification_status: VerificationStatus::Accessible,
            sui_object_id: Some("0xabc".to_string()),
        };

        let report = generator.generate_report(audit_data).unwrap();

        // 序列化
        let json = report.to_json().unwrap();
        assert!(json.contains("test_blob"));
        assert!(json.contains("Dilithium3"));

        // 反序列化
        let deserialized = SignedAuditReport::from_json(&json).unwrap();
        assert_eq!(deserialized.audit_data.blob_id, "test_blob");

        // 驗證反序列化後的簽名
        let is_valid = deserialized.verify_signature().unwrap();
        assert!(is_valid);
    }

    #[test]
    fn test_report_statistics() {
        let mut signer = Dilithium3Signer::new();
        signer.generate_keypair().unwrap();
        let generator = AuditReportGenerator::new(signer, None);

        let reports = vec![
            generator
                .generate_report(AuditData {
                    blob_id: "blob1".to_string(),
                    content_hash: "hash1".to_string(),
                    merkle_root: "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_string(),
                    total_challenges: 10,
                    successful_verifications: 10,
                    failed_verifications: 0,
                    file_size: 1000,
                    timestamp: 1,
                    verification_status: VerificationStatus::Accessible,
                    sui_object_id: None,
                })
                .unwrap(),
            generator
                .generate_report(AuditData {
                    blob_id: "blob2".to_string(),
                    content_hash: "hash2".to_string(),
                    merkle_root: "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb".to_string(),
                    total_challenges: 8,
                    successful_verifications: 6,
                    failed_verifications: 2,
                    file_size: 2000,
                    timestamp: 2,
                    verification_status: VerificationStatus::Unreachable,
                    sui_object_id: None,
                })
                .unwrap(),
            generator
                .generate_report(AuditData {
                    blob_id: "blob3".to_string(),
                    content_hash: "hash3".to_string(),
                    merkle_root: "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc".to_string(),
                    total_challenges: 5,
                    successful_verifications: 1,
                    failed_verifications: 4,
                    file_size: 3000,
                    timestamp: 3,
                    verification_status: VerificationStatus::Corrupted,
                    sui_object_id: None,
                })
                .unwrap(),
        ];

        let stats = ReportStatistics::from_reports(&reports);

        assert_eq!(stats.total_audits, 3);
        assert_eq!(stats.accessible_count, 1);
        assert_eq!(stats.unreachable_count, 1);
        assert_eq!(stats.corrupted_count, 1);
        assert_eq!(stats.total_data_size, 6000);
        assert_eq!(stats.average_file_size, 2000);
    }
}
