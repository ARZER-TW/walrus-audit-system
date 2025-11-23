/**
 * ⚠️⚠️⚠️ DEPRECATED - DO NOT USE IN PRODUCTION ⚠️⚠️⚠️
 *
 * 這個實現使用了錯誤的加密方式（本地 AES-256-GCM）
 *
 * 問題:
 * 1. ❌ 集中式密鑰管理（單點故障）
 * 2. ❌ 本地存儲密鑰（安全風險）
 * 3. ❌ 無法利用 Seal 的門檻加密
 * 4. ❌ 客戶端訪問控制（可繞過）
 *
 * 正確做法:
 * 請使用 seal-ibe-client.ts 中的 SealIBEClient
 * - ✅ Identity-Based Encryption (IBE)
 * - ✅ 3-out-of-5 門檻加密
 * - ✅ 去中心化密鑰管理
 * - ✅ Sui 鏈上訪問控制
 *
 * 遷移指南: 請參閱 MIGRATION_GUIDE.md
 *
 * ================================
 *
 * 審計報告 Seal 客戶端（DEPRECATED）
 *
 * 整合 PQC 簽名驗證、Seal 加密和 Walrus 存儲
 */

import { SealClient } from './client';
import { SealOperator } from './seal';
import { PolicyManager } from './policy';
import { AccessType } from './types';
import * as crypto from 'crypto';

export interface AuditReport {
  blob_id: string;
  blob_object_id: string;
  auditor: string;
  timestamp: number;
  challenge_epoch: number;
  challenge_results: any[];
  total_challenges: number;
  successful_verifications: number;
  failed_verifications: number;
  integrity_hash: number[];
  pqc_signature: number[];
  pqc_algorithm: number;
  is_valid: boolean;
  failure_reason: string | null;
}

export interface EncryptedReportMetadata {
  blob_id: string;
  seal_url: string;
  policy_id: string;
  encrypted_at: number;
  encryption_method: string;
}

/**
 * 審計報告 Seal 客戶端
 *
 * 功能:
 * 1. PQC 簽名驗證
 * 2. Seal 加密（對稱加密 + Threshold 密鑰管理）
 * 3. Walrus 上傳/下載
 * 4. 訪問策略管理
 */
/**
 * @deprecated 請使用 SealIBEClient（來自 seal-ibe-client.ts）
 *
 * 這個類使用了錯誤的加密方式，不應該在生產環境使用。
 * 請參閱 MIGRATION_GUIDE.md 了解如何遷移到正確的實現。
 */
export class AuditReportSealClient {
  private sealClient: SealClient;
  private sealOperator: SealOperator;
  private policyManager: PolicyManager;

  constructor(
    suiRpcUrl: string,
    walrusAggregatorUrl: string,
    accessPolicyPackageId: string,
    privateKey?: string
  ) {
    console.warn('⚠️⚠️⚠️ DEPRECATED WARNING ⚠️⚠️⚠️');
    console.warn('你正在使用已棄用的 AuditReportSealClient');
    console.warn('這個類使用了錯誤的加密方式（本地 AES-256-GCM）');
    console.warn('請遷移到 SealIBEClient（seal-ibe-client.ts）');
    console.warn('遷移指南: MIGRATION_GUIDE.md');
    console.warn('===============================\n');
    this.sealClient = new SealClient({
      suiRpcUrl,
      walrusAggregatorUrl,
      accessPolicyPackageId,
      privateKey,
    });

    this.sealOperator = new SealOperator(walrusAggregatorUrl);
    this.policyManager = new PolicyManager(this.sealClient, accessPolicyPackageId);
  }

  /**
   * 加密並上傳審計報告
   *
   * 流程:
   * 1. 驗證報告的 PQC 簽名（可選）
   * 2. 使用 AES-256-GCM 加密報告
   * 3. 上傳到 Walrus
   * 4. 在 Sui 上創建訪問策略
   *
   * 注意: 這是簡化的 Seal 實現
   * 真正的 Seal 使用 threshold encryption (2-out-of-3 key servers)
   * 這裡我們使用對稱加密作為 MVP 實現
   */
  async encryptAndUpload(
    report: AuditReport,
    publicKey?: Buffer,
    options: {
      allowedReaders?: string[];
      allowedAuditors?: string[];
      expiryDays?: number;
    } = {}
  ): Promise<EncryptedReportMetadata> {
    console.log(`[Seal] 加密並上傳審計報告: ${report.blob_id}`);

    // 1. 驗證 PQC 簽名（如果提供了公鑰）
    if (publicKey && report.pqc_signature.length > 0) {
      console.log('[Seal] 驗證 PQC 簽名...');
      // TODO: 調用 Rust 的 PQC 驗證邏輯
      // 這需要通過 FFI 或子進程調用 auditor-node 的驗證函數
      console.log('[Seal] ⚠️  PQC 簽名驗證暫未實現（需要 Rust FFI）');
    }

    // 2. 序列化報告
    const reportJson = JSON.stringify(report);
    const reportBuffer = Buffer.from(reportJson, 'utf-8');

    // 3. 生成對稱密鑰（AES-256）
    const encryptionKey = crypto.randomBytes(32); // 256 bits
    const iv = crypto.randomBytes(16); // 128 bits IV

    // 4. 使用 AES-256-GCM 加密
    const cipher = crypto.createCipheriv('aes-256-gcm', encryptionKey, iv);
    const encryptedData = Buffer.concat([
      cipher.update(reportBuffer),
      cipher.final(),
    ]);
    const authTag = cipher.getAuthTag();

    // 5. 構建加密包（IV + AuthTag + 加密數據）
    const encryptedPackage = Buffer.concat([
      iv,                    // 16 bytes
      authTag,              // 16 bytes
      encryptedData,        // N bytes
    ]);

    console.log(`[Seal] 加密完成: ${encryptedPackage.length} bytes`);

    // 6. 上傳到 Walrus
    console.log('[Seal] 上傳到 Walrus...');
    const uploadResult = await this.sealOperator.upload(encryptedPackage);
    console.log(`[Seal] 已上傳: blob_id=${uploadResult.blobId}`);

    // 7. 在 Sui 上創建訪問策略
    const expiryTimestamp = Date.now() + (options.expiryDays || 90) * 24 * 60 * 60 * 1000;

    console.log('[Seal] 創建訪問策略...');
    const policyTxDigest = await this.policyManager.createPolicy(
      uploadResult.blobId,
      options.allowedReaders || [],
      options.allowedAuditors || [],
      expiryTimestamp
    );
    console.log(`[Seal] 策略已創建: tx=${policyTxDigest}`);

    // 8. TODO: 將加密密鑰分片存儲到 Threshold Key Servers
    // 真正的 Seal 實現會將 encryptionKey 使用 Shamir Secret Sharing 分片
    // 並存儲到 2-out-of-3 的密鑰服務器
    // 這裡我們簡化為本地存儲密鑰（僅供演示）
    const keyStoragePath = `./keys/${uploadResult.blobId}.key`;
    console.log(`[Seal] ⚠️  密鑰存儲簡化實現: ${keyStoragePath}`);
    // await this.storeEncryptionKey(uploadResult.blobId, encryptionKey);

    return {
      blob_id: uploadResult.blobId,
      seal_url: uploadResult.sealUrl,
      policy_id: policyTxDigest, // 簡化：使用 tx digest 作為 policy ID
      encrypted_at: Date.now(),
      encryption_method: 'AES-256-GCM',
    };
  }

  /**
   * 下載並解密審計報告
   *
   * 流程:
   * 1. 檢查訪問權限（查詢 Sui 策略）
   * 2. 從 Walrus 下載加密數據
   * 3. 從 Threshold Key Servers 獲取解密密鑰
   * 4. 解密報告
   * 5. 驗證 PQC 簽名（可選）
   */
  async downloadAndDecrypt(
    blobId: string,
    requesterAddress: string,
    publicKey?: Buffer
  ): Promise<AuditReport> {
    console.log(`[Seal] 下載並解密審計報告: ${blobId}`);

    // 1. 檢查訪問權限
    console.log(`[Seal] 檢查訪問權限: ${requesterAddress}`);
    // TODO: 實際檢查 Sui 策略
    // const hasAccess = await this.checkAccessPermission(blobId, requesterAddress);
    // if (!hasAccess) {
    //   throw new Error(`訪問被拒絕: ${requesterAddress} 無權訪問 ${blobId}`);
    // }
    console.log('[Seal] ✓ 訪問權限通過（簡化實現）');

    // 2. 從 Walrus 下載
    console.log('[Seal] 從 Walrus 下載...');
    const encryptedPackage = await this.sealOperator.download(blobId);
    console.log(`[Seal] 已下載: ${encryptedPackage.length} bytes`);

    // 3. 解析加密包
    const iv = encryptedPackage.slice(0, 16);
    const authTag = encryptedPackage.slice(16, 32);
    const encryptedData = encryptedPackage.slice(32);

    // 4. 從密鑰服務器獲取解密密鑰
    console.log('[Seal] 獲取解密密鑰...');
    // TODO: 從 Threshold Key Servers 請求密鑰片段並重建
    // const encryptionKey = await this.retrieveEncryptionKey(blobId, requesterAddress);

    // 簡化實現：從本地讀取
    console.log('[Seal] ⚠️  使用簡化的密鑰獲取');
    const encryptionKey = crypto.randomBytes(32); // 占位

    // 5. 解密數據
    try {
      const decipher = crypto.createDecipheriv('aes-256-gcm', encryptionKey, iv);
      decipher.setAuthTag(authTag);

      const decryptedData = Buffer.concat([
        decipher.update(encryptedData),
        decipher.final(),
      ]);

      const reportJson = decryptedData.toString('utf-8');
      const report: AuditReport = JSON.parse(reportJson);

      console.log('[Seal] ✓ 解密成功');

      // 6. 驗證 PQC 簽名
      if (publicKey && report.pqc_signature.length > 0) {
        console.log('[Seal] 驗證 PQC 簽名...');
        // TODO: 調用 Rust 驗證邏輯
        console.log('[Seal] ⚠️  PQC 簽名驗證暫未實現');
      }

      return report;
    } catch (error) {
      throw new Error(`解密失敗: ${error}`);
    }
  }

  /**
   * 檢查訪問權限
   */
  async checkAccessPermission(
    blobId: string,
    requesterAddress: string
  ): Promise<boolean> {
    // TODO: 查詢 Sui 合約
    // 調用 access_policy::report_access::can_access_report
    console.log(`檢查訪問權限: ${requesterAddress} -> ${blobId}`);
    return true; // 占位
  }

  /**
   * 授予審計員訪問權限
   */
  async grantAuditorAccess(
    policyId: string,
    auditorAddress: string
  ): Promise<string> {
    console.log(`授予審計員訪問: ${auditorAddress}`);
    return await this.policyManager.grantAccess(
      policyId,
      auditorAddress,
      AccessType.AUDIT
    );
  }

  /**
   * 授予讀者訪問權限
   */
  async grantReaderAccess(
    policyId: string,
    readerAddress: string
  ): Promise<string> {
    console.log(`授予讀者訪問: ${readerAddress}`);
    return await this.policyManager.grantAccess(
      policyId,
      readerAddress,
      AccessType.READ
    );
  }

  /**
   * 撤銷策略
   */
  async revokePolicy(policyId: string): Promise<string> {
    console.log(`撤銷策略: ${policyId}`);
    return await this.policyManager.revokePolicy(policyId);
  }

  /**
   * 獲取 Seal 操作器（用於直接 Walrus 操作）
   */
  getSealOperator(): SealOperator {
    return this.sealOperator;
  }

  /**
   * 獲取策略管理器
   */
  getPolicyManager(): PolicyManager {
    return this.policyManager;
  }
}
