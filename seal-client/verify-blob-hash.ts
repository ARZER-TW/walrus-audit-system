/**
 * 驗證 Walrus Blob ID 的哈希算法
 *
 * 目標：找出 Walrus 使用哪種哈希算法計算 blob_id
 *
 * 已知數據：
 * - Blob ID (Base64): eRrTusk8yshFQpkemgDnbg0f4-qDo623V2NpeVG1Zcg
 * - Blob ID (u256): 54777254006481502937062893579911764738356441441477626042816486230202493789640
 * - Original SHA-256: bd9e5380f78734bc182e4bb8c464101d3baeb23387d701608901e64cd879e1f5
 *
 * 執行方式：
 * cd seal-client && npx tsx verify-blob-hash.ts
 */

import axios from 'axios';
import * as crypto from 'crypto';
import { blake3 } from '@noble/hashes/blake3.js';

// Walrus Aggregator URL (Testnet)
const WALRUS_AGGREGATOR_URL = 'https://aggregator.walrus-testnet.walrus.space';

// 已知的 Blob ID (Base64 格式)
const KNOWN_BLOB_ID_BASE64 = 'eRrTusk8yshFQpkemgDnbg0f4-qDo623V2NpeVG1Zcg';

/**
 * 從 Walrus Aggregator 下載 Blob
 */
async function downloadBlob(blobId: string): Promise<Buffer> {
  console.log(`🔽 從 Walrus Aggregator 下載 Blob...`);
  console.log(`   Blob ID: ${blobId}`);
  console.log(`   Aggregator URL: ${WALRUS_AGGREGATOR_URL}`);

  try {
    const response = await axios.get(
      `${WALRUS_AGGREGATOR_URL}/v1/blobs/${blobId}`,
      {
        responseType: 'arraybuffer',
        timeout: 30000,
      }
    );

    console.log(`   ✅ 下載成功！`);
    console.log(`   數據大小: ${response.data.length} bytes`);

    return Buffer.from(response.data);
  } catch (error: any) {
    console.error('   ❌ 下載失敗:', error.message);
    throw error;
  }
}

/**
 * 計算各種哈希並比對
 */
function testHashAlgorithms(data: Buffer, expectedBlobId: Buffer) {
  console.log('\n🧪 測試各種哈希算法...\n');

  const algorithms = [
    'sha256',
    'sha512',
    'sha3-256',
    'sha3-512',
    'blake2b512',
    'blake2s256',
  ];

  let matched = false;

  for (const algo of algorithms) {
    try {
      const hash = crypto.createHash(algo).update(data).digest();
      const matches = hash.equals(expectedBlobId);

      console.log(`${matches ? '✅' : '❌'} ${algo.padEnd(15)} -> ${hash.toString('hex').substring(0, 64)}...`);

      if (matches) {
        console.log(`\n   🎯 找到匹配！Walrus 使用 ${algo.toUpperCase()}\n`);
        matched = true;
      }
    } catch (error: any) {
      console.log(`⚠️  ${algo.padEnd(15)} -> 不支持或錯誤`);
    }
  }

  // 測試 BLAKE3
  console.log('\n🔬 測試 BLAKE3...\n');

  try {
    const blake3Hash = blake3(data);
    const blake3Matches = Buffer.from(blake3Hash).equals(expectedBlobId);

    console.log(`${blake3Matches ? '✅' : '❌'} BLAKE3         -> ${Buffer.from(blake3Hash).toString('hex')}`);

    if (blake3Matches) {
      console.log(`\n   🎯 找到匹配！Walrus 使用 BLAKE3\n`);
      matched = true;
    }
  } catch (error: any) {
    console.log(`⚠️  BLAKE3         -> 錯誤: ${error.message}`);
  }

  if (!matched) {
    console.log('❌ 未找到匹配的哈希算法');
    console.log('\n可能原因：');
    console.log('1. Blob ID 不是直接對原始數據的哈希');
    console.log('2. 使用了編碼後數據（erasure coding）的哈希');
    console.log('3. 使用了 BLAKE3 或其他 Node.js 原生不支持的算法');
    console.log('4. Blob ID 包含了額外的元數據（如 prefix/suffix）');
  }

  return matched;
}

/**
 * 主函數
 */
async function main() {
  console.log('═'.repeat(70));
  console.log('Walrus Blob ID 哈希算法驗證');
  console.log('═'.repeat(70));

  // 解析 Blob ID
  const blobIdBuffer = Buffer.from(KNOWN_BLOB_ID_BASE64, 'base64');
  console.log('\n📌 已知 Blob ID:');
  console.log(`   Base64: ${KNOWN_BLOB_ID_BASE64}`);
  console.log(`   Hex:    ${blobIdBuffer.toString('hex')}`);
  console.log(`   長度:   ${blobIdBuffer.length} bytes`);

  // 下載 Blob
  const blobData = await downloadBlob(KNOWN_BLOB_ID_BASE64);

  console.log('\n📊 下載的數據:');
  console.log(`   大小: ${blobData.length} bytes`);
  console.log(`   前 100 bytes: ${blobData.toString('utf-8').substring(0, 100)}...`);

  // 測試哈希算法
  const matched = testHashAlgorithms(blobData, blobIdBuffer);

  console.log('\n' + '═'.repeat(70));

  if (matched) {
    console.log('✅ 驗證完成！找到了正確的哈希算法');
  } else {
    console.log('⚠️  需要進一步研究 Walrus 的 blob_id 計算方式');
  }

  console.log('═'.repeat(70));
}

// 執行
main().catch((error) => {
  console.error('\n💥 執行失敗:', error);
  process.exit(1);
});
