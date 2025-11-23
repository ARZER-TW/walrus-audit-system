//! PQC 密鑰管理與持久化模塊
//!
//! # 安全性注意事項
//!
//! ## 密鑰存儲
//!
//! 私鑰以**未加密**形式存儲在文件系統中：
//! - `pqc_public.key`: 公鑰（1952 bytes，可公開）
//! - `pqc_secret.key`: 私鑰（4032 bytes，**高度敏感**）
//!
//! ## 文件權限（Unix/Linux）
//!
//! - 私鑰文件自動設置為 `0o600`（僅所有者可讀寫）
//! - 公鑰文件設置為 `0o644`（所有者可讀寫，其他人只讀）
//!
//! ## 風險警告
//!
//! ⚠️ **當前實現的限制**:
//! - 私鑰未加密存儲（生產環境應使用硬件 HSM 或加密存儲）
//! - 沒有密鑰輪換機制
//! - 沒有審計日誌
//! - 沒有訪問控制（依賴操作系統文件權限）
//!
//! ## 生產環境建議
//!
//! 1. **使用硬件安全模塊 (HSM)** 存儲私鑰
//! 2. **加密私鑰文件**: 使用密碼或 KMS 加密
//! 3. **訪問控制**: 記錄所有密鑰訪問操作
//! 4. **密鑰輪換**: 定期更新密鑰對
//! 5. **備份**: 安全備份私鑰（加密後異地存儲）
//!
//! # 使用示例
//!
//! ```no_run
//! use auditor_node::keystore::Keystore;
//! use std::path::Path;
//!
//! // 生成新密鑰對
//! let keystore_path = Path::new("./keys");
//! let keystore = Keystore::generate_and_save(keystore_path)?;
//!
//! // 下次啟動時加載
//! let keystore = Keystore::load(keystore_path)?;
//!
//! // 獲取公鑰用於分享
//! let public_key = keystore.public_key_bytes();
//! # Ok::<(), auditor_node::error::AuditorError>(())
//! ```

use crate::error::{AuditorError, Result};
use pqc_signer::{Dilithium3Signer, Signer};
use std::fs;
use std::path::{Path, PathBuf};
use tracing::{info, warn};

/// 密鑰庫：管理 Dilithium3 密鑰對的持久化存儲
///
/// # 文件結構
///
/// ```text
/// {base_path}/
///   ├── pqc_public.key  (1952 bytes, Dilithium3 公鑰)
///   └── pqc_secret.key  (4032 bytes, Dilithium3 私鑰, 僅所有者可讀)
/// ```
pub struct Keystore {
    /// Dilithium3 簽名器（包含公鑰和私鑰）
    signer: Dilithium3Signer,
    /// 密鑰存儲路徑（用於日誌和調試）
    base_path: PathBuf,
}

impl Keystore {
    /// 生成新的 Dilithium3 密鑰對並保存到文件
    ///
    /// # 參數
    ///
    /// - `base_path`: 密鑰文件存儲目錄（會自動創建）
    ///
    /// # 文件操作
    ///
    /// 1. 創建目錄（如果不存在）
    /// 2. 生成 Dilithium3 密鑰對
    /// 3. 保存公鑰到 `pqc_public.key`（權限 644）
    /// 4. 保存私鑰到 `pqc_secret.key`（權限 600）
    ///
    /// # 錯誤
    ///
    /// - 目錄創建失敗
    /// - 密鑰生成失敗
    /// - 文件寫入失敗
    /// - 文件權限設置失敗（僅 Unix）
    ///
    /// # 安全警告
    ///
    /// ⚠️ 私鑰以**明文**形式存儲！確保：
    /// - 存儲目錄位於加密文件系統
    /// - 定期備份（加密後）
    /// - 監控訪問日誌
    ///
    /// # 示例
    ///
    /// ```no_run
    /// # use auditor_node::keystore::Keystore;
    /// # use std::path::Path;
    /// let keystore = Keystore::generate_and_save(Path::new("./keys"))?;
    /// println!("公鑰長度: {} bytes", keystore.public_key_bytes().len());
    /// # Ok::<(), auditor_node::error::AuditorError>(())
    /// ```
    pub fn generate_and_save(base_path: &Path) -> Result<Self> {
        info!("Generating new Dilithium3 keypair at {:?}", base_path);

        // 步驟 1: 確保目錄存在
        fs::create_dir_all(base_path).map_err(|e| {
            AuditorError::Config(format!(
                "Failed to create keystore directory {:?}: {}",
                base_path, e
            ))
        })?;

        // 步驟 2: 生成 Dilithium3 密鑰對
        let mut signer = Dilithium3Signer::new();
        signer.generate_keypair().map_err(|e| {
            AuditorError::PqcSignature(format!("Failed to generate keypair: {}", e))
        })?;

        let public_key = signer.public_key();
        let secret_key = signer.secret_key();

        info!(
            "Generated Dilithium3 keypair: pk={} bytes, sk={} bytes",
            public_key.len(),
            secret_key.len()
        );

        // 步驟 3: 保存公鑰
        let public_path = base_path.join("pqc_public.key");
        fs::write(&public_path, public_key).map_err(|e| {
            AuditorError::Config(format!("Failed to write public key to {:?}: {}", public_path, e))
        })?;

        info!("Public key saved to {:?}", public_path);

        // 步驟 4: 保存私鑰
        let secret_path = base_path.join("pqc_secret.key");
        fs::write(&secret_path, secret_key).map_err(|e| {
            AuditorError::Config(format!("Failed to write secret key to {:?}: {}", secret_path, e))
        })?;

        info!("Secret key saved to {:?}", secret_path);

        // 步驟 5: 設置文件權限（僅 Unix/Linux）
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;

            // 設置私鑰為僅所有者可讀寫 (600)
            let mut secret_perms = fs::metadata(&secret_path)
                .map_err(|e| {
                    AuditorError::Config(format!("Failed to read secret key metadata: {}", e))
                })?
                .permissions();
            secret_perms.set_mode(0o600);
            fs::set_permissions(&secret_path, secret_perms).map_err(|e| {
                AuditorError::Config(format!("Failed to set secret key permissions: {}", e))
            })?;

            info!("Secret key permissions set to 0o600 (owner read/write only)");

            // 設置公鑰為所有者可讀寫，其他人只讀 (644)
            let mut public_perms = fs::metadata(&public_path)
                .map_err(|e| {
                    AuditorError::Config(format!("Failed to read public key metadata: {}", e))
                })?
                .permissions();
            public_perms.set_mode(0o644);
            fs::set_permissions(&public_path, public_perms).map_err(|e| {
                AuditorError::Config(format!("Failed to set public key permissions: {}", e))
            })?;

            info!("Public key permissions set to 0o644 (owner read/write, others read)");
        }

        #[cfg(not(unix))]
        {
            warn!("File permissions not set (non-Unix system). Ensure private key security manually!");
        }

        info!("Keypair successfully saved to {:?}", base_path);

        Ok(Self {
            signer,
            base_path: base_path.to_path_buf(),
        })
    }

    /// 從文件加載現有的 Dilithium3 密鑰對
    ///
    /// # 參數
    ///
    /// - `base_path`: 密鑰文件存儲目錄
    ///
    /// # 文件要求
    ///
    /// 必須存在以下兩個文件：
    /// - `{base_path}/pqc_public.key` (1952 bytes)
    /// - `{base_path}/pqc_secret.key` (4032 bytes)
    ///
    /// # 錯誤
    ///
    /// - 密鑰文件不存在
    /// - 文件讀取失敗
    /// - 密鑰格式無效（長度不正確）
    /// - 密鑰反序列化失敗
    ///
    /// # 示例
    ///
    /// ```no_run
    /// # use auditor_node::keystore::Keystore;
    /// # use std::path::Path;
    /// let keystore = Keystore::load(Path::new("./keys"))?;
    /// println!("Loaded keystore from disk");
    /// # Ok::<(), auditor_node::error::AuditorError>(())
    /// ```
    pub fn load(base_path: &Path) -> Result<Self> {
        info!("Loading Dilithium3 keypair from {:?}", base_path);

        let public_path = base_path.join("pqc_public.key");
        let secret_path = base_path.join("pqc_secret.key");

        // 步驟 1: 檢查文件是否存在
        if !public_path.exists() {
            return Err(AuditorError::Config(format!(
                "Public key file not found: {:?}",
                public_path
            )));
        }

        if !secret_path.exists() {
            return Err(AuditorError::Config(format!(
                "Secret key file not found: {:?}",
                secret_path
            )));
        }

        // 步驟 2: 讀取密鑰文件
        let public_key = fs::read(&public_path).map_err(|e| {
            AuditorError::Config(format!("Failed to read public key from {:?}: {}", public_path, e))
        })?;

        let secret_key = fs::read(&secret_path).map_err(|e| {
            AuditorError::Config(format!("Failed to read secret key from {:?}: {}", secret_path, e))
        })?;

        info!(
            "Read key files: pk={} bytes, sk={} bytes",
            public_key.len(),
            secret_key.len()
        );

        // 步驟 3: 驗證文件權限（僅 Unix）
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;

            let secret_perms = fs::metadata(&secret_path)
                .map_err(|e| {
                    AuditorError::Config(format!("Failed to read secret key metadata: {}", e))
                })?
                .permissions();

            let mode = secret_perms.mode() & 0o777;

            if mode != 0o600 {
                warn!(
                    "WARNING: Secret key file has insecure permissions: {:o} (should be 0o600)",
                    mode
                );
                warn!("Run: chmod 600 {:?}", secret_path);
            }
        }

        // 步驟 4: 從字節恢復密鑰對
        let signer = Dilithium3Signer::from_bytes(&public_key, &secret_key).map_err(|e| {
            AuditorError::PqcSignature(format!(
                "Failed to restore keypair from files: {}. Files may be corrupted.",
                e
            ))
        })?;

        info!("Keypair successfully loaded from {:?}", base_path);

        Ok(Self {
            signer,
            base_path: base_path.to_path_buf(),
        })
    }

    /// 獲取 Dilithium3 簽名器的引用
    ///
    /// # 返回
    ///
    /// 包含公鑰和私鑰的 `Dilithium3Signer` 引用
    ///
    /// # 用途
    ///
    /// - 簽名審計報告
    /// - 驗證簽名
    /// - 獲取公鑰/私鑰
    pub fn signer(&self) -> &Dilithium3Signer {
        &self.signer
    }

    /// 獲取公鑰字節（用於分享給驗證者）
    ///
    /// # 返回
    ///
    /// 1952 bytes 的 Dilithium3 公鑰
    ///
    /// # 用途
    ///
    /// - 分享給其他審計員進行報告驗證
    /// - 上傳到區塊鏈進行身份驗證
    /// - 發布到公共密鑰服務器
    ///
    /// # 示例
    ///
    /// ```no_run
    /// # use auditor_node::keystore::Keystore;
    /// # use std::path::Path;
    /// # let keystore = Keystore::load(Path::new("./keys"))?;
    /// let public_key = keystore.public_key_bytes();
    /// println!("Public key (hex): {}", hex::encode(&public_key[..16]));
    /// # Ok::<(), auditor_node::error::AuditorError>(())
    /// ```
    pub fn public_key_bytes(&self) -> Vec<u8> {
        self.signer.public_key().to_vec()
    }

    /// 獲取密鑰存儲路徑
    ///
    /// # 返回
    ///
    /// 密鑰文件存儲目錄的路徑
    pub fn base_path(&self) -> &Path {
        &self.base_path
    }
}

/// 檢查密鑰文件是否存在
///
/// # 參數
///
/// - `base_path`: 密鑰存儲目錄
///
/// # 返回
///
/// - `true`: 公鑰和私鑰文件都存在
/// - `false`: 至少有一個文件不存在
///
/// # 用途
///
/// 在應用啟動時決定是生成新密鑰還是加載現有密鑰
///
/// # 示例
///
/// ```no_run
/// use auditor_node::keystore::{Keystore, keystore_exists};
/// use std::path::Path;
///
/// let keystore_path = Path::new("./keys");
///
/// let keystore = if keystore_exists(keystore_path) {
///     println!("Loading existing keypair...");
///     Keystore::load(keystore_path)?
/// } else {
///     println!("Generating new keypair...");
///     Keystore::generate_and_save(keystore_path)?
/// };
/// # Ok::<(), auditor_node::error::AuditorError>(())
/// ```
pub fn keystore_exists(base_path: &Path) -> bool {
    let public_exists = base_path.join("pqc_public.key").exists();
    let secret_exists = base_path.join("pqc_secret.key").exists();

    public_exists && secret_exists
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    /// 創建臨時測試目錄
    fn create_temp_dir() -> PathBuf {
        let temp_base = env::temp_dir();
        let test_dir = temp_base.join(format!("pqc_keystore_test_{}", rand_id()));
        fs::create_dir_all(&test_dir).unwrap();
        test_dir
    }

    /// 生成隨機 ID（用於測試目錄名）
    fn rand_id() -> u64 {
        use std::time::{SystemTime, UNIX_EPOCH};
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64
    }

    #[test]
    fn test_keystore_generate_and_save() {
        let temp_dir = create_temp_dir();

        // 生成並保存密鑰
        let keystore = Keystore::generate_and_save(&temp_dir).unwrap();

        // 驗證公鑰長度
        assert_eq!(keystore.public_key_bytes().len(), 1952);

        // 驗證文件存在
        assert!(temp_dir.join("pqc_public.key").exists());
        assert!(temp_dir.join("pqc_secret.key").exists());

        // 清理
        fs::remove_dir_all(&temp_dir).ok();
    }

    #[test]
    fn test_keystore_load() {
        let temp_dir = create_temp_dir();

        // 生成密鑰
        let keystore1 = Keystore::generate_and_save(&temp_dir).unwrap();
        let public1 = keystore1.public_key_bytes();

        // 加載密鑰
        let keystore2 = Keystore::load(&temp_dir).unwrap();
        let public2 = keystore2.public_key_bytes();

        // 驗證公鑰一致
        assert_eq!(public1, public2);

        // 清理
        fs::remove_dir_all(&temp_dir).ok();
    }

    #[test]
    fn test_keystore_sign_and_verify() {
        let temp_dir = create_temp_dir();

        // 生成密鑰
        let keystore = Keystore::generate_and_save(&temp_dir).unwrap();

        // 簽名消息
        let message = b"Test audit report";
        let signature = keystore.signer().sign(message).unwrap();

        // 驗證簽名
        let is_valid = keystore.signer().verify(message, &signature).unwrap();
        assert!(is_valid);

        // 清理
        fs::remove_dir_all(&temp_dir).ok();
    }

    #[test]
    fn test_keystore_exists() {
        let temp_dir = create_temp_dir();

        // 初始不存在
        assert!(!keystore_exists(&temp_dir));

        // 生成後存在
        Keystore::generate_and_save(&temp_dir).unwrap();
        assert!(keystore_exists(&temp_dir));

        // 清理
        fs::remove_dir_all(&temp_dir).ok();
    }

    #[test]
    fn test_keystore_load_nonexistent() {
        let temp_dir = create_temp_dir();

        // 嘗試加載不存在的密鑰
        let result = Keystore::load(&temp_dir);
        assert!(result.is_err());

        match result {
            Err(AuditorError::Config(msg)) => {
                assert!(msg.contains("not found"));
            }
            _ => panic!("Expected Config error"),
        }

        // 清理
        fs::remove_dir_all(&temp_dir).ok();
    }

    #[test]
    #[cfg(unix)]
    fn test_secret_key_permissions() {
        use std::os::unix::fs::PermissionsExt;

        let temp_dir = create_temp_dir();

        // 生成密鑰
        Keystore::generate_and_save(&temp_dir).unwrap();

        // 檢查私鑰權限
        let secret_path = temp_dir.join("pqc_secret.key");
        let perms = fs::metadata(&secret_path).unwrap().permissions();
        let mode = perms.mode() & 0o777;

        assert_eq!(mode, 0o600, "Secret key should have 0o600 permissions");

        // 檢查公鑰權限
        let public_path = temp_dir.join("pqc_public.key");
        let perms = fs::metadata(&public_path).unwrap().permissions();
        let mode = perms.mode() & 0o777;

        assert_eq!(mode, 0o644, "Public key should have 0o644 permissions");

        // 清理
        fs::remove_dir_all(&temp_dir).ok();
    }

    #[test]
    fn test_keystore_persistence_roundtrip() {
        let temp_dir = create_temp_dir();

        // 生成並簽名
        let keystore1 = Keystore::generate_and_save(&temp_dir).unwrap();
        let message = b"Persistence test";
        let signature = keystore1.signer().sign(message).unwrap();

        // 加載並驗證
        let keystore2 = Keystore::load(&temp_dir).unwrap();
        let is_valid = keystore2.signer().verify(message, &signature).unwrap();

        assert!(is_valid, "Signature should be valid after reload");

        // 清理
        fs::remove_dir_all(&temp_dir).ok();
    }
}
