use pqcrypto_dilithium::dilithium3;

fn main() {
    println!("Dilithium3 sizes:");
    println!("  Public key: {} bytes", dilithium3::public_key_bytes());
    println!("  Secret key: {} bytes", dilithium3::secret_key_bytes());
    println!("  Signature:  {} bytes", dilithium3::signature_bytes());
}
