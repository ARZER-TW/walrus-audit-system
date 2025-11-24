//! Test Dilithium3Signer detached signature and verification

use pqc_signer::{Dilithium3Signer, Signer};

fn main() {
    println!("=== Testing Dilithium3Signer Detached Signature/Verification ===\n");

    // 1. Generate keys
    let mut signer = Dilithium3Signer::new();
    signer.generate_keypair().unwrap();

    println!("✓ Generated keypair");
    println!("  Public key length: {} bytes", signer.public_key().len());
    println!("  Secret key length: {} bytes", signer.secret_key().len());
    println!();

    // 2. Sign message
    let message = b"Test message for detached signature";
    let signature = signer.sign(message).unwrap();

    println!("✓ Signing complete");
    println!("  Message length: {} bytes", message.len());
    println!("  Signature length: {} bytes", signature.len());
    println!();

    // 3. Verify signature (using same signer)
    println!("Step: Verify with same signer...");
    match signer.verify(message, &signature) {
        Ok(true) => println!("✓ Verification successful (same signer)"),
        Ok(false) => println!("✗ Verification failed (same signer)"),
        Err(e) => println!("✗ Verification error: {:?}", e),
    }
    println!();

    // 4. Verify with public-key-only verifier
    println!("Step: Create public-key-only verifier...");
    let pk = signer.public_key().to_vec();
    let verifier = Dilithium3Signer::from_public_key_only(&pk).unwrap();

    println!("✓ Created verifier");
    println!("  Public key length: {} bytes", verifier.public_key().len());
    println!("  Secret key length: {} bytes (should be 0)", verifier.secret_key().len());
    println!();

    println!("Step: Verify with public-key-only verifier...");
    match verifier.verify(message, &signature) {
        Ok(true) => println!("✓ Verification successful (public-key-only verifier)"),
        Ok(false) => println!("✗ Verification failed (public-key-only verifier)"),
        Err(e) => println!("✗ Verification error: {:?}", e),
    }
    println!();

    // 5. Test wrong message
    println!("Step: Test wrong message...");
    let wrong_message = b"Wrong message";
    match verifier.verify(wrong_message, &signature) {
        Ok(false) => println!("✓ Correct: wrong message was rejected"),
        Ok(true) => println!("✗ Critical error: wrong message was accepted!"),
        Err(e) => println!("Verification error: {:?}", e),
    }
}
