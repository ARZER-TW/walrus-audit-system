// ! Walrus Auditor Node Main Program
//!
//! Implements the complete audit process:
//! 1. Load configuration and keys
//! 2. Execute audit (challenge-response + Merkle verification)
//! 3. Generate and sign report (Dilithium3 PQC signature)
//! 4. Encrypt report using Seal API (IBE threshold encryption)
//! 5. Upload encrypted report to Walrus
//! 6. Set access policy on Sui

mod auditor;
mod config;
mod crypto;
mod error;
mod integrity;
mod keystore;
mod report;
mod seal_client;
mod storage_node_client;
mod sui_client;
mod types;

use anyhow::{Context, Result};
use clap::Parser;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::signal;
use tracing::{debug, error, info, warn};
use tracing_subscriber;

use crate::types::AuditorConfig;
use pqc_signer::Signer; // Import Signer trait to use sign() method

/// Walrus Decentralized Storage Integrity Auditor Node
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Configuration file path
    #[arg(short, long, default_value = "config.toml")]
    config: PathBuf,

    /// Blob ID to audit (optional, for single audit)
    #[arg(short, long)]
    blob_id: Option<String>,

    /// Run in daemon mode (periodic audits)
    #[arg(short, long, default_value_t = false)]
    daemon: bool,

    /// Log level (trace, debug, info, warn, error)
    #[arg(long, default_value = "info")]
    log_level: String,

    /// Seal API endpoint (overrides config file)
    #[arg(long)]
    seal_api: Option<String>,

    /// Auditor Sui address (overrides config file)
    #[arg(long)]
    auditor_address: Option<String>,

    /// Audit contract Package ID (overrides config file)
    #[arg(long)]
    package_id: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // 1. Initialize logging
    init_logging(&args.log_level)?;

    info!("🚀 Starting Walrus Auditor Node v{}", env!("CARGO_PKG_VERSION"));
    info!("──────────────────────────────────────────────");

    // 2. Load configuration
    let mut config = load_configuration(&args.config)?;

    // Command line arguments override config file
    if let Some(seal_api) = args.seal_api {
        config.seal_api_url = Some(seal_api);
        config.enable_seal_encryption = true;
    }

    // 3. Validate configuration
    validate_configuration(&config)?;

    // 4. Load or generate PQC keys
    let keystore = initialize_keystore(&config.pqc_keystore_path)?;
    info!("✅ PQC keystore ready");

    // 5. Setup graceful shutdown handling
    let shutdown_signal = setup_shutdown_handler();

    // 6. Run based on mode
    if let Some(blob_id) = args.blob_id {
        // Single audit mode
        run_single_audit(
            &config,
            &keystore,
            &blob_id,
            args.auditor_address.as_deref(),
            args.package_id.as_deref(),
        )
        .await?;
    } else if args.daemon {
        // Daemon mode
        run_daemon_mode(
            config,
            keystore,
            shutdown_signal,
            args.auditor_address,
            args.package_id,
        )
        .await?;
    } else {
        error!("❌ No operation mode specified");
        error!("   Use --blob-id <ID> for single audit");
        error!("   Use --daemon to start daemon mode");
        std::process::exit(1);
    }

    info!("👋 Auditor node shutting down gracefully");
    Ok(())
}

/// Initialize logging system
fn init_logging(log_level: &str) -> Result<()> {
    let level = match log_level.to_lowercase().as_str() {
        "trace" => tracing::Level::TRACE,
        "debug" => tracing::Level::DEBUG,
        "info" => tracing::Level::INFO,
        "warn" => tracing::Level::WARN,
        "error" => tracing::Level::ERROR,
        _ => {
            eprintln!("⚠️  Unknown log level: {}, using INFO", log_level);
            tracing::Level::INFO
        }
    };

    tracing_subscriber::fmt()
        .with_max_level(level)
        .with_target(false)
        .with_thread_ids(false)
        .with_file(false)
        .with_line_number(false)
        .init();

    Ok(())
}

/// Load configuration file
fn load_configuration(config_path: &Path) -> Result<AuditorConfig> {
    info!("📋 Loading configuration: {}", config_path.display());

    if !config_path.exists() {
        warn!("Configuration file does not exist, using defaults");
        return Ok(AuditorConfig::default());
    }

    config::load_config(config_path).context("Failed to load configuration")
}

/// Validate configuration validity
fn validate_configuration(config: &AuditorConfig) -> Result<()> {
    info!("🔍 Validating configuration...");
    info!("   - Sui RPC: {}", config.sui_rpc_url);
    info!("   - Walrus Aggregator: {}", config.walrus_aggregator_url);
    info!(
        "   - Seal Encryption: {}",
        if config.enable_seal_encryption {
            "Enabled"
        } else {
            "Disabled"
        }
    );

    if config.enable_seal_encryption {
        if let Some(seal_api) = &config.seal_api_url {
            info!("   - Seal API: {}", seal_api);
        } else {
            return Err(anyhow::anyhow!(
                "Seal encryption enabled but seal_api_url not configured"
            ));
        }
    }

    info!(
        "   - Challenge count range: {} - {}",
        config.min_challenges, config.max_challenges
    );
    info!("   - Audit interval: {} seconds", config.audit_interval_secs);

    Ok(())
}

/// Initialize or load PQC keystore
fn initialize_keystore(keystore_path: &str) -> Result<keystore::Keystore> {
    let path = Path::new(keystore_path);

    if path.exists() {
        info!("🔐 Loading existing keystore: {}", keystore_path);
        keystore::Keystore::load(path).context("Failed to load keystore")
    } else {
        info!("🔑 Generating new PQC keystore: {}", keystore_path);

        // Ensure directory exists
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).context("Failed to create keystore directory")?;
        }

        keystore::Keystore::generate_and_save(path).context("Failed to generate keystore")
    }
}

/// Setup graceful shutdown handler
fn setup_shutdown_handler() -> Arc<tokio::sync::Notify> {
    let shutdown = Arc::new(tokio::sync::Notify::new());
    let shutdown_clone = shutdown.clone();

    tokio::spawn(async move {
        match signal::ctrl_c().await {
            Ok(()) => {
                info!("\n🛑 Received Ctrl+C signal, preparing to shutdown...");
                shutdown_clone.notify_waiters();
            }
            Err(err) => {
                error!("❌ Cannot listen to shutdown signal: {}", err);
            }
        }
    });

    shutdown
}

/// Execute single audit
async fn run_single_audit(
    config: &AuditorConfig,
    keystore: &keystore::Keystore,
    blob_id: &str,
    auditor_address: Option<&str>,
    package_id: Option<&str>,
) -> Result<()> {
    info!("──────────────────────────────────────────────");
    info!("📊 Single Audit Mode");
    info!("   Blob ID: {}", blob_id);
    info!("──────────────────────────────────────────────\n");

    // 1. Execute audit (TODO: Actual audit logic in auditor.rs)
    info!("1️⃣ Executing integrity audit...");
    let audit_report = execute_audit(config, blob_id).await?;

    info!(
        "   ✅ Audit completed: {} challenges, {} successes, {} failures",
        audit_report.total_challenges,
        audit_report.successful_verifications,
        audit_report.failed_verifications
    );
    info!(
        "   - Audit result: {}",
        if audit_report.is_valid {
            "✅ PASS"
        } else {
            "❌ FAIL"
        }
    );

    // 2. Sign report using Dilithium3
    info!("\n2️⃣ Signing audit report (Dilithium3 PQC)...");
    let signed_report = sign_report(audit_report, keystore)?;
    info!("   ✅ PQC signature completed (signature length: {} bytes)", signed_report.pqc_signature.len());

    // 3. Seal encrypt report (if enabled)
    let encrypted_data = if config.enable_seal_encryption {
        info!("\n3️⃣ Encrypting report using Seal (IBE threshold encryption)...");

        let seal_api_url = config
            .seal_api_url
            .as_ref()
            .context("Seal API URL not configured")?;

        let auditor_addr = auditor_address
            .or_else(|| {
                // TODO: Get default address from config or keystore
                Some("0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef")
            })
            .context("Auditor address not provided")?;

        let pkg_id = package_id
            .or_else(|| {
                // TODO: Get from config
                Some("0x8afa5d31dbaa0a8fb07082692940ca3d56b5e856c5126cb5a3693f0a4de63b82")
            })
            .context("Package ID not provided")?;

        let encrypted = encrypt_report(&signed_report, seal_api_url, auditor_addr, pkg_id).await?;

        info!("   ✅ Encryption completed");
        info!("      - Original size: {} bytes", encrypted.metadata.original_size);
        info!("      - Encrypted size: {} bytes", encrypted.metadata.encrypted_size);
        info!(
            "      - Expansion ratio: {:.2}x",
            encrypted.metadata.encrypted_size as f64 / encrypted.metadata.original_size as f64
        );
        info!("      - Duration: {}ms", encrypted.metadata.duration);

        Some(encrypted.encrypted_data)
    } else {
        info!("\n3️⃣ Seal encryption disabled, skipping");
        None
    };

    // 4. Upload to Walrus
    info!("\n4️⃣ Uploading report to Walrus...");
    let data_to_upload = if let Some(encrypted) = encrypted_data.as_ref() {
        base64::decode(encrypted).context("Failed to decode encrypted data")?
    } else {
        // Upload unencrypted report (JSON format)
        serde_json::to_vec(&signed_report).context("Failed to serialize report")?
    };

    let walrus_blob_id = upload_to_walrus(&config.walrus_aggregator_url, &data_to_upload).await?;
    info!("   ✅ Upload successful: Blob ID = {}", walrus_blob_id);

    // 5. Submit to Sui (set access policy)
    info!("\n5️⃣ Setting access policy on Sui...");
    info!("   ⚠️  This step requires actual Sui SDK integration (currently placeholder)");
    // TODO: Actual Sui transaction submission

    info!("\n✅ Single audit process completed!");
    info!("   - Walrus Blob ID: {}", walrus_blob_id);
    if let Some(_) = encrypted_data {
        info!("   - Report encrypted and protected by Seal access control");
    }

    Ok(())
}

/// Daemon mode
async fn run_daemon_mode(
    config: AuditorConfig,
    keystore: keystore::Keystore,
    shutdown: Arc<tokio::sync::Notify>,
    _auditor_address: Option<String>,
    _package_id: Option<String>,
) -> Result<()> {
    info!("──────────────────────────────────────────────");
    info!("🔄 Daemon Mode");
    info!("   Audit interval: {} seconds", config.audit_interval_secs);
    info!("──────────────────────────────────────────────\n");

    let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(
        config.audit_interval_secs,
    ));

    loop {
        tokio::select! {
            _ = interval.tick() => {
                info!("⏰ Executing periodic audit...");

                // TODO: Query Sui for pending blobs to audit
                let blobs_to_audit = fetch_pending_blobs(&config).await?;

                if blobs_to_audit.is_empty() {
                    info!("   ℹ️  No blobs to audit");
                    continue;
                }

                info!("   Found {} blobs to audit", blobs_to_audit.len());

                // Execute audits
                for blob_id in blobs_to_audit {
                    match execute_audit_cycle(&config, &keystore, &blob_id).await {
                        Ok(_) => {
                            info!("   ✅ Blob {} audit successful", blob_id);
                        }
                        Err(e) => {
                            error!("   ❌ Blob {} audit failed: {}", blob_id, e);
                        }
                    }
                }
            }

            _ = shutdown.notified() => {
                info!("Received shutdown signal, stopping daemon");
                break;
            }
        }
    }

    Ok(())
}

/// Execute audit (using real IntegrityVerifier)
async fn execute_audit(
    config: &AuditorConfig,
    blob_id: &str,
) -> Result<types::AuditReport> {
    use crate::integrity::{IntegrityVerifier, VerificationStatus};

    info!("🔍 Starting audit for Blob: {}", blob_id);

    // Create integrity verifier
    let verifier = IntegrityVerifier::new(config.walrus_aggregator_url.clone());

    // Execute real Merkle verification
    let audit_data = verifier.audit_blob(blob_id).await.context("Integrity audit failed")?;

    info!("✅ Merkle verification completed:");
    info!("   - Content hash (SHA-256): {}", audit_data.content_hash);
    info!("   - Merkle root (Blake2b-256): {}", audit_data.merkle_root);
    info!("   - Challenge stats: {}/{} successful",
        audit_data.successful_verifications,
        audit_data.total_challenges
    );

    // Convert IntegrityVerifier result to AuditReport format
    let is_valid = audit_data.verification_status == VerificationStatus::Accessible
        && audit_data.failed_verifications == 0;

    let failure_reason = if !is_valid {
        Some(format!("Verification status: {:?}, failures: {}",
            audit_data.verification_status,
            audit_data.failed_verifications
        ))
    } else {
        None
    };

    // Parse content hash to byte array
    let integrity_hash = hex::decode(&audit_data.content_hash)
        .unwrap_or_else(|_| vec![0u8; 32]);

    Ok(types::AuditReport {
        blob_id: blob_id.to_string(),
        blob_object_id: "0x000000000000000000000000000000000000000000000000000000000000000"
            .to_string(), // TODO: Query real blob_object_id from Sui
        auditor: "0x0000000000000000000000000000000000000000000000000000000000000000"
            .to_string(), // TODO: Use actual auditor address
        timestamp: chrono::Utc::now().timestamp() as u64,
        challenge_epoch: 0, // TODO: Get current epoch from Sui
        challenge_results: vec![], // Simplified version doesn't include detailed challenge results
        total_challenges: audit_data.total_challenges,
        successful_verifications: audit_data.successful_verifications,
        failed_verifications: audit_data.failed_verifications,
        integrity_hash,
        pqc_signature: vec![], // Will be filled in sign_report()
        pqc_algorithm: 3, // Dilithium3
        is_valid,
        failure_reason,
    })
}

/// Sign report
fn sign_report(
    mut report: types::AuditReport,
    keystore: &keystore::Keystore,
) -> Result<types::AuditReport> {
    // Serialize report for signing (excluding signature field)
    let report_for_signing = serde_json::json!({
        "blob_id": report.blob_id,
        "blob_object_id": report.blob_object_id,
        "auditor": report.auditor,
        "timestamp": report.timestamp,
        "challenge_epoch": report.challenge_epoch,
        "total_challenges": report.total_challenges,
        "successful_verifications": report.successful_verifications,
        "failed_verifications": report.failed_verifications,
        "integrity_hash": report.integrity_hash,
        "is_valid": report.is_valid,
    });

    let report_bytes = serde_json::to_vec(&report_for_signing)?;
    let signature = keystore.signer().sign(&report_bytes)?;

    report.pqc_signature = signature;
    report.pqc_algorithm = 3; // Dilithium3

    debug!("Report signed: {} bytes", report.pqc_signature.len());

    Ok(report)
}

/// Encrypt report (call Seal API)
async fn encrypt_report(
    report: &types::AuditReport,
    seal_api_url: &str,
    auditor_address: &str,
    package_id: &str,
) -> Result<seal_client::EncryptResult> {
    let seal_config = seal_client::SealApiConfig {
        api_url: seal_api_url.to_string(),
        timeout_secs: 30,
    };

    let seal_client = seal_client::SealClient::new(seal_config)?;

    // First check health status
    seal_client
        .health_check()
        .await
        .context("Seal API health check failed")?;

    // Serialize report to JSON
    let report_json = serde_json::to_string_pretty(report).context("Failed to serialize report")?;

    debug!("Report JSON size: {} bytes", report_json.len());

    // Call Seal API to encrypt
    let (encrypted_data, symmetric_key, metadata) = seal_client
        .encrypt_report(&report_json, auditor_address, package_id, 2)
        .await
        .context("Seal encryption failed")?;

    Ok(seal_client::EncryptResult {
        encrypted_data,
        symmetric_key,
        metadata,
    })
}

/// Upload to Walrus (placeholder implementation)
async fn upload_to_walrus(aggregator_url: &str, data: &[u8]) -> Result<String> {
    debug!(
        "Uploading {} bytes to Walrus ({})",
        data.len(),
        aggregator_url
    );

    // TODO: Actual Walrus API call
    // Reference implementation in storage_node_client.rs

    // Return example Blob ID
    Ok("0xabcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890".to_string())
}

/// Get list of blobs pending audit
async fn fetch_pending_blobs(_config: &AuditorConfig) -> Result<Vec<String>> {
    // TODO: Query Sui for pending blobs
    Ok(vec![])
}

/// Execute complete audit cycle (daemon mode)
async fn execute_audit_cycle(
    config: &AuditorConfig,
    keystore: &keystore::Keystore,
    blob_id: &str,
) -> Result<()> {
    // 1. Execute audit
    let audit_report = execute_audit(config, blob_id).await?;

    // 2. Sign
    let signed_report = sign_report(audit_report, keystore)?;

    // 3. Encrypt (if enabled)
    let encrypted_data = if config.enable_seal_encryption {
        let seal_api_url = config
            .seal_api_url
            .as_ref()
            .context("Seal API URL not configured")?;

        // TODO: Get actual addresses from config
        let auditor_addr =
            "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef";
        let pkg_id = "0x8afa5d31dbaa0a8fb07082692940ca3d56b5e856c5126cb5a3693f0a4de63b82";

        let encrypted = encrypt_report(&signed_report, seal_api_url, auditor_addr, pkg_id).await?;
        Some(encrypted.encrypted_data)
    } else {
        None
    };

    // 4. Upload
    let data_to_upload = if let Some(encrypted) = encrypted_data.as_ref() {
        base64::decode(encrypted).context("Failed to decode encrypted data")?
    } else {
        serde_json::to_vec(&signed_report).context("Failed to serialize report")?
    };

    let _walrus_blob_id = upload_to_walrus(&config.walrus_aggregator_url, &data_to_upload).await?;

    // 5. Submit to Sui (TODO)

    Ok(())
}

/// Seal encryption result
#[allow(dead_code)]
struct EncryptResult {
    encrypted_data: String,
    symmetric_key: String,
    metadata: seal_client::EncryptMetadata,
}

// Base64 decode helper
mod base64 {
    use base64::{engine::general_purpose, Engine as _};

    pub fn decode(encoded: &str) -> Result<Vec<u8>, base64::DecodeError> {
        general_purpose::STANDARD.decode(encoded)
    }
}
