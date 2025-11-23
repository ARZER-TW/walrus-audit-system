/**
 * 從 Sui 鏈上讀取 Blob 元數據
 *
 * 這個腳本演示如何使用 Sui TypeScript SDK 讀取 Walrus Blob 對象的鏈上元數據
 *
 * 執行方式：
 * cd seal-client && npx tsx read-blob-metadata.ts
 */

import { SuiClient, getFullnodeUrl } from '@mysten/sui/client';

// Sui Testnet RPC URL
const SUI_RPC_URL = getFullnodeUrl('testnet');

// 測試 Blob 對象 ID
const TEST_BLOB_OBJECT_ID = '0x38957e0e7dbb9259b72a06b2c6d252f4f76e6adc72fe247abe381baaac699ac2';

// Walrus Blob 類型
const WALRUS_BLOB_TYPE = '0xd84704c17fc870b8764832c535aa6b11f21a95cd6f5bb38a9b07d2cf42220c66::blob::Blob';

interface BlobMetadata {
  objectId: string;
  blobId: string;
  size: number;
  encodingType: number;
  certifiedEpoch: number;
  registeredEpoch: number;
  storageStartEpoch: number;
  storageEndEpoch: number;
  storageSize: number;
  deletable: boolean;
  owner: string;
}

async function main() {
  console.log('╔════════════════════════════════════════════════════════════════╗');
  console.log('║        從 Sui 鏈上讀取 Walrus Blob 元數據                     ║');
  console.log('╚════════════════════════════════════════════════════════════════╝\n');

  // 初始化 Sui Client
  const client = new SuiClient({ url: SUI_RPC_URL });
  console.log(`📡 連接到 Sui Testnet: ${SUI_RPC_URL}\n`);

  try {
    // 讀取 Blob 對象
    console.log(`🔍 正在讀取 Blob 對象...`);
    console.log(`   Object ID: ${TEST_BLOB_OBJECT_ID}\n`);

    const object = await client.getObject({
      id: TEST_BLOB_OBJECT_ID,
      options: {
        showContent: true,
        showOwner: true,
        showType: true,
      },
    });

    if (!object.data) {
      console.error('❌ 無法找到該對象');
      process.exit(1);
    }

    // 驗證對象類型
    if (object.data.type !== WALRUS_BLOB_TYPE) {
      console.error(`❌ 錯誤的對象類型: ${object.data.type}`);
      console.error(`   預期類型: ${WALRUS_BLOB_TYPE}`);
      process.exit(1);
    }

    console.log('✅ Blob 對象讀取成功！\n');

    // 解析字段
    const content = object.data.content as any;
    const fields = content.fields;

    // 提取 owner 地址
    let ownerAddress = '';
    if (object.data.owner && typeof object.data.owner === 'object' && 'AddressOwner' in object.data.owner) {
      ownerAddress = object.data.owner.AddressOwner as string;
    }

    // 構建元數據對象
    const metadata: BlobMetadata = {
      objectId: TEST_BLOB_OBJECT_ID,
      blobId: fields.blob_id,
      size: parseInt(fields.size),
      encodingType: parseInt(fields.encoding_type),
      certifiedEpoch: parseInt(fields.certified_epoch),
      registeredEpoch: parseInt(fields.registered_epoch),
      storageStartEpoch: parseInt(fields.storage.fields.start_epoch),
      storageEndEpoch: parseInt(fields.storage.fields.end_epoch),
      storageSize: parseInt(fields.storage.fields.storage_size),
      deletable: fields.deletable,
      owner: ownerAddress,
    };

    // 顯示元數據
    console.log('📊 Blob 元數據:\n');
    console.log('╭──────────────────────┬────────────────────────────────────────────────────────────────╮');
    console.log(`│ Object ID            │ ${metadata.objectId.padEnd(62)} │`);
    console.log('├──────────────────────┼────────────────────────────────────────────────────────────────┤');
    console.log(`│ Blob ID (u256)       │ ${metadata.blobId.padEnd(62)} │`);
    console.log('├──────────────────────┼────────────────────────────────────────────────────────────────┤');
    console.log(`│ 文件大小              │ ${metadata.size.toString().padEnd(62)} │`);
    console.log('├──────────────────────┼────────────────────────────────────────────────────────────────┤');
    console.log(`│ 編碼類型              │ ${metadata.encodingType.toString().padEnd(62)} │`);
    console.log('├──────────────────────┼────────────────────────────────────────────────────────────────┤');
    console.log(`│ 認證 Epoch           │ ${metadata.certifiedEpoch.toString().padEnd(62)} │`);
    console.log('├──────────────────────┼────────────────────────────────────────────────────────────────┤');
    console.log(`│ 註冊 Epoch           │ ${metadata.registeredEpoch.toString().padEnd(62)} │`);
    console.log('├──────────────────────┼────────────────────────────────────────────────────────────────┤');
    console.log(`│ 存儲開始 Epoch        │ ${metadata.storageStartEpoch.toString().padEnd(62)} │`);
    console.log('├──────────────────────┼────────────────────────────────────────────────────────────────┤');
    console.log(`│ 存儲結束 Epoch        │ ${metadata.storageEndEpoch.toString().padEnd(62)} │`);
    console.log('├──────────────────────┼────────────────────────────────────────────────────────────────┤');
    console.log(`│ 存儲大小              │ ${metadata.storageSize.toString().padEnd(62)} │`);
    console.log('├──────────────────────┼────────────────────────────────────────────────────────────────┤');
    console.log(`│ 可刪除                │ ${metadata.deletable.toString().padEnd(62)} │`);
    console.log('├──────────────────────┼────────────────────────────────────────────────────────────────┤');
    console.log(`│ 所有者                │ ${metadata.owner.substring(0, 62).padEnd(62)} │`);
    console.log('╰──────────────────────┴────────────────────────────────────────────────────────────────╯\n');

    // 將 Blob ID 轉換為 Base64 格式（用於 Walrus API）
    const blobIdBigInt = BigInt(metadata.blobId);
    const blobIdHex = blobIdBigInt.toString(16).padStart(64, '0');
    const blobIdBuffer = Buffer.from(blobIdHex, 'hex');
    const blobIdBase64 = blobIdBuffer.toString('base64')
      .replace(/\+/g, '-')
      .replace(/\//g, '_')
      .replace(/=/g, '');

    console.log('🔗 Blob ID 格式轉換:\n');
    console.log(`   u256 格式:  ${metadata.blobId}`);
    console.log(`   Hex 格式:   0x${blobIdHex}`);
    console.log(`   Base64 格式: ${blobIdBase64}\n`);

    // 生成 Walrus Aggregator URL
    const walrusUrl = `https://aggregator.walrus-testnet.walrus.space/v1/blobs/${blobIdBase64}`;
    console.log('📥 Walrus 下載連結:\n');
    console.log(`   ${walrusUrl}\n`);

    // 計算存儲有效期
    const remainingEpochs = metadata.storageEndEpoch - metadata.certifiedEpoch;
    console.log('⏰ 存儲有效期:\n');
    console.log(`   剩餘 Epochs: ${remainingEpochs}`);
    console.log(`   當前 Epoch:  ${metadata.certifiedEpoch}`);
    console.log(`   結束 Epoch:  ${metadata.storageEndEpoch}\n`);

    // 保存元數據到 JSON
    const outputPath = '/tmp/blob_metadata.json';
    const metadataJson = JSON.stringify({
      ...metadata,
      blobIdHex: `0x${blobIdHex}`,
      blobIdBase64,
      walrusUrl,
      remainingEpochs,
    }, null, 2);

    require('fs').writeFileSync(outputPath, metadataJson);
    console.log(`💾 元數據已保存: ${outputPath}\n`);

    console.log('╔════════════════════════════════════════════════════════════════╗');
    console.log('║                     讀取成功！                                 ║');
    console.log('╚════════════════════════════════════════════════════════════════╝\n');

  } catch (error: any) {
    console.error('❌ 錯誤:', error.message);
    if (error.stack) {
      console.error('\n堆疊追蹤:');
      console.error(error.stack);
    }
    process.exit(1);
  }
}

main();
