//! 審計報告管理模塊
//!
//! 本模塊提供審計報告的 PQC 簽名、序列化和持久化功能。
//!
//! # 功能
//!
//! - **PQC 簽名**: 使用 Dilithium3 對審計報告進行後量子安全的數字簽名
//! - **簽名驗證**: 驗證報告的 PQC 簽名是否有效
//! - **JSON 序列化**: 將報告導出為 JSON 格式（用於存檔和審計追蹤）
//! - **報告加載**: 從 JSON 文件加載已簽名的審計報告
//!
//! # 安全性
//!
//! - **簽名順序**: 簽名時 `pqc_signature` 字段必須為空（避免循環依賴）
//! - **完整性保證**: 簽名覆蓋整個報告內容（除簽名字段本身）
//! - **量子抗性**: Dilithium3 提供 NIST Level 3 安全性
//! - **長期有效性**: 簽名在量子計算時代仍然安全
//!
//! # 使用場景
//!
//! ## 場景 1：生成簽名報告
//! ```no_run
//! use auditor_node::report::ReportManager;
//! use auditor_node::types::AuditReport;
//! use pqc_signer::Dilithium3Signer;
//!
//! # fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let mut signer = Dilithium3Signer::new();
//! signer.generate_keypair()?;
//!
//! let mut report = AuditReport { /* ... */ };
//! let manager = ReportManager::new(signer);
//! manager.sign_report(&mut report)?;
//!
//! manager.export_json(&report, "audit_report_2024.json")?;
//! # Ok(())
//! # }
//! ```
//!
//! ## 場景 2：驗證已簽名報告
//! ```no_run
//! use auditor_node::report::ReportManager;
//!
//! # fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let report = ReportManager::load_json("audit_report_2024.json")?;
//! let auditor_public_key = vec![/* 審計員公鑰 */];
//!
//! let is_valid = ReportManager::verify_report(&report, &auditor_public_key)?;
//! assert!(is_valid, "Report signature is invalid!");
//! # Ok(())
//! # }
//! ```

use crate::error::{AuditorError, Result};
use crate::types::AuditReport;
use pqc_signer::{Dilithium3Signer, Signer};
use serde_json;
use std::fs;
use std::path::Path;
use tracing::{debug, info, warn};

/// 審計報告管理器
///
/// 負責管理審計報告的簽名、驗證和持久化
pub struct ReportManager {
    /// PQC 簽名器（Dilithium3）
    signer: Dilithium3Signer,
}

impl ReportManager {
    /// 創建新的報告管理器
    ///
    /// # 參數
    /// - `signer`: 已初始化密鑰的 Dilithium3 簽名器
    ///
    /// # 示例
    /// ```no_run
    /// use pqc_signer::{Dilithium3Signer, Signer};
    /// use auditor_node::report::ReportManager;
    ///
    /// let mut signer = Dilithium3Signer::new();
    /// signer.generate_keypair().unwrap();
    ///
    /// let manager = ReportManager::new(signer);
    /// ```
    pub fn new(signer: Dilithium3Signer) -> Self {
        info!("Created ReportManager with Dilithium3 signer");
        Self { signer }
    }

    /// 從密鑰字節創建報告管理器
    ///
    /// # 參數
    /// - `public_key`: PQC 公鑰字節（1952 bytes）
    /// - `secret_key`: PQC 私鑰字節（4032 bytes）
    ///
    /// # 錯誤
    /// - 如果密鑰長度不正確，返回 `PqcSignature` 錯誤
    pub fn from_keypair(public_key: &[u8], secret_key: &[u8]) -> Result<Self> {
        let signer = Dilithium3Signer::from_bytes(public_key, secret_key)
            .map_err(|e| AuditorError::PqcSignature(e.to_string()))?;

        info!(
            "Created ReportManager from keypair: pk_len={}, sk_len={}",
            public_key.len(),
            secret_key.len()
        );

        Ok(Self { signer })
    }

    /// 對審計報告進行 PQC 簽名
    ///
    /// # 簽名流程
    /// 1. 創建報告副本，將 `pqc_signature` 設為空
    /// 2. 將報告序列化為 JSON 字節
    /// 3. 使用 Dilithium3 對字節進行簽名
    /// 4. 將簽名存儲到報告的 `pqc_signature` 字段
    ///
    /// # 參數
    /// - `report`: 要簽名的審計報告（會被修改）
    ///
    /// # 錯誤
    /// - 序列化失敗: 返回 `Serialization` 錯誤
    /// - 簽名失敗: 返回 `PqcSignature` 錯誤
    ///
    /// # 示例
    /// ```no_run
    /// # use auditor_node::report::ReportManager;
    /// # use auditor_node::types::AuditReport;
    /// # use pqc_signer::{Dilithium3Signer, Signer};
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut signer = Dilithium3Signer::new();
    /// signer.generate_keypair()?;
    /// let manager = ReportManager::new(signer);
    ///
    /// let mut report = AuditReport { /* ... */ };
    /// manager.sign_report(&mut report)?;
    ///
    /// assert!(!report.pqc_signature.is_empty());
    /// # Ok(())
    /// # }
    /// ```
    pub fn sign_report(&self, report: &mut AuditReport) -> Result<()> {
        info!(
            "Signing audit report: blob_id={}, challenges={}",
            report.blob_id, report.total_challenges
        );

        // 步驟 1: 創建副本，清空簽名相關字段
        // 注意：必須同時清空 pqc_signature 和 pqc_algorithm，
        // 否則驗證時序列化會與簽名時不一致
        let mut temp_report = report.clone();
        temp_report.pqc_signature = vec![];
        temp_report.pqc_algorithm = 0;

        // 步驟 2: 序列化為 JSON 字節
        let serialized = serde_json::to_vec(&temp_report).map_err(|e| {
            AuditorError::Serialization(format!("Failed to serialize report: {}", e))
        })?;

        debug!(
            "Serialized report for signing: {} bytes",
            serialized.len()
        );

        // 步驟 3: 使用 PQC 簽名
        let signature = self
            .signer
            .sign(&serialized)
            .map_err(|e| AuditorError::PqcSignature(format!("Signing failed: {}", e)))?;

        info!(
            "Report signed successfully: signature_len={} bytes",
            signature.len()
        );

        // 步驟 4: 存儲簽名
        report.pqc_signature = signature;
        report.pqc_algorithm = 3; // Dilithium3

        Ok(())
    }

    /// 驗證審計報告的 PQC 簽名
    ///
    /// # 參數
    /// - `report`: 要驗證的審計報告
    /// - `public_key`: 審計員的 PQC 公鑰字節（1952 bytes）
    ///
    /// # 返回
    /// - `Ok(true)`: 簽名有效
    /// - `Ok(false)`: 簽名無效
    /// - `Err`: 驗證過程發生錯誤
    ///
    /// # 錯誤
    /// - 報告沒有簽名: 返回 `PqcSignature` 錯誤
    /// - 序列化失敗: 返回 `Serialization` 錯誤
    /// - 驗證失敗: 返回 `PqcSignature` 錯誤
    ///
    /// # 示例
    /// ```no_run
    /// # use auditor_node::report::ReportManager;
    /// # use auditor_node::types::AuditReport;
    /// # fn example(report: AuditReport, pk: Vec<u8>) -> Result<(), Box<dyn std::error::Error>> {
    /// let is_valid = ReportManager::verify_report(&report, &pk)?;
    ///
    /// if is_valid {
    ///     println!("Report signature is valid!");
    /// } else {
    ///     println!("WARNING: Report signature is INVALID!");
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn verify_report(report: &AuditReport, public_key: &[u8]) -> Result<bool> {
        info!(
            "Verifying report signature: blob_id={}, algorithm={}",
            report.blob_id, report.pqc_algorithm
        );

        // 檢查簽名是否存在
        if report.pqc_signature.is_empty() {
            return Err(AuditorError::PqcSignature(
                "Report has no signature".to_string(),
            ));
        }

        // 檢查算法
        if report.pqc_algorithm != 3 {
            warn!(
                "Unexpected PQC algorithm: expected 3 (Dilithium3), got {}",
                report.pqc_algorithm
            );
            return Err(AuditorError::PqcSignature(format!(
                "Unsupported PQC algorithm: {}",
                report.pqc_algorithm
            )));
        }

        // 創建臨時副本，清空簽名相關字段（必須與簽名時的清空邏輯一致）
        let mut temp_report = report.clone();
        temp_report.pqc_signature = vec![];
        temp_report.pqc_algorithm = 0;

        // 序列化
        let serialized = serde_json::to_vec(&temp_report).map_err(|e| {
            AuditorError::Serialization(format!("Failed to serialize report: {}", e))
        })?;

        debug!(
            "Serialized report for verification: {} bytes",
            serialized.len()
        );

        // 創建僅驗證的 Signer（只需公鑰，無需私鑰）
        // 使用 from_public_key_only() 而非 from_bytes()，因為驗證不需要私鑰
        let verifier = Dilithium3Signer::from_public_key_only(public_key).map_err(|e| {
            AuditorError::PqcSignature(format!("Invalid public key: {}", e))
        })?;

        debug!("Created verification-only signer with public key");

        // 執行驗證
        let is_valid = verifier
            .verify(&serialized, &report.pqc_signature)
            .map_err(|e| AuditorError::PqcSignature(format!("Verification failed: {}", e)))?;

        if is_valid {
            info!("Report signature verification: VALID ✓");
        } else {
            warn!("Report signature verification: INVALID ✗");
        }

        Ok(is_valid)
    }

    /// 將報告導出為 JSON 文件
    ///
    /// # 參數
    /// - `report`: 要導出的審計報告
    /// - `path`: 輸出文件路徑
    ///
    /// # 錯誤
    /// - 序列化失敗: 返回 `Serialization` 錯誤
    /// - 文件寫入失敗: 返回 `Io` 錯誤
    ///
    /// # 示例
    /// ```no_run
    /// # use auditor_node::report::ReportManager;
    /// # use auditor_node::types::AuditReport;
    /// # fn example(manager: ReportManager, report: AuditReport) -> Result<(), Box<dyn std::error::Error>> {
    /// manager.export_json(&report, "audit_report_2024-11-16.json")?;
    /// println!("Report exported successfully!");
    /// # Ok(())
    /// # }
    /// ```
    pub fn export_json(&self, report: &AuditReport, path: &str) -> Result<()> {
        info!("Exporting report to JSON: {}", path);

        // 序列化為美化的 JSON
        let json = serde_json::to_string_pretty(report).map_err(|e| {
            AuditorError::Serialization(format!("Failed to serialize report: {}", e))
        })?;

        let json_len = json.len();

        // 寫入文件
        fs::write(path, json).map_err(|e| {
            AuditorError::Io(std::io::Error::new(
                e.kind(),
                format!("Failed to write report to {}: {}", path, e),
            ))
        })?;

        info!("Report exported successfully: {} bytes", json_len);

        Ok(())
    }

    /// 從 JSON 文件加載報告
    ///
    /// # 參數
    /// - `path`: JSON 文件路徑
    ///
    /// # 返回
    /// - 加載的審計報告
    ///
    /// # 錯誤
    /// - 文件不存在: 返回 `Io` 錯誤
    /// - JSON 格式錯誤: 返回 `Serialization` 錯誤
    ///
    /// # 示例
    /// ```no_run
    /// # use auditor_node::report::ReportManager;
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let report = ReportManager::load_json("audit_report_2024-11-16.json")?;
    /// println!("Loaded report: blob_id={}", report.blob_id);
    /// # Ok(())
    /// # }
    /// ```
    pub fn load_json(path: &str) -> Result<AuditReport> {
        info!("Loading report from JSON: {}", path);

        // 檢查文件是否存在
        if !Path::new(path).exists() {
            return Err(AuditorError::Config(format!(
                "Report file not found: {}",
                path
            )));
        }

        // 讀取文件
        let json = fs::read_to_string(path).map_err(|e| {
            AuditorError::Io(std::io::Error::new(
                e.kind(),
                format!("Failed to read report from {}: {}", path, e),
            ))
        })?;

        // 反序列化
        let report: AuditReport = serde_json::from_str(&json).map_err(|e| {
            AuditorError::Serialization(format!("Failed to parse report JSON: {}", e))
        })?;

        info!(
            "Report loaded successfully: blob_id={}, challenges={}",
            report.blob_id, report.total_challenges
        );

        Ok(report)
    }

    /// 獲取簽名器的公鑰
    ///
    /// # 返回
    /// - PQC 公鑰字節（1952 bytes）
    pub fn public_key(&self) -> &[u8] {
        self.signer.public_key()
    }

    /// 獲取簽名器的私鑰（用於密鑰庫持久化）
    ///
    /// # 安全警告
    /// 私鑰應該安全存儲，不應該通過網路傳輸或記錄到日誌中
    pub fn secret_key(&self) -> &[u8] {
        self.signer.secret_key()
    }

    /// 獲取算法信息
    pub fn algorithm_name(&self) -> &str {
        self.signer.algorithm_name()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{AuditChallenge, ChallengeResult};

    /// 創建測試用的審計報告
    fn create_test_report() -> AuditReport {
        AuditReport {
            blob_id: "0xtest_blob_id".to_string(),
            blob_object_id: "0xtest_object_id".to_string(),
            auditor: "0xtest_auditor".to_string(),
            timestamp: 1700000000,
            challenge_epoch: 100,
            challenge_results: vec![ChallengeResult {
                challenge: AuditChallenge {
                    sliver_index: 0,
                    shard_id: 0,
                    challenge_type: 1,
                    timestamp: 1700000000,
                },
                verified: true,
                merkle_proof_valid: true,
                response_hash: vec![1, 2, 3, 4],
                failure_reason: None,
            }],
            total_challenges: 1,
            successful_verifications: 1,
            failed_verifications: 0,
            integrity_hash: vec![0u8; 32],
            pqc_signature: vec![],
            pqc_algorithm: 0,
            is_valid: true,
            failure_reason: None,
        }
    }

    #[test]
    fn test_report_manager_creation() {
        let mut signer = Dilithium3Signer::new();
        signer.generate_keypair().unwrap();

        let manager = ReportManager::new(signer);
        assert_eq!(manager.algorithm_name(), "Dilithium3");
        assert_eq!(manager.public_key().len(), 1952);
    }

    #[test]
    fn test_sign_and_verify_report() {
        // 創建簽名器
        let mut signer = Dilithium3Signer::new();
        signer.generate_keypair().unwrap();

        let public_key = signer.public_key().to_vec();

        // 創建管理器
        let manager = ReportManager::new(signer);

        // 創建報告
        let mut report = create_test_report();

        // 簽名前檢查
        assert!(report.pqc_signature.is_empty());

        // 簽名
        manager.sign_report(&mut report).unwrap();

        // 簽名後檢查
        assert!(!report.pqc_signature.is_empty());
        assert_eq!(report.pqc_algorithm, 3); // Dilithium3

        // 驗證簽名
        let is_valid = ReportManager::verify_report(&report, &public_key).unwrap();
        assert!(is_valid, "Report signature should be valid");
    }

    #[test]
    fn test_verify_tampered_report() {
        // 創建簽名器
        let mut signer = Dilithium3Signer::new();
        signer.generate_keypair().unwrap();

        let public_key = signer.public_key().to_vec();

        // 創建管理器並簽名
        let manager = ReportManager::new(signer);
        let mut report = create_test_report();
        manager.sign_report(&mut report).unwrap();

        // 篡改報告內容
        report.blob_id = "0xtampered_blob_id".to_string();

        // 驗證應該失敗
        let is_valid = ReportManager::verify_report(&report, &public_key).unwrap();
        assert!(!is_valid, "Tampered report signature should be invalid");
    }

    #[test]
    fn test_verify_unsigned_report() {
        let report = create_test_report();
        let fake_public_key = vec![0u8; 1952];

        // 驗證未簽名的報告應該返回錯誤
        let result = ReportManager::verify_report(&report, &fake_public_key);
        assert!(result.is_err());
        match result {
            Err(AuditorError::PqcSignature(msg)) => {
                assert!(msg.contains("no signature"));
            }
            _ => panic!("Expected PqcSignature error for unsigned report"),
        }
    }

    #[test]
    fn test_export_and_load_json() {
        use tempfile::NamedTempFile;

        // 創建簽名器和管理器
        let mut signer = Dilithium3Signer::new();
        signer.generate_keypair().unwrap();
        let manager = ReportManager::new(signer);

        // 創建並簽名報告
        let mut report = create_test_report();
        manager.sign_report(&mut report).unwrap();

        // 導出到臨時文件
        let temp_file = NamedTempFile::new().unwrap();
        let temp_path = temp_file.path().to_str().unwrap();

        manager.export_json(&report, temp_path).unwrap();

        // 加載報告
        let loaded_report = ReportManager::load_json(temp_path).unwrap();

        // 驗證內容相同
        assert_eq!(loaded_report.blob_id, report.blob_id);
        assert_eq!(loaded_report.auditor, report.auditor);
        assert_eq!(loaded_report.total_challenges, report.total_challenges);
        assert_eq!(loaded_report.pqc_signature, report.pqc_signature);
        assert_eq!(loaded_report.pqc_algorithm, report.pqc_algorithm);
    }

    #[test]
    fn test_load_nonexistent_file() {
        let result = ReportManager::load_json("/nonexistent/path/report.json");
        assert!(result.is_err());
        match result {
            Err(AuditorError::Config(msg)) => {
                assert!(msg.contains("not found"));
            }
            _ => panic!("Expected Config error for nonexistent file"),
        }
    }

    #[test]
    fn test_signature_determinism() {
        // 測試 pqcrypto-dilithium 的確定性簽名行為
        let mut signer = Dilithium3Signer::new();
        signer.generate_keypair().unwrap();
        let manager = ReportManager::new(signer);

        let mut report1 = create_test_report();
        let mut report2 = create_test_report();

        // 為兩個報告設置不同的時間戳，確保內容不同
        report1.timestamp = 1000;
        report2.timestamp = 2000;

        manager.sign_report(&mut report1).unwrap();
        manager.sign_report(&mut report2).unwrap();

        // pqcrypto-dilithium 使用確定性簽名模式
        // 不同內容應該產生不同的簽名
        assert_ne!(
            report1.pqc_signature, report2.pqc_signature,
            "Different report content should produce different signatures"
        );

        // 測試相同內容產生相同簽名（確定性）
        let mut report3 = create_test_report();
        let mut report4 = create_test_report();
        report3.timestamp = 3000;
        report4.timestamp = 3000; // 相同時間戳

        manager.sign_report(&mut report3).unwrap();
        manager.sign_report(&mut report4).unwrap();

        assert_eq!(
            report3.pqc_signature, report4.pqc_signature,
            "Same content should produce same signature (deterministic mode)"
        );
    }

    #[test]
    fn test_from_keypair() {
        // 生成密鑰對
        let mut original_signer = Dilithium3Signer::new();
        original_signer.generate_keypair().unwrap();

        let pk = original_signer.public_key().to_vec();
        let sk = original_signer.secret_key().to_vec();

        // 從密鑰對創建管理器
        let manager = ReportManager::from_keypair(&pk, &sk).unwrap();

        // 驗證密鑰相同
        assert_eq!(manager.public_key(), pk);
        assert_eq!(manager.secret_key(), sk);

        // 測試簽名功能
        let mut report = create_test_report();
        manager.sign_report(&mut report).unwrap();

        assert!(!report.pqc_signature.is_empty());
    }

    #[test]
    fn test_sign_verify_roundtrip() {
        // 完整的簽名-驗證往返測試
        let mut signer = Dilithium3Signer::new();
        signer.generate_keypair().unwrap();
        let public_key = signer.public_key().to_vec();

        let manager = ReportManager::new(signer);

        // 創建多個挑戰結果的報告
        let mut report = AuditReport {
            blob_id: "0xcomplex_blob".to_string(),
            blob_object_id: "0xcomplex_object".to_string(),
            auditor: "0xtest_auditor".to_string(),
            timestamp: 1700000000,
            challenge_epoch: 100,
            challenge_results: vec![
                ChallengeResult {
                    challenge: AuditChallenge {
                        sliver_index: 0,
                        shard_id: 0,
                        challenge_type: 1,
                        timestamp: 1700000000,
                    },
                    verified: true,
                    merkle_proof_valid: true,
                    response_hash: vec![1, 2, 3, 4],
                    failure_reason: None,
                },
                ChallengeResult {
                    challenge: AuditChallenge {
                        sliver_index: 5,
                        shard_id: 1,
                        challenge_type: 1,
                        timestamp: 1700000001,
                    },
                    verified: false,
                    merkle_proof_valid: false,
                    response_hash: vec![5, 6, 7, 8],
                    failure_reason: Some("Merkle proof invalid".to_string()),
                },
            ],
            total_challenges: 2,
            successful_verifications: 1,
            failed_verifications: 1,
            integrity_hash: vec![0u8; 32],
            pqc_signature: vec![],
            pqc_algorithm: 0,
            is_valid: false,
            failure_reason: Some("1 challenge failed".to_string()),
        };

        // 簽名
        manager.sign_report(&mut report).unwrap();

        // 驗證
        let is_valid = ReportManager::verify_report(&report, &public_key).unwrap();
        assert!(is_valid);
    }
}
