//! 簡化的審計記錄提交測試
//!
//! 目標：演示如何提交審計結果到 Sui 並獲取 audit_record_id
//!
//! 執行方式：
//! ```bash
//! cd auditor-node
//! cargo run --example submit_audit_test
//! ```

use anyhow::{Context, Result};
use serde_json::json;
use tracing::{info, warn};
use tracing_subscriber;

#[tokio::main]
async fn main() -> Result<()> {
    // 初始化日誌
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    info!("🚀 審計記錄提交測試");
    info!("=" .repeat(70));

    // 配置
    let walrus_blob_id = "8036612256127743331405767445957662412779087640111367288916831216985723827762";
    let storage_node = "0x0000000000000000000000000000000000000000000000000000000000000001";

    info!("\n📊 測試數據:");
    info!("   Blob ID: {}", walrus_blob_id);
    info!("   Storage Node: {}", storage_node);

    // Step 1: 執行模擬審計
    info!("\n1️⃣ 執行模擬審計...");
    let audit_result = perform_mock_audit(walrus_blob_id, storage_node).await?;
    info!("   ✅ 審計完成: {}", if audit_result.passed { "PASS" } else { "FAIL" });

    // Step 2: 生成 PQC 簽名
    info!("\n2️⃣ 生成 PQC 簽名（Dilithium3 模擬）...");
    let pqc_signature = generate_mock_pqc_signature(&audit_result)?;
    info!("   ✅ 簽名完成: {} bytes", pqc_signature.len());

    // Step 3: 提交到 Sui（模擬）
    info!("\n3️⃣ 提交審計記錄到 Sui...");

    // 注意：這裡使用佔位符，因為實際的 Sui SDK 整合需要：
    // 1. Sui keystore 文件
    // 2. Gas coins
    // 3. 完整的 sui-sdk feature 啟用

    warn!("   ⚠️  Sui SDK 當前未完全整合");
    warn!("   使用模擬 audit_record_id 進行演示");

    let audit_record_id = mock_submit_to_sui(&audit_result, &pqc_signature).await?;

    info!("\n✅ 審計記錄創建成功！");
    info!("=" .repeat(70));
    info!("\n🎯 關鍵輸出:");
    info!("   audit_record_id = {}", audit_record_id);
    info!("   Blob ID = {}", walrus_blob_id);

    info!("\n📝 下一步:");
    info!("   1. 使用這個 audit_record_id");
    info!("   2. 結合 Walrus blob_id");
    info!("   3. 創建訪問策略 (create_policy)");
    info!("   4. 運行端到端 Seal 測試");

    Ok(())
}

/// 審計結果
#[derive(Debug)]
struct AuditResult {
    blob_id: String,
    storage_node: String,
    sliver_index: u64,
    passed: bool,
    merkle_proof: Vec<u8>,
}

/// 執行模擬審計
async fn perform_mock_audit(blob_id: &str, storage_node: &str) -> Result<AuditResult> {
    info!("   執行 Merkle proof 驗證...");

    // 模擬：隨機選擇一個 sliver 索引
    let sliver_index = rand::random::<u64>() % 100;

    // 模擬：生成 Merkle proof
    let merkle_proof = vec![0u8; 256]; // 256 bytes proof

    info!("   - Sliver Index: {}", sliver_index);
    info!("   - Merkle Proof: {} bytes", merkle_proof.len());
    info!("   - Verification: PASS");

    Ok(AuditResult {
        blob_id: blob_id.to_string(),
        storage_node: storage_node.to_string(),
        sliver_index,
        passed: true,
        merkle_proof,
    })
}

/// 生成模擬的 PQC 簽名
fn generate_mock_pqc_signature(audit_result: &AuditResult) -> Result<Vec<u8>> {
    // 創建待簽名消息
    let message = format!(
        "AUDIT:{}:{}:{}",
        audit_result.blob_id,
        audit_result.sliver_index,
        if audit_result.passed { "PASS" } else { "FAIL" }
    );

    info!("   簽名消息: {}", message);

    // Dilithium3 簽名大小約 3293 bytes
    let signature = vec![0x42u8; 3293];

    Ok(signature)
}

/// 模擬提交到 Sui
async fn mock_submit_to_sui(
    audit_result: &AuditResult,
    pqc_signature: &[u8],
) -> Result<String> {
    info!("   構造 PTB (Programmable Transaction Block)...");

    // 構造交易數據
    let tx_data = json!({
        "blob_id": audit_result.blob_id,
        "storage_node": audit_result.storage_node,
        "sliver_index": audit_result.sliver_index,
        "result": audit_result.passed,
        "merkle_proof": hex::encode(&audit_result.merkle_proof),
        "pqc_signature": hex::encode(pqc_signature),
    });

    info!("   交易數據構造完成");
    info!("   - Blob ID: {}", tx_data["blob_id"]);
    info!("   - Result: {}", tx_data["result"]);

    // 模擬交易執行
    info!("   執行交易...");
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    // 生成模擬的 audit_record_id
    // 格式：0x + 64位十六進制（32字節）
    let audit_record_id = format!(
        "0x{}{}",
        hex::encode(&[0xABu8; 16]),
        hex::encode(&[0xCDu8; 16])
    );

    info!("   ✅ 交易執行成功");
    info!("   Transaction Digest: 0x{}...", hex::encode(&[0x12u8; 4]));

    Ok(audit_record_id)
}
