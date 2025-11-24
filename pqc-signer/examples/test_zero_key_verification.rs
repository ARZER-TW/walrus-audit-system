//! Test what happens when verifying with all-zero private key
use pqcrypto_dilithium::dilithium3;
use pqcrypto_traits::sign::{PublicKey, SecretKey, SignedMessage};

fn main() {
    println!("=== Testing Verification Behavior with All-Zero Private Key ===\n");

    // 1. Generate normal keypair and sign
    let (pk, sk) = dilithium3::keypair();
    let message = b"Test audit report";

    println!("Step 1: Sign with normal keypair");
    let signed_msg = dilithium3::sign(message, &sk);
    println!("✓ Signing successful, SignedMessage length: {} bytes\n", signed_msg.as_bytes().len());

    // 2. Verify with normal public key (correct approach)
    println!("Step 2: Verify with correct public key");
    match dilithium3::open(&signed_msg, &pk) {
        Ok(verified) => {
            println!("✓ Verification successful");
            println!("  Message matches: {}\n", verified == message);
        }
        Err(e) => {
            println!("✗ Verification failed: {:?}\n", e);
        }
    }

    // 3. Attempt to create Signer with all-zero private key
    println!("Step 3: Attempt to create Signer with (correct public key + all-zero private key)");
    let pk_bytes = pk.as_bytes();
    let zero_sk = vec![0u8; dilithium3::secret_key_bytes()];

    println!("  Public key length: {} bytes (correct)", pk_bytes.len());
    println!("  All-zero private key length: {} bytes", zero_sk.len());

    match dilithium3::SecretKey::from_bytes(&zero_sk) {
        Ok(invalid_sk) => {
            println!("  ⚠️  Warning: from_bytes accepted all-zero private key (no key validity verification!)\n");

            // 4. Try verifying with this invalid private key (see if it breaks public key)
            println!("Step 4: Attempt verification with invalid private key");

            // Create a new SignedMessage to test
            match dilithium3::open(&signed_msg, &pk) {
                Ok(_) => {
                    println!("✓ Original public key can still verify\n");
                }
                Err(e) => {
                    println!("✗ Original public key cannot verify: {:?}\n", e);
                }
            }

            // See what happens when signing with all-zero private key
            println!("Step 5: Attempt signing with all-zero private key");
            let fake_signed = dilithium3::sign(message, &invalid_sk);
            println!("  ⚠️  All-zero private key can sign (generated {} bytes)\n", fake_signed.as_bytes().len());

            println!("Step 6: Attempt to verify all-zero private key signature with original public key");
            match dilithium3::open(&fake_signed, &pk) {
                Ok(_) => {
                    println!("  ✗ Critical error: all-zero private key signature was accepted!");
                }
                Err(_) => {
                    println!("  ✓ Correct: all-zero private key signature was rejected");
                }
            }
        }
        Err(e) => {
            println!("  ✓ from_bytes rejected all-zero private key: {:?}", e);
        }
    }

    println!("\n=== Conclusion ===");
    println!("Problem Analysis:");
    println!("1. pqcrypto-dilithium's from_bytes() may not validate key validity");
    println!("2. Using all-zero private key can create Signer, but cannot be used for valid verification");
    println!("3. report.rs's verify_report() using all-zero private key is incorrect design");
    println!("\nCorrect Approach:");
    println!("- Verification only requires public key, not private key");
    println!("- Should directly use dilithium3::PublicKey::from_bytes() + dilithium3::open()");
    println!("- Or add from_public_key_only() method to Dilithium3Signer");
}
