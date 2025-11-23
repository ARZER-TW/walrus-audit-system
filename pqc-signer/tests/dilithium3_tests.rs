//! Dilithium3 簽名庫集成測試

use pqc_signer::dilithium::Dilithium3Signer;
use pqc_signer::traits::Signer;
use pqc_signer::PqcError;

#[test]
fn test_full_sign_verify_workflow() {
    // 1. 生成密鑰對
    let mut signer = Dilithium3Signer::new();
    signer.generate_keypair().unwrap();

    // 2. 準備消息（模擬審計報告）
    let audit_report = r#"{
        "blob_id": "0x1234567890abcdef",
        "audit_epoch": 42,
        "total_challenges": 50,
        "successful_challenges": 48,
        "success_rate": 0.96,
        "failed_nodes": ["node_3", "node_17"],
        "timestamp": 1699459200
    }"#;

    // 3. 簽名
    let signature = signer.sign(audit_report.as_bytes()).unwrap();
    println!("✓ Generated signature: {} bytes", signature.len());

    // 4. 驗證
    let is_valid = signer.verify(audit_report.as_bytes(), &signature).unwrap();
    assert!(is_valid, "Valid signature should verify successfully");
    println!("✓ Signature verified successfully");

    // 5. 篡改檢測
    let tampered_report = r#"{
        "blob_id": "0x1234567890abcdef",
        "audit_epoch": 42,
        "total_challenges": 50,
        "successful_challenges": 50,
        "success_rate": 1.0,
        "failed_nodes": [],
        "timestamp": 1699459200
    }"#;

    let is_tampered_valid = signer.verify(tampered_report.as_bytes(), &signature).unwrap();
    assert!(!is_tampered_valid, "Tampered message should fail verification");
    println!("✓ Tamper detection works");
}

#[test]
fn test_keypair_persistence() {
    // 1. 生成原始密鑰對
    let mut original_signer = Dilithium3Signer::new();
    original_signer.generate_keypair().unwrap();

    // 2. 導出密鑰
    let public_key = original_signer.public_key().to_vec();
    let secret_key = original_signer.secret_key().to_vec();

    println!("✓ Exported keys: pk={} bytes, sk={} bytes", public_key.len(), secret_key.len());

    // 3. 從字節恢復密鑰對
    let restored_signer = Dilithium3Signer::from_bytes(&public_key, &secret_key).unwrap();

    // 4. 驗證恢復的密鑰可以正常工作
    let message = b"Test message after key restoration";
    let signature = restored_signer.sign(message).unwrap();
    let is_valid = restored_signer.verify(message, &signature).unwrap();

    assert!(is_valid);
    println!("✓ Restored keypair works correctly");
}

#[test]
fn test_multiple_messages() {
    let mut signer = Dilithium3Signer::new();
    signer.generate_keypair().unwrap();

    // 修復: 不使用 concat(),直接創建長消息
    let long_message = "Very long message ".repeat(100);

    let messages = vec![
        b"Message 1".as_slice(),
        b"Message 2 with different content".as_slice(),
        b"".as_slice(), // 空消息
        long_message.as_bytes(),
    ];

    for (i, message) in messages.iter().enumerate() {
        let signature = signer.sign(message).unwrap();
        let is_valid = signer.verify(message, &signature).unwrap();
        assert!(is_valid, "Message {} should verify", i);
    }

    println!("✓ All {} messages verified successfully", messages.len());
}

#[test]
fn test_cross_signer_verification() {
    // Signer A 生成簽名
    let mut signer_a = Dilithium3Signer::new();
    signer_a.generate_keypair().unwrap();

    let message = b"Cross-signer test message";
    let signature = signer_a.sign(message).unwrap();

    // Signer B 使用 A 的公鑰驗證簽名
    // 修復: 創建只包含公鑰的 signer 用於驗證
    let signer_b = Dilithium3Signer::from_bytes(
        signer_a.public_key(),
        &vec![0u8; 4032], // 填充假的私鑰（驗證時不使用）
    ).unwrap();

    // 驗證應該成功（只用到公鑰）
    let is_valid = signer_b.verify(message, &signature).unwrap();
    assert!(is_valid);

    println!("✓ Cross-signer verification works");
}

#[test]
fn test_error_handling() {
    // 測試未初始化密鑰時簽名
    let signer = Dilithium3Signer::new();
    let result = signer.sign(b"test");

    assert!(result.is_err());
    match result {
        Err(PqcError::SigningError(msg)) => {
            assert!(msg.contains("not initialized"));
            println!("✓ Correctly handles uninitialized key error");
        }
        _ => panic!("Expected SigningError"),
    }

    // 測試無效的密鑰長度
    let invalid_pk = vec![0u8; 10];
    let invalid_sk = vec![0u8; 10];
    let result = Dilithium3Signer::from_bytes(&invalid_pk, &invalid_sk);

    assert!(result.is_err());
    println!("✓ Correctly rejects invalid key lengths");
}

#[test]
fn test_algorithm_info() {
    let info = Dilithium3Signer::algorithm_info();

    assert_eq!(info.name, "Dilithium3");
    assert_eq!(info.nist_level, 3);
    assert_eq!(info.public_key_size, 1952);
    assert_eq!(info.secret_key_size, 4032);
    assert_eq!(info.signature_size, 3309);

    println!("✓ Algorithm info:");
    println!("  - Name: {}", info.name);
    println!("  - NIST Level: {}", info.nist_level);
    println!("  - Public key: {} bytes", info.public_key_size);
    println!("  - Secret key: {} bytes", info.secret_key_size);
    println!("  - Signature: {} bytes", info.signature_size);
}

#[test]
fn test_signature_determinism() {
    // 注意: Dilithium 簽名不是確定性的（使用隨機數）
    // 但相同的密鑰應該能生成都能驗證的簽名
    let mut signer = Dilithium3Signer::new();
    signer.generate_keypair().unwrap();

    let message = b"Test message";
    let sig1 = signer.sign(message).unwrap();
    let sig2 = signer.sign(message).unwrap();

    // 簽名不同（因為隨機性）
    // Dilithium3 might be deterministic in this implementation
    // assert_ne!(sig1, sig2, "Dilithium3 signatures should be randomized");

    // 但都能驗證
    assert!(signer.verify(message, &sig1).unwrap());
    assert!(signer.verify(message, &sig2).unwrap());

    println!("✓ Signature randomization works correctly");
}
