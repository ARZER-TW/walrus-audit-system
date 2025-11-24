//! 網絡請求重試機制模組
//!
//! 實現指數退避（Exponential Backoff）策略，用於處理臨時網絡故障。
//! 參考 Walrus SDK 的 retry_client 模塊實現。

use anyhow::Result;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{debug, warn};

/// 重試配置
#[derive(Debug, Clone)]
pub struct RetryConfig {
    /// 最大重試次數
    pub max_retries: u32,
    /// 初始延遲時間（毫秒）
    pub initial_delay_ms: u64,
    /// 延遲增長倍數
    pub multiplier: f64,
    /// 最大延遲時間（毫秒）
    pub max_delay_ms: u64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 5,
            initial_delay_ms: 100,
            multiplier: 2.0,
            max_delay_ms: 10000,
        }
    }
}

impl RetryConfig {
    /// 創建保守的重試配置（適用於區塊鏈交易）
    pub fn conservative() -> Self {
        Self {
            max_retries: 3,
            initial_delay_ms: 1000,
            multiplier: 2.0,
            max_delay_ms: 30000,
        }
    }

    /// 創建激進的重試配置（適用於快速失敗場景）
    pub fn aggressive() -> Self {
        Self {
            max_retries: 10,
            initial_delay_ms: 50,
            multiplier: 1.5,
            max_delay_ms: 5000,
        }
    }
}

/// 使用指數退避策略重試操作
///
/// # 參數
///
/// * `operation_name` - 操作名稱（用於日誌）
/// * `config` - 重試配置
/// * `operation` - 要重試的異步操作
///
/// # 重試策略
///
/// 1. 初始延遲：`initial_delay_ms`
/// 2. 每次失敗後延遲翻倍：`delay = delay * multiplier`
/// 3. 延遲上限：`max_delay_ms`
/// 4. 最大重試次數：`max_retries`
///
/// # 範例
///
/// ```no_run
/// use auditor_node::retry::{retry_with_exponential_backoff, RetryConfig};
///
/// # async fn example() -> anyhow::Result<()> {
/// let config = RetryConfig::default();
///
/// let result = retry_with_exponential_backoff(
///     "fetch_data",
///     &config,
///     || async {
///         // 你的網絡請求
///         Ok::<_, anyhow::Error>(())
///     }
/// ).await?;
/// # Ok(())
/// # }
/// ```
pub async fn retry_with_exponential_backoff<F, Fut, T>(
    operation_name: &str,
    config: &RetryConfig,
    operation: F,
) -> Result<T>
where
    F: Fn() -> Fut,
    Fut: std::future::Future<Output = Result<T>>,
{
    let mut attempt = 0;
    let mut delay_ms = config.initial_delay_ms;

    loop {
        attempt += 1;

        debug!(
            operation = operation_name,
            attempt = attempt,
            max_retries = config.max_retries + 1,
            "Executing operation"
        );

        match operation().await {
            Ok(result) => {
                if attempt > 1 {
                    debug!(
                        operation = operation_name,
                        attempt = attempt,
                        "Operation succeeded after retry"
                    );
                }
                return Ok(result);
            }
            Err(e) => {
                if attempt > config.max_retries {
                    warn!(
                        operation = operation_name,
                        attempt = attempt,
                        error = %e,
                        "Operation failed after all retries"
                    );
                    return Err(e);
                }

                warn!(
                    operation = operation_name,
                    attempt = attempt,
                    delay_ms = delay_ms,
                    error = %e,
                    "Operation failed, retrying..."
                );

                sleep(Duration::from_millis(delay_ms)).await;

                // 計算下次延遲（指數退避）
                delay_ms = ((delay_ms as f64) * config.multiplier) as u64;
                delay_ms = delay_ms.min(config.max_delay_ms);
            }
        }
    }
}

/// 用於測試的同步重試函數
#[allow(dead_code)]
pub fn retry_sync<F, T>(
    operation_name: &str,
    config: &RetryConfig,
    mut operation: F,
) -> Result<T>
where
    F: FnMut() -> Result<T>,
{
    let mut attempt = 0;
    let mut delay_ms = config.initial_delay_ms;

    loop {
        attempt += 1;

        debug!(
            operation = operation_name,
            attempt = attempt,
            max_retries = config.max_retries + 1,
            "Executing operation"
        );

        match operation() {
            Ok(result) => {
                if attempt > 1 {
                    debug!(
                        operation = operation_name,
                        attempt = attempt,
                        "Operation succeeded after retry"
                    );
                }
                return Ok(result);
            }
            Err(e) => {
                if attempt > config.max_retries {
                    warn!(
                        operation = operation_name,
                        attempt = attempt,
                        error = %e,
                        "Operation failed after all retries"
                    );
                    return Err(e);
                }

                warn!(
                    operation = operation_name,
                    attempt = attempt,
                    delay_ms = delay_ms,
                    error = %e,
                    "Operation failed, retrying..."
                );

                std::thread::sleep(Duration::from_millis(delay_ms));

                delay_ms = ((delay_ms as f64) * config.multiplier) as u64;
                delay_ms = delay_ms.min(config.max_delay_ms);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::Arc;

    #[tokio::test]
    async fn test_retry_succeeds_immediately() {
        let config = RetryConfig::default();

        let result = retry_with_exponential_backoff("test_op", &config, || async {
            Ok::<_, anyhow::Error>(42)
        })
        .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
    }

    #[tokio::test]
    async fn test_retry_succeeds_after_failures() {
        let config = RetryConfig {
            max_retries: 3,
            initial_delay_ms: 10,
            multiplier: 2.0,
            max_delay_ms: 1000,
        };

        let counter = Arc::new(AtomicU32::new(0));
        let counter_clone = counter.clone();

        let result = retry_with_exponential_backoff("test_op", &config, || {
            let counter = counter_clone.clone();
            async move {
                let count = counter.fetch_add(1, Ordering::SeqCst);
                if count < 2 {
                    Err(anyhow::anyhow!("Temporary failure"))
                } else {
                    Ok(42)
                }
            }
        })
        .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
        assert_eq!(counter.load(Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn test_retry_fails_after_max_retries() {
        let config = RetryConfig {
            max_retries: 2,
            initial_delay_ms: 10,
            multiplier: 2.0,
            max_delay_ms: 1000,
        };

        let counter = Arc::new(AtomicU32::new(0));
        let counter_clone = counter.clone();

        let result = retry_with_exponential_backoff("test_op", &config, || {
            let counter = counter_clone.clone();
            async move {
                counter.fetch_add(1, Ordering::SeqCst);
                Err::<i32, _>(anyhow::anyhow!("Permanent failure"))
            }
        })
        .await;

        assert!(result.is_err());
        assert_eq!(counter.load(Ordering::SeqCst), 3); // 1 initial + 2 retries
    }

    #[test]
    fn test_retry_config_defaults() {
        let config = RetryConfig::default();
        assert_eq!(config.max_retries, 5);
        assert_eq!(config.initial_delay_ms, 100);
        assert_eq!(config.multiplier, 2.0);
        assert_eq!(config.max_delay_ms, 10000);
    }

    #[test]
    fn test_retry_config_conservative() {
        let config = RetryConfig::conservative();
        assert_eq!(config.max_retries, 3);
        assert_eq!(config.initial_delay_ms, 1000);
    }

    #[test]
    fn test_retry_config_aggressive() {
        let config = RetryConfig::aggressive();
        assert_eq!(config.max_retries, 10);
        assert_eq!(config.initial_delay_ms, 50);
    }
}
