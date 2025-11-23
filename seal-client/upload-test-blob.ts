/**
 * 上傳真實測試檔案到 Walrus Testnet
 *
 * 目標：創建一個真實的 Blob，用於審計節點測試
 *
 * 執行方式：
 * cd seal-client && npx tsx upload-test-blob.ts
 */

import axios from 'axios';
import * as crypto from 'crypto';

// Walrus Publisher URL (Testnet)
const WALRUS_PUBLISHER_URL = 'https://publisher.walrus-testnet.walrus.space';

/**
 * 上傳數據到 Walrus
 */
async function uploadToWalrus(data: Buffer): Promise<{
  blobId: string;
  blobIdU256: string;
  suiObjectId?: string;
  endEpoch?: number;
}> {
  console.log('🚀 上傳數據到 Walrus Publisher...');
  console.log(`   數據大小: ${data.length} bytes`);
  console.log(`   Publisher URL: ${WALRUS_PUBLISHER_URL}`);

  try {
    // 使用 PUT 請求上傳數據（正確端點：/v1/blobs）
    // 添加 epochs 參數指定存儲時間
    const response = await axios.put(
      `${WALRUS_PUBLISHER_URL}/v1/blobs?epochs=5`,
      data,
      {
        headers: {
          'Content-Type': 'application/octet-stream',
        },
        // 設置較長的超時時間
        timeout: 60000,
      }
    );

    console.log('\n✅ 上傳成功！');
    console.log('   響應狀態:', response.status);
    console.log('   響應數據:', JSON.stringify(response.data, null, 2));

    // 解析響應
    const responseData = response.data;

    // Walrus 回應格式可能有幾種變化
    let blobId: string;
    let suiObjectId: string | undefined;
    let endEpoch: number | undefined;

    if (responseData.newlyCreated) {
      // 新創建的 blob
      blobId = responseData.newlyCreated.blobObject.blobId;
      suiObjectId = responseData.newlyCreated.blobObject.id;
      endEpoch = responseData.newlyCreated.blobObject.storage.endEpoch;
    } else if (responseData.alreadyCertified) {
      // 已經存在的 blob
      blobId = responseData.alreadyCertified.blobId;
      suiObjectId = responseData.alreadyCertified.blobObject?.id;
      endEpoch = responseData.alreadyCertified.endEpoch;
    } else {
      throw new Error(`未知的響應格式: ${JSON.stringify(responseData)}`);
    }

    // Blob ID 是 Base64 編碼，需要轉換為十六進制
    // 然後再轉換為 u256
    const blobIdBuffer = Buffer.from(blobId, 'base64');
    const blobIdHex = '0x' + blobIdBuffer.toString('hex');
    const blobIdU256 = BigInt(blobIdHex).toString();

    return {
      blobId,
      blobIdU256,
      suiObjectId,
      endEpoch,
    };
  } catch (error: any) {
    console.error('\n❌ 上傳失敗:');
    if (error.response) {
      console.error('   HTTP 狀態:', error.response.status);
      console.error('   響應數據:', error.response.data);
    } else if (error.request) {
      console.error('   無響應 - 網絡錯誤');
      console.error('   請檢查:', WALRUS_PUBLISHER_URL);
    } else {
      console.error('   錯誤:', error.message);
    }
    throw error;
  }
}

/**
 * 驗證 Blob 可以被讀取
 */
async function verifyBlobReadable(blobId: string): Promise<boolean> {
  console.log('\n🔍 驗證 Blob 可讀取性...');

  const aggregatorUrl = 'https://aggregator.walrus-testnet.walrus.space';

  try {
    const response = await axios.get(
      `${aggregatorUrl}/v1/blobs/${blobId}`,
      {
        responseType: 'arraybuffer',
        timeout: 30000,
      }
    );

    console.log('   ✅ Blob 可讀取');
    console.log(`   下載大小: ${response.data.length} bytes`);

    return true;
  } catch (error: any) {
    console.error('   ❌ Blob 讀取失敗:', error.message);
    return false;
  }
}

/**
 * 主函數
 */
async function main() {
  console.log('═'.repeat(70));
  console.log('Walrus 審計系統 - 測試 Blob 上傳');
  console.log('═'.repeat(70));

  // 創建測試數據
  const testData = {
    purpose: 'Walrus Audit System Test Blob',
    timestamp: new Date().toISOString(),
    content: 'This is a test blob for the decentralized storage integrity audit system.',
    metadata: {
      project: 'walrus-audit-system',
      hackathon: 'Walrus Haulout',
      track: 'DATA SECURITY & PRIVACY',
      team: 'ARZER-TW',
    },
    randomData: crypto.randomBytes(256).toString('hex'),
  };

  const testDataJson = JSON.stringify(testData, null, 2);
  const testDataBuffer = Buffer.from(testDataJson, 'utf-8');

  console.log('\n📋 測試數據:');
  console.log(testDataJson.substring(0, 200) + '...');
  console.log(`\n   數據大小: ${testDataBuffer.length} bytes`);
  console.log(`   SHA-256: ${crypto.createHash('sha256').update(testDataBuffer).digest('hex')}`);

  // 上傳到 Walrus
  const result = await uploadToWalrus(testDataBuffer);

  // 等待一下讓數據傳播
  console.log('\n⏳ 等待 5 秒讓數據傳播...');
  await new Promise(resolve => setTimeout(resolve, 5000));

  // 驗證可讀取
  await verifyBlobReadable(result.blobId);

  // 輸出最終結果
  console.log('\n' + '═'.repeat(70));
  console.log('🎯 上傳完成！關鍵資訊：');
  console.log('═'.repeat(70));
  console.log(`\n📌 Blob ID (hex):     ${result.blobId}`);
  console.log(`📌 Blob ID (u256):    ${result.blobIdU256}`);
  if (result.suiObjectId) {
    console.log(`📌 Sui Object ID:     ${result.suiObjectId}`);
  }
  if (result.endEpoch) {
    console.log(`📌 Storage End Epoch: ${result.endEpoch}`);
  }

  console.log('\n📝 下一步:');
  console.log('   1. 記錄這個 Blob ID');
  console.log('   2. 使用 Sui Client 查詢鏈上元數據');
  console.log('   3. 運行審計節點驗證完整性');
  console.log('   4. 生成 PQC 簽名的審計報告');

  console.log('\n🔗 查看 Blob:');
  console.log(`   Aggregator: https://aggregator.walrus-testnet.walrus.space/v1/blobs/${result.blobId}`);
  if (result.suiObjectId) {
    console.log(`   Sui Explorer: https://testnet.suivision.xyz/object/${result.suiObjectId}`);
  }

  console.log('\n' + '═'.repeat(70));
}

// 執行
main().catch((error) => {
  console.error('\n💥 執行失敗:', error);
  process.exit(1);
});
