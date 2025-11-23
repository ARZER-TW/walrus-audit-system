import fetch from 'node-fetch';
import * as crypto from 'crypto';

/**
 * Walrus 客戶端配置
 */
export interface WalrusClientConfig {
  aggregatorUrl: string;
  publisherUrl: string;
  network: 'testnet' | 'mainnet';
}

/**
 * Walrus 上傳結果
 */
export interface WalrusUploadResult {
  blobId: string;           // Blob ID (十六進制字符串)
  blobIdU256: bigint;       // Blob ID (u256 格式，用於合約)
  size: number;             // 文件大小（字節）
  epochs: number;           // 存儲周期數
  cost: string;             // 存儲成本（SUI）
  certified: boolean;       // 是否已認證
}

/**
 * Walrus 客戶端
 * 用於與 Walrus 去中心化存儲網絡交互
 */
export class WalrusClient {
  private config: WalrusClientConfig;

  constructor(config: WalrusClientConfig) {
    this.config = config;
  }

  /**
   * 上傳數據到 Walrus
   *
   * @param data - 要上傳的數據（Buffer）
   * @param epochs - 存儲周期數（默認 5）
   * @returns 上傳結果，包含 blob_id
   */
  async upload(
    data: Buffer,
    epochs: number = 5
  ): Promise<WalrusUploadResult> {
    console.log('\n📤 上傳數據到 Walrus...');
    console.log(`   數據大小: ${data.length} bytes`);
    console.log(`   存儲周期: ${epochs} epochs`);
    console.log(`   網絡: ${this.config.network}`);

    try {
      // Walrus HTTP API: PUT /v1/blobs
      const url = `${this.config.publisherUrl}/v1/blobs?epochs=${epochs}`;

      console.log(`   請求 URL: ${url}`);

      const response = await fetch(url, {
        method: 'PUT',
        headers: {
          'Content-Type': 'application/octet-stream'
        },
        body: data
      });

      if (!response.ok) {
        const errorText = await response.text();
        throw new Error(`Walrus 上傳失敗: ${response.status} ${errorText}`);
      }

      const result = await response.json() as any;

      console.log('✅ 上傳成功！');
      console.log(`   Blob ID: ${result.newlyCreated?.blobObject?.blobId || result.alreadyCertified?.blobId}`);

      // 提取 blob ID
      let blobId: string;
      let certified: boolean;

      if (result.newlyCreated) {
        blobId = result.newlyCreated.blobObject.blobId;
        certified = false;
        console.log('   狀態: 新創建（等待認證）');
      } else if (result.alreadyCertified) {
        blobId = result.alreadyCertified.blobId;
        certified = true;
        console.log('   狀態: 已存在且已認證');
      } else {
        throw new Error('無法從響應中提取 blob ID');
      }

      // 轉換為 u256 格式（用於 Sui 合約）
      const blobIdU256 = this.blobIdToU256(blobId);

      const uploadResult: WalrusUploadResult = {
        blobId,
        blobIdU256,
        size: data.length,
        epochs,
        cost: result.cost || '0',
        certified
      };

      console.log(`   Blob ID (u256): ${blobIdU256.toString()}`);

      return uploadResult;

    } catch (error: any) {
      console.error('❌ Walrus 上傳失敗:', error);
      throw error;
    }
  }

  /**
   * 從 Walrus 下載數據
   *
   * @param blobId - Blob ID（十六進制字符串）
   * @returns 下載的數據（Buffer）
   */
  async download(blobId: string): Promise<Buffer> {
    console.log('\n📥 從 Walrus 下載數據...');
    console.log(`   Blob ID: ${blobId}`);
    console.log(`   網絡: ${this.config.network}`);

    try {
      // Walrus HTTP API: GET /v1/blobs/{blob_id}
      const url = `${this.config.aggregatorUrl}/v1/blobs/${blobId}`;

      console.log(`   請求 URL: ${url}`);

      const response = await fetch(url);

      if (!response.ok) {
        throw new Error(`Walrus 下載失敗: ${response.status}`);
      }

      const arrayBuffer = await response.arrayBuffer();
      const buffer = Buffer.from(arrayBuffer);

      console.log('✅ 下載成功！');
      console.log(`   數據大小: ${buffer.length} bytes`);

      return buffer;

    } catch (error: any) {
      console.error('❌ Walrus 下載失敗:', error);
      throw error;
    }
  }

  /**
   * 驗證上傳的數據完整性
   *
   * @param originalData - 原始數據
   * @param blobId - Blob ID
   * @returns 是否完整
   */
  async verifyIntegrity(
    originalData: Buffer,
    blobId: string
  ): Promise<boolean> {
    console.log('\n🔍 驗證數據完整性...');

    try {
      const downloadedData = await this.download(blobId);

      const originalHash = crypto
        .createHash('sha256')
        .update(originalData)
        .digest('hex');

      const downloadedHash = crypto
        .createHash('sha256')
        .update(downloadedData)
        .digest('hex');

      const isValid = originalHash === downloadedHash;

      if (isValid) {
        console.log('✅ 數據完整性驗證通過');
        console.log(`   SHA-256: ${originalHash}`);
      } else {
        console.log('❌ 數據完整性驗證失敗');
        console.log(`   原始哈希: ${originalHash}`);
        console.log(`   下載哈希: ${downloadedHash}`);
      }

      return isValid;

    } catch (error: any) {
      console.error('❌ 完整性驗證失敗:', error);
      return false;
    }
  }

  /**
   * 將 Blob ID 轉換為 u256 格式
   * 用於 Sui Move 合約
   *
   * @param blobId - Blob ID（Walrus 使用 URL-safe Base64 編碼）
   * @returns u256 格式的 BigInt
   */
  private blobIdToU256(blobId: string): bigint {
    try {
      // Walrus 使用 URL-safe Base64 編碼 (RFC 4648)
      // 需要轉換為標準 Base64: 替換 '-' 為 '+' 和 '_' 為 '/'
      let base64 = blobId.replace(/-/g, '+').replace(/_/g, '/');

      // 添加填充（如果需要）
      while (base64.length % 4 !== 0) {
        base64 += '=';
      }

      // 解碼 Base64 為字節數組
      const binaryString = Buffer.from(base64, 'base64');

      // 如果是十六進制格式，直接轉換
      if (blobId.startsWith('0x')) {
        const cleanId = blobId.slice(2);
        return BigInt('0x' + cleanId);
      }

      // 將字節數組轉換為十六進制字符串
      const hexString = binaryString.toString('hex');

      // 轉換為 BigInt（u256）
      return BigInt('0x' + hexString);

    } catch (error: any) {
      console.error(`❌ Blob ID 轉換失敗: ${error.message}`);
      console.error(`   Blob ID: ${blobId}`);
      throw new Error(`無法轉換 Blob ID 為 u256 格式: ${blobId}`);
    }
  }

  /**
   * 檢查 Walrus 網絡狀態
   */
  async checkHealth(): Promise<boolean> {
    console.log('\n💓 檢查 Walrus 網絡狀態...');

    try {
      // 嘗試訪問 aggregator（不是所有端點都有 /health，所以直接嘗試請求根路徑）
      const response = await fetch(`${this.config.aggregatorUrl}`, {
        method: 'GET'
      });

      // 如果返回任何響應（即使是 404），說明網絡是可達的
      if (response) {
        console.log('✅ Walrus 網絡正常');
        return true;
      } else {
        console.log('⚠️  Walrus 網絡異常');
        return false;
      }
    } catch (error: any) {
      console.log('❌ 無法連接到 Walrus 網絡');
      console.log(`   錯誤: ${error.message}`);
      return false;
    }
  }
}

/**
 * 創建 Testnet Walrus 客戶端
 */
export function createTestnetWalrusClient(): WalrusClient {
  return new WalrusClient({
    aggregatorUrl: 'https://aggregator.walrus-testnet.walrus.space',
    publisherUrl: 'https://publisher.walrus-testnet.walrus.space',
    network: 'testnet'
  });
}
