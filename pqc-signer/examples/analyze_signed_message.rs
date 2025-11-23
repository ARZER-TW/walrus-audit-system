//! 分析 Dilithium3 SignedMessage 格式
use pqcrypto_dilithium::dilithium3;
use pqcrypto_traits::sign::{PublicKey, SecretKey, SignedMessage};

fn main() {
    println!("=== Dilithium3 SignedMessage 格式分析 ===\n");

    // 生成密鑰對
    let (pk, sk) = dilithium3::keypair();

    // 測試消息
    let message = b"Test message";

    // 簽名
    let signed_msg = dilithium3::sign(message, &sk);
    let signed_bytes = signed_msg.as_bytes();

    println!("原始消息: {:?}", std::str::from_utf8(message).unwrap());
    println!("原始消息長度: {} bytes", message.len());
    println!("SignedMessage 總長度: {} bytes", signed_bytes.len());
    println!("Dilithium3 簽名大小常量: {} bytes", dilithium3::signature_bytes());
    println!();

    // 計算差異
    let expected_total = dilithium3::signature_bytes() + message.len();
    println!("預期總長度 (簽名 + 消息): {} bytes", expected_total);
    println!("實際總長度: {} bytes", signed_bytes.len());
    println!("差異: {} bytes", signed_bytes.len() as i32 - expected_total as i32);
    println!();

    // 嘗試驗證
    match dilithium3::open(&signed_msg, &pk) {
        Ok(verified_msg) => {
            println!("✓ 驗證成功!");
            println!("  驗證後提取的消息: {:?}", std::str::from_utf8(&verified_msg).unwrap());
            println!("  消息匹配: {}", verified_msg == message);
        }
        Err(_) => {
            println!("✗ 驗證失敗!");
        }
    }
    println!();

    // 檢查 SignedMessage 的開頭和結尾
    println!("=== SignedMessage 前 64 bytes ===");
    let first_64 = &signed_bytes[..64.min(signed_bytes.len())];
    for (i, chunk) in first_64.chunks(16).enumerate() {
        print!("{:04x}: ", i * 16);
        for byte in chunk {
            print!("{:02x} ", byte);
        }
        println!();
    }
    println!();

    println!("=== SignedMessage 後 64 bytes ===");
    let start = signed_bytes.len().saturating_sub(64);
    let last_64 = &signed_bytes[start..];
    for (i, chunk) in last_64.chunks(16).enumerate() {
        print!("{:04x}: ", start + i * 16);
        for byte in chunk {
            print!("{:02x} ", byte);
        }
        // 顯示 ASCII 表示
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

    // 檢查消息是否在 SignedMessage 末尾
    let msg_at_end = &signed_bytes[signed_bytes.len() - message.len()..];
    println!("檢查消息是否在 SignedMessage 末尾:");
    println!("  末尾 {} bytes 是否等於原始消息: {}", message.len(), msg_at_end == message);
    if msg_at_end == message {
        println!("  ✓ 確認: SignedMessage = [簽名] + [原始消息]");
    }
    println!();

    // 測試簽名的隨機性/確定性
    println!("=== 測試簽名隨機性 ===");
    let signed_msg2 = dilithium3::sign(message, &sk);
    let signed_bytes2 = signed_msg2.as_bytes();

    if signed_bytes == signed_bytes2 {
        println!("✗ 相同消息產生 **相同** SignedMessage");
        println!("  結論: Dilithium3 簽名是確定性的 (這不正常!)");
    } else {
        println!("✓ 相同消息產生 **不同** SignedMessage");
        println!("  結論: Dilithium3 簽名包含隨機性 (正常行為)");

        // 找出差異
        let mut diff_count = 0;
        for (i, (b1, b2)) in signed_bytes.iter().zip(signed_bytes2.iter()).enumerate() {
            if b1 != b2 {
                diff_count += 1;
                if diff_count <= 5 {
                    println!("  差異位置 {}: {:02x} vs {:02x}", i, b1, b2);
                }
            }
        }
        println!("  總共 {} bytes 不同", diff_count);
    }
}
