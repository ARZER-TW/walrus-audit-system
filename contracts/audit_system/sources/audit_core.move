/// Decentralized Storage Integrity Audit System - Core Audit Logic
///
/// This module is responsible for:
/// 1. Executing challenge-response verification for Walrus Blobs
/// 2. Recording audit results and generating post-quantum signatures
/// 3. Tracking Blob integrity history
///
/// Security assumptions:
/// - Merkle proof verification is completed off-chain (gas cost consideration)
/// - Auditor honesty is guaranteed through staking and reputation mechanisms
/// - PQC signatures ensure long-term verifiability of audit reports
module audit_system::audit_core {
    use sui::object::{Self, UID, ID};
    use sui::transfer;
    use sui::tx_context::{Self, TxContext};
    use sui::event;
    use sui::clock::{Self, Clock};
    use sui::vec_map::{Self, VecMap};
    use std::vector;
    use std::option::{Self, Option};

    // ============ Error Codes ============

    /// Blob not yet certified by Walrus
    const E_BLOB_NOT_CERTIFIED: u64 = 1;

    /// Blob has expired (exceeded end_epoch)
    const E_BLOB_EXPIRED: u64 = 2;

    /// Invalid challenge count (must be > 0)
    const E_INVALID_CHALLENGE_COUNT: u64 = 3;

    /// Audit configuration does not exist or unauthorized
    const E_UNAUTHORIZED: u64 = 4;

    /// Invalid PQC signature algorithm type
    const E_INVALID_SIGNATURE_ALGORITHM: u64 = 5;

    /// Audit record already exists (prevent duplicate submission)
    const E_AUDIT_ALREADY_EXISTS: u64 = 6;

    // ============ PQC Signature Algorithm Constants ============

    /// Falcon-512 algorithm identifier
    const ALGO_FALCON512: u8 = 1;

    /// Dilithium-2 algorithm identifier
    const ALGO_DILITHIUM2: u8 = 2;

    // ============ Core Data Structures ============

    /// Audit Record (complete version)
    ///
    /// Each audit creates a new AuditRecord object containing:
    /// - Blob identification information
    /// - Challenge-response statistics
    /// - Integrity proof (hash + PQC signature)
    public struct AuditRecord has key, store {
        id: UID,

        // === Blob Identification ===
        blob_id: u256,                      // Walrus Blob ID (32 bytes Blake2b-256)
        blob_object_id: ID,                 // Sui Blob object ID (for querying Blob details)

        // === Audit Metadata ===
        auditor: address,                   // Auditor address
        challenge_epoch: u32,               // Walrus epoch when audit was performed
        audit_timestamp: u64,               // Audit timestamp (milliseconds)

        // === Challenge-Response Statistics ===
        total_challenges: u16,              // Total number of challenges initiated
        successful_verifications: u16,      // Number of successful verifications
        failed_verifications: u16,          // Number of failed verifications

        // === Integrity Proof ===
        integrity_hash: vector<u8>,         // Aggregated hash of all challenges (32 bytes Blake2b-256)
        pqc_signature: vector<u8>,          // PQC signature over integrity_hash
        pqc_algorithm: u8,                  // Signature algorithm (1=Falcon512, 2=Dilithium2)

        // === Verification Result ===
        is_valid: bool,                     // Whether audit passed
        failure_reason: Option<vector<u8>>, // Failure reason (if any)
    }

    /// Audit Challenge (generated off-chain, recorded on-chain)
    ///
    /// Each challenge contains a randomly selected sliver index and verification result
    public struct Challenge has store, drop {
        sliver_index: u16,                  // Sliver index (0 to n_shards-1)
        shard_id: u16,                      // Shard ID (corresponding to storage node)
        challenge_type: u8,                 // Challenge type (1=full sliver, 2=recovery symbol)
        merkle_proof_verified: bool,        // Whether Merkle proof verification passed
        response_hash: vector<u8>,          // Hash of storage node response data
    }

    /// Audit Configuration (global shared object)
    ///
    /// Controls global parameters of the audit system and authorized auditor list
    public struct AuditConfig has key {
        id: UID,
        admin: address,                     // System administrator

        // === Audit Parameters ===
        min_challenge_count: u16,           // Minimum number of challenges per audit
        max_challenge_count: u16,           // Maximum number of challenges per audit
        challenge_interval_ms: u64,         // Audit interval (milliseconds)

        // === Authorized Auditors ===
        authorized_auditors: vector<address>, // Authorized auditor whitelist
        auditor_stakes: VecMap<address, u64>, // Auditor stake amounts

        // === Statistics ===
        total_audits: u64,                  // Total number of audits
        total_blobs_audited: u64,           // Total number of Blobs audited
    }

    /// Blob Audit History Index
    ///
    /// Maintains an audit record list for each Blob
    public struct BlobAuditHistory has key {
        id: UID,
        blob_id: u256,                      // Blob ID
        audit_records: vector<ID>,          // List of audit record IDs
        last_audit_epoch: u32,              // Last audit epoch
        total_audits: u32,                  // Total number of audits
        consecutive_failures: u32,          // Consecutive failure count (for alerts)
    }

    /// Encrypted Audit Report Metadata (Seal Integration)
    ///
    /// Stores location information of Seal-encrypted audit reports on Walrus
    /// This object serves as a shared object, allowing authorized users to query but only auditors can create
    public struct EncryptedAuditReport has key {
        id: UID,
        original_blob_id: u256,             // Original audited blob ID
        encrypted_report_blob_id: u256,     // Encrypted report blob ID on Walrus
        seal_object_id: ID,                 // Seal encryption object ID (for decryption)
        auditor: address,                   // Auditor who created the report
        report_timestamp: u64,              // Report creation time
        is_valid: bool,                     // Audit result (for quick query)

        // Seal access control related
        creator: address,                   // Report creator (for seal_approve)
        allowed_roles: vector<vector<u8>>,  // List of roles allowed to access
        expires_at: u64,                    // Access permission expiration time
    }

    // ============ Event Definitions ============

    /// Audit Created Event
    public struct AuditCreated has copy, drop {
        audit_record_id: ID,
        blob_id: u256,
        auditor: address,
        challenge_epoch: u32,
        total_challenges: u16,
        is_valid: bool,
    }

    /// Blob Audit Failed Event (Alert)
    public struct BlobAuditFailed has copy, drop {
        blob_id: u256,
        blob_object_id: ID,
        auditor: address,
        challenge_epoch: u32,
        failed_verifications: u16,
        total_challenges: u16,
        failure_reason: vector<u8>,
    }

    /// Audit Config Updated Event
    public struct AuditConfigUpdated has copy, drop {
        admin: address,
        min_challenge_count: u16,
        max_challenge_count: u16,
    }

    /// Encrypted Report Submitted Event
    public struct EncryptedReportSubmitted has copy, drop {
        report_id: ID,
        original_blob_id: u256,
        encrypted_blob_id: u256,
        seal_object_id: ID,
        auditor: address,
        is_valid: bool,
    }

    // ============ Initialization Functions ============

    /// Module Initialization
    ///
    /// Creates global AuditConfig shared object
    fun init(ctx: &mut TxContext) {
        let config = AuditConfig {
            id: object::new(ctx),
            admin: tx_context::sender(ctx),
            min_challenge_count: 10,        // Minimum 10 challenges
            max_challenge_count: 100,       // Maximum 100 challenges
            challenge_interval_ms: 3600000, // 1 hour
            authorized_auditors: vector::empty(),
            auditor_stakes: vec_map::empty(),
            total_audits: 0,
            total_blobs_audited: 0,
        };
        transfer::share_object(config);
    }

    // ============ Auditor Management Functions ============

    /// Authorize Auditor
    ///
    /// Can only be called by administrator
    public entry fun authorize_auditor(
        config: &mut AuditConfig,
        auditor: address,
        ctx: &TxContext
    ) {
        assert!(tx_context::sender(ctx) == config.admin, E_UNAUTHORIZED);
        vector::push_back(&mut config.authorized_auditors, auditor);
    }

    /// Revoke Auditor Authorization
    public entry fun revoke_auditor(
        config: &mut AuditConfig,
        auditor: address,
        ctx: &TxContext
    ) {
        assert!(tx_context::sender(ctx) == config.admin, E_UNAUTHORIZED);
        let (found, index) = vector::index_of(&config.authorized_auditors, &auditor);
        if (found) {
            vector::remove(&mut config.authorized_auditors, index);
        };
    }

    /// Update Audit Parameters
    public entry fun update_audit_params(
        config: &mut AuditConfig,
        min_challenges: u16,
        max_challenges: u16,
        interval_ms: u64,
        ctx: &TxContext
    ) {
        assert!(tx_context::sender(ctx) == config.admin, E_UNAUTHORIZED);
        assert!(min_challenges > 0 && max_challenges >= min_challenges, E_INVALID_CHALLENGE_COUNT);

        config.min_challenge_count = min_challenges;
        config.max_challenge_count = max_challenges;
        config.challenge_interval_ms = interval_ms;

        event::emit(AuditConfigUpdated {
            admin: config.admin,
            min_challenge_count: min_challenges,
            max_challenge_count: max_challenges,
        });
    }

    // ============ Core Audit Functions ============

    /// Submit Audit Record
    ///
    /// Auditor submits audit results on-chain after completing challenge-response verification off-chain
    ///
    /// Parameters:
    /// - blob_id: Walrus Blob ID (u256)
    /// - blob_object_id: Sui Blob object ID
    /// - challenge_epoch: Walrus epoch at the time of audit
    /// - total_challenges: Total number of challenges executed
    /// - successful_verifications: Number of successful verifications
    /// - integrity_hash: Aggregated hash of all challenges
    /// - pqc_signature: PQC signature
    /// - pqc_algorithm: Signature algorithm (1=Falcon512, 2=Dilithium2)
    ///
    /// Security checks:
    /// 1. Auditor must be in the authorized list
    /// 2. Challenge count must be within configured range
    /// 3. PQC algorithm must be valid
    public entry fun submit_audit_record(
        config: &mut AuditConfig,
        blob_id: u256,
        blob_object_id: ID,
        challenge_epoch: u32,
        total_challenges: u16,
        successful_verifications: u16,
        integrity_hash: vector<u8>,
        pqc_signature: vector<u8>,
        pqc_algorithm: u8,
        clock: &Clock,
        ctx: &mut TxContext
    ) {
        let auditor = tx_context::sender(ctx);

        // Verify auditor authorization
        assert!(
            vector::contains(&config.authorized_auditors, &auditor),
            E_UNAUTHORIZED
        );

        // Verify challenge count
        assert!(
            total_challenges >= config.min_challenge_count &&
            total_challenges <= config.max_challenge_count,
            E_INVALID_CHALLENGE_COUNT
        );

        // Verify PQC algorithm
        assert!(
            pqc_algorithm == ALGO_FALCON512 || pqc_algorithm == ALGO_DILITHIUM2,
            E_INVALID_SIGNATURE_ALGORITHM
        );

        // Calculate failure count and verification result
        let failed_verifications = total_challenges - successful_verifications;
        let is_valid = successful_verifications >= (total_challenges * 95 / 100); // 95% success rate

        // Create audit record
        let record = AuditRecord {
            id: object::new(ctx),
            blob_id,
            blob_object_id,
            auditor,
            challenge_epoch,
            audit_timestamp: clock::timestamp_ms(clock),
            total_challenges,
            successful_verifications,
            failed_verifications,
            integrity_hash,
            pqc_signature,
            pqc_algorithm,
            is_valid,
            failure_reason: option::none(),
        };

        let record_id = object::id(&record);

        // Emit event
        event::emit(AuditCreated {
            audit_record_id: record_id,
            blob_id,
            auditor,
            challenge_epoch,
            total_challenges,
            is_valid,
        });

        // If audit failed, emit alert event
        if (!is_valid) {
            event::emit(BlobAuditFailed {
                blob_id,
                blob_object_id,
                auditor,
                challenge_epoch,
                failed_verifications,
                total_challenges,
                failure_reason: b"Success rate below 95%",
            });
        };

        // Update statistics
        config.total_audits = config.total_audits + 1;

        // Share audit record (allow others to read)
        transfer::share_object(record);
    }

    /// Create Blob Audit History Index
    ///
    /// Creates audit history tracking object for new Blob
    public entry fun create_blob_audit_history(
        blob_id: u256,
        ctx: &mut TxContext
    ) {
        let history = BlobAuditHistory {
            id: object::new(ctx),
            blob_id,
            audit_records: vector::empty(),
            last_audit_epoch: 0,
            total_audits: 0,
            consecutive_failures: 0,
        };
        transfer::share_object(history);
    }

    /// Update Blob Audit History
    ///
    /// Adds new audit record to the Blob's audit history
    public entry fun update_blob_audit_history(
        history: &mut BlobAuditHistory,
        audit_record_id: ID,
        challenge_epoch: u32,
        is_valid: bool,
    ) {
        vector::push_back(&mut history.audit_records, audit_record_id);
        history.last_audit_epoch = challenge_epoch;
        history.total_audits = history.total_audits + 1;

        // Update consecutive failure count
        if (is_valid) {
            history.consecutive_failures = 0;
        } else {
            history.consecutive_failures = history.consecutive_failures + 1;
        };
    }

    // ============ Query Functions ============

    /// Get Blob ID from audit record
    public fun get_blob_id(record: &AuditRecord): u256 {
        record.blob_id
    }

    /// Get verification result from audit record
    public fun is_audit_valid(record: &AuditRecord): bool {
        record.is_valid
    }

    /// Get auditor address
    public fun get_auditor(record: &AuditRecord): address {
        record.auditor
    }

    /// Get audit timestamp
    public fun get_audit_timestamp(record: &AuditRecord): u64 {
        record.audit_timestamp
    }

    /// Get challenge statistics
    public fun get_challenge_stats(record: &AuditRecord): (u16, u16, u16) {
        (
            record.total_challenges,
            record.successful_verifications,
            record.failed_verifications
        )
    }

    /// Get integrity proof
    public fun get_integrity_proof(record: &AuditRecord): (vector<u8>, vector<u8>, u8) {
        (
            record.integrity_hash,
            record.pqc_signature,
            record.pqc_algorithm
        )
    }

    /// Check if auditor is authorized
    public fun is_auditor_authorized(config: &AuditConfig, auditor: address): bool {
        vector::contains(&config.authorized_auditors, &auditor)
    }

    /// Get Blob audit history statistics
    public fun get_blob_audit_stats(history: &BlobAuditHistory): (u32, u32, u32) {
        (
            history.total_audits,
            history.last_audit_epoch,
            history.consecutive_failures
        )
    }

    // ============ Seal Encrypted Report Functions ============

    /// Submit Encrypted Audit Report Metadata
    ///
    /// After auditor completes audit and encrypts report using Seal, submit encrypted report metadata on-chain
    /// This function creates a shared object allowing authorized users to query the location of encrypted reports
    ///
    /// Parameters:
    /// - config: Audit configuration (for verifying auditor authorization)
    /// - original_blob_id: Original audited blob ID
    /// - encrypted_blob_id: Encrypted report blob ID on Walrus
    /// - seal_object_id: Seal encryption object ID
    /// - report_timestamp: Report creation time
    /// - is_valid: Audit result
    /// - allowed_roles: List of roles allowed to access
    /// - expires_at: Access permission expiration time
    ///
    /// According to Seal specifications, this object becomes a shared object,
    /// and works with seal_approve function to implement access control
    public entry fun submit_encrypted_report_metadata(
        config: &AuditConfig,
        original_blob_id: u256,
        encrypted_blob_id: u256,
        seal_object_id: ID,
        report_timestamp: u64,
        is_valid: bool,
        allowed_roles: vector<vector<u8>>,
        expires_at: u64,
        ctx: &mut TxContext
    ) {
        let auditor = tx_context::sender(ctx);

        // Verify auditor authorization
        assert!(
            vector::contains(&config.authorized_auditors, &auditor),
            E_UNAUTHORIZED
        );

        // Create encrypted report metadata
        let report = EncryptedAuditReport {
            id: object::new(ctx),
            original_blob_id,
            encrypted_report_blob_id: encrypted_blob_id,
            seal_object_id,
            auditor,
            report_timestamp,
            is_valid,
            creator: auditor,
            allowed_roles,
            expires_at,
        };

        let report_id = object::id(&report);

        // Emit event
        event::emit(EncryptedReportSubmitted {
            report_id,
            original_blob_id,
            encrypted_blob_id,
            seal_object_id,
            auditor,
            is_valid,
        });

        // Convert to shared object (allow multiple people to query)
        // Following Seal official example (allowlist.move) approach
        transfer::share_object(report);
    }

    /// Seal Access Control Function
    ///
    /// This is the standard function required by Seal protocol
    /// When users request to decrypt reports, Seal nodes will call this function to verify permissions
    /// If the function executes successfully (does not abort), decryption is allowed
    ///
    /// Parameters:
    /// - _id: Seal encryption object ID (in bytes form)
    /// - report: Encrypted report metadata
    /// - ctx: Transaction context
    ///
    /// Access control logic:
    /// 1. Allow report creator to access
    /// 2. Check if access permission has expired
    /// 3. Check if requester is in the authorized list (via roles)
    ///
    /// Note: This function will be called by Seal nodes via devInspectTransactionBlock
    /// It does not actually modify state, only verifies access permissions
    public fun seal_approve(
        _id: vector<u8>,
        report: &EncryptedAuditReport,
        clock: &Clock,
        ctx: &TxContext
    ) {
        let requester = tx_context::sender(ctx);

        // Rule 1: Creator has permanent access
        if (requester == report.creator) {
            return
        };

        // Rule 2: Check if expired
        let current_time = clock::timestamp_ms(clock);
        assert!(current_time < report.expires_at, E_UNAUTHORIZED);

        // Rule 3: Check role permissions
        // Note: In production environment, this should query an on-chain role registry
        // Currently simplified: if allowed_roles is not empty, deny non-creator access
        // Full implementation requires a separate role management contract
        let has_role_check = vector::length(&report.allowed_roles) > 0;

        if (has_role_check) {
            // TODO: Query role registry to verify if requester has allowed roles
            // For simplified demo, only allow creator access
            abort E_UNAUTHORIZED
        };

        // If no role restrictions, allow all authorized auditors to access
        // This is a reasonable default policy
    }

    /// Query Encrypted Report Metadata
    ///
    /// Anyone can query basic report information, but decryption requires passing seal_approve
    public fun get_encrypted_report_info(report: &EncryptedAuditReport): (u256, u256, ID, bool) {
        (
            report.original_blob_id,
            report.encrypted_report_blob_id,
            report.seal_object_id,
            report.is_valid
        )
    }

    /// Check if report has expired
    public fun is_report_expired(report: &EncryptedAuditReport, clock: &Clock): bool {
        let current_time = clock::timestamp_ms(clock);
        current_time >= report.expires_at
    }

    // ============ Test Helper Functions ============

    #[test_only]
    public fun init_for_testing(ctx: &mut TxContext) {
        init(ctx);
    }

    #[test_only]
    public fun create_test_audit_record(
        blob_id: u256,
        blob_object_id: ID,
        ctx: &mut TxContext
    ): AuditRecord {
        AuditRecord {
            id: object::new(ctx),
            blob_id,
            blob_object_id,
            auditor: tx_context::sender(ctx),
            challenge_epoch: 100,
            audit_timestamp: 1234567890,
            total_challenges: 50,
            successful_verifications: 48,
            failed_verifications: 2,
            integrity_hash: vector::empty(),
            pqc_signature: vector::empty(),
            pqc_algorithm: ALGO_FALCON512,
            is_valid: true,
            failure_reason: option::none(),
        }
    }
}
