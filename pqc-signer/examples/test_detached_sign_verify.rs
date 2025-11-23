//! 測試 Dilithium3Signer 的分離式簽名和驗證

use pqc_signer::{Dilithium3Signer, Signer};

fn main() {
    println!("=== 測試 Dilithium3Signer 分離式簽名/驗證 ===\n");

    // 1. 生成密鑰
    let mut signer = Dilithium3Signer::new();
    signer.generate_keypair().unwrap();

    println!("✓ 生成密鑰對");
    println!("  公鑰長度: {} bytes", signer.public_key().len());
    println!("  私鑰長度: {} bytes", signer.secret_key().len());
    println!();

    // 2. 簽名消息
    let message = b"Test message for detached signature";
    let signature = signer.sign(message).unwrap();

    println!("✓ 簽名完成");
    println!("  消息長度: {} bytes", message.len());
    println!("  簽名長度: {} bytes", signature.len());
    println!();

    // 3. 驗證簽名（使用同一個 signer）
    println!("步驟: 用同一 signer 驗證...");
    match signer.verify(message, &signature) {
        Ok(true) => println!("✓ 驗證成功（同一 signer）"),
        Ok(false) => println!("✗ 驗證失敗（同一 signer）"),
        Err(e) => println!("✗ 驗證錯誤: {:?}", e),
    }
    println!();

    // 4. 用僅公鑰的 verifier 驗證
    println!("步驟: 創建僅公鑰 verifier...");
    let pk = signer.public_key().to_vec();
    let verifier = Dilithium3Signer::from_public_key_only(&pk).unwrap();

    println!("✓ 創建驗證器");
    println!("  公鑰長度: {} bytes", verifier.public_key().len());
    println!("  私鑰長度: {} bytes (應該為 0)", verifier.secret_key().len());
    println!();

    println!("步驟: 用僅公鑰 verifier 驗證...");
    match verifier.verify(message, &signature) {
        Ok(true) => println!("✓ 驗證成功（僅公鑰 verifier）"),
        Ok(false) => println!("✗ 驗證失敗（僅公鑰 verifier）"),
        Err(e) => println!("✗ 驗證錯誤: {:?}", e),
    }
    println!();

    // 5. 測試錯誤消息
    println!("步驟: 測試錯誤消息...");
    let wrong_message = b"Wrong message";
    match verifier.verify(wrong_message, &signature) {
        Ok(false) => println!("✓ 正確: 錯誤消息被拒絕"),
        Ok(true) => println!("✗ 嚴重錯誤: 錯誤消息被接受!"),
        Err(e) => println!("驗證錯誤: {:?}", e),
    }
}
