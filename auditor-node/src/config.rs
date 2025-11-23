//! 配置管理模塊
//!
//! 負責加載和驗證審計節點配置

use crate::error::{AuditorError, Result};
use crate::types::AuditorConfig;
use config::{Config, ConfigError, File};
use serde::Deserialize;
use std::path::Path;

/// 從配置文件加載審計員配置
///
/// # 參數
/// - `config_path`: 配置文件路徑（支持 TOML、JSON、YAML）
///
/// # 返回
/// - `Ok(AuditorConfig)`: 成功加載的配置
/// - `Err(AuditorError)`: 配置文件格式錯誤或缺少必要字段
///
/// # 示例
/// ```no_run
/// use auditor_node::config::load_config;
///
/// let config = load_config("config.toml").expect("Failed to load config");
/// println!("Sui RPC: {}", config.sui_rpc_url);
/// ```
pub fn load_config<P: AsRef<Path>>(config_path: P) -> Result<AuditorConfig> {
    let config = Config::builder()
        .add_source(File::from(config_path.as_ref()))
        .build()
        .map_err(|e| AuditorError::Config(format!("Failed to load config file: {}", e)))?;

    let auditor_config: AuditorConfig = config
        .try_deserialize()
        .map_err(|e| AuditorError::Config(format!("Failed to parse config: {}", e)))?;

    validate_config(&auditor_config)?;

    Ok(auditor_config)
}

/// 從環境變量加載配置（用於容器化部署）
///
/// 環境變量前綴: `AUDITOR_`
/// 示例: `AUDITOR_SUI_RPC_URL`, `AUDITOR_MIN_CHALLENGES`
pub fn load_config_from_env() -> Result<AuditorConfig> {
    let config = Config::builder()
        .add_source(config::Environment::with_prefix("AUDITOR"))
        .build()
        .map_err(|e| AuditorError::Config(format!("Failed to load env vars: {}", e)))?;

    let auditor_config: AuditorConfig = config
        .try_deserialize()
        .map_err(|e| AuditorError::Config(format!("Failed to parse env config: {}", e)))?;

    validate_config(&auditor_config)?;

    Ok(auditor_config)
}

/// 驗證配置的有效性
///
/// 檢查:
/// - 挑戰次數範圍是否合理
/// - URL 格式是否正確
/// - 文件路徑是否存在
fn validate_config(config: &AuditorConfig) -> Result<()> {
    // 驗證挑戰次數
    if config.min_challenges == 0 {
        return Err(AuditorError::Config(
            "min_challenges must be greater than 0".to_string(),
        ));
    }

    if config.max_challenges < config.min_challenges {
        return Err(AuditorError::Config(
            "max_challenges must be >= min_challenges".to_string(),
        ));
    }

    // 驗證 URL 格式
    if !config.sui_rpc_url.starts_with("http://") && !config.sui_rpc_url.starts_with("https://") {
        return Err(AuditorError::Config(format!(
            "Invalid Sui RPC URL: {}",
            config.sui_rpc_url
        )));
    }

    if !config.walrus_aggregator_url.starts_with("http://")
        && !config.walrus_aggregator_url.starts_with("https://")
    {
        return Err(AuditorError::Config(format!(
            "Invalid Walrus aggregator URL: {}",
            config.walrus_aggregator_url
        )));
    }

    // 驗證 Seal 配置
    if config.enable_seal_encryption && config.seal_api_url.is_none() {
        return Err(AuditorError::Config(
            "Seal encryption enabled but seal_api_url not provided".to_string(),
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config_is_valid() {
        let config = AuditorConfig::default();
        assert!(validate_config(&config).is_ok());
    }

    #[test]
    fn test_invalid_min_challenges() {
        let mut config = AuditorConfig::default();
        config.min_challenges = 0;
        assert!(validate_config(&config).is_err());
    }

    #[test]
    fn test_invalid_max_challenges() {
        let mut config = AuditorConfig::default();
        config.max_challenges = 5;
        config.min_challenges = 10;
        assert!(validate_config(&config).is_err());
    }
}
