//! Post-quantum cryptography signature library
//!
//! Provides two NIST-standardized post-quantum digital signature schemes: Dilithium3 and Falcon-512.
//!
//! # Quick Start
//!
//! ```rust
//! use pqc_signer::dilithium::Dilithium3Signer;
//! use pqc_signer::traits::Signer;
//!
//! // Generate keypair
//! let mut signer = Dilithium3Signer::new();
//! signer.generate_keypair().unwrap();
//!
//! // Sign message
//! let message = b"Audit report data";
//! let signature = signer.sign(message).unwrap();
//!
//! // Verify signature
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
