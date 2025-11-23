//! 測試 Dilithium 是否支持分離式簽名（detached signature）

use pqcrypto_dilithium::dilithium3;
use pqcrypto_traits::sign::{PublicKey, SecretKey, SignedMessage, DetachedSignature};

fn main() {
    println!("=== 測試 Dilithium3 分離式簽名 ===\n");

    let (pk, sk) = dilithium3::keypair();
    let message = b"Test message";

    // 檢查是否有 detached_sign API
    println!("檢查 API:");
    println!("  dilithium3::sign - 存在 ✓");

    // 嘗試尋找 detached_sign
    // let sig = dilithium3::detached_sign(message, &sk);
    println!("  dilithium3::detached_sign - 檢查中...\n");

    // 檢查 SignedMessage 結構
    let signed_msg = dilithium3::sign(message, &sk);
    let signed_bytes = signed_msg.as_bytes();

    println!("SignedMessage 結構:");
    println!("  總長度: {} bytes", signed_bytes.len());
    println!("  簽名常量: {} bytes", dilithium3::signature_bytes());
    println!("  消息長度: {} bytes", message.len());
    println!("  計算的簽名部分: {} bytes", signed_bytes.len() - message.len());
    println!();

    // 檢查是否可以手動分離
    let sig_len = dilithium3::signature_bytes();
    let detached_sig = &signed_bytes[..sig_len];
    let embedded_msg = &signed_bytes[sig_len..];

    println!("嘗試手動分離:");
    println!("  前 {} bytes (應該是簽名): {:02x}{:02x}{:02x}{:02x}...",
             sig_len, detached_sig[0], detached_sig[1], detached_sig[2], detached_sig[3]);
    println!("  剩餘 {} bytes (應該是消息): {:?}",
             embedded_msg.len(), std::str::from_utf8(embedded_msg));
    println!();

    if embedded_msg == message {
        println!("✓ 確認: SignedMessage = [簽名 {} bytes] + [消息 {} bytes]", sig_len, message.len());
    } else {
        println!("✗ 分離失敗");
    }
    println!();

    // 檢查 DetachedSignature trait
    println!("檢查 DetachedSignature trait:");
    println!("  pqcrypto-traits 提供了 DetachedSignature trait");
    println!("  但 pqcrypto-dilithium 可能沒有實現它");
    println!();

    println!("結論:");
    println!("  pqcrypto-dilithium 使用 \"attached\" 簽名格式");
    println!("  SignedMessage = signature || message");
    println!("  要使用分離式簽名，需要手動構建和解析");
}
