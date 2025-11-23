/// 審計員註冊與質押管理模塊
///
/// 本模塊負責:
/// 1. 審計員註冊和質押管理
/// 2. PQC 公鑰註冊和管理
/// 3. 聲譽分數追蹤
/// 4. 審計報告元數據管理
module audit_system::auditor_registry {
    use sui::object::{Self, UID, ID};
    use sui::transfer;
    use sui::tx_context::{Self, TxContext};
    use sui::coin::{Self, Coin};
    use sui::sui::SUI;
    use sui::table::{Self, Table};
    use sui::event;
    use sui::clock::{Self, Clock};
    use std::vector;

    // ============ 錯誤代碼 ============

    /// 質押金額不足
    const EInsufficientStake: u64 = 100;

    /// 審計員未註冊
    const ENotRegistered: u64 = 101;

    /// 審計員已註冊
    const EAlreadyRegistered: u64 = 102;

    /// 無效的 PQC 公鑰長度
    const EInvalidPQCKeyLength: u64 = 103;

    // ============ 常量 ============

    /// Dilithium3 公鑰長度 (1952 bytes)
    const DILITHIUM3_PK_SIZE: u64 = 1952;

    /// Falcon-512 公鑰長度 (897 bytes)
    const FALCON512_PK_SIZE: u64 = 897;

    // ============ 核心數據結構 ============

    /// 審計員註冊信息（全局共享對象）
    public struct AuditorRegistry has key {
        id: UID,
        /// 已註冊的審計員列表（地址 -> 質押金額）
        auditors: Table<address, u64>,
        /// 審計員的聲譽分數（地址 -> 分數）
        reputation: Table<address, u64>,
        /// 審計員的 PQC 公鑰（用於驗證報告簽名）
        pqc_public_keys: Table<address, vector<u8>>,
        /// 最低質押要求
        min_stake: u64,
        /// 質押國庫地址
        treasury: address,
    }

    /// 審計報告元數據（指向 Walrus 上的加密報告）
    public struct AuditReportMetadata has key, store {
        id: UID,
        /// 報告生成時間
        timestamp: u64,
        /// 審計員地址
        auditor: address,
        /// Walrus blob ID（存儲加密的報告）
        encrypted_report_blob_id: ID,
        /// 報告覆蓋的審計記錄 ID 列表
        audit_record_ids: vector<ID>,
        /// PQC 簽名（對報告的簽名）
        pqc_signature: vector<u8>,
    }

    // ============ 事件定義 ============

    /// 審計員註冊事件
    public struct AuditorRegistered has copy, drop {
        auditor: address,
        stake_amount: u64,
        pqc_key_length: u64,
    }

    /// 審計員註銷事件
    public struct AuditorUnregistered has copy, drop {
        auditor: address,
        stake_returned: u64,
    }

    /// 聲譽更新事件
    public struct ReputationUpdated has copy, drop {
        auditor: address,
        old_reputation: u64,
        new_reputation: u64,
    }

    /// 審計報告提交事件
    public struct AuditReportSubmitted has copy, drop {
        report_id: ID,
        auditor: address,
        blob_id: ID,
        audit_count: u64,
    }

    // ============ 初始化函數 ============

    /// 模塊初始化
    ///
    /// 創建全局 AuditorRegistry 共享對象
    fun init(ctx: &mut TxContext) {
        let registry = AuditorRegistry {
            id: object::new(ctx),
            auditors: table::new(ctx),
            reputation: table::new(ctx),
            pqc_public_keys: table::new(ctx),
            min_stake: 1_000_000_000, // 1 SUI
            treasury: @0x0,           // 初始化為零地址,部署後需要設置
        };
        transfer::share_object(registry);
    }

    // ============ 審計員管理函數 ============

    /// 審計員註冊（需要質押 SUI 並提供 PQC 公鑰）
    ///
    /// 參數:
    /// - registry: 審計員註冊表
    /// - stake: 質押的 SUI 代幣
    /// - pqc_public_key: 後量子公鑰（Dilithium3 或 Falcon-512）
    ///
    /// 安全檢查:
    /// 1. 質押金額必須 >= min_stake
    /// 2. 審計員不能重複註冊
    /// 3. PQC 公鑰長度必須有效
    public entry fun register_auditor(
        registry: &mut AuditorRegistry,
        stake: Coin<SUI>,
        pqc_public_key: vector<u8>,
        ctx: &mut TxContext
    ) {
        let auditor = tx_context::sender(ctx);
        let stake_amount = coin::value(&stake);

        // 檢查質押金額
        assert!(stake_amount >= registry.min_stake, EInsufficientStake);

        // 檢查是否已註冊
        assert!(!table::contains(&registry.auditors, auditor), EAlreadyRegistered);

        // 驗證 PQC 公鑰長度
        let key_len = vector::length(&pqc_public_key);
        assert!(
            key_len == DILITHIUM3_PK_SIZE || key_len == FALCON512_PK_SIZE,
            EInvalidPQCKeyLength
        );

        // 轉移質押到國庫
        transfer::public_transfer(stake, registry.treasury);

        // 註冊審計員
        table::add(&mut registry.auditors, auditor, stake_amount);
        table::add(&mut registry.reputation, auditor, 0);
        table::add(&mut registry.pqc_public_keys, auditor, pqc_public_key);

        event::emit(AuditorRegistered {
            auditor,
            stake_amount,
            pqc_key_length: key_len,
        });
    }

    /// 審計員註銷（退出並取回質押）
    ///
    /// 注意: 實際應用中應該有鎖定期,這裡簡化處理
    public entry fun unregister_auditor(
        registry: &mut AuditorRegistry,
        ctx: &mut TxContext
    ) {
        let auditor = tx_context::sender(ctx);

        // 檢查是否已註冊
        assert!(table::contains(&registry.auditors, auditor), ENotRegistered);

        // 移除審計員數據
        let stake_amount = table::remove(&mut registry.auditors, auditor);
        table::remove(&mut registry.reputation, auditor);
        table::remove(&mut registry.pqc_public_keys, auditor);

        // 注意: 這裡無法直接從國庫退回質押,需要管理員操作
        // 實際應用中應該有自動退款機制

        event::emit(AuditorUnregistered {
            auditor,
            stake_returned: stake_amount,
        });
    }

    /// 增加審計員聲譽
    ///
    /// 只有合約本身可以調用（通過其他模塊）
    public(package) fun increase_reputation(
        registry: &mut AuditorRegistry,
        auditor: address,
        delta: u64,
    ) {
        assert!(table::contains(&registry.reputation, auditor), ENotRegistered);

        let old_rep = *table::borrow(&registry.reputation, auditor);
        let new_rep = old_rep + delta;

        table::remove(&mut registry.reputation, auditor);
        table::add(&mut registry.reputation, auditor, new_rep);

        event::emit(ReputationUpdated {
            auditor,
            old_reputation: old_rep,
            new_reputation: new_rep,
        });
    }

    /// 減少審計員聲譽
    ///
    /// 只有合約本身可以調用（通過其他模塊）
    public(package) fun decrease_reputation(
        registry: &mut AuditorRegistry,
        auditor: address,
        delta: u64,
    ) {
        assert!(table::contains(&registry.reputation, auditor), ENotRegistered);

        let old_rep = *table::borrow(&registry.reputation, auditor);
        let new_rep = if (delta > old_rep) {
            0
        } else {
            old_rep - delta
        };

        table::remove(&mut registry.reputation, auditor);
        table::add(&mut registry.reputation, auditor, new_rep);

        event::emit(ReputationUpdated {
            auditor,
            old_reputation: old_rep,
            new_reputation: new_rep,
        });
    }

    /// 設置國庫地址（管理員功能）
    ///
    /// 注意: 這裡簡化處理,實際應該有 admin 字段
    public entry fun set_treasury(
        registry: &mut AuditorRegistry,
        treasury: address,
        _ctx: &mut TxContext
    ) {
        registry.treasury = treasury;
    }

    /// 更新最低質押要求（管理員功能）
    public entry fun update_min_stake(
        registry: &mut AuditorRegistry,
        new_min_stake: u64,
        _ctx: &mut TxContext
    ) {
        registry.min_stake = new_min_stake;
    }

    // ============ 審計報告管理函數 ============

    /// 提交審計報告元數據（鏈接到 Walrus 上的加密報告）
    ///
    /// 參數:
    /// - registry: 審計員註冊表（用於驗證）
    /// - encrypted_report_blob_id: Walrus 上加密報告的 Blob ID
    /// - audit_record_ids: 報告涵蓋的審計記錄 ID 列表
    /// - pqc_signature: 對報告的 PQC 簽名
    /// - clock: 時鐘對象
    ///
    /// 返回: 報告元數據對象 ID
    public entry fun submit_audit_report_metadata(
        registry: &AuditorRegistry,
        encrypted_report_blob_id: ID,
        audit_record_ids: vector<ID>,
        pqc_signature: vector<u8>,
        clock: &Clock,
        ctx: &mut TxContext
    ) {
        let auditor = tx_context::sender(ctx);

        // 驗證審計員已註冊
        assert!(table::contains(&registry.auditors, auditor), ENotRegistered);

        let metadata = AuditReportMetadata {
            id: object::new(ctx),
            timestamp: clock::timestamp_ms(clock),
            auditor,
            encrypted_report_blob_id,
            audit_record_ids: audit_record_ids,
            pqc_signature,
        };

        let report_id = object::id(&metadata);
        let audit_count = vector::length(&metadata.audit_record_ids);

        // 發出事件
        event::emit(AuditReportSubmitted {
            report_id,
            auditor,
            blob_id: encrypted_report_blob_id,
            audit_count,
        });

        // 共享報告元數據
        transfer::share_object(metadata);
    }

    // ============ 查詢函數 ============

    /// 查詢審計員的聲譽分數
    public fun get_auditor_reputation(
        registry: &AuditorRegistry,
        auditor: address
    ): u64 {
        if (table::contains(&registry.reputation, auditor)) {
            *table::borrow(&registry.reputation, auditor)
        } else {
            0
        }
    }

    /// 獲取審計員的 PQC 公鑰
    public fun get_auditor_pqc_key(
        registry: &AuditorRegistry,
        auditor: address
    ): vector<u8> {
        assert!(table::contains(&registry.pqc_public_keys, auditor), ENotRegistered);
        *table::borrow(&registry.pqc_public_keys, auditor)
    }

    /// 檢查審計員是否已註冊
    public fun is_auditor_registered(
        registry: &AuditorRegistry,
        auditor: address
    ): bool {
        table::contains(&registry.auditors, auditor)
    }

    /// 獲取審計員的質押金額
    public fun get_auditor_stake(
        registry: &AuditorRegistry,
        auditor: address
    ): u64 {
        assert!(table::contains(&registry.auditors, auditor), ENotRegistered);
        *table::borrow(&registry.auditors, auditor)
    }

    /// 獲取最低質押要求
    public fun get_min_stake(registry: &AuditorRegistry): u64 {
        registry.min_stake
    }

    /// 獲取審計報告元數據
    public fun get_report_metadata(
        report: &AuditReportMetadata
    ): (u64, address, ID, u64) {
        (
            report.timestamp,
            report.auditor,
            report.encrypted_report_blob_id,
            vector::length(&report.audit_record_ids)
        )
    }

    // ============ 測試輔助函數 ============

    #[test_only]
    public fun init_for_testing(ctx: &mut TxContext) {
        init(ctx);
    }
}
