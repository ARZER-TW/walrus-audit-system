/**
 * Seal Client - Walrus Seal 客戶端封裝
 *
 * 提供與 Walrus Seal 和 Sui 區塊鏈的集成接口
 * 用於訪問控制、數據加密和完整性驗證
 *
 * ⚠️ IMPORTANT: 請使用 SealIBEClient 而不是 AuditReportSealClient
 *
 * 正確用法:
 * ```typescript
 * import { SealIBEClient, createSealIBEClient } from 'seal-client';
 *
 * const client = createSealIBEClient({
 *   network: 'testnet',
 *   auditPackageId: '0x...',
 *   threshold: 3
 * });
 * ```
 */

export * from './types';
export * from './client';
export * from './policy';
export * from './seal';

// ✅ 正確實現 - 使用 IBE + 門檻加密
// 明確導出以避免 SealConfig 重複（types.ts 已導出 SealConfig）
export { SealIBEClient, createSealIBEClient } from './seal-ibe-client';

// ❌ 已棄用 - 錯誤的本地 AES 加密實現
// 僅為向後兼容保留，請勿在新代碼中使用
export * from './audit-report';
