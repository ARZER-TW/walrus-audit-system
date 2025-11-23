/**
 * Seal HTTP 客戶端
 *
 * 通過 HTTP 調用 TypeScript Seal API 服務來進行 IBE 門檻加密
 */

use anyhow::{Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

/// Seal API 端點配置
#[derive(Debug, Clone)]
pub struct SealApiConfig {
    /// Seal API 服務器 URL（例如 http://localhost:3001）
    pub api_url: String,
    /// HTTP 請求超時（秒）
    pub timeout_secs: u64,
}

impl Default for SealApiConfig {
    fn default() -> Self {
        Self {
            api_url: "http://localhost:3001".to_string(),
            timeout_secs: 30,
        }
    }
}

/// 加密請求
#[derive(Debug, Serialize)]
pub struct EncryptRequest {
    /// Base64 編碼的數據
    pub data: String,
    /// 審計員 Sui 地址（作為 IBE identity）
    pub identity: String,
    /// 審計合約 Package ID
    #[serde(rename = "packageId")]
    pub package_id: String,
    /// 門檻值（例如 2 表示 2-out-of-3）
    pub threshold: u32,
}

/// 加密響應
#[derive(Debug, Deserialize)]
pub struct EncryptResponse {
    pub success: bool,
    #[serde(rename = "encryptedData")]
    pub encrypted_data: Option<String>,
    #[serde(rename = "symmetricKey")]
    pub symmetric_key: Option<String>,
    pub metadata: Option<EncryptMetadata>,
    pub error: Option<String>,
}

/// 加密元數據
#[derive(Debug, Deserialize, Clone)]
pub struct EncryptMetadata {
    pub identity: String,
    #[serde(rename = "packageId")]
    pub package_id: String,
    pub threshold: u32,
    #[serde(rename = "encryptedAt")]
    pub encrypted_at: u64,
    #[serde(rename = "originalSize")]
    pub original_size: usize,
    #[serde(rename = "encryptedSize")]
    pub encrypted_size: usize,
    pub duration: u64,
}

/// 健康檢查響應
#[derive(Debug, Deserialize)]
pub struct HealthResponse {
    pub status: String,
    pub service: String,
    pub version: String,
    pub timestamp: String,
}

/// Seal 加密結果
pub struct EncryptResult {
    pub encrypted_data: String,
    pub symmetric_key: String,
    pub metadata: EncryptMetadata,
}

/// Seal HTTP 客戶端
pub struct SealClient {
    config: SealApiConfig,
    client: Client,
}

impl SealClient {
    /// 創建新的 Seal 客戶端
    pub fn new(config: SealApiConfig) -> Result<Self> {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(config.timeout_secs))
            .build()
            .context("Failed to create HTTP client")?;

        Ok(Self { config, client })
    }

    /// 使用默認配置創建客戶端
    pub fn with_defaults() -> Result<Self> {
        Self::new(SealApiConfig::default())
    }

    /// 健康檢查
    pub async fn health_check(&self) -> Result<HealthResponse> {
        let url = format!("{}/health", self.config.api_url);
        debug!("Checking Seal API health at {}", url);

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .context("Failed to send health check request")?;

        if !response.status().is_success() {
            anyhow::bail!("Health check failed with status: {}", response.status());
        }

        let health: HealthResponse = response
            .json()
            .await
            .context("Failed to parse health check response")?;

        debug!("Health check successful: {:?}", health);
        Ok(health)
    }

    /// 加密審計報告
    ///
    /// # Arguments
    /// * `report_json` - 審計報告 JSON 字串
    /// * `auditor_address` - 審計員 Sui 地址（32 字節十六進制，0x 開頭）
    /// * `package_id` - 審計合約 Package ID（32 字節十六進制，0x 開頭）
    /// * `threshold` - 門檻值（默認 2）
    ///
    /// # Returns
    /// 返回 (加密數據, 對稱密鑰, 元數據) 的元組，所有數據均為 Base64 編碼
    pub async fn encrypt_report(
        &self,
        report_json: &str,
        auditor_address: &str,
        package_id: &str,
        threshold: u32,
    ) -> Result<(String, String, EncryptMetadata)> {
        // 驗證地址格式
        if !auditor_address.starts_with("0x") || auditor_address.len() != 66 {
            anyhow::bail!(
                "Invalid auditor address format (must be 32-byte hex with 0x prefix): {}",
                auditor_address
            );
        }

        if !package_id.starts_with("0x") || package_id.len() != 66 {
            anyhow::bail!(
                "Invalid package ID format (must be 32-byte hex with 0x prefix): {}",
                package_id
            );
        }

        info!(
            "Encrypting audit report for auditor {} using package {}",
            auditor_address, package_id
        );
        debug!("Report size: {} bytes", report_json.len());
        debug!("Threshold: {}", threshold);

        // 將報告 JSON 編碼為 Base64
        let data_base64 = base64::encode(report_json.as_bytes());

        // 構建請求
        let request = EncryptRequest {
            data: data_base64,
            identity: auditor_address.to_string(),
            package_id: package_id.to_string(),
            threshold,
        };

        // 發送加密請求
        let url = format!("{}/api/seal/encrypt", self.config.api_url);
        debug!("Sending encrypt request to {}", url);

        let response = self
            .client
            .post(&url)
            .json(&request)
            .send()
            .await
            .context("Failed to send encrypt request")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            anyhow::bail!("Encrypt request failed with status {}: {}", status, error_text);
        }

        // 解析響應
        let encrypt_response: EncryptResponse = response
            .json()
            .await
            .context("Failed to parse encrypt response")?;

        if !encrypt_response.success {
            let error_msg = encrypt_response
                .error
                .unwrap_or_else(|| "Unknown error".to_string());
            anyhow::bail!("Encryption failed: {}", error_msg);
        }

        // 提取結果
        let encrypted_data = encrypt_response
            .encrypted_data
            .context("Missing encrypted data in response")?;

        let symmetric_key = encrypt_response
            .symmetric_key
            .context("Missing symmetric key in response")?;

        let metadata = encrypt_response
            .metadata
            .context("Missing metadata in response")?;

        info!(
            "Report encrypted successfully (original: {} bytes, encrypted: {} bytes, duration: {}ms)",
            metadata.original_size, metadata.encrypted_size, metadata.duration
        );

        Ok((encrypted_data, symmetric_key, metadata))
    }
}

// Base64 編碼/解碼輔助模塊
mod base64 {
    use base64::{engine::general_purpose, Engine as _};

    pub fn encode(data: &[u8]) -> String {
        general_purpose::STANDARD.encode(data)
    }

    pub fn decode(encoded: &str) -> Result<Vec<u8>, base64::DecodeError> {
        general_purpose::STANDARD.decode(encoded)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore] // 需要 Seal API 服務運行
    async fn test_health_check() -> Result<()> {
        let client = SealClient::with_defaults()?;
        let health = client.health_check().await?;
        assert_eq!(health.status, "healthy");
        assert_eq!(health.service, "seal-api-server");
        Ok(())
    }

    #[tokio::test]
    #[ignore] // 需要 Seal API 服務運行
    async fn test_encrypt_report() -> Result<()> {
        let client = SealClient::with_defaults()?;

        let test_report = r#"{"version":"1.0","timestamp":1234567890}"#;
        let test_auditor = "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef";
        let test_package = "0x8afa5d31dbaa0a8fb07082692940ca3d56b5e856c5126cb5a3693f0a4de63b82";

        let (encrypted_data, symmetric_key, metadata) = client
            .encrypt_report(test_report, test_auditor, test_package, 2)
            .await?;

        assert!(!encrypted_data.is_empty());
        assert!(!symmetric_key.is_empty());
        assert_eq!(metadata.threshold, 2);
        assert_eq!(metadata.identity, test_auditor);
        assert_eq!(metadata.package_id, test_package);

        println!("✅ Encryption test passed:");
        println!("   - Encrypted data length: {} bytes", encrypted_data.len());
        println!("   - Symmetric key length: {} bytes", symmetric_key.len());
        println!("   - Metadata: {:?}", metadata);

        Ok(())
    }

    #[test]
    fn test_invalid_address_format() {
        let client = SealClient::with_defaults().unwrap();
        let rt = tokio::runtime::Runtime::new().unwrap();

        // 測試無效的地址格式
        let result = rt.block_on(client.encrypt_report(
            "{}",
            "invalid-address",
            "0x8afa5d31dbaa0a8fb07082692940ca3d56b5e856c5126cb5a3693f0a4de63b82",
            2,
        ));

        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Invalid auditor address"));
    }
}
