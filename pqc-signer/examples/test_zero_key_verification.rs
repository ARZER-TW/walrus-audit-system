//! 測試用全零私鑰進行驗證會發生什麼
use pqcrypto_dilithium::dilithium3;
use pqcrypto_traits::sign::{PublicKey, SecretKey, SignedMessage};

fn main() {
    println!("=== 測試用全零私鑰的驗證行為 ===\n");

    // 1. 生成正常的密鑰對並簽名
    let (pk, sk) = dilithium3::keypair();
    let message = b"Test audit report";

    println!("步驟 1: 用正常密鑰對簽名");
    let signed_msg = dilithium3::sign(message, &sk);
    println!("✓ 簽名成功，SignedMessage 長度: {} bytes\n", signed_msg.as_bytes().len());

    // 2. 用正常的公鑰驗證（正確方式）
    println!("步驟 2: 用正確的公鑰驗證");
    match dilithium3::open(&signed_msg, &pk) {
        Ok(verified) => {
            println!("✓ 驗證成功");
            println!("  消息匹配: {}\n", verified == message);
        }
        Err(e) => {
            println!("✗ 驗證失敗: {:?}\n", e);
        }
    }

    // 3. 嘗試用全零私鑰創建 Signer
    println!("步驟 3: 嘗試用 (正確公鑰 + 全零私鑰) 創建 Signer");
    let pk_bytes = pk.as_bytes();
    let zero_sk = vec![0u8; dilithium3::secret_key_bytes()];

    println!("  公鑰長度: {} bytes (正確)", pk_bytes.len());
    println!("  全零私鑰長度: {} bytes", zero_sk.len());

    match dilithium3::SecretKey::from_bytes(&zero_sk) {
        Ok(invalid_sk) => {
            println!("  ⚠️  警告: from_bytes 接受了全零私鑰 (沒有驗證密鑰有效性!)\n");

            // 4. 嘗試用這個無效的私鑰創建 PublicKey (看看會不會破壞公鑰)
            println!("步驟 4: 用無效私鑰嘗試驗證");

            // 創建一個新的 SignedMessage 來測試
            match dilithium3::open(&signed_msg, &pk) {
                Ok(_) => {
                    println!("✓ 原始公鑰仍然可以驗證\n");
                }
                Err(e) => {
                    println!("✗ 原始公鑰無法驗證: {:?}\n", e);
                }
            }

            // 嘗試用全零私鑰簽名會發生什麼
            println!("步驟 5: 嘗試用全零私鑰簽名");
            let fake_signed = dilithium3::sign(message, &invalid_sk);
            println!("  ⚠️  全零私鑰可以簽名 (生成了 {} bytes)\n", fake_signed.as_bytes().len());

            println!("步驟 6: 嘗試用原始公鑰驗證全零私鑰的簽名");
            match dilithium3::open(&fake_signed, &pk) {
                Ok(_) => {
                    println!("  ✗ 嚴重錯誤: 全零私鑰的簽名被接受了!");
                }
                Err(_) => {
                    println!("  ✓ 正確: 全零私鑰的簽名被拒絕");
                }
            }
        }
        Err(e) => {
            println!("  ✓ from_bytes 拒絕了全零私鑰: {:?}", e);
        }
    }

    println!("\n=== 結論 ===");
    println!("問題分析:");
    println!("1. pqcrypto-dilithium 的 from_bytes() 可能不驗證密鑰有效性");
    println!("2. 使用全零私鑰可以創建 Signer，但不能用於有效驗證");
    println!("3. report.rs 的 verify_report() 使用全零私鑰是錯誤的設計");
    println!("\n正確做法:");
    println!("- 驗證只需要公鑰，不需要私鑰");
    println!("- 應該直接使用 dilithium3::PublicKey::from_bytes() + dilithium3::open()");
    println!("- 或者在 Dilithium3Signer 中添加 from_public_key_only() 方法");
}
