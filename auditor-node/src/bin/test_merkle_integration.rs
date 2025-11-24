// ! Test Merkle Integration Binary
//!
//! This program tests the complete Merkle Tree audit workflow

use anyhow::Result;
use auditor_node::integrity::{IntegrityVerifier, AuditData};
use tracing_subscriber;
use serde_json;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    println!("╔════════════════════════════════════════════════════════════════╗");
    println!("║           Merkle Tree Integration Test                        ║");
    println!("╚════════════════════════════════════════════════════════════════╝\n");

    // Create integrity verifier
    let aggregator_url = "https://aggregator.walrus-testnet.walrus.space".to_string();
    let verifier = IntegrityVerifier::new(aggregator_url);

    // Test with real Walrus Testnet blob
    let blob_id = "eRrTusk8yshFQpkemgDnbg0f4-qDo623V2NpeVG1Zcg";

    println!("📋 Test Configuration:");
    println!("   Blob ID: {}", blob_id);
    println!("   Chunk size: 4096 bytes (4KB)");
    println!("   Hash algorithm: Blake2b-256");
    println!("   Challenge count: min(10, leaf_count)\n");

    println!("🚀 Starting audit...\n");

    // Execute audit (includes Merkle verification)
    let audit_data = match verifier.audit_blob(blob_id).await {
        Ok(data) => {
            print_audit_results(&data);
            data
        }
        Err(e) => {
            eprintln!("❌ Audit failed: {}", e);
            std::process::exit(1);
        }
    };

    println!("\n✅ Test completed!");

    // Generate and save signed report (for Seal encryption)
    println!("\n📝 Generating PQC signed report...");

    use pqc_signer::dilithium::Dilithium3Signer;
    use pqc_signer::traits::Signer as PqcSigner;
    use std::fs;

    // Create signer
    let mut signer = Dilithium3Signer::new();
    signer.generate_keypair()?;

    // Create signed report structure
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

    // Save to temporary file
    let report_path = "/tmp/signed_audit_report.json";
    fs::write(report_path, serde_json::to_string_pretty(&signed_report)?)?;

    println!("✅ Signed report saved: {}", report_path);
    println!("\n💡 Next step (optional): Encrypt report with Seal");
    println!("   cd seal-client && npx tsx encrypt-and-submit-report.ts");
    println!("   (Note: Seal encryption has graceful fallback if API unavailable)\n");

    Ok(())
}

fn print_audit_results(data: &AuditData) {
    println!("╔════════════════════════════════════════════════════════════════╗");
    println!("║                    Audit Results                               ║");
    println!("╚════════════════════════════════════════════════════════════════╝\n");

    println!("📊 Basic Information:");
    println!("   Blob ID: {}", data.blob_id);
    println!("   File size: {} bytes", data.file_size);
    println!("   Audit timestamp: {}", data.timestamp);
    println!();

    println!("🔐 Hash Proofs:");
    if data.content_hash.len() >= 16 {
        println!("   SHA-256 (application): {}...", &data.content_hash[..16]);
    } else {
        println!("   SHA-256 (application): {}", data.content_hash);
    }
    if data.merkle_root.len() >= 16 {
        println!("   Merkle Root (protocol): {}...", &data.merkle_root[..16]);
    } else {
        println!("   Merkle Root (protocol): {}", data.merkle_root);
    }
    println!();

    println!("🎯 Merkle Challenge-Response Statistics:");
    println!("   Total challenges: {}", data.total_challenges);
    println!("   Successful verifications: {}", data.successful_verifications);
    println!("   Failed verifications: {}", data.failed_verifications);

    let success_rate = if data.total_challenges > 0 {
        (data.successful_verifications as f64 / data.total_challenges as f64) * 100.0
    } else {
        0.0
    };
    println!("   Success rate: {:.2}%", success_rate);
    println!();

    println!("✅ Verification status: {:?}", data.verification_status);
}
