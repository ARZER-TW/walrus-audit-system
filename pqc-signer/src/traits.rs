/// Unified interface for post-quantum signatures
use crate::error::Result;

/// Signer trait
pub trait Signer {
    /// Generate keypair
    fn generate_keypair(&mut self) -> Result<()>;

    /// Sign message
    fn sign(&self, message: &[u8]) -> Result<Vec<u8>>;

    /// Verify signature
    fn verify(&self, message: &[u8], signature: &[u8]) -> Result<bool>;

    /// Get public key
    fn public_key(&self) -> &[u8];

    /// Algorithm name
    fn algorithm_name(&self) -> &str;
}
