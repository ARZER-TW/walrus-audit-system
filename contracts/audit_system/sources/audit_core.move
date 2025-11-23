/// 去中心化存儲完整性審計系統 - 核心審計邏輯
///
/// 本模組負責：
/// 1. 對 Walrus Blob 執行挑戰-響應驗證
/// 2. 記錄審計結果並生成後量子簽名
/// 3. 追蹤 Blob 的完整性歷史
///
/// 安全假設：
/// - Merkle proof 驗證在鏈下完成（gas 成本考量）
/// - 審計員誠實性通過質押和聲譽機制保證
/// - PQC 簽名確保審計報告長期可驗證性
module audit_system::audit_core {
    use sui::object::{Self, UID, ID};
    use sui::transfer;
    use sui::tx_context::{Self, TxContext};
    use sui::event;
    use sui::clock::{Self, Clock};
    use sui::vec_map::{Self, VecMap};
    use std::vector;
    use std::option::{Self, Option};

    // ============ 錯誤代碼 ============

    /// Blob 尚未被 Walrus 認證
    const E_BLOB_NOT_CERTIFIED: u64 = 1;

    /// Blob 已過期（超過 end_epoch）
    const E_BLOB_EXPIRED: u64 = 2;

    /// 無效的挑戰次數（必須 > 0）
    const E_INVALID_CHALLENGE_COUNT: u64 = 3;

    /// 審計配置不存在或未授權
    const E_UNAUTHORIZED: u64 = 4;

    /// 無效的 PQC 簽名算法類型
    const E_INVALID_SIGNATURE_ALGORITHM: u64 = 5;

    /// 審計記錄已存在（防止重複提交）
    const E_AUDIT_ALREADY_EXISTS: u64 = 6;

    // ============ PQC 簽名算法常量 ============

    /// Falcon-512 算法標識
    const ALGO_FALCON512: u8 = 1;

    /// Dilithium-2 算法標識
    const ALGO_DILITHIUM2: u8 = 2;

    // ============ 核心數據結構 ============

    /// 審計記錄（完整版）
    ///
    /// 每次審計會創建一個新的 AuditRecord 對象，包含：
    /// - Blob 標識信息
    /// - 挑戰-響應統計
    /// - 完整性證明（哈希 + PQC 簽名）
    public struct AuditRecord has key, store {
        id: UID,

        // === Blob 標識 ===
        blob_id: u256,                      // Walrus Blob ID（32 bytes Blake2b-256）
        blob_object_id: ID,                 // Sui Blob 對象 ID（用於查詢 Blob 詳情）

        // === 審計元數據 ===
        auditor: address,                   // 審計者地址
        challenge_epoch: u32,               // 執行審計時的 Walrus epoch
        audit_timestamp: u64,               // 審計時間戳（毫秒）

        // === 挑戰-響應統計 ===
        total_challenges: u16,              // 發起的總挑戰次數
        successful_verifications: u16,      // 成功驗證的次數
        failed_verifications: u16,          // 失敗驗證的次數

        // === 完整性證明 ===
        integrity_hash: vector<u8>,         // 所有挑戰的聚合哈希（32 bytes Blake2b-256）
        pqc_signature: vector<u8>,          // 對 integrity_hash 的 PQC 簽名
        pqc_algorithm: u8,                  // 簽名算法（1=Falcon512, 2=Dilithium2）

        // === 驗證結果 ===
        is_valid: bool,                     // 審計是否通過
        failure_reason: Option<vector<u8>>, // 失敗原因（如有）
    }

    /// 審計挑戰（鏈下生成，鏈上記錄）
    ///
    /// 每個挑戰包含隨機選擇的 sliver 索引和驗證結果
    public struct Challenge has store, drop {
        sliver_index: u16,                  // Sliver 索引（0 到 n_shards-1）
        shard_id: u16,                      // Shard ID（對應 storage node）
        challenge_type: u8,                 // 挑戰類型（1=完整 sliver, 2=recovery symbol）
        merkle_proof_verified: bool,        // Merkle proof 是否驗證通過
        response_hash: vector<u8>,          // Storage node 響應數據的哈希
    }

    /// 審計配置（全局共享對象）
    ///
    /// 控制審計系統的全局參數和授權審計者列表
    public struct AuditConfig has key {
        id: UID,
        admin: address,                     // 系統管理員

        // === 審計參數 ===
        min_challenge_count: u16,           // 每次審計的最少挑戰次數
        max_challenge_count: u16,           // 每次審計的最多挑戰次數
        challenge_interval_ms: u64,         // 審計間隔（毫秒）

        // === 授權審計者 ===
        authorized_auditors: vector<address>, // 授權審計者白名單
        auditor_stakes: VecMap<address, u64>, // 審計者質押金額

        // === 統計數據 ===
        total_audits: u64,                  // 總審計次數
        total_blobs_audited: u64,           // 已審計 Blob 總數
    }

    /// Blob 審計歷史索引
    ///
    /// 為每個 Blob 維護一個審計記錄列表
    public struct BlobAuditHistory has key {
        id: UID,
        blob_id: u256,                      // Blob ID
        audit_records: vector<ID>,          // 審計記錄 ID 列表
        last_audit_epoch: u32,              // 最後審計的 epoch
        total_audits: u32,                  // 總審計次數
        consecutive_failures: u32,          // 連續失敗次數（用於告警）
    }

    // ============ 事件定義 ============

    /// 審計創建事件
    public struct AuditCreated has copy, drop {
        audit_record_id: ID,
        blob_id: u256,
        auditor: address,
        challenge_epoch: u32,
        total_challenges: u16,
        is_valid: bool,
    }

    /// Blob 審計失敗事件（告警）
    public struct BlobAuditFailed has copy, drop {
        blob_id: u256,
        blob_object_id: ID,
        auditor: address,
        challenge_epoch: u32,
        failed_verifications: u16,
        total_challenges: u16,
        failure_reason: vector<u8>,
    }

    /// 審計配置更新事件
    public struct AuditConfigUpdated has copy, drop {
        admin: address,
        min_challenge_count: u16,
        max_challenge_count: u16,
    }

    // ============ 初始化函數 ============

    /// 模組初始化
    ///
    /// 創建全局 AuditConfig 共享對象
    fun init(ctx: &mut TxContext) {
        let config = AuditConfig {
            id: object::new(ctx),
            admin: tx_context::sender(ctx),
            min_challenge_count: 10,        // 最少 10 次挑戰
            max_challenge_count: 100,       // 最多 100 次挑戰
            challenge_interval_ms: 3600000, // 1 小時
            authorized_auditors: vector::empty(),
            auditor_stakes: vec_map::empty(),
            total_audits: 0,
            total_blobs_audited: 0,
        };
        transfer::share_object(config);
    }

    // ============ 審計者管理函數 ============

    /// 授權審計者
    ///
    /// 只有管理員可以調用
    public entry fun authorize_auditor(
        config: &mut AuditConfig,
        auditor: address,
        ctx: &TxContext
    ) {
        assert!(tx_context::sender(ctx) == config.admin, E_UNAUTHORIZED);
        vector::push_back(&mut config.authorized_auditors, auditor);
    }

    /// 撤銷審計者授權
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

    /// 更新審計參數
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

    // ============ 核心審計函數 ============

    /// 提交審計記錄
    ///
    /// 審計員在鏈下完成挑戰-響應驗證後，提交審計結果到鏈上
    ///
    /// 參數：
    /// - blob_id: Walrus Blob ID（u256）
    /// - blob_object_id: Sui Blob 對象 ID
    /// - challenge_epoch: 審計時的 Walrus epoch
    /// - total_challenges: 執行的挑戰總數
    /// - successful_verifications: 成功驗證次數
    /// - integrity_hash: 所有挑戰的聚合哈希
    /// - pqc_signature: PQC 簽名
    /// - pqc_algorithm: 簽名算法（1=Falcon512, 2=Dilithium2）
    ///
    /// 安全檢查：
    /// 1. 審計者必須在授權列表中
    /// 2. 挑戰次數必須在配置範圍內
    /// 3. PQC 算法必須有效
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

        // 驗證審計者授權
        assert!(
            vector::contains(&config.authorized_auditors, &auditor),
            E_UNAUTHORIZED
        );

        // 驗證挑戰次數
        assert!(
            total_challenges >= config.min_challenge_count &&
            total_challenges <= config.max_challenge_count,
            E_INVALID_CHALLENGE_COUNT
        );

        // 驗證 PQC 算法
        assert!(
            pqc_algorithm == ALGO_FALCON512 || pqc_algorithm == ALGO_DILITHIUM2,
            E_INVALID_SIGNATURE_ALGORITHM
        );

        // 計算失敗次數和驗證結果
        let failed_verifications = total_challenges - successful_verifications;
        let is_valid = successful_verifications >= (total_challenges * 95 / 100); // 95% 成功率

        // 創建審計記錄
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

        // 發出事件
        event::emit(AuditCreated {
            audit_record_id: record_id,
            blob_id,
            auditor,
            challenge_epoch,
            total_challenges,
            is_valid,
        });

        // 如果審計失敗，發出告警事件
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

        // 更新統計
        config.total_audits = config.total_audits + 1;

        // 共享審計記錄（允許其他人讀取）
        transfer::share_object(record);
    }

    /// 創建 Blob 審計歷史索引
    ///
    /// 為新的 Blob 創建審計歷史追蹤對象
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

    /// 更新 Blob 審計歷史
    ///
    /// 將新的審計記錄添加到 Blob 的審計歷史中
    public entry fun update_blob_audit_history(
        history: &mut BlobAuditHistory,
        audit_record_id: ID,
        challenge_epoch: u32,
        is_valid: bool,
    ) {
        vector::push_back(&mut history.audit_records, audit_record_id);
        history.last_audit_epoch = challenge_epoch;
        history.total_audits = history.total_audits + 1;

        // 更新連續失敗計數
        if (is_valid) {
            history.consecutive_failures = 0;
        } else {
            history.consecutive_failures = history.consecutive_failures + 1;
        };
    }

    // ============ 查詢函數 ============

    /// 獲取審計記錄的 Blob ID
    public fun get_blob_id(record: &AuditRecord): u256 {
        record.blob_id
    }

    /// 獲取審計記錄的驗證結果
    public fun is_audit_valid(record: &AuditRecord): bool {
        record.is_valid
    }

    /// 獲取審計者地址
    public fun get_auditor(record: &AuditRecord): address {
        record.auditor
    }

    /// 獲取審計時間戳
    public fun get_audit_timestamp(record: &AuditRecord): u64 {
        record.audit_timestamp
    }

    /// 獲取挑戰統計
    public fun get_challenge_stats(record: &AuditRecord): (u16, u16, u16) {
        (
            record.total_challenges,
            record.successful_verifications,
            record.failed_verifications
        )
    }

    /// 獲取完整性證明
    public fun get_integrity_proof(record: &AuditRecord): (vector<u8>, vector<u8>, u8) {
        (
            record.integrity_hash,
            record.pqc_signature,
            record.pqc_algorithm
        )
    }

    /// 檢查審計者是否已授權
    public fun is_auditor_authorized(config: &AuditConfig, auditor: address): bool {
        vector::contains(&config.authorized_auditors, &auditor)
    }

    /// 獲取 Blob 審計歷史統計
    public fun get_blob_audit_stats(history: &BlobAuditHistory): (u32, u32, u32) {
        (
            history.total_audits,
            history.last_audit_epoch,
            history.consecutive_failures
        )
    }

    // ============ 測試輔助函數 ============

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
