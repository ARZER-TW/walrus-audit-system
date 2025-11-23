/**
 * 驗證鏈上審計記錄
 *
 * 這個腳本演示如何從 Sui 鏈上讀取並驗證審計記錄
 *
 * 執行方式：
 * cd seal-client && npx tsx verify-audit-record.ts <audit_record_id>
 */

import { SuiClient, getFullnodeUrl } from '@mysten/sui/client';
import * as fs from 'fs';

// ============ 配置 ============

const SUI_RPC_URL = getFullnodeUrl('testnet');

// PQC 算法映射
const PQC_ALGORITHMS: { [key: number]: string } = {
  1: 'Falcon-512',
  2: 'Dilithium2/3',
};

// ============ 工具函數 ============

/**
 * 格式化時間戳
 */
function formatTimestamp(timestampMs: number): string {
  const date = new Date(timestampMs);
  return date.toISOString();
}

/**
 * 格式化 Blob ID (u256)
 */
function formatBlobId(blobIdU256: string): string {
  // 轉換為 BigInt
  const bigInt = BigInt(blobIdU256);

  // 轉換為 Hex
  const hex = bigInt.toString(16).padStart(64, '0');

  // 轉換為 Base64 (URL-safe)
  const buffer = Buffer.from(hex, 'hex');
  const base64 = buffer.toString('base64')
    .replace(/\+/g, '-')
    .replace(/\//g, '_')
    .replace(/=/g, '');

  return base64;
}

/**
 * 計算成功率
 */
function calculateSuccessRate(successful: number, total: number): number {
  return Math.round((successful / total) * 10000) / 100;
}

// ============ 主函數 ============

async function main() {
  console.log('╔════════════════════════════════════════════════════════════════╗');
  console.log('║              驗證 Sui 鏈上審計記錄                             ║');
  console.log('╚════════════════════════════════════════════════════════════════╝\n');

  try {
    // 從命令行參數獲取 Audit Record ID
    const auditRecordId = process.argv[2];

    if (!auditRecordId) {
      // 從之前的提交結果讀取
      const resultPath = '/tmp/audit_submission_result.json';
      if (fs.existsSync(resultPath)) {
        const result = JSON.parse(fs.readFileSync(resultPath, 'utf-8'));
        const createdObject = result.objectChanges?.find(
          (change: any) => change.type === 'created' &&
                           change.objectType?.includes('AuditRecord')
        );

        if (createdObject) {
          console.log('📋 使用最近提交的審計記錄...\n');
          await verifyAuditRecord(createdObject.objectId);
          return;
        }
      }

      console.error('❌ 請提供審計記錄 ID：');
      console.error('   用法: npx tsx verify-audit-record.ts <audit_record_id>\n');
      process.exit(1);
    }

    await verifyAuditRecord(auditRecordId);

  } catch (error: any) {
    console.error('\n❌ 錯誤:', error.message);
    if (error.stack) {
      console.error('\n堆疊追蹤:');
      console.error(error.stack);
    }
    process.exit(1);
  }
}

async function verifyAuditRecord(auditRecordId: string) {
  // 初始化 Sui 客戶端
  const client = new SuiClient({ url: SUI_RPC_URL });
  console.log(`📡 連接到 Sui Testnet: ${SUI_RPC_URL}\n`);

  // 步驟 1: 讀取審計記錄對象
  console.log(`🔍 正在讀取審計記錄...`);
  console.log(`   Object ID: ${auditRecordId}\n`);

  const object = await client.getObject({
    id: auditRecordId,
    options: {
      showContent: true,
      showOwner: true,
      showType: true,
    },
  });

  if (!object.data) {
    console.error('❌ 無法找到該審計記錄\n');
    process.exit(1);
  }

  // 驗證對象類型
  if (!object.data.type?.includes('AuditRecord')) {
    console.error(`❌ 錯誤的對象類型: ${object.data.type}`);
    console.error(`   預期類型: AuditRecord\n`);
    process.exit(1);
  }

  console.log('✅ 審計記錄讀取成功！\n');

  // 步驟 2: 解析審計記錄字段
  const content = object.data.content as any;
  const fields = content.fields;

  // 提取所有字段
  const auditRecord = {
    objectId: auditRecordId,
    blobId: fields.blob_id,
    blobObjectId: fields.blob_object_id,
    auditor: fields.auditor,
    challengeEpoch: parseInt(fields.challenge_epoch),
    auditTimestamp: parseInt(fields.audit_timestamp),
    totalChallenges: parseInt(fields.total_challenges),
    successfulVerifications: parseInt(fields.successful_verifications),
    failedVerifications: parseInt(fields.failed_verifications),
    integrityHash: fields.integrity_hash,
    pqcSignature: fields.pqc_signature,
    pqcAlgorithm: parseInt(fields.pqc_algorithm),
    isValid: fields.is_valid,
    failureReason: fields.failure_reason?.fields?.vec || null,
  };

  // 步驟 3: 顯示審計記錄詳情
  console.log('📊 審計記錄詳情:\n');
  console.log('╭───────────────────────┬────────────────────────────────────────────────────────────╮');
  console.log(`│ 對象 ID               │ ${auditRecord.objectId.substring(0, 58).padEnd(58)} │`);
  console.log('├───────────────────────┼────────────────────────────────────────────────────────────┤');
  console.log(`│ Blob ID (u256)        │ ${auditRecord.blobId.substring(0, 58).padEnd(58)} │`);
  console.log('├───────────────────────┼────────────────────────────────────────────────────────────┤');
  console.log(`│ Blob ID (Base64)      │ ${formatBlobId(auditRecord.blobId).padEnd(58)} │`);
  console.log('├───────────────────────┼────────────────────────────────────────────────────────────┤');
  console.log(`│ Blob Object ID        │ ${auditRecord.blobObjectId.substring(0, 58).padEnd(58)} │`);
  console.log('├───────────────────────┼────────────────────────────────────────────────────────────┤');
  console.log(`│ 審計者地址            │ ${auditRecord.auditor.substring(0, 58).padEnd(58)} │`);
  console.log('├───────────────────────┼────────────────────────────────────────────────────────────┤');
  console.log(`│ 挑戰 Epoch            │ ${auditRecord.challengeEpoch.toString().padEnd(58)} │`);
  console.log('├───────────────────────┼────────────────────────────────────────────────────────────┤');
  console.log(`│ 審計時間              │ ${formatTimestamp(auditRecord.auditTimestamp).padEnd(58)} │`);
  console.log('├───────────────────────┼────────────────────────────────────────────────────────────┤');
  console.log(`│ 總挑戰次數            │ ${auditRecord.totalChallenges.toString().padEnd(58)} │`);
  console.log('├───────────────────────┼────────────────────────────────────────────────────────────┤');
  console.log(`│ 成功驗證次數          │ ${auditRecord.successfulVerifications.toString().padEnd(58)} │`);
  console.log('├───────────────────────┼────────────────────────────────────────────────────────────┤');
  console.log(`│ 失敗驗證次數          │ ${auditRecord.failedVerifications.toString().padEnd(58)} │`);
  console.log('├───────────────────────┼────────────────────────────────────────────────────────────┤');
  const successRate = calculateSuccessRate(auditRecord.successfulVerifications, auditRecord.totalChallenges);
  console.log(`│ 成功率                │ ${successRate.toString()}%`.padEnd(59) + ' │');
  console.log('├───────────────────────┼────────────────────────────────────────────────────────────┤');
  console.log(`│ PQC 算法              │ ${PQC_ALGORITHMS[auditRecord.pqcAlgorithm].padEnd(58)} │`);
  console.log('├───────────────────────┼────────────────────────────────────────────────────────────┤');
  console.log(`│ PQC 簽名長度          │ ${auditRecord.pqcSignature.length.toString()} bytes`.padEnd(59) + ' │');
  console.log('├───────────────────────┼────────────────────────────────────────────────────────────┤');
  console.log(`│ 完整性哈希長度        │ ${auditRecord.integrityHash.length.toString()} bytes`.padEnd(59) + ' │');
  console.log('├───────────────────────┼────────────────────────────────────────────────────────────┤');
  const validStatus = auditRecord.isValid ? '✅ 通過' : '❌ 失敗';
  console.log(`│ 驗證結果              │ ${validStatus.padEnd(58)} │`);
  console.log('╰───────────────────────┴────────────────────────────────────────────────────────────╯\n');

  // 步驟 4: 顯示完整性證明
  console.log('🔐 完整性證明:\n');

  // 將哈希轉換為 Hex 格式顯示
  const hashHex = Buffer.from(auditRecord.integrityHash).toString('hex');
  console.log(`   SHA-256 哈希: 0x${hashHex}\n`);

  // 步驟 5: 驗證審計結果
  console.log('🔍 驗證審計結果:\n');

  const threshold = 0.95; // 95% 成功率閾值
  const actualRate = successRate / 100;

  console.log(`   閾值標準: ${threshold * 100}% 成功率`);
  console.log(`   實際成功率: ${successRate}%`);

  if (actualRate >= threshold) {
    console.log(`   ✅ 審計結果有效 - 成功率達標\n`);
  } else {
    console.log(`   ❌ 審計結果無效 - 成功率不足\n`);

    if (auditRecord.failureReason) {
      console.log(`   失敗原因: ${Buffer.from(auditRecord.failureReason).toString('utf-8')}\n`);
    }
  }

  // 步驟 6: 生成 Walrus 下載連結
  const blobIdBase64 = formatBlobId(auditRecord.blobId);
  const walrusUrl = `https://aggregator.walrus-testnet.walrus.space/v1/blobs/${blobIdBase64}`;

  console.log('📥 Walrus 下載連結:\n');
  console.log(`   ${walrusUrl}\n`);

  // 步驟 7: 查詢相關事件
  console.log('📋 查詢相關事件...\n');

  const events = await client.queryEvents({
    query: {
      MoveEventType: `${object.data.type?.split('::')[0]}::audit_core::AuditCreated`,
    },
    limit: 50,
  });

  const relatedEvent = events.data.find((event) => {
    const parsedJson = event.parsedJson as any;
    return parsedJson?.audit_record_id === auditRecordId;
  });

  if (relatedEvent) {
    console.log('   ✅ 找到對應的 AuditCreated 事件');
    console.log(`   交易摘要: ${relatedEvent.id.txDigest}\n`);
  }

  // 步驟 8: 保存驗證結果
  const outputPath = '/tmp/audit_verification_result.json';
  const verificationResult = {
    auditRecord: {
      ...auditRecord,
      integrityHashHex: hashHex,
      blobIdBase64,
    },
    verification: {
      threshold,
      actualSuccessRate: actualRate,
      isValid: actualRate >= threshold,
      timestamp: new Date().toISOString(),
    },
    walrusUrl,
    relatedTransaction: relatedEvent?.id.txDigest || null,
  };

  fs.writeFileSync(outputPath, JSON.stringify(verificationResult, null, 2));
  console.log(`💾 驗證結果已保存: ${outputPath}\n`);

  console.log('╔════════════════════════════════════════════════════════════════╗');
  console.log('║                   驗證完成！                                   ║');
  console.log('╚════════════════════════════════════════════════════════════════╝\n');

  console.log('🔗 查看審計記錄:');
  console.log(`   https://testnet.suivision.xyz/object/${auditRecordId}\n`);
}

main();
