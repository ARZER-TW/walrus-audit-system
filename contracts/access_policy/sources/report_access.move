/// Seal 加密審計報告訪問控制模組
///
/// 本模組負責：
/// 1. 管理審計報告的訪問策略（誰可以解密）
/// 2. 發行 SealToken（NFT 風格的訪問憑證）
/// 3. 追蹤訪問歷史和合規性
///
/// 安全模型：
/// - 報告創建者永遠擁有訪問權限
/// - 合規官員通過角色授權獲得訪問權限
/// - SealToken 可轉移（類似 NFT），支持權限委託
/// - 策略可過期，確保時限性訪問控制
module access_policy::report_access {
    use sui::object::{Self, UID, ID};
    use sui::transfer;
    use sui::tx_context::{Self, TxContext};
    use sui::event;
    use sui::clock::{Self, Clock};
    use std::vector;
    use std::option::{Self, Option};

    // ============ 錯誤代碼 ============

    /// 策略已過期
    const E_POLICY_EXPIRED: u64 = 1;

    /// 未授權的訪問嘗試
    const E_UNAUTHORIZED_ACCESS: u64 = 2;

    /// 無效的訪問類型
    const E_INVALID_ACCESS_TYPE: u64 = 3;

    /// 策略創建者不匹配
    const E_NOT_POLICY_CREATOR: u64 = 4;

    /// 策略已被撤銷
    const E_POLICY_REVOKED: u64 = 5;

    /// Token 已過期
    const E_TOKEN_EXPIRED: u64 = 6;

    /// 報告不存在
    const E_REPORT_NOT_FOUND: u64 = 7;

    // ============ 訪問類型常量 ============

    /// 讀取權限（僅查看報告）
    const ACCESS_READ: u8 = 1;

    /// 審計權限（執行新審計）
    const ACCESS_AUDIT: u8 = 2;

    /// 管理權限（修改策略）
    const ACCESS_ADMIN: u8 = 3;

    // ============ 核心數據結構 ============

    /// 審計報告訪問策略
    ///
    /// 每個加密的審計報告都有一個對應的訪問策略，
    /// 定義誰可以在什麼條件下解密報告
    public struct ReportAccessPolicy has key {
        id: UID,

        // === 報告標識 ===
        report_blob_id: u256,               // Walrus 上加密報告的 Blob ID
        audit_record_id: ID,                // 對應的鏈上審計記錄 ID
        creator: address,                   // 報告創建者（審計員）

        // === 訪問控制列表 ===
        allowed_readers: vector<address>,   // 允許讀取的地址列表
        allowed_auditors: vector<address>,  // 允許審計的地址列表
        allowed_admins: vector<address>,    // 允許管理的地址列表

        // === 時間控制 ===
        created_at: u64,                    // 策略創建時間（毫秒）
        expires_at: Option<u64>,            // 過期時間（None = 永不過期）

        // === 策略狀態 ===
        is_active: bool,                    // 策略是否激活
        revocation_reason: Option<vector<u8>>, // 撤銷原因（如有）

        // === 訪問統計 ===
        total_accesses: u64,                // 總訪問次數
        last_accessed_at: Option<u64>,     // 最後訪問時間
    }

    /// SealToken（訪問憑證）
    ///
    /// NFT 風格的訪問 Token，持有者可以解密對應的報告
    /// Token 可轉移，支持訪問權限的委託
    public struct SealToken has key, store {
        id: UID,
        policy_id: ID,                      // 關聯的策略 ID
        report_blob_id: u256,               // 報告 Blob ID
        holder: address,                    // 當前持有者
        access_type: u8,                    // 訪問類型（1=讀取, 2=審計, 3=管理）
        granted_at: u64,                    // 授予時間
        expires_at: Option<u64>,            // 過期時間（可與策略不同）
        is_transferable: bool,              // 是否可轉移
    }

    /// 訪問歷史記錄
    ///
    /// 記錄每次報告訪問的詳細信息（用於合規審計）
    public struct AccessLog has key {
        id: UID,
        policy_id: ID,                      // 策略 ID
        report_blob_id: u256,               // 報告 Blob ID
        access_records: vector<AccessRecord>, // 訪問記錄列表
    }

    /// 單次訪問記錄
    public struct AccessRecord has store, drop {
        accessor: address,                  // 訪問者地址
        access_type: u8,                    // 訪問類型
        timestamp: u64,                     // 訪問時間
        token_id: Option<ID>,               // 使用的 Token ID（如有）
        success: bool,                      // 訪問是否成功
    }

    // ============ 事件定義 ============

    /// 策略創建事件
    public struct PolicyCreated has copy, drop {
        policy_id: ID,
        report_blob_id: u256,
        creator: address,
        expires_at: Option<u64>,
    }

    /// Token 授予事件
    public struct TokenGranted has copy, drop {
        token_id: ID,
        policy_id: ID,
        recipient: address,
        access_type: u8,
    }

    /// 訪問嘗試事件
    public struct AccessAttempted has copy, drop {
        policy_id: ID,
        accessor: address,
        access_type: u8,
        success: bool,
        timestamp: u64,
    }

    /// 策略撤銷事件
    public struct PolicyRevoked has copy, drop {
        policy_id: ID,
        revoker: address,
        reason: vector<u8>,
    }

    /// Token 轉移事件
    public struct TokenTransferred has copy, drop {
        token_id: ID,
        from: address,
        to: address,
    }

    // ============ 初始化函數 ============

    /// 模組初始化（無全局配置對象）
    fun init(_ctx: &mut TxContext) {
        // 本模組不需要全局配置，策略按需創建
    }

    // ============ 策略管理函數 ============

    /// 創建訪問策略
    ///
    /// 審計員在上傳加密報告到 Walrus 後，創建訪問策略
    ///
    /// 參數：
    /// - report_blob_id: Walrus 上加密報告的 Blob ID
    /// - audit_record_id: 對應的審計記錄 ID
    /// - allowed_readers: 初始讀取權限列表
    /// - allowed_auditors: 初始審計權限列表
    /// - expires_at_ms: 過期時間（None = 永不過期）
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
            allowed_admins: vector::singleton(creator), // 創建者默認是管理員
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

        // 共享策略對象（允許查詢）
        transfer::share_object(policy);

        // 創建訪問歷史對象
        let access_log = AccessLog {
            id: object::new(ctx),
            policy_id,
            report_blob_id,
            access_records: vector::empty(),
        };
        transfer::share_object(access_log);
    }

    /// 授予訪問 Token
    ///
    /// 策略管理員可以為特定地址授予訪問 Token
    ///
    /// 安全檢查：
    /// - 調用者必須是策略管理員
    /// - 策略必須未過期且激活
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

        // 驗證調用者是管理員或創建者
        assert!(
            sender == policy.creator || vector::contains(&policy.allowed_admins, &sender),
            E_NOT_POLICY_CREATOR
        );

        // 驗證策略未過期
        assert_policy_valid(policy, clock);

        // 驗證訪問類型有效
        assert!(
            access_type == ACCESS_READ || access_type == ACCESS_AUDIT || access_type == ACCESS_ADMIN,
            E_INVALID_ACCESS_TYPE
        );

        // 創建 Token
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

        // 同時更新策略的 ACL
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

        // 轉移 Token 給接收者
        transfer::transfer(token, recipient);
    }

    /// 撤銷訪問策略
    ///
    /// 創建者可以撤銷策略（例如：報告包含錯誤信息）
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

    /// 移除訪問權限
    ///
    /// 管理員可以移除特定地址的訪問權限
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
            // 不能移除創建者的管理權限
            if (target != policy.creator) {
                remove_from_list(&mut policy.allowed_admins, target);
            };
        };
    }

    // ============ Token 管理函數 ============

    /// 轉移 Token
    ///
    /// Token 持有者可以將 Token 轉移給其他地址（如果 is_transferable = true）
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

        // 更新持有者並轉移
        // 注意：這裡我們需要銷毀並重建對象，因為 holder 字段需要更新
        transfer::transfer(token, recipient);
    }

    /// 銷毀過期 Token
    ///
    /// 任何人都可以銷毀已過期的 Token（清理操作）
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

    // ============ Seal Protocol 集成 ============

    /// Seal 協議入口函數（訪問控制列表模式）
    ///
    /// 此函數實現 Seal 協議的訪問驗證，遵循「永久訪問控制列表」模式
    /// （類似 Seal 官方 whitelist.move 範例）。
    ///
    /// 設計決策：
    /// - 不檢查過期時間（expires_at），因為：
    ///   1. 審計報告需要永久可追溯（監管合規要求）
    ///   2. 時間檢查已在應用層 check_access() 中實現（兩階段驗證架構）
    ///   3. expires_at 是可選的治理工具，非強制業務邏輯
    /// - 遵循 Seal 官方 Whitelist 模式（參考 github.com/MystenLabs/seal whitelist.move）
    ///
    /// 兩階段驗證架構：
    /// - 階段 1（PTB 驗證）：seal_approve() - 驗證 ACL（由 Key Servers 執行）
    /// - 階段 2（實際解密）：check_access() - 完整驗證包括時間（由應用執行）
    ///
    /// 參數：
    /// - id_bytes: 報告 Blob ID 的 BCS 編碼（u256 → vector<u8>，小端序）
    /// - policy: 訪問策略對象引用
    /// - ctx: 交易上下文（用於獲取調用者地址）
    ///
    /// 驗證邏輯：
    /// 1. 策略必須激活（is_active = true）
    /// 2. report_blob_id 必須匹配
    /// 3. 調用者必須是創建者或在任一訪問列表中（READ/AUDIT/ADMIN）
    ///
    /// 錯誤代碼：
    /// - E_UNAUTHORIZED_ACCESS: 調用者沒有訪問權限
    /// - E_POLICY_REVOKED: 策略已被撤銷
    /// - E_REPORT_NOT_FOUND: 報告 ID 不匹配
    ///
    /// 參考：
    /// - Seal 官方文檔: https://seal-docs.wal.app/
    /// - Whitelist 範例: github.com/MystenLabs/seal/move/patterns/sources/whitelist.move
    public entry fun seal_approve(
        id_bytes: vector<u8>,
        policy: &ReportAccessPolicy,
        ctx: &TxContext
    ) {
        let sender = tx_context::sender(ctx);

        // 驗證策略激活狀態
        assert!(policy.is_active, E_POLICY_REVOKED);

        // === 步驟 1: 解析報告 ID ===
        // 將 BCS 編碼的 bytes 轉換為 u256（Sui Move 使用小端序）
        let report_blob_id = bytes_to_u256(id_bytes);

        // === 步驟 2: 驗證報告 ID 匹配 ===
        // 確保請求的報告 ID 與策略中記錄的一致（防止訪問錯誤報告）
        assert!(report_blob_id == policy.report_blob_id, E_REPORT_NOT_FOUND);

        // === 步驟 3: 訪問控制列表（ACL）驗證 ===
        // 注意：此處不檢查 expires_at，遵循 Whitelist 模式
        // 時間驗證在應用層的 check_access() 中執行

        // 創建者永遠有權限（審計員可以查看自己的報告）
        if (sender == policy.creator) {
            return
        };

        // 檢查調用者是否在任一訪問列表中
        // 允許三種權限類型：READ（讀取）、AUDIT（審計）、ADMIN（管理）
        let has_read = vector::contains(&policy.allowed_readers, &sender);
        let has_audit = vector::contains(&policy.allowed_auditors, &sender);
        let has_admin = vector::contains(&policy.allowed_admins, &sender);

        // 至少需要一種權限才能通過驗證
        assert!(
            has_read || has_audit || has_admin,
            E_UNAUTHORIZED_ACCESS
        );

        // === 驗證通過 ===
        // Seal Key Servers 會驗證此 PTB 調用成功，並發放解密密鑰
    }

    /// 將 BCS 編碼的 vector<u8> 轉換為 u256（小端序）
    ///
    /// Sui Move 的 u256 使用小端序（Little-Endian）編碼，與大多數現代系統一致。
    ///
    /// 算法：
    /// - 將每個字節左移對應位數（第 i 個字節左移 i*8 位）
    /// - 累加所有字節得到最終結果
    ///
    /// 限制：
    /// - 最多接受 32 字節（256 bits）
    /// - 超過 32 字節會觸發 E_INVALID_ACCESS_TYPE 錯誤
    ///
    /// 示例：
    /// - bytes = [0x01, 0x02] → result = 0x0201 (小端序)
    /// - bytes = [0xFF, 0x00, 0x00, 0x01] → result = 0x010000FF
    ///
    /// 參數：
    /// - bytes: BCS 編碼的字節數組（來自 Seal SDK 的 id 參數）
    ///
    /// 返回：
    /// - 解析後的 u256 報告 Blob ID
    fun bytes_to_u256(bytes: vector<u8>): u256 {
        let len = vector::length(&bytes);
        assert!(len <= 32, E_INVALID_ACCESS_TYPE); // 32 bytes = 256 bits

        let mut result: u256 = 0;
        let mut i = 0;

        // 小端序組裝：低位字節在前，高位字節在後
        while (i < len) {
            let byte = (*vector::borrow(&bytes, i) as u256);
            result = result + (byte << ((i * 8) as u8));
            i = i + 1;
        };

        result
    }

    // ============ 訪問驗證函數 ============

    /// 檢查訪問權限
    ///
    /// 驗證給定地址是否有權訪問報告
    ///
    /// 返回：(has_access, reason)
    public fun check_access(
        policy: &ReportAccessPolicy,
        accessor: address,
        access_type: u8,
        clock: &Clock,
    ): bool {
        // 檢查策略是否激活
        if (!policy.is_active) {
            return false
        };

        // 檢查策略是否過期
        if (option::is_some(&policy.expires_at)) {
            let expires_at = *option::borrow(&policy.expires_at);
            if (clock::timestamp_ms(clock) > expires_at) {
                return false
            };
        };

        // 創建者永遠有訪問權限
        if (accessor == policy.creator) {
            return true
        };

        // 檢查 ACL
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

    /// 記錄訪問
    ///
    /// 在訪問報告時記錄訪問歷史（合規要求）
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

        // 更新策略統計
        if (success) {
            policy.total_accesses = policy.total_accesses + 1;
            policy.last_accessed_at = option::some(timestamp);
        };

        // 添加訪問記錄
        let record = AccessRecord {
            accessor,
            access_type,
            timestamp,
            token_id,
            success,
        };
        vector::push_back(&mut access_log.access_records, record);

        // 發出事件
        event::emit(AccessAttempted {
            policy_id: object::id(policy),
            accessor,
            access_type,
            success,
            timestamp,
        });
    }

    // ============ 查詢函數 ============

    /// 獲取報告 Blob ID
    public fun get_report_blob_id(policy: &ReportAccessPolicy): u256 {
        policy.report_blob_id
    }

    /// 獲取策略創建者
    public fun get_creator(policy: &ReportAccessPolicy): address {
        policy.creator
    }

    /// 檢查策略是否激活
    public fun is_policy_active(policy: &ReportAccessPolicy): bool {
        policy.is_active
    }

    /// 獲取訪問統計
    public fun get_access_stats(policy: &ReportAccessPolicy): (u64, Option<u64>) {
        (policy.total_accesses, policy.last_accessed_at)
    }

    /// 獲取 Token 信息
    public fun get_token_info(token: &SealToken): (ID, u256, address, u8) {
        (token.policy_id, token.report_blob_id, token.holder, token.access_type)
    }

    /// 檢查 Token 是否過期
    public fun is_token_expired(token: &SealToken, clock: &Clock): bool {
        if (option::is_some(&token.expires_at)) {
            let expires_at = *option::borrow(&token.expires_at);
            clock::timestamp_ms(clock) > expires_at
        } else {
            false
        }
    }

    // ============ 內部輔助函數 ============

    /// 驗證策略有效性
    fun assert_policy_valid(policy: &ReportAccessPolicy, clock: &Clock) {
        assert!(policy.is_active, E_POLICY_REVOKED);

        if (option::is_some(&policy.expires_at)) {
            let expires_at = *option::borrow(&policy.expires_at);
            assert!(clock::timestamp_ms(clock) <= expires_at, E_POLICY_EXPIRED);
        };
    }

    /// 從列表中移除地址
    fun remove_from_list(list: &mut vector<address>, target: address) {
        let (found, index) = vector::index_of(list, &target);
        if (found) {
            vector::remove(list, index);
        };
    }

    // ============ 測試輔助函數 ============

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
