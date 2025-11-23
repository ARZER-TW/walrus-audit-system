/**
 * Session Key Manager
 *
 * 管理臨時 Session Key，用於 Seal 解密授權
 *
 * 核心功能：
 * 1. 生成臨時 Ed25519 密鑰對
 * 2. 構建需要用戶錢包簽名的授權消息
 * 3. 驗證 Session Key 簽名和有效期
 * 4. 提供解密所需的私鑰（僅在驗證通過後）
 *
 * 安全模型：
 * - Session Key 是臨時的（預設 24 小時有效期）
 * - 用戶通過錢包簽署授權消息來授權 Session Key
 * - 後端驗證簽名和有效期，確保只有授權用戶才能解密
 * - Session Key 私鑰僅存在於後端記憶體中，不持久化
 */

import { Ed25519Keypair } from '@mysten/sui/keypairs/ed25519';
import { verifyPersonalMessageSignature } from '@mysten/sui/verify';

/**
 * Session Key 配置
 */
export interface SessionKeyConfig {
  address: string;      // 用戶 Sui 地址
  packageId: string;    // Audit 合約 Package ID
  ttlMin?: number;      // 有效期（分鐘），預設 1440（24小時）
}

/**
 * Session Key 資訊（返回給前端）
 */
export interface SessionKeyInfo {
  message: string;        // 需要錢包簽名的消息
  publicKey: string;      // Session Key 公鑰（Sui 地址格式）
  expiresAt: number;      // 過期時間戳（毫秒）
}

/**
 * 已簽名的 Session Key（前端提交給後端）
 */
export interface SignedSessionKey {
  publicKey: string;      // Session Key 公鑰
  signature: string;      // 用戶錢包簽名（Base64）
  expiresAt: number;      // 過期時間戳
}

/**
 * Session Key Manager
 *
 * 生命週期：
 * 1. 前端請求創建 Session Key
 * 2. 後端生成臨時密鑰對，返回公鑰和待簽名消息
 * 3. 前端用戶用錢包簽署消息
 * 4. 前端將簽名提交給後端
 * 5. 後端驗證簽名，解密數據
 */
export class SessionKeyManager {
  private keypair: Ed25519Keypair;
  private expiresAt: number;
  private createdAt: number;
  private address: string;
  private packageId: string;

  /**
   * 創建新的 Session Key
   *
   * @param config - Session Key 配置
   */
  constructor(config: SessionKeyConfig) {
    // 生成臨時密鑰對
    this.keypair = new Ed25519Keypair();

    // 設置創建時間和過期時間
    this.createdAt = Date.now();
    const ttlMin = config.ttlMin || 1440;
    this.expiresAt = this.createdAt + ttlMin * 60 * 1000;

    // 保存用戶資訊
    this.address = config.address;
    this.packageId = config.packageId;
  }

  /**
   * 獲取需要用戶簽名的消息
   *
   * 🔥 格式來自 seal-sdk/src/lib.rs signed_message 函數:
   * "Accessing keys of package {package_id} for {ttl_min} mins from {created_at}, session key {public_key}"
   *
   * 注意：必須精確匹配此格式，否則 Seal SDK 會拒絕簽名
   */
  getSignMessage(): string {
    const ttlMin = Math.floor((this.expiresAt - this.createdAt) / 60000);
    const publicKey = this.keypair.getPublicKey().toSuiAddress();

    // 🔥 精確匹配 Seal SDK 的消息格式
    return `Accessing keys of package ${this.packageId} for ${ttlMin} mins from ${this.createdAt}, session key ${publicKey}`;
  }

  /**
   * 獲取 Session Key 資訊（返回給前端）
   */
  getSessionKeyInfo(): SessionKeyInfo {
    return {
      message: this.getSignMessage(),
      publicKey: this.keypair.getPublicKey().toSuiAddress(),
      expiresAt: this.expiresAt
    };
  }

  /**
   * 驗證用戶的錢包簽名
   *
   * @param signature - 用戶錢包簽名（Base64 編碼）
   * @param address - 用戶 Sui 地址
   * @returns 簽名是否有效
   */
  async verifySignature(signature: string, address: string): Promise<boolean> {
    try {
      // 檢查地址是否匹配
      if (address !== this.address) {
        console.warn(`[SessionKey] 地址不匹配: expected ${this.address}, got ${address}`);
        return false;
      }

      // 驗證簽名
      const message = new TextEncoder().encode(this.getSignMessage());
      const publicKey = await verifyPersonalMessageSignature(message, signature);

      // 檢查簽名者地址是否為授權用戶
      if (publicKey.toSuiAddress() !== this.address) {
        console.warn(`[SessionKey] 簽名者地址不匹配`);
        return false;
      }

      console.log(`[SessionKey] ✅ 簽名驗證成功: ${address}`);
      return true;

    } catch (error: any) {
      console.error(`[SessionKey] ❌ 簽名驗證失敗:`, error.message);
      return false;
    }
  }

  /**
   * 檢查 Session Key 是否仍然有效
   */
  isValid(): boolean {
    const now = Date.now();
    const valid = now < this.expiresAt;

    if (!valid) {
      console.warn(`[SessionKey] Session Key 已過期: ${new Date(this.expiresAt).toISOString()}`);
    }

    return valid;
  }

  /**
   * 獲取 Session Key 的私鑰（僅用於解密）
   *
   * ⚠️ 安全警告：
   * - 僅在簽名驗證通過後調用此方法
   * - 私鑰僅存在於記憶體中，不應持久化
   * - 解密完成後應立即銷毀 Session Key
   */
  getPrivateKey(): string {
    return this.keypair.getSecretKey();
  }

  /**
   * 獲取 Session Key 的公鑰（Sui 地址格式）
   */
  getPublicKey(): string {
    return this.keypair.getPublicKey().toSuiAddress();
  }

  /**
   * 銷毀 Session Key（清除私鑰）
   *
   * 注意：JavaScript 沒有顯式的記憶體清理機制
   * 這裡只是將引用設為 null，依賴垃圾回收器
   */
  destroy(): void {
    // @ts-ignore - 允許設置為 null 以觸發垃圾回收
    this.keypair = null;
    console.log(`[SessionKey] Session Key 已銷毀`);
  }

  /**
   * 獲取剩餘有效時間（秒）
   */
  getRemainingTime(): number {
    const now = Date.now();
    const remaining = Math.max(0, this.expiresAt - now);
    return Math.floor(remaining / 1000);
  }

  /**
   * 格式化顯示 Session Key 資訊
   */
  toString(): string {
    return `SessionKey {
  Public Key: ${this.getPublicKey()}
  Requester: ${this.address}
  Package: ${this.packageId}
  Expires: ${new Date(this.expiresAt).toISOString()}
  Remaining: ${this.getRemainingTime()}s
  Valid: ${this.isValid()}
}`;
  }
}

/**
 * Session Key 儲存池（管理多個 Session Key）
 *
 * 用途：
 * - 後端可能同時處理多個用戶的 Session Key 請求
 * - 需要一個臨時儲存來關聯公鑰和私鑰
 * - 定期清理過期的 Session Key
 */
export class SessionKeyStore {
  private store: Map<string, SessionKeyManager>;
  private cleanupInterval: NodeJS.Timeout | null;

  constructor() {
    this.store = new Map();

    // 每 5 分鐘清理一次過期的 Session Key
    this.cleanupInterval = setInterval(() => {
      this.cleanup();
    }, 5 * 60 * 1000);
  }

  /**
   * 創建並存儲新的 Session Key
   */
  create(config: SessionKeyConfig): SessionKeyInfo {
    const manager = new SessionKeyManager(config);
    const publicKey = manager.getPublicKey();

    // 存儲 Session Key（使用公鑰作為索引）
    this.store.set(publicKey, manager);

    console.log(`[SessionKeyStore] 創建新 Session Key: ${publicKey}`);
    console.log(`   - Requester: ${config.address}`);
    console.log(`   - Package: ${config.packageId}`);
    console.log(`   - TTL: ${config.ttlMin || 1440} 分鐘`);

    return manager.getSessionKeyInfo();
  }

  /**
   * 根據公鑰獲取 Session Key
   */
  get(publicKey: string): SessionKeyManager | undefined {
    return this.store.get(publicKey);
  }

  /**
   * 驗證並使用 Session Key
   *
   * @param signedKey - 前端提交的已簽名 Session Key
   * @param requesterAddress - 請求者地址
   * @returns Session Key Manager（如果驗證成功）
   */
  async verify(
    signedKey: SignedSessionKey,
    requesterAddress: string
  ): Promise<SessionKeyManager | null> {
    // 1. 檢查 Session Key 是否存在
    const manager = this.store.get(signedKey.publicKey);
    if (!manager) {
      console.warn(`[SessionKeyStore] Session Key 不存在: ${signedKey.publicKey}`);
      return null;
    }

    // 2. 檢查是否過期
    if (!manager.isValid()) {
      console.warn(`[SessionKeyStore] Session Key 已過期`);
      this.store.delete(signedKey.publicKey);
      return null;
    }

    // 3. 驗證簽名
    const isValidSignature = await manager.verifySignature(
      signedKey.signature,
      requesterAddress
    );

    if (!isValidSignature) {
      console.warn(`[SessionKeyStore] 簽名驗證失敗`);
      return null;
    }

    console.log(`[SessionKeyStore] ✅ Session Key 驗證成功: ${signedKey.publicKey}`);
    return manager;
  }

  /**
   * 刪除 Session Key
   */
  delete(publicKey: string): boolean {
    const manager = this.store.get(publicKey);
    if (manager) {
      manager.destroy();
      this.store.delete(publicKey);
      console.log(`[SessionKeyStore] 刪除 Session Key: ${publicKey}`);
      return true;
    }
    return false;
  }

  /**
   * 清理過期的 Session Key
   */
  cleanup(): void {
    const before = this.store.size;

    for (const [publicKey, manager] of this.store.entries()) {
      if (!manager.isValid()) {
        manager.destroy();
        this.store.delete(publicKey);
      }
    }

    const after = this.store.size;
    const cleaned = before - after;

    if (cleaned > 0) {
      console.log(`[SessionKeyStore] 清理了 ${cleaned} 個過期 Session Key`);
    }
  }

  /**
   * 獲取當前存儲的 Session Key 數量
   */
  size(): number {
    return this.store.size;
  }

  /**
   * 停止清理定時器
   */
  stop(): void {
    if (this.cleanupInterval) {
      clearInterval(this.cleanupInterval);
      this.cleanupInterval = null;
    }
  }
}
