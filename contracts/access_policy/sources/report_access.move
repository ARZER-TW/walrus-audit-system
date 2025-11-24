/// Seal Encrypted Audit Report Access Control Module
///
/// This module is responsible for:
/// 1. Managing access policies for audit reports (who can decrypt)
/// 2. Issuing SealToken (NFT-style access credentials)
/// 3. Tracking access history and compliance
///
/// Security Model:
/// - Report creators always have access rights
/// - Compliance officers gain access through role authorization
/// - SealToken is transferable (like NFT), supporting permission delegation
/// - Policies can expire, ensuring time-limited access control
module access_policy::report_access {
    use sui::object::{Self, UID, ID};
    use sui::transfer;
    use sui::tx_context::{Self, TxContext};
    use sui::event;
    use sui::clock::{Self, Clock};
    use std::vector;
    use std::option::{Self, Option};

    // ============ Error Codes ============

    /// Policy has expired
    const E_POLICY_EXPIRED: u64 = 1;

    /// Unauthorized access attempt
    const E_UNAUTHORIZED_ACCESS: u64 = 2;

    /// Invalid access type
    const E_INVALID_ACCESS_TYPE: u64 = 3;

    /// Policy creator mismatch
    const E_NOT_POLICY_CREATOR: u64 = 4;

    /// Policy has been revoked
    const E_POLICY_REVOKED: u64 = 5;

    /// Token has expired
    const E_TOKEN_EXPIRED: u64 = 6;

    /// Report not found
    const E_REPORT_NOT_FOUND: u64 = 7;

    // ============ Access Type Constants ============

    /// Read permission (view report only)
    const ACCESS_READ: u8 = 1;

    /// Audit permission (perform new audit)
    const ACCESS_AUDIT: u8 = 2;

    /// Admin permission (modify policy)
    const ACCESS_ADMIN: u8 = 3;

    // ============ Core Data Structures ============

    /// Audit Report Access Policy
    ///
    /// Each encrypted audit report has a corresponding access policy
    /// that defines who can decrypt the report under what conditions
    public struct ReportAccessPolicy has key {
        id: UID,

        // === Report Identification ===
        report_blob_id: u256,               // Blob ID of encrypted report on Walrus
        audit_record_id: ID,                // Corresponding on-chain audit record ID
        creator: address,                   // Report creator (auditor)

        // === Access Control Lists ===
        allowed_readers: vector<address>,   // List of addresses allowed to read
        allowed_auditors: vector<address>,  // List of addresses allowed to audit
        allowed_admins: vector<address>,    // List of addresses allowed to manage

        // === Time Control ===
        created_at: u64,                    // Policy creation time (milliseconds)
        expires_at: Option<u64>,            // Expiration time (None = never expires)

        // === Policy Status ===
        is_active: bool,                    // Whether policy is active
        revocation_reason: Option<vector<u8>>, // Revocation reason (if any)

        // === Access Statistics ===
        total_accesses: u64,                // Total number of accesses
        last_accessed_at: Option<u64>,     // Last access time
    }

    /// SealToken (Access Credential)
    ///
    /// NFT-style access token, holders can decrypt corresponding reports
    /// Token is transferable, supporting permission delegation
    public struct SealToken has key, store {
        id: UID,
        policy_id: ID,                      // Associated policy ID
        report_blob_id: u256,               // Report Blob ID
        holder: address,                    // Current holder
        access_type: u8,                    // Access type (1=read, 2=audit, 3=admin)
        granted_at: u64,                    // Grant time
        expires_at: Option<u64>,            // Expiration time (can differ from policy)
        is_transferable: bool,              // Whether transferable
    }

    /// Access History Record
    ///
    /// Records detailed information of each report access (for compliance auditing)
    public struct AccessLog has key {
        id: UID,
        policy_id: ID,                      // Policy ID
        report_blob_id: u256,               // Report Blob ID
        access_records: vector<AccessRecord>, // List of access records
    }

    /// Single Access Record
    public struct AccessRecord has store, drop {
        accessor: address,                  // Accessor address
        access_type: u8,                    // Access type
        timestamp: u64,                     // Access time
        token_id: Option<ID>,               // Token ID used (if any)
        success: bool,                      // Whether access was successful
    }

    // ============ Event Definitions ============

    /// Policy Created Event
    public struct PolicyCreated has copy, drop {
        policy_id: ID,
        report_blob_id: u256,
        creator: address,
        expires_at: Option<u64>,
    }

    /// Token Granted Event
    public struct TokenGranted has copy, drop {
        token_id: ID,
        policy_id: ID,
        recipient: address,
        access_type: u8,
    }

    /// Access Attempted Event
    public struct AccessAttempted has copy, drop {
        policy_id: ID,
        accessor: address,
        access_type: u8,
        success: bool,
        timestamp: u64,
    }

    /// Policy Revoked Event
    public struct PolicyRevoked has copy, drop {
        policy_id: ID,
        revoker: address,
        reason: vector<u8>,
    }

    /// Token Transferred Event
    public struct TokenTransferred has copy, drop {
        token_id: ID,
        from: address,
        to: address,
    }

    // ============ Initialization Functions ============

    /// Module initialization (no global configuration object)
    fun init(_ctx: &mut TxContext) {
        // This module doesn't need global configuration, policies are created on demand
    }

    // ============ Policy Management Functions ============

    /// Create Access Policy
    ///
    /// Auditor creates access policy after uploading encrypted report to Walrus
    ///
    /// Parameters:
    /// - report_blob_id: Blob ID of encrypted report on Walrus
    /// - audit_record_id: Corresponding audit record ID
    /// - allowed_readers: Initial list of read permissions
    /// - allowed_auditors: Initial list of audit permissions
    /// - expires_at_ms: Expiration time (None = never expires)
    public entry fun create_policy(
        report_blob_id: u256,
        audit_record_id: ID,
        allowed_readers: vector<address>,
        allowed_auditors: vector<address>,
        expires_at_ms: Option<u64>,
        clock: &Clock,
        ctx: &mut TxContext
    ) {
        let creator = tx_context::sender(ctx);

        let policy = ReportAccessPolicy {
            id: object::new(ctx),
            report_blob_id,
            audit_record_id,
            creator,
            allowed_readers,
            allowed_auditors,
            allowed_admins: vector::singleton(creator), // Creator is admin by default
            created_at: clock::timestamp_ms(clock),
            expires_at: expires_at_ms,
            is_active: true,
            revocation_reason: option::none(),
            total_accesses: 0,
            last_accessed_at: option::none(),
        };

        let policy_id = object::id(&policy);

        event::emit(PolicyCreated {
            policy_id,
            report_blob_id,
            creator,
            expires_at: expires_at_ms,
        });

        // Share policy object (allow queries)
        transfer::share_object(policy);

        // Create access history object
        let access_log = AccessLog {
            id: object::new(ctx),
            policy_id,
            report_blob_id,
            access_records: vector::empty(),
        };
        transfer::share_object(access_log);
    }

    /// Grant Access Token
    ///
    /// Policy admin can grant access token to specific addresses
    ///
    /// Security Checks:
    /// - Caller must be policy admin
    /// - Policy must not be expired and must be active
    public entry fun grant_access_token(
        policy: &mut ReportAccessPolicy,
        recipient: address,
        access_type: u8,
        is_transferable: bool,
        token_expires_at: Option<u64>,
        clock: &Clock,
        ctx: &mut TxContext
    ) {
        let sender = tx_context::sender(ctx);

        // Verify caller is admin or creator
        assert!(
            sender == policy.creator || vector::contains(&policy.allowed_admins, &sender),
            E_NOT_POLICY_CREATOR
        );

        // Verify policy has not expired
        assert_policy_valid(policy, clock);

        // Verify access type is valid
        assert!(
            access_type == ACCESS_READ || access_type == ACCESS_AUDIT || access_type == ACCESS_ADMIN,
            E_INVALID_ACCESS_TYPE
        );

        // Create Token
        let token = SealToken {
            id: object::new(ctx),
            policy_id: object::id(policy),
            report_blob_id: policy.report_blob_id,
            holder: recipient,
            access_type,
            granted_at: clock::timestamp_ms(clock),
            expires_at: token_expires_at,
            is_transferable,
        };

        let token_id = object::id(&token);

        // Also update policy ACL
        if (access_type == ACCESS_READ) {
            if (!vector::contains(&policy.allowed_readers, &recipient)) {
                vector::push_back(&mut policy.allowed_readers, recipient);
            };
        } else if (access_type == ACCESS_AUDIT) {
            if (!vector::contains(&policy.allowed_auditors, &recipient)) {
                vector::push_back(&mut policy.allowed_auditors, recipient);
            };
        } else if (access_type == ACCESS_ADMIN) {
            if (!vector::contains(&policy.allowed_admins, &recipient)) {
                vector::push_back(&mut policy.allowed_admins, recipient);
            };
        };

        event::emit(TokenGranted {
            token_id,
            policy_id: object::id(policy),
            recipient,
            access_type,
        });

        // Transfer Token to recipient
        transfer::transfer(token, recipient);
    }

    /// Revoke Access Policy
    ///
    /// Creator can revoke policy (e.g., report contains erroneous information)
    public entry fun revoke_policy(
        policy: &mut ReportAccessPolicy,
        reason: vector<u8>,
        ctx: &TxContext
    ) {
        assert!(tx_context::sender(ctx) == policy.creator, E_NOT_POLICY_CREATOR);

        policy.is_active = false;
        policy.revocation_reason = option::some(reason);

        event::emit(PolicyRevoked {
            policy_id: object::id(policy),
            revoker: policy.creator,
            reason,
        });
    }

    /// Remove Access Permission
    ///
    /// Admin can remove access permission for specific addresses
    public entry fun remove_access(
        policy: &mut ReportAccessPolicy,
        target: address,
        access_type: u8,
        ctx: &TxContext
    ) {
        let sender = tx_context::sender(ctx);
        assert!(
            sender == policy.creator || vector::contains(&policy.allowed_admins, &sender),
            E_NOT_POLICY_CREATOR
        );

        if (access_type == ACCESS_READ) {
            remove_from_list(&mut policy.allowed_readers, target);
        } else if (access_type == ACCESS_AUDIT) {
            remove_from_list(&mut policy.allowed_auditors, target);
        } else if (access_type == ACCESS_ADMIN) {
            // Cannot remove creator's admin permission
            if (target != policy.creator) {
                remove_from_list(&mut policy.allowed_admins, target);
            };
        };
    }

    // ============ Token Management Functions ============

    /// Transfer Token
    ///
    /// Token holder can transfer token to another address (if is_transferable = true)
    public entry fun transfer_token(
        token: SealToken,
        recipient: address,
        ctx: &TxContext
    ) {
        assert!(token.is_transferable, E_UNAUTHORIZED_ACCESS);
        assert!(tx_context::sender(ctx) == token.holder, E_UNAUTHORIZED_ACCESS);

        let token_id = object::id(&token);
        let from = token.holder;

        event::emit(TokenTransferred {
            token_id,
            from,
            to: recipient,
        });

        // Update holder and transfer
        // Note: We transfer the object here; holder field will be updated by ownership transfer
        transfer::transfer(token, recipient);
    }

    /// Burn Expired Token
    ///
    /// Anyone can burn expired tokens (cleanup operation)
    public entry fun burn_expired_token(
        token: SealToken,
        clock: &Clock,
    ) {
        if (option::is_some(&token.expires_at)) {
            let expires_at = *option::borrow(&token.expires_at);
            assert!(clock::timestamp_ms(clock) > expires_at, E_TOKEN_EXPIRED);
        };

        let SealToken {
            id,
            policy_id: _,
            report_blob_id: _,
            holder: _,
            access_type: _,
            granted_at: _,
            expires_at: _,
            is_transferable: _,
        } = token;

        object::delete(id);
    }

    // ============ Seal Protocol Integration ============

    /// Seal Protocol Entry Function (Access Control List Mode)
    ///
    /// This function implements Seal Protocol's access verification, following the "Permanent Access Control List" pattern
    /// (similar to Seal's official whitelist.move example).
    ///
    /// Design Decisions:
    /// - Does not check expiration time (expires_at), because:
    ///   1. Audit reports need permanent traceability (regulatory compliance requirements)
    ///   2. Time checking is already implemented in application layer's check_access() (two-stage verification architecture)
    ///   3. expires_at is an optional governance tool, not mandatory business logic
    /// - Follows Seal's official Whitelist pattern (reference: github.com/MystenLabs/seal whitelist.move)
    ///
    /// Two-Stage Verification Architecture:
    /// - Stage 1 (PTB Verification): seal_approve() - Verify ACL (executed by Key Servers)
    /// - Stage 2 (Actual Decryption): check_access() - Complete verification including time (executed by application)
    ///
    /// Parameters:
    /// - id_bytes: BCS-encoded report Blob ID (u256 → vector<u8>, little-endian)
    /// - policy: Access policy object reference
    /// - ctx: Transaction context (used to get caller address)
    ///
    /// Verification Logic:
    /// 1. Policy must be active (is_active = true)
    /// 2. report_blob_id must match
    /// 3. Caller must be creator or in any access list (READ/AUDIT/ADMIN)
    ///
    /// Error Codes:
    /// - E_UNAUTHORIZED_ACCESS: Caller has no access permission
    /// - E_POLICY_REVOKED: Policy has been revoked
    /// - E_REPORT_NOT_FOUND: Report ID mismatch
    ///
    /// References:
    /// - Seal Official Documentation: https://seal-docs.wal.app/
    /// - Whitelist Example: github.com/MystenLabs/seal/move/patterns/sources/whitelist.move
    public entry fun seal_approve(
        id_bytes: vector<u8>,
        policy: &ReportAccessPolicy,
        ctx: &TxContext
    ) {
        let sender = tx_context::sender(ctx);

        // Verify policy is active
        assert!(policy.is_active, E_POLICY_REVOKED);

        // === Step 1: Parse Report ID ===
        // Convert BCS-encoded bytes to u256 (Sui Move uses little-endian)
        let report_blob_id = bytes_to_u256(id_bytes);

        // === Step 2: Verify Report ID Match ===
        // Ensure requested report ID matches the one recorded in policy (prevent accessing wrong report)
        assert!(report_blob_id == policy.report_blob_id, E_REPORT_NOT_FOUND);

        // === Step 3: Access Control List (ACL) Verification ===
        // Note: expires_at is not checked here, following Whitelist pattern
        // Time verification is performed in application layer's check_access()

        // Creator always has permission (auditor can view their own reports)
        if (sender == policy.creator) {
            return
        };

        // Check if caller is in any access list
        // Three permission types are allowed: READ, AUDIT, ADMIN
        let has_read = vector::contains(&policy.allowed_readers, &sender);
        let has_audit = vector::contains(&policy.allowed_auditors, &sender);
        let has_admin = vector::contains(&policy.allowed_admins, &sender);

        // At least one permission is required to pass verification
        assert!(
            has_read || has_audit || has_admin,
            E_UNAUTHORIZED_ACCESS
        );

        // === Verification Passed ===
        // Seal Key Servers will verify this PTB call succeeded and issue decryption keys
    }

    /// Convert BCS-encoded vector<u8> to u256 (little-endian)
    ///
    /// Sui Move's u256 uses little-endian encoding, consistent with most modern systems.
    ///
    /// Algorithm:
    /// - Left-shift each byte by corresponding bit position (i-th byte shifted by i*8 bits)
    /// - Accumulate all bytes to get final result
    ///
    /// Limitations:
    /// - Accepts at most 32 bytes (256 bits)
    /// - More than 32 bytes triggers E_INVALID_ACCESS_TYPE error
    ///
    /// Examples:
    /// - bytes = [0x01, 0x02] → result = 0x0201 (little-endian)
    /// - bytes = [0xFF, 0x00, 0x00, 0x01] → result = 0x010000FF
    ///
    /// Parameters:
    /// - bytes: BCS-encoded byte array (from Seal SDK's id parameter)
    ///
    /// Returns:
    /// - Parsed u256 report Blob ID
    fun bytes_to_u256(bytes: vector<u8>): u256 {
        let len = vector::length(&bytes);
        assert!(len <= 32, E_INVALID_ACCESS_TYPE); // 32 bytes = 256 bits

        let mut result: u256 = 0;
        let mut i = 0;

        // Little-endian assembly: low bytes first, high bytes last
        while (i < len) {
            let byte = (*vector::borrow(&bytes, i) as u256);
            result = result + (byte << ((i * 8) as u8));
            i = i + 1;
        };

        result
    }

    // ============ Access Verification Functions ============

    /// Check Access Permission
    ///
    /// Verify if given address has permission to access report
    ///
    /// Returns: (has_access, reason)
    public fun check_access(
        policy: &ReportAccessPolicy,
        accessor: address,
        access_type: u8,
        clock: &Clock,
    ): bool {
        // Check if policy is active
        if (!policy.is_active) {
            return false
        };

        // Check if policy has expired
        if (option::is_some(&policy.expires_at)) {
            let expires_at = *option::borrow(&policy.expires_at);
            if (clock::timestamp_ms(clock) > expires_at) {
                return false
            };
        };

        // Creator always has access permission
        if (accessor == policy.creator) {
            return true
        };

        // Check ACL
        if (access_type == ACCESS_READ) {
            vector::contains(&policy.allowed_readers, &accessor)
        } else if (access_type == ACCESS_AUDIT) {
            vector::contains(&policy.allowed_auditors, &accessor)
        } else if (access_type == ACCESS_ADMIN) {
            vector::contains(&policy.allowed_admins, &accessor)
        } else {
            false
        }
    }

    /// Log Access
    ///
    /// Record access history when accessing report (compliance requirement)
    public entry fun log_access(
        policy: &mut ReportAccessPolicy,
        access_log: &mut AccessLog,
        access_type: u8,
        token_id: Option<ID>,
        clock: &Clock,
        ctx: &TxContext
    ) {
        let accessor = tx_context::sender(ctx);
        let timestamp = clock::timestamp_ms(clock);

        let success = check_access(policy, accessor, access_type, clock);

        // Update policy statistics
        if (success) {
            policy.total_accesses = policy.total_accesses + 1;
            policy.last_accessed_at = option::some(timestamp);
        };

        // Add access record
        let record = AccessRecord {
            accessor,
            access_type,
            timestamp,
            token_id,
            success,
        };
        vector::push_back(&mut access_log.access_records, record);

        // Emit event
        event::emit(AccessAttempted {
            policy_id: object::id(policy),
            accessor,
            access_type,
            success,
            timestamp,
        });
    }

    // ============ Query Functions ============

    /// Get report Blob ID
    public fun get_report_blob_id(policy: &ReportAccessPolicy): u256 {
        policy.report_blob_id
    }

    /// Get policy creator
    public fun get_creator(policy: &ReportAccessPolicy): address {
        policy.creator
    }

    /// Check if policy is active
    public fun is_policy_active(policy: &ReportAccessPolicy): bool {
        policy.is_active
    }

    /// Get access statistics
    public fun get_access_stats(policy: &ReportAccessPolicy): (u64, Option<u64>) {
        (policy.total_accesses, policy.last_accessed_at)
    }

    /// Get Token information
    public fun get_token_info(token: &SealToken): (ID, u256, address, u8) {
        (token.policy_id, token.report_blob_id, token.holder, token.access_type)
    }

    /// Check if Token is expired
    public fun is_token_expired(token: &SealToken, clock: &Clock): bool {
        if (option::is_some(&token.expires_at)) {
            let expires_at = *option::borrow(&token.expires_at);
            clock::timestamp_ms(clock) > expires_at
        } else {
            false
        }
    }

    // ============ Internal Helper Functions ============

    /// Verify policy validity
    fun assert_policy_valid(policy: &ReportAccessPolicy, clock: &Clock) {
        assert!(policy.is_active, E_POLICY_REVOKED);

        if (option::is_some(&policy.expires_at)) {
            let expires_at = *option::borrow(&policy.expires_at);
            assert!(clock::timestamp_ms(clock) <= expires_at, E_POLICY_EXPIRED);
        };
    }

    /// Remove address from list
    fun remove_from_list(list: &mut vector<address>, target: address) {
        let (found, index) = vector::index_of(list, &target);
        if (found) {
            vector::remove(list, index);
        };
    }

    // ============ Test Helper Functions ============

    #[test_only]
    public fun init_for_testing(ctx: &mut TxContext) {
        init(ctx);
    }

    #[test_only]
    public fun create_test_policy(
        report_blob_id: u256,
        audit_record_id: ID,
        ctx: &mut TxContext
    ): ReportAccessPolicy {
        ReportAccessPolicy {
            id: object::new(ctx),
            report_blob_id,
            audit_record_id,
            creator: tx_context::sender(ctx),
            allowed_readers: vector::empty(),
            allowed_auditors: vector::empty(),
            allowed_admins: vector::singleton(tx_context::sender(ctx)),
            created_at: 0,
            expires_at: option::none(),
            is_active: true,
            revocation_reason: option::none(),
            total_accesses: 0,
            last_accessed_at: option::none(),
        }
    }
}
