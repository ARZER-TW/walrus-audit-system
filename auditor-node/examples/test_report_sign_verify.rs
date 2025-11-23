//! 測試報告簽名和驗證的詳細調試

use auditor_node::report::ReportManager;
use auditor_node::types::AuditReport;
use pqc_signer::{Dilithium3Signer, Signer};

fn main() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();

    println!("=== 測試報告簽名和驗證流程 ===\n");

    // 1. 創建簽名器
    let mut signer = Dilithium3Signer::new();
    signer.generate_keypair().unwrap();
    let public_key = signer.public_key().to_vec();

    println!("✓ 生成密鑰對");
    println!("  公鑰長度: {} bytes", public_key.len());
    println!();

    // 2. 創建管理器
    let manager = ReportManager::new(signer);

    // 3. 創建測試報告
    let mut report = AuditReport {
        blob_id: "test-blob-123".to_string(),
        blob_object_id: "0x1234".to_string(),
        auditor: "test-auditor".to_string(),
        timestamp: 1000,
        challenge_epoch: 1,
        challenge_results: vec![],
        total_challenges: 5,
        successful_verifications: 5,
        failed_verifications: 0,
        integrity_hash: vec![1, 2, 3, 4],
        pqc_signature: vec![], // 初始為空
        pqc_algorithm: 0,
        is_valid: true,
        failure_reason: None,
    };

    println!("✓ 創建測試報告");
    println!("  Blob ID: {}", report.blob_id);
    println!("  簽名前: pqc_signature.len() = {}", report.pqc_signature.len());
    println!();

    // 4. 簽名
    println!("步驟: 簽名報告...");

    // 打印簽名前的序列化結果
    let mut temp_for_sign = report.clone();
    temp_for_sign.pqc_signature = vec![];
    temp_for_sign.pqc_algorithm = 0;
    let sign_serialized = serde_json::to_vec(&temp_for_sign).unwrap();
    println!("  簽名前序列化: {} bytes", sign_serialized.len());
    println!("  前50字節: {:02x?}", &sign_serialized[..50.min(sign_serialized.len())]);

    manager.sign_report(&mut report).unwrap();

    println!("✓ 簽名完成");
    println!("  簽名後: pqc_signature.len() = {}", report.pqc_signature.len());
    println!("  PQC 算法: {}", report.pqc_algorithm);
    println!();

    // 5. 驗證
    println!("步驟: 驗證報告簽名...");

    // 打印驗證前的序列化結果
    let mut temp_for_verify = report.clone();
    temp_for_verify.pqc_signature = vec![];
    let verify_serialized = serde_json::to_vec(&temp_for_verify).unwrap();
    println!("  驗證前序列化: {} bytes", verify_serialized.len());
    println!("  前50字節: {:02x?}", &verify_serialized[..50.min(verify_serialized.len())]);
    println!("  序列化是否一致: {}", sign_serialized == verify_serialized);
    println!();

    match ReportManager::verify_report(&report, &public_key) {
        Ok(is_valid) => {
            if is_valid {
                println!("✓ 驗證成功!");
            } else {
                println!("✗ 驗證失敗: 簽名無效");
            }
        }
        Err(e) => {
            println!("✗ 驗證過程出錯: {:?}", e);
        }
    }
    println!();

    // 6. 測試修改報告後驗證應該失敗
    println!("步驟: 測試篡改報告...");
    let original_signature = report.pqc_signature.clone();
    report.total_challenges = 999; // 篡改數據

    match ReportManager::verify_report(&report, &public_key) {
        Ok(is_valid) => {
            if !is_valid {
                println!("✓ 正確: 篡改後的報告簽名無效");
            } else {
                println!("✗ 嚴重錯誤: 篡改後的報告簽名仍然有效!");
            }
        }
        Err(e) => {
            println!("驗證錯誤: {:?}", e);
        }
    }

    // 恢復原始簽名和數據
    report.total_challenges = 5;
    report.pqc_signature = original_signature;
    println!();

    // 7. 測試用錯誤的公鑰驗證
    println!("步驟: 測試用錯誤的公鑰驗證...");
    let mut wrong_signer = Dilithium3Signer::new();
    wrong_signer.generate_keypair().unwrap();
    let wrong_public_key = wrong_signer.public_key().to_vec();

    match ReportManager::verify_report(&report, &wrong_public_key) {
        Ok(is_valid) => {
            if !is_valid {
                println!("✓ 正確: 錯誤的公鑰無法驗證簽名");
            } else {
                println!("✗ 嚴重錯誤: 錯誤的公鑰驗證成功!");
            }
        }
        Err(e) => {
            println!("驗證錯誤: {:?}", e);
        }
    }
}
