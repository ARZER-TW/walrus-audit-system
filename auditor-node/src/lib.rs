//! Walrus Decentralized Storage Integrity Auditor Node
//!
//! This crate implements a complete auditor node responsible for:
//! 1. Executing challenge-response verification against Walrus storage nodes
//! 2. Verifying data integrity using Merkle proofs
//! 3. Generating audit reports with PQC signatures
//! 4. Submitting audit results to Sui blockchain
//!
//! # Architecture
//!
//! ```text
//! ┌──────────────┐
//! │   Auditor    │  ← Core audit logic
//! └──────┬───────┘
//!        │
//!   ┌────┴────┬─────────┬──────────┬──────────┐
//!   ▼         ▼         ▼          ▼          ▼
//! SuiClient Storage  Report   Keystore   Config
//!           Node    Generator
//!           Client
//! ```
//!
//! # Example Usage
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

// Public modules
pub mod audit_report; // PQC-signed audit reports
pub mod auditor;
pub mod config;
pub mod crypto;
pub mod error;
pub mod integrity; // Application-layer integrity verification
pub mod keystore;
pub mod report;
pub mod seal_client;
pub mod storage_node_client;
pub mod sui_client;
pub mod types;

// Re-export commonly used types
pub use auditor::Auditor;
pub use error::{AuditorError, Result};
pub use types::{AuditReport, AuditorConfig, BlobMetadata};
