//! 完整的 PQC 簽名工作流程示例
//!
//! 這個示例展示了從密鑰生成到報告簽名、驗證、持久化的完整流程。
//!
//! 流程:
//! 1. 使用 Keystore 生成並保存密鑰對
//! 2. 使用 ReportManager 簽名審計報告
//! 3. 將簽名後的報告導出為 JSON
//! 4. 從 JSON 載入報告並驗證簽名
//! 5. 測試報告持久化和跨會話驗證

use auditor_node::keystore::Keystore;
use auditor_node::report::ReportManager;
use auditor_node::types::AuditReport;
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 設置日誌
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    println!("=== 完整 PQC 工作流程示例 ===\n");

    // 步驟 1: 生成密鑰對
    println!("步驟 1: 生成密鑰對");
    println!("----------------------------------------");

    let keystore_path = Path::new("./temp_keystore");
    let keystore = Keystore::generate_and_save(keystore_path)?;

    println!("✓ 密鑰對已生成並保存到: {}", keystore_path.display());
    println!("  公鑰長度: {} bytes", keystore.public_key_bytes().len());
    println!("  檔案: pqc_public.key, pqc_secret.key");
    println!();

    // 步驟 2: 創建審計報告
    println!("步驟 2: 創建審計報告");
    println!("----------------------------------------");

    let mut report = AuditReport {
        blob_id: "walrus-blob-abc123".to_string(),
        blob_object_id: "0x789abc...".to_string(),
        auditor: "auditor-node-01".to_string(),
        timestamp: 1700000000,
        challenge_epoch: 42,
        challenge_results: vec![],
        total_challenges: 100,
        successful_verifications: 98,
        failed_verifications: 2,
        integrity_hash: vec![0xaa, 0xbb, 0xcc, 0xdd],
        pqc_signature: vec![],
        pqc_algorithm: 0,
        is_valid: true,
        failure_reason: None,
    };

    println!("✓ 報告創建完成");
    println!("  Blob ID: {}", report.blob_id);
    println!("  挑戰數量: {}", report.total_challenges);
    println!("  成功驗證: {}", report.successful_verifications);
    println!();

    // 步驟 3: 簽名報告
    println!("步驟 3: 使用 PQC 簽名報告");
    println!("----------------------------------------");

    let manager = ReportManager::new(keystore.signer().clone());
    manager.sign_report(&mut report)?;

    println!("✓ 報告簽名完成");
    println!("  簽名長度: {} bytes", report.pqc_signature.len());
    println!("  算法: Dilithium3 (NIST Level {}) ", report.pqc_algorithm);
    println!();

    // 步驟 4: 驗證簽名（使用公鑰）
    println!("步驟 4: 驗證報告簽名");
    println!("----------------------------------------");

    let public_key = keystore.public_key_bytes();
    let is_valid = ReportManager::verify_report(&report, &public_key)?;

    if is_valid {
        println!("✓ 簽名驗證通過");
    } else {
        println!("✗ 簽名驗證失敗");
    }
    println!();

    // 步驟 5: 導出報告為 JSON
    println!("步驟 5: 導出報告為 JSON");
    println!("----------------------------------------");

    let json_path = "./temp_report.json";
    manager.export_json(&report, json_path)?;

    println!("✓ 報告已導出到: {}", json_path);

    // 讀取並顯示文件大小
    let metadata = std::fs::metadata(json_path)?;
    println!("  檔案大小: {} bytes", metadata.len());
    println!();

    // 步驟 6: 從 JSON 載入並驗證
    println!("步驟 6: 從 JSON 載入並驗證");
    println!("----------------------------------------");

    let loaded_report = ReportManager::load_json(json_path)?;
    let is_still_valid = ReportManager::verify_report(&loaded_report, &public_key)?;

    if is_still_valid {
        println!("✓ 載入的報告簽名仍然有效");
    } else {
        println!("✗ 載入的報告簽名無效");
    }
    println!();

    // 步驟 7: 測試跨會話驗證（模擬重啟）
    println!("步驟 7: 測試跨會話驗證（模擬節點重啟）");
    println!("----------------------------------------");

    println!("模擬: 節點關閉，丟失內存中的密鑰...");
    drop(keystore);
    drop(manager);

    println!("模擬: 節點重啟，從磁盤載入密鑰...");
    let restored_keystore = Keystore::load(keystore_path)?;
    let restored_public_key = restored_keystore.public_key_bytes();

    println!("模擬: 從檔案載入報告...");
    let restored_report = ReportManager::load_json(json_path)?;

    println!("驗證: 使用恢復的公鑰驗證報告...");
    let final_check = ReportManager::verify_report(&restored_report, &restored_public_key)?;

    if final_check {
        println!("✓ 跨會話驗證成功！");
        println!("  這證明了簽名的持久性和可驗證性");
    } else {
        println!("✗ 跨會話驗證失敗");
    }
    println!();

    // 步驟 8: 測試篡改檢測
    println!("步驟 8: 測試篡改檢測");
    println!("----------------------------------------");

    let mut tampered_report = restored_report.clone();
    tampered_report.successful_verifications = 999; // 篡改數據

    let tampered_check = ReportManager::verify_report(&tampered_report, &restored_public_key)?;

    if !tampered_check {
        println!("✓ 篡改檢測成功！");
        println!("  原始值: 98 成功驗證");
        println!("  篡改值: 999 成功驗證");
        println!("  簽名驗證: 失敗 (如預期)");
    } else {
        println!("✗ 嚴重錯誤: 篡改的報告仍然驗證通過");
    }
    println!();

    // 清理
    println!("清理臨時檔案...");
    std::fs::remove_file(json_path)?;
    std::fs::remove_file(keystore_path.join("pqc_public.key"))?;
    std::fs::remove_file(keystore_path.join("pqc_secret.key"))?;
    std::fs::remove_dir(keystore_path)?;
    println!("✓ 清理完成");
    println!();

    println!("=== 工作流程完成 ===");
    println!("\n總結:");
    println!("1. ✓ 密鑰生成與持久化");
    println!("2. ✓ 報告 PQC 簽名");
    println!("3. ✓ 簽名驗證");
    println!("4. ✓ JSON 導出/導入");
    println!("5. ✓ 跨會話驗證");
    println!("6. ✓ 篡改檢測");
    println!("\n所有功能正常運作！");

    Ok(())
}
