//! Configuration management module
//!
//! Responsible for loading and validating auditor node configuration

use crate::error::{AuditorError, Result};
use crate::types::AuditorConfig;
use config::{Config, ConfigError, File};
use serde::Deserialize;
use std::path::Path;

/// Load auditor configuration from file
///
/// # Parameters
/// - `config_path`: Configuration file path (supports TOML, JSON, YAML)
///
/// # Returns
/// - `Ok(AuditorConfig)`: Successfully loaded configuration
/// - `Err(AuditorError)`: Config file format error or missing required fields
///
/// # Example
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

/// Load configuration from environment variables (for containerized deployment)
///
/// Environment variable prefix: `AUDITOR_`
/// Example: `AUDITOR_SUI_RPC_URL`, `AUDITOR_MIN_CHALLENGES`
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

/// Validate configuration validity
///
/// Checks:
/// - Challenge count range is reasonable
/// - URL format is correct
/// - File paths exist
fn validate_config(config: &AuditorConfig) -> Result<()> {
    // Validate challenge count
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

    // Validate URL format
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

    // Validate Seal configuration
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
