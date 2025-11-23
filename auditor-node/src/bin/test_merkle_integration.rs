// ! 測試 Merkle 整合的獨立程式
//!
//! 這個程式用來測試完整的 Merkle Tree 審計流程

use anyhow::Result;
use auditor_node::integrity::{IntegrityVerifier, AuditData};
use tracing_subscriber;
use serde_json;

#[tokio::main]
async fn main() -> Result<()> {
    // 初始化日誌
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    println!("╔════════════════════════════════════════════════════════════════╗");
    println!("║           Merkle Tree 整合測試                                 ║");
    println!("╚════════════════════════════════════════════════════════════════╝\n");

    // 創建完整性驗證器
    let aggregator_url = "https://aggregator.walrus-testnet.walrus.space".to_string();
    let verifier = IntegrityVerifier::new(aggregator_url);

    // 測試之前上傳的真實 Blob (來自最近的審計記錄)
    let blob_id = "eRrTusk8yshFQpkemgDnbg0f4-qDo623V2NpeVG1Zcg";

    println!("📋 測試配置:");
    println!("   Blob ID: {}", blob_id);
    println!("   切片大小: 4096 bytes (4KB)");
    println!("   哈希算法: Blake2b-256");
    println!("   挑戰次數: min(10, leaf_count)\n");

    println!("🚀 開始審計...\n");

    // 執行審計（包含 Merkle 驗證）
    let audit_data = match verifier.audit_blob(blob_id).await {
        Ok(data) => {
            print_audit_results(&data);
            data
        }
        Err(e) => {
            eprintln!("❌ 審計失敗: {}", e);
            std::process::exit(1);
        }
    };

    println!("\n✅ 測試完成！");

    // 生成並保存簽名報告 (用於 Seal 加密)
    println!("\n📝 生成 PQC 簽名報告...");

    use pqc_signer::dilithium::Dilithium3Signer;
    use pqc_signer::traits::Signer as PqcSigner;
    use std::fs;

    // 創建簽名器
    let mut signer = Dilithium3Signer::new();
    signer.generate_keypair()?;

    // 創建簽名報告結構
    let signed_report = serde_json::json!({
        "audit_data": {
            "blob_id": audit_data.blob_id,
            "content_hash": audit_data.content_hash,
            "merkle_root": audit_data.merkle_root,
            "total_challenges": audit_data.total_challenges,
            "successful_verifications": audit_data.successful_verifications,
            "failed_verifications": audit_data.failed_verifications,
            "file_size": audit_data.file_size,
            "timestamp": audit_data.timestamp,
            "verification_status": format!("{:?}", audit_data.verification_status),
        },
        "signature": hex::encode(signer.sign(&serde_json::to_vec(&audit_data)?)?),
        "algorithm": "Dilithium3",
        "auditor_public_key": hex::encode(signer.public_key()),
        "report_timestamp": chrono::Utc::now().timestamp() as u64,
    });

    // 保存到臨時文件
    let report_path = "/tmp/signed_audit_report.json";
    fs::write(report_path, serde_json::to_string_pretty(&signed_report)?)?;

    println!("✅ 簽名報告已保存: {}", report_path);
    println!("\n💡 下一步: 使用 Seal 加密報告");
    println!("   cd seal-client && npx tsx encrypt-and-submit-report.ts\n");

    Ok(())
}

fn print_audit_results(data: &AuditData) {
    println!("╔════════════════════════════════════════════════════════════════╗");
    println!("║                    審計結果                                     ║");
    println!("╚════════════════════════════════════════════════════════════════╝\n");

    println!("📊 基本資訊:");
    println!("   Blob ID: {}", data.blob_id);
    println!("   文件大小: {} bytes", data.file_size);
    println!("   審計時間: {}", data.timestamp);
    println!();

    println!("🔐 哈希證明:");
    if data.content_hash.len() >= 16 {
        println!("   SHA-256 (應用層): {}...", &data.content_hash[..16]);
    } else {
        println!("   SHA-256 (應用層): {}", data.content_hash);
    }
    if data.merkle_root.len() >= 16 {
        println!("   Merkle Root (協議層): {}...", &data.merkle_root[..16]);
    } else {
        println!("   Merkle Root (協議層): {}", data.merkle_root);
    }
    println!();

    println!("🎯 Merkle 挑戰-響應統計:");
    println!("   總挑戰次數: {}", data.total_challenges);
    println!("   成功驗證: {}", data.successful_verifications);
    println!("   失敗驗證: {}", data.failed_verifications);

    let success_rate = if data.total_challenges > 0 {
        (data.successful_verifications as f64 / data.total_challenges as f64) * 100.0
    } else {
        0.0
    };
    println!("   成功率: {:.2}%", success_rate);
    println!();

    println!("✅ 驗證狀態: {:?}", data.verification_status);
}
