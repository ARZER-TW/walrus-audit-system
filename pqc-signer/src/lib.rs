//! 後量子密碼學簽名庫
//!
//! 提供 Dilithium3 和 Falcon-512 兩種 NIST 標準化的後量子數字簽名方案。
//!
//! # 快速開始
//!
//! ```rust
//! use pqc_signer::dilithium::Dilithium3Signer;
//! use pqc_signer::traits::Signer;
//!
//! // 生成密鑰對
//! let mut signer = Dilithium3Signer::new();
//! signer.generate_keypair().unwrap();
//!
//! // 簽名消息
//! let message = b"Audit report data";
//! let signature = signer.sign(message).unwrap();
//!
//! // 驗證簽名
//! let is_valid = signer.verify(message, &signature).unwrap();
//! assert!(is_valid);
//! ```

pub mod error;
pub mod falcon;
pub mod dilithium;
pub mod traits;

// Re-export commonly used types
pub use error::{PqcError, Result};
pub use dilithium::Dilithium3Signer;
pub use traits::Signer;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dilithium3_integration() {
        let mut signer = Dilithium3Signer::new();
        signer.generate_keypair().unwrap();

        let message = b"Integration test message";
        let signature = signer.sign(message).unwrap();
        let is_valid = signer.verify(message, &signature).unwrap();

        assert!(is_valid);
    }
}
