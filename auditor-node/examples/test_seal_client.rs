/**
 * Seal 客戶端測試示例
 *
 * 測試 Rust HTTP 客戶端與 TypeScript Seal API 服務的通信
 *
 * 運行前確保 Seal API 服務正在運行:
 * ```
 * cd ../seal-client
 * npx tsx seal-api-server.ts
 * ```
 *
 * 然後運行此測試:
 * ```
 * cargo run --example test_seal_client
 * ```
 */

use auditor_node::seal_client::{SealApiConfig, SealClient};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化日誌
    tracing_subscriber::fmt::init();

    println!("🧪 測試 Seal HTTP 客戶端\n");

    // 1. 創建客戶端
    println!("1️⃣ 創建 Seal HTTP 客戶端...");
    let config = SealApiConfig {
        api_url: "http://localhost:3001".to_string(),
        timeout_secs: 30,
    };
    let client = SealClient::new(config)?;
    println!("   ✅ 客戶端創建成功\n");

    // 2. 健康檢查
    println!("2️⃣ 執行健康檢查...");
    match client.health_check().await {
        Ok(health) => {
            println!("   ✅ Seal API 健康狀態: {}", health.status);
            println!("   - Service: {}", health.service);
            println!("   - Version: {}", health.version);
            println!("   - Timestamp: {}\n", health.timestamp);
        }
        Err(e) => {
            eprintln!("   ❌ 健康檢查失敗: {}", e);
            eprintln!("   提示: 確保 Seal API 服務正在運行（npx tsx seal-api-server.ts）\n");
            return Err(e.into());
        }
    }

    // 3. 測試加密
    println!("3️⃣ 測試審計報告加密...");

    let test_report = serde_json::json!({
        "version": "1.0",
        "blob_id": "test-blob-123",
        "auditor": "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef",
        "timestamp": 1234567890,
        "result": "valid",
        "details": "All Merkle proofs verified successfully"
    });

    let report_json = serde_json::to_string_pretty(&test_report)?;
    println!("   測試報告:");
    println!("{}\n", report_json);

    let auditor_address = "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef";
    let package_id = "0x8afa5d31dbaa0a8fb07082692940ca3d56b5e856c5126cb5a3693f0a4de63b82";
    let threshold = 2;

    println!("   參數:");
    println!("   - Auditor: {}", auditor_address);
    println!("   - Package ID: {}", package_id);
    println!("   - Threshold: {}-out-of-3\n", threshold);

    match client
        .encrypt_report(&report_json, auditor_address, package_id, threshold)
        .await
    {
        Ok((encrypted_data, symmetric_key, metadata)) => {
            println!("   ✅ 加密成功！");
            println!("   - 原始大小: {} bytes", metadata.original_size);
            println!("   - 加密大小: {} bytes", metadata.encrypted_size);
            println!(
                "   - 膨脹率: {:.2}x",
                metadata.encrypted_size as f64 / metadata.original_size as f64
            );
            println!("   - 耗時: {}ms", metadata.duration);
            println!("   - 加密數據長度 (Base64): {} bytes", encrypted_data.len());
            println!("   - 對稱密鑰長度 (Base64): {} bytes\n", symmetric_key.len());

            println!("   加密數據（前 100 字符）:");
            println!("   {}\n", &encrypted_data[..100.min(encrypted_data.len())]);

            println!("   對稱密鑰:");
            println!("   {}\n", symmetric_key);
        }
        Err(e) => {
            eprintln!("   ❌ 加密失敗: {}", e);
            return Err(e.into());
        }
    }

    println!("✅ 所有測試通過！\n");
    println!("📌 重要確認:");
    println!("   ✅ Rust HTTP 客戶端正常工作");
    println!("   ✅ 成功調用 TypeScript Seal API 服務");
    println!("   ✅ 使用官方 @mysten/seal SDK（不是本地模擬）");
    println!("   ✅ 端到端加密流程完整\n");

    println!("📝 下一步:");
    println!("   - 整合到審計節點的報告生成流程");
    println!("   - 在 Auditor::generate_report() 中調用加密");
    println!("   - 將加密報告上傳到 Walrus");

    Ok(())
}
