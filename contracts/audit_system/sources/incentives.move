/// 審計員獎勵分配模塊（簡化版）
///
/// 本模塊負責:
/// 1. 管理獎勵池資金
/// 2. 向發現作弊節點的審計員發放獎勵
/// 3. 接收獎勵池存款
///
/// 簡化假設:
/// - 獎勵金額固定（實際應該根據作弊嚴重程度動態計算）
/// - 不考慮複雜的質押懲罰機制
/// - 獎勵直接發放（實際應該有鎖定期）
module audit_system::incentives {
    use sui::object::{Self, UID, ID};
    use sui::transfer;
    use sui::tx_context::{Self, TxContext};
    use sui::coin::{Self, Coin};
    use sui::sui::SUI;
    use sui::balance::{Self, Balance};
    use sui::event;
    use sui::table::{Self, Table};

    // ============ 錯誤代碼 ============

    /// 獎勵池餘額不足
    const EInsufficientPoolBalance: u64 = 200;

    /// 審計記錄不符合獎勵條件
    const EInvalidAuditRecord: u64 = 201;

    /// 審計員未授權
    const EUnauthorizedAuditor: u64 = 202;

    /// 獎勵已被領取
    const ERewardAlreadyClaimed: u64 = 203;

    /// 只有管理員可以操作
    const ENotAdmin: u64 = 204;

    // ============ 常量 ============

    /// 發現作弊節點的固定獎勵金額（0.5 SUI）
    const CHEATING_DISCOVERY_REWARD: u64 = 500_000_000;

    /// 最低存款金額（0.1 SUI）
    const MIN_DEPOSIT_AMOUNT: u64 = 100_000_000;

    // ============ 核心數據結構 ============

    /// 獎勵池（全局共享對象）
    public struct RewardPool has key {
        id: UID,
        /// 獎勵池餘額
        balance: Balance<SUI>,
        /// 管理員地址
        admin: address,
        /// 已發放的總獎勵金額
        total_rewards_paid: u64,
        /// 獎勵發放次數
        total_claims: u64,
        /// 已領取獎勵的審計記錄 ID（防止重複領取）
        claimed_audits: Table<ID, address>,
    }

    /// 獎勵領取權能（一次性）
    ///
    /// 當審計員發現作弊節點時，系統發放此權能
    /// 審計員可以用此權能從獎勵池領取獎勵
    public struct RewardClaim has key {
        id: UID,
        /// 審計記錄 ID
        audit_record_id: ID,
        /// 審計員地址
        auditor: address,
        /// 獎勵金額
        reward_amount: u64,
        /// 發現的作弊類型（1=數據缺失, 2=默克爾證明失敗, 3=簽名偽造）
        cheating_type: u8,
    }

    // ============ 事件定義 ============

    /// 獎勵領取事件
    public struct RewardClaimed has copy, drop {
        audit_record_id: ID,
        auditor: address,
        reward_amount: u64,
        cheating_type: u8,
        pool_balance_after: u64,
    }

    /// 獎勵池存款事件
    public struct PoolDeposit has copy, drop {
        depositor: address,
        amount: u64,
        pool_balance_after: u64,
    }

    /// 獎勵權能發放事件
    public struct RewardClaimIssued has copy, drop {
        claim_id: ID,
        audit_record_id: ID,
        auditor: address,
        reward_amount: u64,
    }

    // ============ 初始化函數 ============

    /// 模塊初始化
    ///
    /// 創建全局 RewardPool 共享對象
    fun init(ctx: &mut TxContext) {
        let pool = RewardPool {
            id: object::new(ctx),
            balance: balance::zero(),
            admin: tx_context::sender(ctx),
            total_rewards_paid: 0,
            total_claims: 0,
            claimed_audits: table::new(ctx),
        };
        transfer::share_object(pool);
    }

    // ============ 獎勵管理函數 ============

    /// 向獎勵池存款
    ///
    /// 任何人都可以向獎勵池注資
    ///
    /// 參數:
    /// - pool: 獎勵池
    /// - deposit: 存入的 SUI 代幣
    public entry fun deposit_to_pool(
        pool: &mut RewardPool,
        deposit: Coin<SUI>,
        ctx: &mut TxContext
    ) {
        let amount = coin::value(&deposit);
        assert!(amount >= MIN_DEPOSIT_AMOUNT, EInsufficientPoolBalance);

        // 將代幣存入獎勵池
        let deposit_balance = coin::into_balance(deposit);
        balance::join(&mut pool.balance, deposit_balance);

        event::emit(PoolDeposit {
            depositor: tx_context::sender(ctx),
            amount,
            pool_balance_after: balance::value(&pool.balance),
        });
    }

    /// 發放獎勵權能（內部函數，由 audit_core 模塊調用）
    ///
    /// 當審計員發現作弊節點時，創建 RewardClaim 對象
    ///
    /// 參數:
    /// - pool: 獎勵池（用於檢查是否已領取）
    /// - audit_record_id: 審計記錄 ID
    /// - auditor: 審計員地址
    /// - cheating_type: 作弊類型
    ///
    /// 安全檢查:
    /// 1. 同一審計記錄不能重複領取獎勵
    /// 2. 獎勵池餘額必須充足
    public(package) fun issue_reward_claim(
        pool: &mut RewardPool,
        audit_record_id: ID,
        auditor: address,
        cheating_type: u8,
        ctx: &mut TxContext
    ) {
        // 檢查是否已領取
        assert!(
            !table::contains(&pool.claimed_audits, audit_record_id),
            ERewardAlreadyClaimed
        );

        // 檢查獎勵池餘額
        assert!(
            balance::value(&pool.balance) >= CHEATING_DISCOVERY_REWARD,
            EInsufficientPoolBalance
        );

        // 創建獎勵權能
        let claim = RewardClaim {
            id: object::new(ctx),
            audit_record_id,
            auditor,
            reward_amount: CHEATING_DISCOVERY_REWARD,
            cheating_type,
        };

        let claim_id = object::id(&claim);

        // 標記此審計記錄已發放獎勵
        table::add(&mut pool.claimed_audits, audit_record_id, auditor);

        event::emit(RewardClaimIssued {
            claim_id,
            audit_record_id,
            auditor,
            reward_amount: CHEATING_DISCOVERY_REWARD,
        });

        // 轉移權能給審計員
        transfer::transfer(claim, auditor);
    }

    /// 領取審計獎勵
    ///
    /// 審計員使用 RewardClaim 權能從獎勵池領取獎勵
    ///
    /// 參數:
    /// - pool: 獎勵池
    /// - claim: 獎勵權能（一次性使用後銷毀）
    ///
    /// 安全檢查:
    /// 1. 獎勵池餘額充足
    /// 2. 只有權能持有者可以領取
    public entry fun claim_audit_reward(
        pool: &mut RewardPool,
        claim: RewardClaim,
        ctx: &mut TxContext
    ) {
        let RewardClaim {
            id,
            audit_record_id,
            auditor,
            reward_amount,
            cheating_type,
        } = claim;

        // 驗證調用者是獎勵接收者
        assert!(tx_context::sender(ctx) == auditor, EUnauthorizedAuditor);

        // 檢查獎勵池餘額
        assert!(
            balance::value(&pool.balance) >= reward_amount,
            EInsufficientPoolBalance
        );

        // 從獎勵池提取獎勵
        let reward_balance = balance::split(&mut pool.balance, reward_amount);
        let reward_coin = coin::from_balance(reward_balance, ctx);

        // 更新統計
        pool.total_rewards_paid = pool.total_rewards_paid + reward_amount;
        pool.total_claims = pool.total_claims + 1;

        // 發出事件
        event::emit(RewardClaimed {
            audit_record_id,
            auditor,
            reward_amount,
            cheating_type,
            pool_balance_after: balance::value(&pool.balance),
        });

        // 轉移獎勵給審計員
        transfer::public_transfer(reward_coin, auditor);

        // 銷毀權能
        object::delete(id);
    }

    // ============ 管理員函數 ============

    /// 更新管理員地址
    ///
    /// 只有當前管理員可以調用
    public entry fun update_admin(
        pool: &mut RewardPool,
        new_admin: address,
        ctx: &TxContext
    ) {
        assert!(tx_context::sender(ctx) == pool.admin, ENotAdmin);
        pool.admin = new_admin;
    }

    /// 緊急提款（管理員功能）
    ///
    /// 只有管理員可以提取獎勵池資金（用於緊急情況）
    public entry fun emergency_withdraw(
        pool: &mut RewardPool,
        amount: u64,
        ctx: &mut TxContext
    ) {
        assert!(tx_context::sender(ctx) == pool.admin, ENotAdmin);
        assert!(balance::value(&pool.balance) >= amount, EInsufficientPoolBalance);

        let withdraw_balance = balance::split(&mut pool.balance, amount);
        let withdraw_coin = coin::from_balance(withdraw_balance, ctx);

        transfer::public_transfer(withdraw_coin, pool.admin);
    }

    // ============ 查詢函數 ============

    /// 獲取獎勵池餘額
    public fun get_pool_balance(pool: &RewardPool): u64 {
        balance::value(&pool.balance)
    }

    /// 獲取已發放的總獎勵金額
    public fun get_total_rewards_paid(pool: &RewardPool): u64 {
        pool.total_rewards_paid
    }

    /// 獲取獎勵領取次數
    public fun get_total_claims(pool: &RewardPool): u64 {
        pool.total_claims
    }

    /// 檢查審計記錄是否已領取獎勵
    public fun is_reward_claimed(pool: &RewardPool, audit_record_id: ID): bool {
        table::contains(&pool.claimed_audits, audit_record_id)
    }

    /// 獲取固定獎勵金額
    public fun get_reward_amount(): u64 {
        CHEATING_DISCOVERY_REWARD
    }

    /// 獲取獎勵權能詳情
    public fun get_claim_details(claim: &RewardClaim): (ID, address, u64, u8) {
        (
            claim.audit_record_id,
            claim.auditor,
            claim.reward_amount,
            claim.cheating_type
        )
    }

    // ============ 測試輔助函數 ============

    #[test_only]
    public fun init_for_testing(ctx: &mut TxContext) {
        init(ctx);
    }

    #[test_only]
    public fun create_test_reward_claim(
        audit_record_id: ID,
        auditor: address,
        ctx: &mut TxContext
    ): RewardClaim {
        RewardClaim {
            id: object::new(ctx),
            audit_record_id,
            auditor,
            reward_amount: CHEATING_DISCOVERY_REWARD,
            cheating_type: 1,
        }
    }
}
