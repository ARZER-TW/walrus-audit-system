//! 測試 Dilithium3 SignedMessage 格式
use pqcrypto_dilithium::dilithium3;
use pqcrypto_traits::sign::{PublicKey, SecretKey, SignedMessage};

fn main() {
    // 生成密鑰對
    let (pk, sk) = dilithium3::keypair();

    // 測試消息
    let message = b"Test message";

    // 簽名
    let signed_msg = dilithium3::sign(message, &sk);
    let signed_bytes = signed_msg.as_bytes();

    println!("=== Dilithium3 簽名格式分析 ===");
    println!("原始消息長度: {} bytes", message.len());
    println!("SignedMessage 總長度: {} bytes", signed_bytes.len());
    println!("Dilithium3 簽名常量: {} bytes", dilithium3::signature_bytes());
    println!();

    // 計算差異
    let diff = signed_bytes.len() as i32 - message.len() as i32;
    println!("長度差異: {} bytes", diff);
    println!("SignedMessage = 簽名({}) + 消息({}) = {} bytes?",
             dilithium3::signature_bytes(),
             message.len(),
             dilithium3::signature_bytes() + message.len());
    println!();

    // 嘗試驗證
    match dilithium3::open(&signed_msg, &pk) {
        Ok(verified_msg) => {
            println!("✓ 驗證成功!");
            println!("驗證消息長度: {} bytes", verified_msg.len());
            println!("消息匹配: {}", verified_msg == message);
        }
        Err(_) => {
            println!("✗ 驗證失敗!");
        }
    }
    println!();

    // 檢查 SignedMessage 前後內容
    println!("=== SignedMessage 前 50 bytes (十六進制) ===");
    for (i, byte) in signed_bytes.iter().take(50).enumerate() {
        print!("{:02x} ", byte);
        if (i + 1) % 16 == 0 {
            println!();
        }
    }
    println!();
    println!();

    println!("=== SignedMessage 後 50 bytes (十六進制 + ASCII) ===");
    let start = signed_bytes.len().saturating_sub(50);
    for (i, byte) in signed_bytes.iter().skip(start).enumerate() {
        print!("{:02x} ", byte);
        if (i + 1) % 16 == 0 {
            println!();
        }
    }
    println!();
    println!();

    // 嘗試將後面的字節解析為 ASCII
    println!("=== 後 20 bytes 的 ASCII 表示 ===");
    let last_20 = &signed_bytes[signed_bytes.len()-20..];
    for &byte in last_20 {
        if byte >= 32 && byte <= 126 {
            print!("{}", byte as char);
        } else {
            print!(".");
        }
    }
    println!();
    println!();

    // 測試確定性
    println!("=== 測試簽名確定性 ===");
    let signed_msg2 = dilithium3::sign(message, &sk);
    let signed_bytes2 = signed_msg2.as_bytes();

    if signed_bytes == signed_bytes2 {
        println!("✓ 相同消息產生相同 SignedMessage (確定性)");
    } else {
        println!("✗ 相同消息產生不同 SignedMessage (隨機性)");

        // 找出差異位置
        for (i, (b1, b2)) in signed_bytes.iter().zip(signed_bytes2.iter()).enumerate() {
            if b1 != b2 {
                println!("  第 {} byte 不同: {:02x} vs {:02x}", i, b1, b2);
                if i < 10 {
                    break;
                }
            }
        }
    }
}
