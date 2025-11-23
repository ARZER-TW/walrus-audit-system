//! Walrus 去中心化存儲完整性審計節點
//!
//! 本 crate 實現了一個完整的審計節點，負責:
//! 1. 對 Walrus 存儲節點執行挑戰-響應驗證
//! 2. 使用默克爾證明驗證數據完整性
//! 3. 生成帶 PQC 簽名的審計報告
//! 4. 提交審計結果到 Sui 區塊鏈
//!
//! # 架構
//!
//! ```text
//! ┌──────────────┐
//! │   Auditor    │  ← 核心審計邏輯
//! └──────┬───────┘
//!        │
//!   ┌────┴────┬─────────┬──────────┬──────────┐
//!   ▼         ▼         ▼          ▼          ▼
//! SuiClient Storage  Report   Keystore   Config
//!           Node    Generator
//!           Client
//! ```
//!
//! # 示例用法
//!
//! ```no_run
//! use auditor_node::{Auditor, config::load_config};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let config = load_config("config.toml")?;
//!     let auditor = Auditor::new()?;
//!
//!     let report = auditor.audit_blob("blob_id_here").await?;
//!     println!("Audit result: {}", report.is_valid);
//!
//!     Ok(())
//! }
//! ```

// 公開模塊
pub mod audit_report; // PQC 簽名的審計報告
pub mod auditor;
pub mod config;
pub mod crypto;
pub mod error;
pub mod integrity; // 應用層完整性驗證
pub mod keystore;
pub mod report;
pub mod seal_client;
pub mod storage_node_client;
pub mod sui_client;
pub mod types;

// Re-export 常用類型
pub use auditor::Auditor;
pub use error::{AuditorError, Result};
pub use types::{AuditReport, AuditorConfig, BlobMetadata};
