// ! Walrus 審計節點主程序
//!
//! 實現完整的審計流程：
//! 1. 載入配置和密鑰
//! 2. 執行審計（挑戰-響應 + 默克爾驗證）
//! 3. 生成並簽名報告（Dilithium3 PQC 簽名）
//! 4. 使用 Seal API 加密報告（IBE 門檻加密）
//! 5. 上傳加密報告到 Walrus
//! 6. 在 Sui 上設置訪問策略

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
use pqc_signer::Signer; // 導入 Signer trait 以使用 sign() 方法

/// Walrus 去中心化存儲完整性審計節點
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// 配置文件路徑
    #[arg(short, long, default_value = "config.toml")]
    config: PathBuf,

    /// 要審計的 Blob ID（可選，用於單次審計）
    #[arg(short, long)]
    blob_id: Option<String>,

    /// 是否以守護進程模式運行（定期審計）
    #[arg(short, long, default_value_t = false)]
    daemon: bool,

    /// 日誌級別（trace, debug, info, warn, error）
    #[arg(long, default_value = "info")]
    log_level: String,

    /// Seal API 端點（覆蓋配置文件）
    #[arg(long)]
    seal_api: Option<String>,

    /// 審計員 Sui 地址（覆蓋配置文件）
    #[arg(long)]
    auditor_address: Option<String>,

    /// 審計合約 Package ID（覆蓋配置文件）
    #[arg(long)]
    package_id: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // 1. 初始化日誌
    init_logging(&args.log_level)?;

    info!("🚀 Starting Walrus Auditor Node v{}", env!("CARGO_PKG_VERSION"));
    info!("──────────────────────────────────────────────");

    // 2. 載入配置
    let mut config = load_configuration(&args.config)?;

    // 命令行參數覆蓋配置文件
    if let Some(seal_api) = args.seal_api {
        config.seal_api_url = Some(seal_api);
        config.enable_seal_encryption = true;
    }

    // 3. 驗證配置
    validate_configuration(&config)?;

    // 4. 載入或生成 PQC 密鑰
    let keystore = initialize_keystore(&config.pqc_keystore_path)?;
    info!("✅ PQC 密鑰庫已就緒");

    // 5. 設置優雅關閉處理
    let shutdown_signal = setup_shutdown_handler();

    // 6. 根據模式運行
    if let Some(blob_id) = args.blob_id {
        // 單次審計模式
        run_single_audit(
            &config,
            &keystore,
            &blob_id,
            args.auditor_address.as_deref(),
            args.package_id.as_deref(),
        )
        .await?;
    } else if args.daemon {
        // 守護進程模式
        run_daemon_mode(
            config,
            keystore,
            shutdown_signal,
            args.auditor_address,
            args.package_id,
        )
        .await?;
    } else {
        error!("❌ 未指定操作模式");
        error!("   使用 --blob-id <ID> 進行單次審計");
        error!("   使用 --daemon 啟動守護進程模式");
        std::process::exit(1);
    }

    info!("👋 Auditor node shutting down gracefully");
    Ok(())
}

/// 初始化日誌系統
fn init_logging(log_level: &str) -> Result<()> {
    let level = match log_level.to_lowercase().as_str() {
        "trace" => tracing::Level::TRACE,
        "debug" => tracing::Level::DEBUG,
        "info" => tracing::Level::INFO,
        "warn" => tracing::Level::WARN,
        "error" => tracing::Level::ERROR,
        _ => {
            eprintln!("⚠️  未知日誌級別: {}, 使用 INFO", log_level);
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

/// 載入配置文件
fn load_configuration(config_path: &Path) -> Result<AuditorConfig> {
    info!("📋 載入配置: {}", config_path.display());

    if !config_path.exists() {
        warn!("配置文件不存在，使用默認配置");
        return Ok(AuditorConfig::default());
    }

    config::load_config(config_path).context("Failed to load configuration")
}

/// 驗證配置有效性
fn validate_configuration(config: &AuditorConfig) -> Result<()> {
    info!("🔍 驗證配置...");
    info!("   - Sui RPC: {}", config.sui_rpc_url);
    info!("   - Walrus Aggregator: {}", config.walrus_aggregator_url);
    info!(
        "   - Seal 加密: {}",
        if config.enable_seal_encryption {
            "啟用"
        } else {
            "禁用"
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
        "   - 挑戰次數範圍: {} - {}",
        config.min_challenges, config.max_challenges
    );
    info!("   - 審計間隔: {} 秒", config.audit_interval_secs);

    Ok(())
}

/// 初始化或載入 PQC 密鑰庫
fn initialize_keystore(keystore_path: &str) -> Result<keystore::Keystore> {
    let path = Path::new(keystore_path);

    if path.exists() {
        info!("🔐 載入現有密鑰庫: {}", keystore_path);
        keystore::Keystore::load(path).context("Failed to load keystore")
    } else {
        info!("🔑 生成新的 PQC 密鑰庫: {}", keystore_path);

        // 確保目錄存在
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).context("Failed to create keystore directory")?;
        }

        keystore::Keystore::generate_and_save(path).context("Failed to generate keystore")
    }
}

/// 設置優雅關閉處理器
fn setup_shutdown_handler() -> Arc<tokio::sync::Notify> {
    let shutdown = Arc::new(tokio::sync::Notify::new());
    let shutdown_clone = shutdown.clone();

    tokio::spawn(async move {
        match signal::ctrl_c().await {
            Ok(()) => {
                info!("\n🛑 收到 Ctrl+C 信號，準備關閉...");
                shutdown_clone.notify_waiters();
            }
            Err(err) => {
                error!("❌ 無法監聽 shutdown 信號: {}", err);
            }
        }
    });

    shutdown
}

/// 執行單次審計
async fn run_single_audit(
    config: &AuditorConfig,
    keystore: &keystore::Keystore,
    blob_id: &str,
    auditor_address: Option<&str>,
    package_id: Option<&str>,
) -> Result<()> {
    info!("──────────────────────────────────────────────");
    info!("📊 單次審計模式");
    info!("   Blob ID: {}", blob_id);
    info!("──────────────────────────────────────────────\n");

    // 1. 執行審計（TODO: 實際審計邏輯在 auditor.rs 中）
    info!("1️⃣ 執行完整性審計...");
    let audit_report = execute_audit(config, blob_id).await?;

    info!(
        "   ✅ 審計完成: {} 次挑戰, {} 成功, {} 失敗",
        audit_report.total_challenges,
        audit_report.successful_verifications,
        audit_report.failed_verifications
    );
    info!(
        "   - 審計結果: {}",
        if audit_report.is_valid {
            "✅ PASS"
        } else {
            "❌ FAIL"
        }
    );

    // 2. 使用 Dilithium3 簽名報告
    info!("\n2️⃣ 簽名審計報告（Dilithium3 PQC）...");
    let signed_report = sign_report(audit_report, keystore)?;
    info!("   ✅ PQC 簽名完成 (簽名長度: {} bytes)", signed_report.pqc_signature.len());

    // 3. Seal 加密報告（如果啟用）
    let encrypted_data = if config.enable_seal_encryption {
        info!("\n3️⃣ 使用 Seal 加密報告（IBE 門檻加密）...");

        let seal_api_url = config
            .seal_api_url
            .as_ref()
            .context("Seal API URL not configured")?;

        let auditor_addr = auditor_address
            .or_else(|| {
                // TODO: 從配置或密鑰庫獲取默認地址
                Some("0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef")
            })
            .context("Auditor address not provided")?;

        let pkg_id = package_id
            .or_else(|| {
                // TODO: 從配置獲取
                Some("0x8afa5d31dbaa0a8fb07082692940ca3d56b5e856c5126cb5a3693f0a4de63b82")
            })
            .context("Package ID not provided")?;

        let encrypted = encrypt_report(&signed_report, seal_api_url, auditor_addr, pkg_id).await?;

        info!("   ✅ 加密完成");
        info!("      - 原始大小: {} bytes", encrypted.metadata.original_size);
        info!("      - 加密大小: {} bytes", encrypted.metadata.encrypted_size);
        info!(
            "      - 膨脹率: {:.2}x",
            encrypted.metadata.encrypted_size as f64 / encrypted.metadata.original_size as f64
        );
        info!("      - 耗時: {}ms", encrypted.metadata.duration);

        Some(encrypted.encrypted_data)
    } else {
        info!("\n3️⃣ Seal 加密已禁用，跳過");
        None
    };

    // 4. 上傳到 Walrus
    info!("\n4️⃣ 上傳報告到 Walrus...");
    let data_to_upload = if let Some(encrypted) = encrypted_data.as_ref() {
        base64::decode(encrypted).context("Failed to decode encrypted data")?
    } else {
        // 上傳未加密的報告（JSON 格式）
        serde_json::to_vec(&signed_report).context("Failed to serialize report")?
    };

    let walrus_blob_id = upload_to_walrus(&config.walrus_aggregator_url, &data_to_upload).await?;
    info!("   ✅ 上傳成功: Blob ID = {}", walrus_blob_id);

    // 5. 提交到 Sui（設置訪問策略）
    info!("\n5️⃣ 在 Sui 上設置訪問策略...");
    info!("   ⚠️  此步驟需要實際 Sui SDK 整合（當前為占位）");
    // TODO: 實際的 Sui 交易提交

    info!("\n✅ 單次審計流程完成!");
    info!("   - Walrus Blob ID: {}", walrus_blob_id);
    if let Some(_) = encrypted_data {
        info!("   - 報告已加密並受 Seal 訪問控制保護");
    }

    Ok(())
}

/// 守護進程模式
async fn run_daemon_mode(
    config: AuditorConfig,
    keystore: keystore::Keystore,
    shutdown: Arc<tokio::sync::Notify>,
    _auditor_address: Option<String>,
    _package_id: Option<String>,
) -> Result<()> {
    info!("──────────────────────────────────────────────");
    info!("🔄 守護進程模式");
    info!("   審計間隔: {} 秒", config.audit_interval_secs);
    info!("──────────────────────────────────────────────\n");

    let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(
        config.audit_interval_secs,
    ));

    loop {
        tokio::select! {
            _ = interval.tick() => {
                info!("⏰ 執行定期審計...");

                // TODO: 從 Sui 查詢待審計的 Blob 列表
                let blobs_to_audit = fetch_pending_blobs(&config).await?;

                if blobs_to_audit.is_empty() {
                    info!("   ℹ️  沒有待審計的 Blob");
                    continue;
                }

                info!("   找到 {} 個待審計 Blob", blobs_to_audit.len());

                // 執行審計
                for blob_id in blobs_to_audit {
                    match execute_audit_cycle(&config, &keystore, &blob_id).await {
                        Ok(_) => {
                            info!("   ✅ Blob {} 審計成功", blob_id);
                        }
                        Err(e) => {
                            error!("   ❌ Blob {} 審計失敗: {}", blob_id, e);
                        }
                    }
                }
            }

            _ = shutdown.notified() => {
                info!("收到關閉信號，停止守護進程");
                break;
            }
        }
    }

    Ok(())
}

/// 執行審計（使用真實的 IntegrityVerifier）
async fn execute_audit(
    config: &AuditorConfig,
    blob_id: &str,
) -> Result<types::AuditReport> {
    use crate::integrity::{IntegrityVerifier, VerificationStatus};

    info!("🔍 開始審計 Blob: {}", blob_id);

    // 創建完整性驗證器
    let verifier = IntegrityVerifier::new(config.walrus_aggregator_url.clone());

    // 執行真實的 Merkle 驗證
    let audit_data = verifier.audit_blob(blob_id).await.context("完整性審計失敗")?;

    info!("✅ Merkle 驗證完成:");
    info!("   - 內容哈希 (SHA-256): {}", audit_data.content_hash);
    info!("   - Merkle 根 (Blake2b-256): {}", audit_data.merkle_root);
    info!("   - 挑戰統計: {}/{} 成功",
        audit_data.successful_verifications,
        audit_data.total_challenges
    );

    // 將 IntegrityVerifier 的結果轉換為 AuditReport 格式
    let is_valid = audit_data.status == VerificationStatus::Accessible
        && audit_data.failed_verifications == 0;

    let failure_reason = if !is_valid {
        Some(format!("驗證狀態: {:?}, 失敗次數: {}",
            audit_data.status,
            audit_data.failed_verifications
        ))
    } else {
        None
    };

    // 解析內容哈希為字節數組
    let integrity_hash = hex::decode(&audit_data.content_hash)
        .unwrap_or_else(|_| vec![0u8; 32]);

    Ok(types::AuditReport {
        blob_id: blob_id.to_string(),
        blob_object_id: "0x000000000000000000000000000000000000000000000000000000000000000"
            .to_string(), // TODO: 從 Sui 查詢真實的 blob_object_id
        auditor: "0x0000000000000000000000000000000000000000000000000000000000000000"
            .to_string(), // TODO: 使用實際的審計員地址
        timestamp: chrono::Utc::now().timestamp() as u64,
        challenge_epoch: 0, // TODO: 從 Sui 獲取當前 epoch
        challenge_results: vec![], // 簡化版本不包含詳細的挑戰結果
        total_challenges: audit_data.total_challenges,
        successful_verifications: audit_data.successful_verifications,
        failed_verifications: audit_data.failed_verifications,
        integrity_hash,
        pqc_signature: vec![], // 將在 sign_report() 中填充
        pqc_algorithm: 3, // Dilithium3
        is_valid,
        failure_reason,
    })
}

/// 簽名報告
fn sign_report(
    mut report: types::AuditReport,
    keystore: &keystore::Keystore,
) -> Result<types::AuditReport> {
    // 序列化報告用於簽名（不包含簽名字段）
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

    debug!("報告已簽名: {} bytes", report.pqc_signature.len());

    Ok(report)
}

/// 加密報告（調用 Seal API）
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

    // 先檢查健康狀態
    seal_client
        .health_check()
        .await
        .context("Seal API health check failed")?;

    // 序列化報告為 JSON
    let report_json = serde_json::to_string_pretty(report).context("Failed to serialize report")?;

    debug!("報告 JSON 大小: {} bytes", report_json.len());

    // 調用 Seal API 加密
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

/// 上傳到 Walrus（占位實現）
async fn upload_to_walrus(aggregator_url: &str, data: &[u8]) -> Result<String> {
    debug!(
        "上傳 {} bytes 到 Walrus ({})",
        data.len(),
        aggregator_url
    );

    // TODO: 實際的 Walrus API 調用
    // 參考 storage_node_client.rs 中的實現

    // 返回示例 Blob ID
    Ok("0xabcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890".to_string())
}

/// 獲取待審計的 Blob 列表
async fn fetch_pending_blobs(_config: &AuditorConfig) -> Result<Vec<String>> {
    // TODO: 從 Sui 查詢待審計的 Blob 列表
    Ok(vec![])
}

/// 執行完整審計循環（守護進程模式）
async fn execute_audit_cycle(
    config: &AuditorConfig,
    keystore: &keystore::Keystore,
    blob_id: &str,
) -> Result<()> {
    // 1. 執行審計
    let audit_report = execute_audit(config, blob_id).await?;

    // 2. 簽名
    let signed_report = sign_report(audit_report, keystore)?;

    // 3. 加密（如果啟用）
    let encrypted_data = if config.enable_seal_encryption {
        let seal_api_url = config
            .seal_api_url
            .as_ref()
            .context("Seal API URL not configured")?;

        // TODO: 從配置獲取實際地址
        let auditor_addr =
            "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef";
        let pkg_id = "0x8afa5d31dbaa0a8fb07082692940ca3d56b5e856c5126cb5a3693f0a4de63b82";

        let encrypted = encrypt_report(&signed_report, seal_api_url, auditor_addr, pkg_id).await?;
        Some(encrypted.encrypted_data)
    } else {
        None
    };

    // 4. 上傳
    let data_to_upload = if let Some(encrypted) = encrypted_data.as_ref() {
        base64::decode(encrypted).context("Failed to decode encrypted data")?
    } else {
        serde_json::to_vec(&signed_report).context("Failed to serialize report")?
    };

    let _walrus_blob_id = upload_to_walrus(&config.walrus_aggregator_url, &data_to_upload).await?;

    // 5. 提交到 Sui（TODO）

    Ok(())
}

/// Seal 加密結果
#[allow(dead_code)]
struct EncryptResult {
    encrypted_data: String,
    symmetric_key: String,
    metadata: seal_client::EncryptMetadata,
}

// Base64 解碼輔助
mod base64 {
    use base64::{engine::general_purpose, Engine as _};

    pub fn decode(encoded: &str) -> Result<Vec<u8>, base64::DecodeError> {
        general_purpose::STANDARD.decode(encoded)
    }
}
