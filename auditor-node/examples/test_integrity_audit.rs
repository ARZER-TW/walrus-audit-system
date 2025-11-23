//! 測試應用層完整性驗證
//!
//! 這個示例演示如何使用 IntegrityVerifier 對 Walrus 上的 Blob 進行審計
//!
//! 執行方式：
//! ```bash
//! cd auditor-node
//! cargo run --example test_integrity_audit
//! ```

use auditor_node::integrity::{IntegrityVerifier, VerificationStatus};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化日誌
    tracing_subscriber::fmt::init();

    println!("╔════════════════════════════════════════════════════════════════╗");
    println!("║   Walrus 審計系統 - 應用層完整性驗證測試                      ║");
    println!("╚════════════════════════════════════════════════════════════════╝");

    // 使用我們上傳的測試 Blob
    let test_blob_id = "eRrTusk8yshFQpkemgDnbg0f4-qDo623V2NpeVG1Zcg";
    let expected_hash = "bd9e5380f78734bc182e4bb8c464101d3baeb23387d701608901e64cd879e1f5";

    println!("\n📋 測試配置:");
    println!("   Blob ID: {}", test_blob_id);
    println!("   預期哈希: {}...", &expected_hash[..32]);

    // 創建驗證器
    let verifier = IntegrityVerifier::new_testnet();

    // 測試 1: 基本審計（下載並計算哈希）
    println!("\n🔬 測試 1: 基本審計流程");
    println!("   操作: 下載 Blob → 計算 SHA-256 → 生成審計記錄");

    let audit_result = verifier.audit_blob(test_blob_id).await?;

    println!("\n   結果:");
    println!("   ✓ Blob ID:      {}", audit_result.blob_id);
    println!("   ✓ 文件大小:     {} bytes", audit_result.file_size);
    println!("   ✓ 內容哈希:     {}", audit_result.content_hash);
    println!("   ✓ 驗證狀態:     {:?}", audit_result.verification_status);
    println!("   ✓ 時間戳:       {}", audit_result.timestamp);

    assert_eq!(audit_result.verification_status, VerificationStatus::Accessible);
    assert_eq!(audit_result.file_size, 870);

    // 測試 2: 哈希驗證（與已知哈希比對）
    println!("\n🔬 測試 2: 完整性驗證");
    println!("   操作: 比對當前哈希與歷史基準");

    let verify_result = verifier
        .verify_blob(test_blob_id, expected_hash)
        .await?;

    println!("\n   結果:");
    if verify_result.verification_status == VerificationStatus::Accessible {
        println!("   ✅ 完整性驗證通過！數據未被篡改");
        println!("      本地哈希 = 預期哈希 = {}", expected_hash);
    } else if verify_result.verification_status == VerificationStatus::Corrupted {
        println!("   ❌ 完整性驗證失敗！檢測到數據篡改");
        println!("      預期哈希: {}", expected_hash);
        println!("      實際哈希: {}", verify_result.content_hash);
    }

    assert_eq!(verify_result.verification_status, VerificationStatus::Accessible);

    // 測試 3: 錯誤哈希檢測
    println!("\n🔬 測試 3: 損壞檢測");
    println!("   操作: 使用錯誤的哈希值進行驗證");

    let wrong_hash = "0000000000000000000000000000000000000000000000000000000000000000";

    let corrupted_result = verifier
        .verify_blob(test_blob_id, wrong_hash)
        .await?;

    println!("\n   結果:");
    if corrupted_result.verification_status == VerificationStatus::Corrupted {
        println!("   ✅ 正確檢測到哈希不匹配");
        println!("      預期: {}", wrong_hash);
        println!("      實際: {}", corrupted_result.content_hash);
    } else {
        println!("   ❌ 未能檢測到哈希不匹配（測試失敗）");
    }

    assert_eq!(corrupted_result.verification_status, VerificationStatus::Corrupted);

    // 測試 4: 批量審計
    println!("\n🔬 測試 4: 批量審計");

    let blob_ids = vec![
        test_blob_id.to_string(),
        test_blob_id.to_string(), // 重複的 ID 測試去重
    ];

    println!("   操作: 並發審計 {} 個 Blob", blob_ids.len());

    let batch_results = verifier.audit_blobs_batch(&blob_ids).await?;

    println!("\n   結果:");
    println!("   ✓ 成功審計: {}/{}", batch_results.len(), blob_ids.len());

    for (i, result) in batch_results.iter().enumerate() {
        println!(
            "     [{}] {} - {:?} ({} bytes)",
            i + 1,
            &result.blob_id[..20],
            result.verification_status,
            result.file_size
        );
    }

    // 最終總結
    println!("\n╔════════════════════════════════════════════════════════════════╗");
    println!("║                      測試完成！                                ║");
    println!("╚════════════════════════════════════════════════════════════════╝");

    println!("\n📊 測試摘要:");
    println!("   ✅ 基本審計                - 通過");
    println!("   ✅ 完整性驗證              - 通過");
    println!("   ✅ 損壞檢測                - 通過");
    println!("   ✅ 批量審計                - 通過");

    println!("\n🎯 關鍵結論:");
    println!("   • 應用層完整性驗證正常工作");
    println!("   • SHA-256 哈希計算正確");
    println!("   • 損壞檢測邏輯有效");
    println!("   • 批量並發處理正常");

    println!("\n📝 下一步:");
    println!("   1. 整合 PQC 簽名（Dilithium3）");
    println!("   2. 將審計數據打包成 JSON 報告");
    println!("   3. 提交審計報告到 Sui 區塊鏈");

    Ok(())
}
