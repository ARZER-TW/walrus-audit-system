//! Walrus 存儲節點客戶端模塊
//!
//! 負責與 Walrus 存儲節點通信:
//! - 發送審計挑戰（請求特定 sliver）
//! - 接收 sliver 數據和默克爾證明
//! - 驗證存儲節點健康狀態
//! - 處理網絡錯誤和重試
//!
//! # API 端點
//!
//! 基於 Walrus 存儲節點的 HTTP API:
//! - `POST /v1/challenge` - 挑戰特定 sliver
//! - `GET /health` - 健康檢查
//!
//! # 重試策略
//!
//! - 最多重試 3 次
//! - 指數退避（1s, 2s, 4s）
//! - 僅對網絡錯誤重試，不對邏輯錯誤重試

use crate::error::{AuditorError, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::{debug, error, info, warn};

/// 默認重試次數
const DEFAULT_MAX_RETRIES: u32 = 3;

/// 默認超時（秒）
const DEFAULT_TIMEOUT_SECS: u64 = 30;

/// 挑戰請求（發送給存儲節點）
///
/// 請求特定 Blob 的特定 Sliver 數據和默克爾證明
#[derive(Serialize, Debug, Clone)]
pub struct ChallengeRequest {
    /// Blob ID（u256 as hex string 或 bytes）
    pub blob_id: String,

    /// 請求的 Sliver 索引（0 到 n-1）
    pub sliver_index: u64,

    /// 可選：請求者簽名（用於防止 DoS）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signature: Option<Vec<u8>>,
}

/// 存儲節點的響應
///
/// 包含請求的 Sliver 數據和對應的默克爾證明
#[derive(Deserialize, Debug, Clone)]
pub struct ChallengeResponse {
    /// Sliver 原始數據（經過 erasure coding 的片段）
    pub sliver_data: Vec<u8>,

    /// 默克爾證明（從該 sliver 到 merkle root 的路徑）
    /// 格式: Vec<[u8; 32]> 序列化後的字節
    pub merkle_proof: Vec<u8>,

    /// 可選：存儲節點簽名（用於證明數據來源）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub node_signature: Option<Vec<u8>>,

    /// 可選：時間戳
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<u64>,
}

/// 健康檢查響應
#[derive(Deserialize, Debug)]
pub struct HealthCheckResponse {
    /// 節點狀態（"healthy", "degraded", "unhealthy"）
    pub status: String,

    /// 當前 epoch
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_epoch: Option<u64>,

    /// 存儲的 Blob 數量
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blob_count: Option<u64>,

    /// 節點版本
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
}

/// 存儲節點客戶端
///
/// 封裝與單個 Walrus 存儲節點的所有 HTTP 交互
pub struct StorageNodeClient {
    /// HTTP 客戶端
    http_client: Client,

    /// 存儲節點的 HTTP API 基礎 URL
    /// 例如: "http://node1.walrus.network:8080"
    base_url: String,

    /// 最大重試次數
    max_retries: u32,

    /// 請求超時時間
    timeout: Duration,
}

impl StorageNodeClient {
    /// 創建新的存儲節點客戶端
    ///
    /// # 參數
    /// - `base_url`: 存儲節點 API 基礎 URL
    ///
    /// # 示例
    /// ```no_run
    /// use auditor_node::storage_node_client::StorageNodeClient;
    ///
    /// let client = StorageNodeClient::new(
    ///     "http://storage-node-1.walrus.network:8080".to_string()
    /// );
    /// ```
    pub fn new(base_url: String) -> Self {
        let http_client = Client::builder()
            .timeout(Duration::from_secs(DEFAULT_TIMEOUT_SECS))
            .build()
            .expect("Failed to build HTTP client");

        info!("Created StorageNodeClient for {}", base_url);

        Self {
            http_client,
            base_url,
            max_retries: DEFAULT_MAX_RETRIES,
            timeout: Duration::from_secs(DEFAULT_TIMEOUT_SECS),
        }
    }

    /// 創建帶自定義配置的客戶端
    pub fn with_config(base_url: String, timeout_secs: u64, max_retries: u32) -> Self {
        let http_client = Client::builder()
            .timeout(Duration::from_secs(timeout_secs))
            .build()
            .expect("Failed to build HTTP client");

        info!(
            "Created StorageNodeClient for {} (timeout: {}s, max_retries: {})",
            base_url, timeout_secs, max_retries
        );

        Self {
            http_client,
            base_url,
            max_retries,
            timeout: Duration::from_secs(timeout_secs),
        }
    }

    /// 向存儲節點發送挑戰
    ///
    /// 請求特定 Blob 的特定 Sliver，並獲取默克爾證明
    ///
    /// # 參數
    /// - `blob_id`: Blob ID（字符串格式）
    /// - `sliver_index`: Sliver 索引（0 到 n-1）
    ///
    /// # 返回
    /// - `Ok(ChallengeResponse)`: Sliver 數據和默克爾證明
    /// - `Err(AuditorError)`: 網絡錯誤或節點返回錯誤
    ///
    /// # 重試邏輯
    /// - 網絡超時: 重試
    /// - 連接失敗: 重試
    /// - HTTP 4xx: 不重試（客戶端錯誤）
    /// - HTTP 5xx: 重試（服務器錯誤）
    ///
    /// # 示例
    /// ```no_run
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # use auditor_node::storage_node_client::StorageNodeClient;
    /// let client = StorageNodeClient::new("http://node.walrus.network:8080".to_string());
    ///
    /// let response = client.challenge("blob_id_here", 0).await?;
    /// println!("Received {} bytes of sliver data", response.sliver_data.len());
    /// # Ok(())
    /// # }
    /// ```
    pub async fn challenge(
        &self,
        blob_id: &str,
        sliver_index: u64,
    ) -> Result<ChallengeResponse> {
        let request = ChallengeRequest {
            blob_id: blob_id.to_string(),
            sliver_index,
            signature: None,
        };

        info!(
            "Challenging storage node {} for blob {} sliver {}",
            self.base_url, blob_id, sliver_index
        );

        // 帶重試的請求
        self.challenge_with_retry(request).await
    }

    /// 帶重試邏輯的挑戰請求
    async fn challenge_with_retry(&self, request: ChallengeRequest) -> Result<ChallengeResponse> {
        let url = format!("{}/v1/challenge", self.base_url);

        for attempt in 0..=self.max_retries {
            if attempt > 0 {
                // 指數退避: 2^(attempt-1) 秒
                let delay_secs = 2u64.pow(attempt - 1);
                warn!(
                    "Retry attempt {}/{} after {}s delay",
                    attempt, self.max_retries, delay_secs
                );
                tokio::time::sleep(Duration::from_secs(delay_secs)).await;
            }

            debug!(
                "Sending challenge (attempt {}/{}): {:?}",
                attempt + 1,
                self.max_retries + 1,
                request
            );

            match self.send_challenge_request(&url, &request).await {
                Ok(response) => {
                    info!(
                        "Challenge successful on attempt {}: received {} bytes",
                        attempt + 1,
                        response.sliver_data.len()
                    );
                    return Ok(response);
                }
                Err(e) => {
                    if attempt == self.max_retries {
                        error!("Challenge failed after {} attempts: {}", attempt + 1, e);
                        return Err(e);
                    }

                    // 判斷是否應該重試
                    if !self.should_retry(&e) {
                        error!("Non-retryable error: {}", e);
                        return Err(e);
                    }

                    warn!("Challenge attempt {} failed (retryable): {}", attempt + 1, e);
                }
            }
        }

        unreachable!()
    }

    /// 實際發送 HTTP 請求
    async fn send_challenge_request(
        &self,
        url: &str,
        request: &ChallengeRequest,
    ) -> Result<ChallengeResponse> {
        let response = self
            .http_client
            .post(url)
            .json(request)
            .send()
            .await
            .map_err(|e| {
                if e.is_timeout() {
                    AuditorError::StorageNodeUnreachable(format!(
                        "{}: request timeout after {}s",
                        self.base_url,
                        self.timeout.as_secs()
                    ))
                } else if e.is_connect() {
                    AuditorError::StorageNodeUnreachable(format!(
                        "{}: connection failed - {}",
                        self.base_url, e
                    ))
                } else {
                    AuditorError::StorageNodeUnreachable(format!("{}: {}", self.base_url, e))
                }
            })?;

        let status = response.status();

        if !status.is_success() {
            let error_body = response
                .text()
                .await
                .unwrap_or_else(|_| "Unable to read error body".to_string());

            return Err(if status.is_client_error() {
                // 4xx - 客戶端錯誤（不重試）
                AuditorError::InvalidSliver(format!(
                    "HTTP {}: {} - {}",
                    status, status.canonical_reason().unwrap_or("Unknown"), error_body
                ))
            } else {
                // 5xx - 服務器錯誤（可重試）
                AuditorError::StorageNodeUnreachable(format!(
                    "HTTP {}: {} - {}",
                    status, status.canonical_reason().unwrap_or("Unknown"), error_body
                ))
            });
        }

        let challenge_response = response.json::<ChallengeResponse>().await.map_err(|e| {
            AuditorError::Serialization(format!("Failed to parse challenge response: {}", e))
        })?;

        // 驗證響應有效性
        if challenge_response.sliver_data.is_empty() {
            return Err(AuditorError::InvalidSliver(
                "Received empty sliver data".to_string(),
            ));
        }

        if challenge_response.merkle_proof.is_empty() {
            warn!("Received empty merkle proof - this may be invalid");
        }

        debug!(
            "Challenge response: {} bytes sliver, {} bytes proof",
            challenge_response.sliver_data.len(),
            challenge_response.merkle_proof.len()
        );

        Ok(challenge_response)
    }

    /// 判斷錯誤是否應該重試
    fn should_retry(&self, error: &AuditorError) -> bool {
        match error {
            // 網絡錯誤 - 重試
            AuditorError::StorageNodeUnreachable(_) => true,

            // 邏輯錯誤 - 不重試
            AuditorError::InvalidSliver(_) => false,
            AuditorError::MerkleVerificationFailed => false,
            AuditorError::Serialization(_) => false,

            // 其他錯誤 - 不重試
            _ => false,
        }
    }

    /// 測試存儲節點是否在線
    ///
    /// # 返回
    /// - `Ok(true)`: 節點在線且健康
    /// - `Ok(false)`: 節點離線或不健康
    /// - `Err(_)`: 網絡錯誤
    pub async fn health_check(&self) -> Result<bool> {
        let url = format!("{}/health", self.base_url);

        debug!("Performing health check on {}", self.base_url);

        match self.http_client.get(&url).send().await {
            Ok(response) => {
                if response.status().is_success() {
                    // 嘗試解析詳細的健康狀態
                    if let Ok(health) = response.json::<HealthCheckResponse>().await {
                        info!(
                            "Storage node {} is {}: epoch={:?}, blobs={:?}",
                            self.base_url, health.status, health.current_epoch, health.blob_count
                        );
                        Ok(health.status == "healthy")
                    } else {
                        // 無法解析響應，但 HTTP 狀態成功
                        info!("Storage node {} responded with 200 OK", self.base_url);
                        Ok(true)
                    }
                } else {
                    warn!(
                        "Storage node {} health check failed: HTTP {}",
                        self.base_url,
                        response.status()
                    );
                    Ok(false)
                }
            }
            Err(e) => {
                warn!(
                    "Storage node {} is unreachable: {}",
                    self.base_url, e
                );
                Ok(false)
            }
        }
    }

    /// 獲取詳細的健康狀態
    pub async fn get_health_status(&self) -> Result<HealthCheckResponse> {
        let url = format!("{}/health", self.base_url);

        let response = self
            .http_client
            .get(&url)
            .send()
            .await
            .map_err(|e| {
                AuditorError::StorageNodeUnreachable(format!("{}: {}", self.base_url, e))
            })?;

        if !response.status().is_success() {
            return Err(AuditorError::StorageNodeUnreachable(format!(
                "HTTP {}",
                response.status()
            )));
        }

        response.json::<HealthCheckResponse>().await.map_err(|e| {
            AuditorError::Serialization(format!("Failed to parse health response: {}", e))
        })
    }

    /// 獲取節點基礎 URL
    pub fn base_url(&self) -> &str {
        &self.base_url
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let client = StorageNodeClient::new("http://localhost:8080".to_string());
        assert_eq!(client.base_url(), "http://localhost:8080");
        assert_eq!(client.max_retries, DEFAULT_MAX_RETRIES);
    }

    #[test]
    fn test_client_with_config() {
        let client =
            StorageNodeClient::with_config("http://localhost:8080".to_string(), 10, 5);
        assert_eq!(client.max_retries, 5);
        assert_eq!(client.timeout, Duration::from_secs(10));
    }

    #[test]
    fn test_should_retry() {
        let client = StorageNodeClient::new("http://localhost:8080".to_string());

        // 應該重試的錯誤
        assert!(client.should_retry(&AuditorError::StorageNodeUnreachable(
            "timeout".to_string()
        )));

        // 不應該重試的錯誤
        assert!(!client.should_retry(&AuditorError::InvalidSliver("bad index".to_string())));
        assert!(!client.should_retry(&AuditorError::MerkleVerificationFailed));
    }

    // 集成測試需要實際的存儲節點或 mockito
    #[tokio::test]
    #[ignore] // 需要實際的存儲節點
    async fn test_health_check_integration() {
        let client = StorageNodeClient::new("http://localhost:8080".to_string());
        let result = client.health_check().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    #[ignore] // 需要實際的存儲節點
    async fn test_challenge_integration() {
        let client = StorageNodeClient::new("http://localhost:8080".to_string());
        let result = client.challenge("test_blob_id", 0).await;
        // 根據實際情況驗證結果
        println!("{:?}", result);
    }
}
