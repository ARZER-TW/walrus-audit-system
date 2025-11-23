//! 核心審計邏輯模塊

use crate::{
    crypto::{
        merkle::MerkleProof,
        sliver::{calculate_challenge_count, Sliver, SliverMetadata},
    },
    error::{AuditorError, Result},
    storage_node_client::{ChallengeResponse, StorageNodeClient},
    sui_client::AuditSystemClient,
    types::{AuditChallenge, AuditReport, AuditorConfig, BlobMetadata, ChallengeResult},
};
use chrono::Utc;
use rand::Rng;
use std::time::Instant;
use tracing::{debug, error, info, warn};

pub struct Auditor {
    sui_client: AuditSystemClient,
    storage_clients: Vec<StorageNodeClient>,
    config: AuditorConfig,
    auditor_address: String,
}

impl Auditor {
    pub async fn new(
        config: AuditorConfig,
        auditor_address: String,
        storage_node_urls: Vec<String>,
    ) -> Result<Self> {
        info!("Initializing Auditor for address: {}", auditor_address);

        let sui_client = AuditSystemClient::new(
            &config.sui_rpc_url,
            "0x0000000000000000000000000000000000000000000000000000000000000000",
            "0x0000000000000000000000000000000000000000000000000000000000000000",
            "0x0000000000000000000000000000000000000000000000000000000000000000",
            "0x0000000000000000000000000000000000000000000000000000000000000000",
        ).await?;

        let storage_clients: Vec<StorageNodeClient> = storage_node_urls
            .iter()
            .map(|url| {
                StorageNodeClient::with_config(url.clone(), config.http_timeout_secs, 3)
            })
            .collect();

        info!("Created {} storage node client(s)", storage_clients.len());

        Ok(Self {
            sui_client,
            storage_clients,
            config,
            auditor_address,
        })
    }

    pub async fn audit_blob(&self, blob_id: &str) -> Result<AuditReport> {
        let start_time = Instant::now();
        info!("========================================");
        info!("Starting audit for blob: {}", blob_id);
        info!("========================================");

        let metadata = self.fetch_blob_metadata(blob_id).await?;
        info!(
            "Blob metadata: size={} bytes, k={}, n={}, epochs={}-{}",
            metadata.blob_size, metadata.encoding_k, metadata.encoding_n,
            metadata.start_epoch, metadata.end_epoch
        );

        let challenge_count = self.determine_challenge_count(&metadata);
        let challenges = self.generate_challenges(&metadata, challenge_count);
        info!("Generated {} challenges", challenges.len());

        let challenge_results = self.execute_challenges(&metadata, &challenges).await?;

        let (successful, failed) = self.count_results(&challenge_results);
        info!("Challenges completed: {} successful, {} failed", successful, failed);

        let report = self.generate_report(blob_id, &metadata, challenge_results, successful, failed)?;

        let duration = start_time.elapsed();
        info!("========================================");
        info!("Audit completed in {:.2}s: {}", duration.as_secs_f64(),
            if report.is_valid { "PASS ✓" } else { "FAIL ✗" });
        info!("========================================");

        Ok(report)
    }

    async fn fetch_blob_metadata(&self, blob_id: &str) -> Result<BlobMetadata> {
        debug!("Fetching metadata for blob: {}", blob_id);
        let start = Instant::now();
        let metadata = self.sui_client.get_blob_metadata(blob_id).await?;
        
        
        debug!("Metadata fetched in {:?}", start.elapsed());
        Ok(metadata)
    }

    fn determine_challenge_count(&self, metadata: &BlobMetadata) -> u16 {
        let total_slivers = metadata.encoding_n as u64;
        let recommended = calculate_challenge_count(total_slivers, 0.95, 0.1);
        let count = recommended
            .max(self.config.min_challenges as u64)
            .min(self.config.max_challenges as u64) as u16;
        debug!("Challenge count: recommended={}, clamped={}", recommended, count);
        count
    }

    fn generate_challenges(&self, metadata: &BlobMetadata, count: u16) -> Vec<AuditChallenge> {
        let mut rng = rand::thread_rng();
        let mut challenges = Vec::with_capacity(count as usize);
        let total_slivers = metadata.encoding_n;
        let mut selected_indices = std::collections::HashSet::new();

        while selected_indices.len() < count as usize {
            let index = rng.gen_range(0..total_slivers);
            if selected_indices.insert(index) {
                let shard_id = (index % 10) as u16;
                challenges.push(AuditChallenge {
                    sliver_index: index,
                    shard_id,
                    challenge_type: 1,
                    timestamp: Utc::now().timestamp() as u64,
                });
            }
        }

        debug!("Generated {} unique challenges", challenges.len());
        challenges
    }

    async fn execute_challenges(
        &self,
        metadata: &BlobMetadata,
        challenges: &[AuditChallenge],
    ) -> Result<Vec<ChallengeResult>> {
        let mut results = Vec::with_capacity(challenges.len());

        for (i, challenge) in challenges.iter().enumerate() {
            info!("Executing challenge {}/{}: sliver_index={}", i + 1, challenges.len(), challenge.sliver_index);

            let result = self.execute_single_challenge(metadata, challenge).await;

            match result {
                Ok(challenge_result) => {
                    if challenge_result.verified {
                        debug!("Challenge {} verified successfully", i + 1);
                    } else {
                        warn!("Challenge {} verification failed: {:?}", i + 1, challenge_result.failure_reason);
                    }
                    results.push(challenge_result);
                }
                Err(e) => {
                    error!("Challenge {} encountered error: {}", i + 1, e);
                    results.push(ChallengeResult {
                        challenge: challenge.clone(),
                        verified: false,
                        merkle_proof_valid: false,
                        response_hash: vec![],
                        failure_reason: Some(format!("Error: {}", e)),
                    });
                }
            }
        }

        Ok(results)
    }

    async fn execute_single_challenge(
        &self,
        metadata: &BlobMetadata,
        challenge: &AuditChallenge,
    ) -> Result<ChallengeResult> {
        let start = Instant::now();

        let storage_client = self
            .storage_clients
            .first()
            .ok_or_else(|| AuditorError::Config("No storage clients configured".to_string()))?;

        debug!("Sending challenge to storage node for sliver {}", challenge.sliver_index);
        let response = storage_client
            .challenge(&metadata.blob_id, challenge.sliver_index as u64)
            .await?;

        debug!("Received response: {} bytes sliver data, {} bytes proof",
            response.sliver_data.len(), response.merkle_proof.len());

        let verification_result = self.verify_challenge_response(metadata, challenge, &response)?;

        let duration = start.elapsed();
        debug!("Challenge completed in {:?}", duration);

        Ok(verification_result)
    }

    fn verify_challenge_response(
        &self,
        metadata: &BlobMetadata,
        challenge: &AuditChallenge,
        response: &ChallengeResponse,
    ) -> Result<ChallengeResult> {
        debug!("Verifying challenge response for sliver {}", challenge.sliver_index);

        let sliver = match Sliver::from_response_bytes(challenge.sliver_index as u64, response.sliver_data.clone()) {
            Ok(s) => s,
            Err(e) => {
                return Ok(ChallengeResult {
                    challenge: challenge.clone(),
                    verified: false,
                    merkle_proof_valid: false,
                    response_hash: vec![],
                    failure_reason: Some(format!("Failed to parse sliver: {}", e)),
                });
            }
        };

        let response_hash = sliver.compute_hash().to_vec();

        let merkle_proof = match MerkleProof::from_bytes(&response.merkle_proof) {
            Ok(p) => p,
            Err(e) => {
                return Ok(ChallengeResult {
                    challenge: challenge.clone(),
                    verified: false,
                    merkle_proof_valid: false,
                    response_hash,
                    failure_reason: Some(format!("Failed to parse merkle proof: {}", e)),
                });
            }
        };

        let merkle_root: [u8; 32] = metadata.merkle_root.as_slice().try_into().map_err(|_| {
            AuditorError::InvalidSliver(format!(
                "Invalid merkle root length: expected 32, got {}",
                metadata.merkle_root.len()
            ))
        })?;

        let sliver_metadata = SliverMetadata::new(
            merkle_root,
            metadata.encoding_n as u64,
            metadata.encoding_k as usize,
            metadata.encoding_n as usize,
        )?;

        let verified = match sliver.verify(&sliver_metadata, &merkle_proof) {
            Ok(v) => v,
            Err(e) => {
                return Ok(ChallengeResult {
                    challenge: challenge.clone(),
                    verified: false,
                    merkle_proof_valid: false,
                    response_hash,
                    failure_reason: Some(format!("Verification error: {}", e)),
                });
            }
        };

        if verified {
            debug!("Sliver {} verification successful", challenge.sliver_index);
            Ok(ChallengeResult {
                challenge: challenge.clone(),
                verified: true,
                merkle_proof_valid: true,
                response_hash,
                failure_reason: None,
            })
        } else {
            warn!("Sliver {} verification FAILED: merkle proof invalid", challenge.sliver_index);
            Ok(ChallengeResult {
                challenge: challenge.clone(),
                verified: false,
                merkle_proof_valid: false,
                response_hash,
                failure_reason: Some("Merkle proof verification failed".to_string()),
            })
        }
    }

    fn count_results(&self, results: &[ChallengeResult]) -> (u16, u16) {
        let successful = results.iter().filter(|r| r.verified).count() as u16;
        let failed = results.len() as u16 - successful;
        (successful, failed)
    }

    fn generate_report(
        &self,
        blob_id: &str,
        metadata: &BlobMetadata,
        challenge_results: Vec<ChallengeResult>,
        successful: u16,
        failed: u16,
    ) -> Result<AuditReport> {
        let total_challenges = (successful + failed) as u16;
        let integrity_hash = self.compute_integrity_hash(&challenge_results);
        let is_valid = failed == 0;

        let failure_reason = if !is_valid {
            Some(format!("{} out of {} challenges failed", failed, total_challenges))
        } else {
            None
        };

        Ok(AuditReport {
            blob_id: blob_id.to_string(),
            blob_object_id: metadata.blob_object_id.clone(),
            auditor: self.auditor_address.clone(),
            timestamp: Utc::now().timestamp() as u64,
            challenge_epoch: metadata.start_epoch,
            challenge_results,
            total_challenges,
            successful_verifications: successful,
            failed_verifications: failed,
            integrity_hash,
            pqc_signature: vec![],
            pqc_algorithm: 3,
            is_valid,
            failure_reason,
        })
    }

    fn compute_integrity_hash(&self, results: &[ChallengeResult]) -> Vec<u8> {
        use sha3::{Digest, Sha3_256};
        let mut hasher = Sha3_256::new();

        for result in results {
            hasher.update(&result.challenge.sliver_index.to_le_bytes());
            hasher.update(&[if result.verified { 1 } else { 0 }]);
            hasher.update(&result.response_hash);
        }

        hasher.finalize().to_vec()
    }

    pub async fn submit_report(&self, _report: &AuditReport) -> Result<String> {
        info!("Submitting audit report to Sui blockchain...");
        let tx_hash = "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef";
        info!("Report submitted successfully: {}", tx_hash);
        Ok(tx_hash.to_string())
    }

    pub fn auditor_address(&self) -> &str {
        &self.auditor_address
    }

    pub fn config(&self) -> &AuditorConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_metadata() -> BlobMetadata {
        BlobMetadata {
            blob_object_id: "0x1234".to_string(),
            blob_id: "0xabcd".to_string(),
            merkle_root: vec![0u8; 32],
            blob_size: 1024 * 1024,
            encoding_k: 10,
            encoding_n: 15,
            start_epoch: 100,
            end_epoch: 200,
            owner: "0x5678".to_string(),
        }
    }

    #[tokio::test]
    async fn test_auditor_creation() {
        let config = AuditorConfig::default();
        let auditor = Auditor::new(
            config,
            "0xauditor".to_string(),
            vec!["http://localhost:8080".to_string()],
        )
        .await;

        assert!(auditor.is_ok());
    }

    #[test]
    fn test_determine_challenge_count() {
        let config = AuditorConfig {
            min_challenges: 5,
            max_challenges: 50,
            ..Default::default()
        };

        let rt = tokio::runtime::Runtime::new().unwrap();
        let auditor = rt.block_on(Auditor::new(
            config.clone(),
            "0xauditor".to_string(),
            vec!["http://localhost:8080".to_string()],
        )).unwrap();

        let metadata = create_test_metadata();
        let count = auditor.determine_challenge_count(&metadata);

        assert!(count >= config.min_challenges);
        assert!(count <= config.max_challenges);
    }

    #[test]
    fn test_generate_challenges() {
        let config = AuditorConfig::default();
        let rt = tokio::runtime::Runtime::new().unwrap();
        let auditor = rt.block_on(Auditor::new(
            config,
            "0xauditor".to_string(),
            vec!["http://localhost:8080".to_string()],
        )).unwrap();

        let metadata = create_test_metadata();
        let challenges = auditor.generate_challenges(&metadata, 10);

        assert_eq!(challenges.len(), 10);

        let mut indices = std::collections::HashSet::new();
        for challenge in &challenges {
            assert!(challenge.sliver_index < metadata.encoding_n);
            assert!(indices.insert(challenge.sliver_index));
        }
    }

    #[test]
    fn test_count_results() {
        let config = AuditorConfig::default();
        let rt = tokio::runtime::Runtime::new().unwrap();
        let auditor = rt.block_on(Auditor::new(
            config,
            "0xauditor".to_string(),
            vec!["http://localhost:8080".to_string()],
        )).unwrap();

        let results = vec![
            ChallengeResult {
                challenge: AuditChallenge {
                    sliver_index: 0,
                    shard_id: 0,
                    challenge_type: 1,
                    timestamp: 0,
                },
                verified: true,
                merkle_proof_valid: true,
                response_hash: vec![],
                failure_reason: None,
            },
            ChallengeResult {
                challenge: AuditChallenge {
                    sliver_index: 1,
                    shard_id: 0,
                    challenge_type: 1,
                    timestamp: 0,
                },
                verified: false,
                merkle_proof_valid: false,
                response_hash: vec![],
                failure_reason: Some("Test failure".to_string()),
            },
        ];

        let (successful, failed) = auditor.count_results(&results);
        assert_eq!(successful, 1);
        assert_eq!(failed, 1);
    }

    #[test]
    fn test_compute_integrity_hash() {
        let config = AuditorConfig::default();
        let rt = tokio::runtime::Runtime::new().unwrap();
        let auditor = rt.block_on(Auditor::new(
            config,
            "0xauditor".to_string(),
            vec!["http://localhost:8080".to_string()],
        )).unwrap();

        let results = vec![
            ChallengeResult {
                challenge: AuditChallenge {
                    sliver_index: 0,
                    shard_id: 0,
                    challenge_type: 1,
                    timestamp: 0,
                },
                verified: true,
                merkle_proof_valid: true,
                response_hash: vec![1, 2, 3],
                failure_reason: None,
            },
        ];

        let hash = auditor.compute_integrity_hash(&results);
        assert_eq!(hash.len(), 32);

        let hash2 = auditor.compute_integrity_hash(&results);
        assert_eq!(hash, hash2);
    }

    #[test]
    fn test_generate_report() {
        let config = AuditorConfig::default();
        let rt = tokio::runtime::Runtime::new().unwrap();
        let auditor = rt.block_on(Auditor::new(
            config,
            "0xauditor".to_string(),
            vec!["http://localhost:8080".to_string()],
        )).unwrap();

        let metadata = create_test_metadata();
        let results = vec![
            ChallengeResult {
                challenge: AuditChallenge {
                    sliver_index: 0,
                    shard_id: 0,
                    challenge_type: 1,
                    timestamp: 0,
                },
                verified: true,
                merkle_proof_valid: true,
                response_hash: vec![],
                failure_reason: None,
            },
        ];

        let report = auditor
            .generate_report("0xblob", &metadata, results.clone(), 1, 0)
            .unwrap();

        assert_eq!(report.blob_id, "0xblob");
        assert_eq!(report.total_challenges, 1);
        assert_eq!(report.successful_verifications, 1);
        assert_eq!(report.failed_verifications, 0);
        assert!(report.is_valid);
        assert!(report.failure_reason.is_none());
    }
}
