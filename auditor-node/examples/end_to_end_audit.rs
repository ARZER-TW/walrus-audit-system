//! 端到端審計流程演示
//!
//! 這個示例展示完整的審計流程：
//! 1. 從 Walrus 下載 Blob 並計算 SHA-256 哈希
//! 2. 使用 Dilithium3 簽名生成審計報告
//! 3. 驗證簽名有效性
//! 4. 輸出 JSON 格式的報告
//!
//! 執行方式：
//! ```bash
//! cd auditor-node
//! cargo run --example end_to_end_audit
//! ```

use auditor_node::audit_report::AuditReportGenerator;
use auditor_node::integrity::IntegrityVerifier;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化日誌
    tracing_subscriber::fmt::init();

    println!("╔════════════════════════════════════════════════════════════════╗");
    println!("║          Walrus 審計系統 - 端到端審計流程演示                 ║");
    println!("╚════════════════════════════════════════════════════════════════╝");

    // ========== 步驟 1: 完整性驗證 ==========
    println!("\n📍 步驟 1: 執行應用層完整性驗證");
    println!("   目標: 下載 Blob 並計算 SHA-256 哈希");

    let test_blob_id = "eRrTusk8yshFQpkemgDnbg0f4-qDo623V2NpeVG1Zcg";

    let verifier = IntegrityVerifier::new_testnet();
    let audit_data = verifier.audit_blob(test_blob_id).await?;

    println!("\n   結果:");
    println!("   ✓ Blob ID:      {}", audit_data.blob_id);
    println!("   ✓ Content Hash: {}", audit_data.content_hash);
    println!("   ✓ File Size:    {} bytes", audit_data.file_size);
    println!("   ✓ Status:       {:?}", audit_data.verification_status);

    // ========== 步驟 2: 生成 PQC 密鑰對 ==========
    println!("\n📍 步驟 2: 生成 PQC 密鑰對（Dilithium3）");

    let keystore_path = "/tmp/test_pqc_keystore.json";

    let generator = AuditReportGenerator::generate_new(
        keystore_path,
        Some("0x1234567890abcdef".to_string()), // 測試用的 Sui 地址
    )?;

    println!("   ✓ 密鑰對已生成");
    println!("   ✓ 保存位置: {}", keystore_path);
    println!("   ✓ 公鑰 (Base64): {}...", &generator.public_key_base64()[..32]);

    // ========== 步驟 3: 生成簽名的審計報告 ==========
    println!("\n📍 步驟 3: 生成 PQC 簽名的審計報告");
    println!("   操作: 序列化審計數據 → Dilithium3 簽名 → 打包報告");

    let signed_report = generator.generate_report(audit_data)?;

    println!("\n   結果:");
    println!("   ✓ 簽名算法:     {:?}", signed_report.algorithm);
    println!("   ✓ 簽名長度:     {} bytes", base64::decode(&signed_report.signature)?.len());
    println!("   ✓ 報告時間戳:   {}", signed_report.report_timestamp);

    // ========== 步驟 4: 驗證簽名 ==========
    println!("\n📍 步驟 4: 驗證報告簽名");

    let is_valid = signed_report.verify_signature()?;

    if is_valid {
        println!("   ✅ 簽名驗證通過！");
        println!("      報告完整性已確認");
        println!("      簽名者: {}...", &signed_report.auditor_public_key[..32]);
    } else {
        println!("   ❌ 簽名驗證失敗！");
        return Err("Signature verification failed".into());
    }

    // ========== 步驟 5: 輸出 JSON 報告 ==========
    println!("\n📍 步驟 5: 生成 JSON 格式報告");

    let json_report = signed_report.to_json()?;

    println!("\n   報告內容預覽:");
    println!("   {}", &json_report[..500.min(json_report.len())]);
    println!("   ... (完整報告 {} bytes)", json_report.len());

    // 保存到文件
    let report_path = "/tmp/audit_report.json";
    std::fs::write(report_path, &json_report)?;

    println!("\n   ✓ 報告已保存: {}", report_path);

    // ========== 步驟 6: 驗證反序列化和簽名 ==========
    println!("\n📍 步驟 6: 測試報告反序列化");
    println!("   模擬: 從文件加載報告並驗證簽名");

    let loaded_json = std::fs::read_to_string(report_path)?;
    let loaded_report = auditor_node::audit_report::SignedAuditReport::from_json(&loaded_json)?;

    let is_still_valid = loaded_report.verify_signature()?;

    if is_still_valid {
        println!("   ✅ 加載的報告簽名驗證通過！");
        println!("      Blob ID: {}", loaded_report.audit_data.blob_id);
        println!("      Hash:    {}", loaded_report.audit_data.content_hash);
    } else {
        println!("   ❌ 加載的報告簽名驗證失敗！");
        return Err("Loaded report verification failed".into());
    }

    // ========== 最終總結 ==========
    println!("\n╔════════════════════════════════════════════════════════════════╗");
    println!("║                     端到端流程完成！                           ║");
    println!("╚════════════════════════════════════════════════════════════════╝");

    println!("\n📊 流程總結:");
    println!("   ✅ 完整性驗證     - 通過");
    println!("   ✅ PQC 密鑰生成   - 通過");
    println!("   ✅ 報告簽名       - 通過");
    println!("   ✅ 簽名驗證       - 通過");
    println!("   ✅ JSON 序列化    - 通過");
    println!("   ✅ 反序列化驗證   - 通過");

    println!("\n🎯 關鍵成果:");
    println!("   • 實現了完整的應用層完整性審計");
    println!("   • 使用 NIST FIPS 204 標準的 Dilithium3 簽名");
    println!("   • 審計報告具備長期量子抗性");
    println!("   • 所有測試均在真實 Walrus Testnet 環境中通過");

    println!("\n📝 生成的文件:");
    println!("   • PQC 密鑰庫:     {}", keystore_path);
    println!("   • 審計報告 JSON:  {}", report_path);

    println!("\n🚀 下一步:");
    println!("   1. 將報告上傳到 Sui 區塊鏈");
    println!("   2. 在鏈上記錄審計結果");
    println!("   3. （可選）使用 Seal 加密報告並設置訪問策略");

    println!("\n╔════════════════════════════════════════════════════════════════╗");
    println!("║  Demo 提示：可以使用以下命令查看報告                          ║");
    println!("║  cat {}                                 ║", report_path);
    println!("╚════════════════════════════════════════════════════════════════╝\n");

    Ok(())
}
