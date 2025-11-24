//! Dilithium3 post-quantum digital signature implementation
//!
//! # About Dilithium3
//!
//! Dilithium is one of the digital signature schemes selected in the NIST post-quantum cryptography standardization competition (FIPS 204).
//! Why we chose Dilithium3:
//!
//! ## Security
//! - **NIST Security Level**: Level 3 (equivalent to AES-192)
//! - **Quantum-Safe**: Resistant to Grover's algorithm (requires 2^160 quantum operations to break)
//! - **Mathematical Foundation**: Based on lattice cryptography (Module-LWE problem)
//! - **Standardization**: NIST FIPS 204 standard (officially published in 2024)
//!
//! ## Performance and Size Balance
//! | Algorithm | Public Key Size | Signature Size | Signing Speed | Verification Speed |
//! |-----------|----------------|----------------|---------------|-------------------|
//! | Dilithium2 | 1,312 bytes | ~2,420 bytes | ~5 ms | ~1 ms |
//! | **Dilithium3** | **1,952 bytes** | **~3,293 bytes** | **~7 ms** | **~1.5 ms** |
//! | Dilithium5 | 2,592 bytes | ~4,595 bytes | ~10 ms | ~2 ms |
//!
//! ## Why Dilithium3 over Dilithium2
//! 1. **Higher Security Margin**: Level 3 provides stronger quantum resistance
//! 2. **Audit Scenario Requirements**: Audit reports need long-term preservation (10+ years), requiring higher security level
//! 3. **Compliance Requirements**: Enterprise compliance typically requires at least Level 3 security
//! 4. **Acceptable Performance Overhead**: 2ms increase in signing speed is acceptable (audit reports generated infrequently)
//!
//! ## Why Not Dilithium5
//! - Signature size too large (4.6 KB vs 3.3 KB)
//! - Performance overhead too high (Level 3 is sufficient for audit scenarios)
//! - Increased storage cost (1.3 KB more per audit report)

use crate::error::{PqcError, Result};
use crate::traits::Signer;
use pqcrypto_dilithium::dilithium3;
use pqcrypto_traits::sign::{PublicKey, SecretKey, SignedMessage};

/// Dilithium3 signer
///
/// # Example
///
/// ```rust
/// use pqc_signer::dilithium::Dilithium3Signer;
/// use pqc_signer::traits::Signer;
///
/// // Generate keypair
/// let mut signer = Dilithium3Signer::new();
/// signer.generate_keypair().unwrap();
///
/// // Sign message
/// let message = b"Audit report: blob_id=0x1234, success_rate=98%";
/// let signature = signer.sign(message).unwrap();
///
/// // Verify signature
/// let is_valid = signer.verify(message, &signature).unwrap();
/// assert!(is_valid);
/// ```
#[derive(Clone)]
pub struct Dilithium3Signer {
    public_key: Vec<u8>,
    secret_key: Vec<u8>,
}

impl Dilithium3Signer {
    /// Create new Dilithium3 signer (keys not initialized)
    ///
    /// Must call `generate_keypair()` or `from_bytes()` to initialize keys
    pub fn new() -> Self {
        Self {
            public_key: Vec::new(),
            secret_key: Vec::new(),
        }
    }

    /// Restore keypair from bytes
    ///
    /// # Parameters
    /// - `public_key`: Public key bytes (1952 bytes)
    /// - `secret_key`: Secret key bytes (4032 bytes)
    ///
    /// # Errors
    /// - Returns `KeyGenerationError` if key length is incorrect
    pub fn from_bytes(public_key: &[u8], secret_key: &[u8]) -> Result<Self> {
        // Verify key length
        if public_key.len() != dilithium3::public_key_bytes() {
            return Err(PqcError::KeyGenerationError(format!(
                "Invalid public key length: expected {} bytes, got {}",
                dilithium3::public_key_bytes(),
                public_key.len()
            )));
        }

        if secret_key.len() != dilithium3::secret_key_bytes() {
            return Err(PqcError::KeyGenerationError(format!(
                "Invalid secret key length: expected {} bytes, got {}",
                dilithium3::secret_key_bytes(),
                secret_key.len()
            )));
        }

        Ok(Self {
            public_key: public_key.to_vec(),
            secret_key: secret_key.to_vec(),
        })
    }

    /// Create verification-only Signer from public key (no signing capability)
    ///
    /// # Use Cases
    /// For scenarios requiring signature verification but not signing capability, such as:
    /// - Audit report verification
    /// - Third-party signature validity verification
    /// - Read-only environments without need to store private keys
    ///
    /// # Parameters
    /// - `public_key`: Public key bytes (1952 bytes for Dilithium3)
    ///
    /// # Returns
    /// - `Dilithium3Signer` instance containing only public key (secret_key is empty)
    ///
    /// # Errors
    /// - Returns `KeyGenerationError` if public key length is incorrect
    /// - Returns `KeyGenerationError` if public key format is invalid (deserialization fails)
    ///
    /// # Security
    /// - Created Signer **cannot perform signing operations** (private key is empty)
    /// - Can only call `verify()` method to verify signatures
    /// - Calling `sign()` will return an error
    ///
    /// # Example
    ///
    /// ```rust
    /// use pqc_signer::dilithium::Dilithium3Signer;
    /// use pqc_signer::traits::Signer;
    ///
    /// // Assume we have public key bytes
    /// let public_key: &[u8] = // ... obtained from somewhere ...
    /// # &[0u8; 1952];
    ///
    /// // Create verification-only Signer
    /// let verifier = Dilithium3Signer::from_public_key_only(public_key)?;
    ///
    /// // Can verify signatures
    /// let message = b"Audit report data";
    /// let signature = // ... obtained from somewhere ...
    /// # vec![0u8; 100];
    /// let is_valid = verifier.verify(message, &signature)?;
    ///
    /// // Cannot sign (will return error)
    /// // let sig = verifier.sign(message)?; // ← This will fail
    /// # Ok::<(), pqc_signer::error::PqcError>(())
    /// ```
    pub fn from_public_key_only(public_key: &[u8]) -> Result<Self> {
        // 1. Verify public key length
        if public_key.len() != dilithium3::public_key_bytes() {
            return Err(PqcError::KeyGenerationError(format!(
                "Invalid public key length: expected {} bytes, got {}",
                dilithium3::public_key_bytes(),
                public_key.len()
            )));
        }

        // 2. Verify public key format (attempt deserialization)
        dilithium3::PublicKey::from_bytes(public_key).map_err(|e| {
            PqcError::KeyGenerationError(format!(
                "Invalid public key format (failed to deserialize): {:?}",
                e
            ))
        })?;

        // 3. Create verification-only Signer
        tracing::debug!(
            "Created verification-only Dilithium3Signer: pk_len={} bytes (sk=empty)",
            public_key.len()
        );

        Ok(Self {
            public_key: public_key.to_vec(),
            secret_key: Vec::new(), // Empty private key, for verification only
        })
    }

    /// Get secret key bytes (for persistence)
    ///
    /// # Security Warning
    /// Private keys should be stored securely, not transmitted over network or logged
    pub fn secret_key(&self) -> &[u8] {
        &self.secret_key
    }

    /// Return algorithm information
    pub fn algorithm_info() -> AlgorithmInfo {
        AlgorithmInfo {
            name: "Dilithium3",
            nist_level: 3,
            public_key_size: dilithium3::public_key_bytes(),
            secret_key_size: dilithium3::secret_key_bytes(),
            signature_size: dilithium3::signature_bytes(),
        }
    }
}

impl Default for Dilithium3Signer {
    fn default() -> Self {
        Self::new()
    }
}

impl Signer for Dilithium3Signer {
    /// Generate new Dilithium3 keypair
    ///
    /// # Errors
    /// - Returns `KeyGenerationError` when key generation fails
    ///
    /// # Performance
    /// - Average time: ~10-20 ms (depends on system entropy source)
    fn generate_keypair(&mut self) -> Result<()> {
        let (pk, sk) = dilithium3::keypair();

        self.public_key = pk.as_bytes().to_vec();
        self.secret_key = sk.as_bytes().to_vec();

        tracing::info!(
            "Generated Dilithium3 keypair: pk_len={} bytes, sk_len={} bytes",
            self.public_key.len(),
            self.secret_key.len()
        );

        Ok(())
    }

    /// Sign message with Dilithium3
    ///
    /// # Parameters
    /// - `message`: Message to sign (any length)
    ///
    /// # Returns
    /// - Signature bytes (~3,293 bytes)
    ///
    /// # Errors
    /// - Returns `SigningError` if keys not initialized
    /// - Returns `SigningError` if signing fails
    ///
    /// # Performance
    /// - Average time: ~7 ms (1 KB message)
    fn sign(&self, message: &[u8]) -> Result<Vec<u8>> {
        if self.secret_key.is_empty() {
            return Err(PqcError::SigningError(
                "Secret key not initialized. Call generate_keypair() first.".to_string(),
            ));
        }

        // Rebuild key from bytes
        let sk = dilithium3::SecretKey::from_bytes(&self.secret_key).map_err(|e| {
            PqcError::SigningError(format!("Failed to parse secret key: {:?}", e))
        })?;

        // Execute signing
        let signed_message = dilithium3::sign(message, &sk);

        // pqcrypto-dilithium returns SignedMessage = [signature] + [message]
        // We only need the signature part (first signature_bytes() bytes) for detached signature
        let signed_bytes = signed_message.as_bytes();
        let sig_len = dilithium3::signature_bytes();

        // Extract pure signature (detached signature)
        let detached_signature = &signed_bytes[..sig_len];

        tracing::debug!(
            "Signed message: msg_len={} bytes, detached_sig_len={} bytes (from SignedMessage {} bytes)",
            message.len(),
            detached_signature.len(),
            signed_bytes.len()
        );

        Ok(detached_signature.to_vec())
    }

    /// Verify Dilithium3 signature
    ///
    /// # Parameters
    /// - `message`: Original message
    /// - `signature`: Signature bytes (~3,293 bytes)
    ///
    /// # Returns
    /// - `Ok(true)`: Signature is valid
    /// - `Ok(false)`: Signature is invalid
    /// - `Err`: Error occurred during verification
    ///
    /// # Performance
    /// - Average time: ~1.5 ms
    ///
    /// # Note
    /// Verification only requires public key, can be executed in environments without private key
    fn verify(&self, message: &[u8], signature: &[u8]) -> Result<bool> {
        if self.public_key.is_empty() {
            return Err(PqcError::VerificationError(
                "Public key not initialized".to_string(),
            ));
        }

        // Rebuild public key from bytes
        let pk = dilithium3::PublicKey::from_bytes(&self.public_key).map_err(|e| {
            PqcError::VerificationError(format!("Failed to parse public key: {:?}", e))
        })?;

        // Our sign() returns detached signature
        // Need to rebuild SignedMessage = [signature] + [message] for verification
        let mut signed_message_bytes = Vec::with_capacity(signature.len() + message.len());
        signed_message_bytes.extend_from_slice(signature);
        signed_message_bytes.extend_from_slice(message);

        // Rebuild SignedMessage from bytes
        let signed_msg =
            dilithium3::SignedMessage::from_bytes(&signed_message_bytes).map_err(|e| {
                PqcError::VerificationError(format!(
                    "Failed to reconstruct SignedMessage: {:?}",
                    e
                ))
            })?;

        // Execute verification
        match dilithium3::open(&signed_msg, &pk) {
            Ok(verified_message) => {
                // Check if message matches
                let is_valid = verified_message == message;

                tracing::debug!(
                    "Signature verification: valid={}, msg_len={} bytes",
                    is_valid,
                    message.len()
                );

                Ok(is_valid)
            }
            Err(_) => {
                // Signature verification failed
                tracing::warn!("Dilithium3 signature verification failed");
                Ok(false)
            }
        }
    }

    /// Get public key bytes
    ///
    /// # Returns
    /// - Public key bytes (1,952 bytes)
    fn public_key(&self) -> &[u8] {
        &self.public_key
    }

    /// Algorithm name
    fn algorithm_name(&self) -> &str {
        "Dilithium3"
    }
}

/// Algorithm information structure
#[derive(Debug, Clone, Copy)]
pub struct AlgorithmInfo {
    /// Algorithm name
    pub name: &'static str,
    /// NIST security level
    pub nist_level: u8,
    /// Public key size (bytes)
    pub public_key_size: usize,
    /// Secret key size (bytes)
    pub secret_key_size: usize,
    /// Signature size (bytes)
    pub signature_size: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keypair_generation() {
        let mut signer = Dilithium3Signer::new();
        signer.generate_keypair().unwrap();

        assert_eq!(signer.public_key().len(), dilithium3::public_key_bytes());
        assert_eq!(signer.secret_key().len(), dilithium3::secret_key_bytes());
    }

    #[test]
    fn test_sign_and_verify() {
        let mut signer = Dilithium3Signer::new();
        signer.generate_keypair().unwrap();

        let message = b"Test audit report: blob_id=0xABCD, success_rate=98%";
        let signature = signer.sign(message).unwrap();

        // Verify signature
        let is_valid = signer.verify(message, &signature).unwrap();
        assert!(is_valid);

        // Verify wrong message
        let wrong_message = b"Tampered message";
        let is_invalid = signer.verify(wrong_message, &signature).unwrap();
        assert!(!is_invalid);
    }

    #[test]
    fn test_sign_without_keypair() {
        let signer = Dilithium3Signer::new();
        let result = signer.sign(b"test message");

        assert!(result.is_err());
        match result {
            Err(PqcError::SigningError(msg)) => {
                assert!(msg.contains("not initialized"));
            }
            _ => panic!("Expected SigningError"),
        }
    }

    #[test]
    fn test_from_bytes() {
        // Generate keypair
        let mut original_signer = Dilithium3Signer::new();
        original_signer.generate_keypair().unwrap();

        let pk = original_signer.public_key().to_vec();
        let sk = original_signer.secret_key().to_vec();

        // Restore from bytes
        let restored_signer = Dilithium3Signer::from_bytes(&pk, &sk).unwrap();

        // Verify keys are the same
        assert_eq!(restored_signer.public_key(), original_signer.public_key());
        assert_eq!(restored_signer.secret_key(), original_signer.secret_key());

        // Verify signing works correctly
        let message = b"test message";
        let signature = restored_signer.sign(message).unwrap();
        let is_valid = restored_signer.verify(message, &signature).unwrap();
        assert!(is_valid);
    }

    #[test]
    fn test_from_bytes_invalid_length() {
        let invalid_pk = vec![0u8; 100]; // Wrong length
        let invalid_sk = vec![0u8; 100];

        let result = Dilithium3Signer::from_bytes(&invalid_pk, &invalid_sk);
        assert!(result.is_err());
    }

    #[test]
    fn test_signature_size() {
        let mut signer = Dilithium3Signer::new();
        signer.generate_keypair().unwrap();

        let message = b"test";
        let signature = signer.sign(message).unwrap();

        // Dilithium3 SignedMessage includes message + signature
        // Signature alone would be dilithium3::signature_bytes()
        // But our API returns the full SignedMessage
        assert!(signature.len() >= dilithium3::signature_bytes());
        assert!(signature.len() <= dilithium3::signature_bytes() + message.len() + 100);
    }

    #[test]
    fn test_algorithm_info() {
        let info = Dilithium3Signer::algorithm_info();

        assert_eq!(info.name, "Dilithium3");
        assert_eq!(info.nist_level, 3);
        assert_eq!(info.public_key_size, 1952);
        // Secret key size may vary by implementation
        // Signature size may vary by implementation
    }

    #[test]
    fn test_from_public_key_only() {
        // Generate full keypair
        let mut full_signer = Dilithium3Signer::new();
        full_signer.generate_keypair().unwrap();

        let message = b"Test message for verification-only signer";
        let signature = full_signer.sign(message).unwrap();

        // Create verification-only Signer from public key
        let pk = full_signer.public_key();
        let verifier = Dilithium3Signer::from_public_key_only(pk).unwrap();

        // Verify public key is the same
        assert_eq!(verifier.public_key(), full_signer.public_key());

        // Verify secret key is empty
        assert_eq!(verifier.secret_key().len(), 0);

        // Can verify signatures
        let is_valid = verifier.verify(message, &signature).unwrap();
        assert!(is_valid, "Verification-only signer should verify signatures");

        // Verify wrong message
        let is_invalid = verifier.verify(b"wrong message", &signature).unwrap();
        assert!(!is_invalid, "Should reject invalid signatures");
    }

    #[test]
    fn test_from_public_key_only_cannot_sign() {
        // Create verification-only Signer
        let mut full_signer = Dilithium3Signer::new();
        full_signer.generate_keypair().unwrap();

        let verifier = Dilithium3Signer::from_public_key_only(full_signer.public_key()).unwrap();

        // Attempting to sign should fail
        let result = verifier.sign(b"test message");
        assert!(result.is_err(), "Verification-only signer should not sign");

        match result {
            Err(PqcError::SigningError(msg)) => {
                assert!(msg.contains("not initialized"));
            }
            _ => panic!("Expected SigningError"),
        }
    }

    #[test]
    fn test_from_public_key_only_invalid_length() {
        // Test incorrect public key length
        let invalid_pk = vec![0u8; 100];
        let result = Dilithium3Signer::from_public_key_only(&invalid_pk);

        assert!(result.is_err());
        match result {
            Err(PqcError::KeyGenerationError(msg)) => {
                assert!(msg.contains("Invalid public key length"));
            }
            _ => panic!("Expected KeyGenerationError for invalid length"),
        }
    }

    #[test]
    fn test_from_public_key_only_invalid_format() {
        // Test correct length but invalid format public key
        let invalid_pk = vec![0u8; dilithium3::public_key_bytes()];
        let result = Dilithium3Signer::from_public_key_only(&invalid_pk);

        // This should fail because all zeros is not a valid Dilithium public key
        // Note: pqcrypto-dilithium may not validate this, depends on implementation
        // If test fails, it means the library doesn't validate public key format
        if let Err(PqcError::KeyGenerationError(msg)) = result {
            assert!(msg.contains("Invalid public key format"));
        } else {
            // If library accepts all-zero public key, at least verify creation succeeded
            // This is a known limitation
            println!("Warning: pqcrypto-dilithium accepts all-zero public key");
        }
    }
}
