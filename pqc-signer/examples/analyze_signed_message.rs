//! Analyze Dilithium3 SignedMessage format
use pqcrypto_dilithium::dilithium3;
use pqcrypto_traits::sign::{PublicKey, SecretKey, SignedMessage};

fn main() {
    println!("=== Dilithium3 SignedMessage Format Analysis ===\n");

    // Generate keypair
    let (pk, sk) = dilithium3::keypair();

    // Test message
    let message = b"Test message";

    // Sign
    let signed_msg = dilithium3::sign(message, &sk);
    let signed_bytes = signed_msg.as_bytes();

    println!("Original message: {:?}", std::str::from_utf8(message).unwrap());
    println!("Original message length: {} bytes", message.len());
    println!("SignedMessage total length: {} bytes", signed_bytes.len());
    println!("Dilithium3 signature size constant: {} bytes", dilithium3::signature_bytes());
    println!();

    // Calculate difference
    let expected_total = dilithium3::signature_bytes() + message.len();
    println!("Expected total length (signature + message): {} bytes", expected_total);
    println!("Actual total length: {} bytes", signed_bytes.len());
    println!("Difference: {} bytes", signed_bytes.len() as i32 - expected_total as i32);
    println!();

    // Attempt verification
    match dilithium3::open(&signed_msg, &pk) {
        Ok(verified_msg) => {
            println!("✓ Verification successful!");
            println!("  Extracted message after verification: {:?}", std::str::from_utf8(&verified_msg).unwrap());
            println!("  Message matches: {}", verified_msg == message);
        }
        Err(_) => {
            println!("✗ Verification failed!");
        }
    }
    println!();

    // Check beginning and end of SignedMessage
    println!("=== SignedMessage First 64 bytes ===");
    let first_64 = &signed_bytes[..64.min(signed_bytes.len())];
    for (i, chunk) in first_64.chunks(16).enumerate() {
        print!("{:04x}: ", i * 16);
        for byte in chunk {
            print!("{:02x} ", byte);
        }
        println!();
    }
    println!();

    println!("=== SignedMessage Last 64 bytes ===");
    let start = signed_bytes.len().saturating_sub(64);
    let last_64 = &signed_bytes[start..];
    for (i, chunk) in last_64.chunks(16).enumerate() {
        print!("{:04x}: ", start + i * 16);
        for byte in chunk {
            print!("{:02x} ", byte);
        }
        // Display ASCII representation
        print!("  |");
        for byte in chunk {
            if *byte >= 32 && *byte <= 126 {
                print!("{}", *byte as char);
            } else {
                print!(".");
            }
        }
        println!("|");
    }
    println!();

    // Check if message is at end of SignedMessage
    let msg_at_end = &signed_bytes[signed_bytes.len() - message.len()..];
    println!("Check if message is at end of SignedMessage:");
    println!("  Last {} bytes equals original message: {}", message.len(), msg_at_end == message);
    if msg_at_end == message {
        println!("  ✓ Confirmed: SignedMessage = [signature] + [original message]");
    }
    println!();

    // Test signature randomness/determinism
    println!("=== Testing Signature Randomness ===");
    let signed_msg2 = dilithium3::sign(message, &sk);
    let signed_bytes2 = signed_msg2.as_bytes();

    if signed_bytes == signed_bytes2 {
        println!("✗ Same message produces **identical** SignedMessage");
        println!("  Conclusion: Dilithium3 signature is deterministic (abnormal!)");
    } else {
        println!("✓ Same message produces **different** SignedMessage");
        println!("  Conclusion: Dilithium3 signature contains randomness (normal behavior)");

        // Find differences
        let mut diff_count = 0;
        for (i, (b1, b2)) in signed_bytes.iter().zip(signed_bytes2.iter()).enumerate() {
            if b1 != b2 {
                diff_count += 1;
                if diff_count <= 5 {
                    println!("  Difference at position {}: {:02x} vs {:02x}", i, b1, b2);
                }
            }
        }
        println!("  Total {} bytes different", diff_count);
    }
}
