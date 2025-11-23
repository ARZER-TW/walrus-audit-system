/// Falcon-512 簽名實現
use crate::error::{PqcError, Result};
use crate::traits::Signer;

pub struct FalconSigner {
    public_key: Vec<u8>,
    secret_key: Vec<u8>,
}

impl FalconSigner {
    pub fn new() -> Self {
        Self {
            public_key: Vec::new(),
            secret_key: Vec::new(),
        }
    }
}

impl Default for FalconSigner {
    fn default() -> Self {
        Self::new()
    }
}

impl Signer for FalconSigner {
    fn generate_keypair(&mut self) -> Result<()> {
        // TODO: 實現 Falcon-512 密鑰生成
        // 使用 pqcrypto-falcon crate
        Err(PqcError::KeyGenerationError(
            "Falcon 密鑰生成尚未實現".to_string(),
        ))
    }

    fn sign(&self, _message: &[u8]) -> Result<Vec<u8>> {
        // TODO: 實現 Falcon-512 簽名
        Err(PqcError::SigningError("Falcon 簽名尚未實現".to_string()))
    }

    fn verify(&self, _message: &[u8], _signature: &[u8]) -> Result<bool> {
        // TODO: 實現 Falcon-512 驗證
        Err(PqcError::VerificationError(
            "Falcon 驗證尚未實現".to_string(),
        ))
    }

    fn public_key(&self) -> &[u8] {
        &self.public_key
    }

    fn algorithm_name(&self) -> &str {
        "Falcon-512"
    }
}
