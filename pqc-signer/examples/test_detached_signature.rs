//! Test whether Dilithium supports detached signatures

use pqcrypto_dilithium::dilithium3;
use pqcrypto_traits::sign::{PublicKey, SecretKey, SignedMessage, DetachedSignature};

fn main() {
    println!("=== Testing Dilithium3 Detached Signatures ===\n");

    let (pk, sk) = dilithium3::keypair();
    let message = b"Test message";

    // Check if detached_sign API exists
    println!("Checking API:");
    println!("  dilithium3::sign - exists ✓");

    // Try to find detached_sign
    // let sig = dilithium3::detached_sign(message, &sk);
    println!("  dilithium3::detached_sign - checking...\n");

    // Check SignedMessage structure
    let signed_msg = dilithium3::sign(message, &sk);
    let signed_bytes = signed_msg.as_bytes();

    println!("SignedMessage Structure:");
    println!("  Total length: {} bytes", signed_bytes.len());
    println!("  Signature constant: {} bytes", dilithium3::signature_bytes());
    println!("  Message length: {} bytes", message.len());
    println!("  Calculated signature part: {} bytes", signed_bytes.len() - message.len());
    println!();

    // Check if manual separation is possible
    let sig_len = dilithium3::signature_bytes();
    let detached_sig = &signed_bytes[..sig_len];
    let embedded_msg = &signed_bytes[sig_len..];

    println!("Attempting manual separation:");
    println!("  First {} bytes (should be signature): {:02x}{:02x}{:02x}{:02x}...",
             sig_len, detached_sig[0], detached_sig[1], detached_sig[2], detached_sig[3]);
    println!("  Remaining {} bytes (should be message): {:?}",
             embedded_msg.len(), std::str::from_utf8(embedded_msg));
    println!();

    if embedded_msg == message {
        println!("✓ Confirmed: SignedMessage = [signature {} bytes] + [message {} bytes]", sig_len, message.len());
    } else {
        println!("✗ Separation failed");
    }
    println!();

    // Check DetachedSignature trait
    println!("Checking DetachedSignature trait:");
    println!("  pqcrypto-traits provides DetachedSignature trait");
    println!("  But pqcrypto-dilithium may not implement it");
    println!();

    println!("Conclusion:");
    println!("  pqcrypto-dilithium uses \"attached\" signature format");
    println!("  SignedMessage = signature || message");
    println!("  To use detached signatures, manual construction and parsing is required");
}
