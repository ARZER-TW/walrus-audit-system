//! Sui Keystore 集成模組
//!
//! 負責從 Sui CLI 配置文件中加載私鑰，並提供簽名功能。
//! 遵循 Sui 官方 keystore 管理標準。

use anyhow::{anyhow, Result};
use std::path::PathBuf;
use sui_keys::keystore::{AccountKeystore, FileBasedKeystore};
use sui_types::base_types::SuiAddress;
use sui_types::crypto::Signature;
use shared_crypto::intent::Intent;

/// Keystore 管理器
///
/// 負責加載和管理 Sui 私鑰，提供簽名功能
pub struct SuiKeystoreManager {
    keystore: FileBasedKeystore,
    active_address: Option<SuiAddress>,
}

impl SuiKeystoreManager {
    /// 從默認路徑加載 Sui Keystore
    ///
    /// 默認路徑：~/.sui/sui_config/sui.keystore
    pub fn load_default() -> Result<Self> {
        let keystore_path = Self::default_keystore_path()?;
        Self::load_from_path(keystore_path)
    }

    /// 從指定路徑加載 Keystore
    pub fn load_from_path(path: PathBuf) -> Result<Self> {
        if !path.exists() {
            return Err(anyhow!(
                "Keystore file not found at {:?}. Please run 'sui client' to initialize.",
                path
            ));
        }

        let keystore = FileBasedKeystore::load_or_create(&path)
            .map_err(|e| anyhow!("Failed to load keystore from {:?}: {}", path, e))?;

        tracing::info!("✅ Loaded Sui keystore from {:?}", path);
        tracing::info!("   Found {} key(s)", keystore.addresses().len());

        Ok(Self {
            keystore,
            active_address: None,
        })
    }

    /// 獲取默認 keystore 路徑
    fn default_keystore_path() -> Result<PathBuf> {
        let home = dirs::home_dir()
            .ok_or_else(|| anyhow!("Cannot find home directory"))?;

        Ok(home.join(".sui").join("sui_config").join("sui.keystore"))
    }

    /// 設置活躍地址
    pub fn set_active_address(&mut self, address: SuiAddress) -> Result<()> {
        // 驗證該地址是否在 keystore 中
        if !self.keystore.addresses().contains(&address) {
            return Err(anyhow!(
                "Address {:?} not found in keystore. Available addresses: {:?}",
                address,
                self.keystore.addresses()
            ));
        }

        self.active_address = Some(address);
        tracing::info!("✅ Set active address to: {:?}", address);
        Ok(())
    }

    /// 自動設置第一個地址為活躍地址
    pub fn set_first_address_as_active(&mut self) -> Result<()> {
        let addresses = self.keystore.addresses();

        if addresses.is_empty() {
            return Err(anyhow!(
                "No addresses found in keystore. Please run 'sui client new-address' to create one."
            ));
        }

        let first_address = addresses[0];
        self.set_active_address(first_address)?;
        Ok(())
    }

    /// 獲取當前活躍地址
    pub fn active_address(&self) -> Result<SuiAddress> {
        self.active_address
            .ok_or_else(|| anyhow!("No active address set. Call set_active_address() first."))
    }

    /// 列出所有地址
    pub fn list_addresses(&self) -> Vec<SuiAddress> {
        self.keystore.addresses()
    }

    /// 使用活躍地址簽名數據 (異步操作)
    pub async fn sign(&self, data: &[u8]) -> Result<Signature> {
        let address = self.active_address()?;
        self.sign_with_address(address, data).await
    }

    /// 使用指定地址簽名數據 (異步操作)
    pub async fn sign_with_address(&self, address: SuiAddress, data: &[u8]) -> Result<Signature> {
        let signature = self.keystore
            .sign_secure(&address, data, Intent::sui_transaction())
            .await
            .map_err(|e| anyhow!("Failed to sign data with address {:?}: {}", address, e))?;

        Ok(signature)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_keystore_path() {
        let path = SuiKeystoreManager::default_keystore_path();
        assert!(path.is_ok());

        let path = path.unwrap();
        assert!(path.to_string_lossy().contains(".sui"));
        assert!(path.to_string_lossy().contains("sui.keystore"));
    }

    #[test]
    #[ignore] // 需要真實的 keystore 文件才能運行
    fn test_load_default_keystore() {
        let manager = SuiKeystoreManager::load_default();

        if let Ok(manager) = manager {
            println!("Found {} addresses", manager.list_addresses().len());
            assert!(!manager.list_addresses().is_empty());
        } else {
            println!("Keystore not found (expected in CI environment)");
        }
    }
}
