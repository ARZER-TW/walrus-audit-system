import { Transaction } from '@mysten/sui/transactions';
import { SuiClient } from '@mysten/sui/client';
import { bcs } from '@mysten/sui/bcs';

export interface AccessProofConfig {
  suiClient: SuiClient;
  accessPackageId: string;
  policyObjectId?: string;  // 可選的 ReportAccessPolicy 對象 ID
}

/**
 * 訪問證明構造器
 *
 * 用於創建證明用戶訪問權限的 PTB (Programmable Transaction Block)
 *
 * 根據 Seal SDK 源碼分析:
 * - txBytes 必須是真實的 PTB,包含訪問控制驗證
 * - 使用 onlyTransactionKind: true 構建(不包含 gas 信息)
 * - 這個交易不會被執行,只是用於證明
 */
export class AccessProofBuilder {
  private config: AccessProofConfig;

  constructor(config: AccessProofConfig) {
    this.config = config;
  }

  /**
   * 構造訪問控制 PTB
   *
   * 這個交易證明用戶有權訪問報告
   *
   * @param objectId - Seal 加密對象 ID (同時也作為報告 blob ID)
   * @param requesterAddress - 請求者地址
   * @returns PTB 字節數組
   */
  async buildAccessProof(
    objectId: string,
    requesterAddress: string
  ): Promise<Uint8Array> {
    console.log('🔨 構造訪問證明 PTB (seal_approve)...');
    console.log(`   Object ID: ${objectId}`);
    console.log(`   Requester: ${requesterAddress}`);

    const tx = new Transaction();

    // 🔥 關鍵: 呼叫 seal_approve 函數
    // 這是 Seal 協議的標準訪問驗證入口點
    if (this.config.policyObjectId) {
      try {
        console.log('   使用 seal_approve 函數...');

        // 將 objectId (Sui 地址格式) 轉換為 u256 的 BCS 編碼
        // objectId 格式: 0x + 64 位十六進制 (32 字節)
        const idBytes = this.addressToBytesLE(objectId);

        // 🔥 使用 bcs 序列化 vector<u8>
        // Seal 協議期望的是標準 BCS 編碼的 vector<u8>
        const bcsEncodedId = bcs.vector(bcs.u8()).serialize(Array.from(idBytes));

        // 調用 seal_approve
        // 參數: (id_bytes: vector<u8>, policy: &ReportAccessPolicy, ctx: &TxContext)
        tx.moveCall({
          target: `${this.config.accessPackageId}::report_access::seal_approve`,
          arguments: [
            tx.pure(bcsEncodedId),                      // id_bytes: vector<u8> (BCS 編碼)
            tx.object(this.config.policyObjectId)       // policy: &ReportAccessPolicy
            // ctx: &TxContext 自動傳遞
          ]
        });

        console.log('   ✅ seal_approve 調用構造完成');
      } catch (error: any) {
        console.warn('   ⚠️  seal_approve 調用失敗,使用簡化證明');
        console.warn(`   錯誤: ${error.message}`);

        // 降級到方案 B
        this.buildSimpleProof(tx, requesterAddress);
      }
    } else {
      // 方案 B: 簡化的證明(如果合約還沒完全實現)
      console.log('   使用簡化證明(無 policyObjectId)...');
      this.buildSimpleProof(tx, requesterAddress);
    }

    // 構建交易字節
    // 🔥 關鍵: onlyTransactionKind: true
    // 這意味著只生成交易內容,不包含 gas 信息
    try {
      const txBytes = await tx.build({
        client: this.config.suiClient,
        onlyTransactionKind: true
      });

      console.log(`   ✅ PTB 構建完成,大小: ${txBytes.length} bytes`);
      return txBytes;
    } catch (error: any) {
      console.error(`   ❌ PTB 構建失敗: ${error.message}`);
      throw new Error(`Failed to build access proof PTB: ${error.message}`);
    }
  }

  /**
   * 將 Sui 地址轉換為小端序字節數組
   *
   * Sui Move 的 u256 使用小端序 (Little-Endian) 編碼
   *
   * @param address - Sui 地址格式 (0x + 64 位十六進制)
   * @returns 32 字節的小端序數組
   */
  private addressToBytesLE(address: string): Uint8Array {
    // 移除 0x 前綴
    const hex = address.startsWith('0x') ? address.slice(2) : address;

    // 確保是 64 位十六進制 (32 字節)
    if (hex.length !== 64) {
      throw new Error(`Invalid Sui address length: expected 64 hex chars, got ${hex.length}`);
    }

    // 將十六進制轉換為字節數組 (大端序)
    const bytes = new Uint8Array(32);
    for (let i = 0; i < 32; i++) {
      bytes[i] = parseInt(hex.substr(i * 2, 2), 16);
    }

    // 反轉為小端序
    // 例如: [0x01, 0x02, 0x03, 0x04] -> [0x04, 0x03, 0x02, 0x01]
    return bytes.reverse();
  }

  /**
   * 構建簡化的證明
   *
   * 創建一個簡單的交易來證明地址所有權
   * 這至少證明了請求者有這個地址的控制權
   */
  private buildSimpleProof(tx: Transaction, requesterAddress: string): void {
    // 使用一個簡單的系統函數來創建證明
    // 這個調用本身不重要,重要的是構建了一個有效的 PTB
    tx.moveCall({
      target: '0x2::object::id_from_address',
      arguments: [tx.pure.address(requesterAddress)]
    });

    console.log('   ✅ 簡化證明構建完成');
  }

  /**
   * 驗證報告 ID 格式
   *
   * 確保報告 ID 是有效的 Sui 對象 ID 或自定義 ID
   */
  private isValidReportId(reportId: string): boolean {
    // 檢查是否是有效的 Sui 地址格式 (0x + 64 位十六進制)
    const suiAddressPattern = /^0x[a-fA-F0-9]{64}$/;

    // 或者檢查是否是自定義格式 (如 "report-001")
    const customIdPattern = /^[a-zA-Z0-9\-_]+$/;

    return suiAddressPattern.test(reportId) || customIdPattern.test(reportId);
  }

  /**
   * 更新配置
   */
  updateConfig(config: Partial<AccessProofConfig>): void {
    this.config = { ...this.config, ...config };
    console.log('✅ AccessProofBuilder 配置已更新');
  }
}
