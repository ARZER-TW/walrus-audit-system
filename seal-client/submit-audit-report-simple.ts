/**
 * 提交審計報告元數據到 Sui Testnet (簡化版)
 *
 * 使用 auditor_registry::submit_audit_report_metadata 而不是 audit_core::submit_audit_record
 *
 * 執行方式：
 * cd seal-client && npx tsx submit-audit-report-simple.ts
 */

import { Ed25519Keypair } from '@mysten/sui/keypairs/ed25519';
import { Transaction } from '@mysten/sui/transactions';
import { SuiClient } from '@mysten/sui/client';
import * as crypto from 'crypto';
import * as fs from 'fs';
import * as os from 'os';
import * as path from 'path';

// 配置
const AUDIT_PACKAGE_ID = '0x1bc5c277f6c0fd20f97cf555d83ea6f9df753d93fbf99b8890a97df31af21804';
const AUDITOR_REGISTRY_ID = '0x3ff5961eae0235665d355293820459a8da4ce564bed87f8680a7552d5553227f';

// Walrus Blob ID (來自之前的測試)
const WALRUS_BLOB_ID = 8036612256127743331405767445957662412779087640111367288916831216985723827762n;

// 用戶的錢包地址（有測試網代幣）
const USER_ADDRESS = '0xab8e37e25fe9f46493c4c1ef0c548750dae56ca47ed35324c61b9bed574104d9';

/**
 * 從 Sui CLI keystore 載入指定地址的私鑰
 */
function loadKeypairFromSuiKeystore(targetAddress: string): Ed25519Keypair {
  const keystorePath = path.join(os.homedir(), '.sui', 'sui_config', 'sui.keystore');

  if (!fs.existsSync(keystorePath)) {
    throw new Error(`Sui keystore 不存在於: ${keystorePath}`);
  }

  const keystoreData = JSON.parse(fs.readFileSync(keystorePath, 'utf-8'));

  for (const encodedKey of keystoreData) {
    try {
      const keyBytes = Buffer.from(encodedKey, 'base64');
      const flag = keyBytes[0];
      if (flag !== 0x00) continue;

      const privateKeyBytes = keyBytes.slice(1, 33);
      const keypair = Ed25519Keypair.fromSecretKey(privateKeyBytes);

      const address = keypair.getPublicKey().toSuiAddress();
      if (address === targetAddress) {
        console.log(`✅ 成功載入錢包: ${address}`);
        return keypair;
      }
    } catch (error) {
      continue;
    }
  }

  throw new Error(`在 keystore 中找不到地址 ${targetAddress} 對應的私鑰`);
}

/**
 * Blob ID (u256) 轉換為 object::ID 格式
 */
function blobIdToObjectId(blobId: bigint): string {
  // 將 u256 轉換為 32 字節的十六進制字符串
  const hex = blobId.toString(16).padStart(64, '0');
  return '0x' + hex;
}

/**
 * 生成 PQC 簽名（Dilithium3 模擬）
 */
function generateMockPqcSignature(): Uint8Array {
  // 先使用極小的簽名進行測試
  // 只是一個佔位符，用來測試合約是否接受
  return crypto.randomBytes(32);
}

async function submitAuditReport() {
  console.log('🚀 提交審計報告元數據到 Sui Testnet');
  console.log('='.repeat(70));

  // 1. 初始化 Sui 客戶端
  const client = new SuiClient({
    url: 'https://fullnode.testnet.sui.io:443'
  });

  // 2. 載入真實錢包
  console.log('\n1️⃣ 載入錢包...');
  const keypair = loadKeypairFromSuiKeystore(USER_ADDRESS);
  const auditor = keypair.getPublicKey().toSuiAddress();
  console.log(`   Blob ID: ${WALRUS_BLOB_ID.toString()}`);

  // 3. 檢查 Gas balance
  console.log('\n2️⃣ 檢查 Gas balance...');
  const balance = await client.getBalance({ owner: auditor });
  console.log(`   餘額: ${balance.totalBalance} MIST (${Number(balance.totalBalance) / 1_000_000_000} SUI)`);

  if (BigInt(balance.totalBalance) < 10_000_000n) {
    throw new Error('餘額不足，無法執行交易（需至少 0.01 SUI）');
  }

  // 4. 生成 PQC 簽名
  console.log('\n3️⃣ 生成 PQC 簽名（Dilithium3 模擬）...');
  const pqcSignature = generateMockPqcSignature();
  console.log(`   簽名長度: ${pqcSignature.length} bytes`);

  // 5. 構造交易
  console.log('\n4️⃣ 構造 Sui 交易...');
  const tx = new Transaction();

  // 獲取 Clock 對象
  const clock = tx.object('0x6');

  // 調用 auditor_registry::submit_audit_report_metadata
  // 函數簽名：
  // public entry fun submit_audit_report_metadata(
  //     registry: &AuditorRegistry,
  //     encrypted_report_blob_id: ID,
  //     audit_record_ids: vector<ID>,
  //     pqc_signature: vector<u8>,
  //     clock: &Clock,
  //     ctx: &mut TxContext
  // )

  const encryptedReportBlobId = blobIdToObjectId(WALRUS_BLOB_ID);
  const auditRecordIds: string[] = [];  // 暫時為空（可以後續添加）

  tx.moveCall({
    target: `${AUDIT_PACKAGE_ID}::auditor_registry::submit_audit_report_metadata`,
    arguments: [
      tx.object(AUDITOR_REGISTRY_ID),                         // registry
      tx.pure.id(encryptedReportBlobId),                      // encrypted_report_blob_id
      tx.pure.vector('id', auditRecordIds),                   // audit_record_ids
      tx.pure.vector('u8', Array.from(pqcSignature)),         // pqc_signature (使用 pure.vector)
      clock                                                    // clock
    ]
  });

  console.log('   ✅ 交易構造完成');

  // 6. 執行交易
  console.log('\n5️⃣ 執行交易...');
  try {
    const result = await client.signAndExecuteTransaction({
      signer: keypair,
      transaction: tx,
      options: {
        showEffects: true,
        showObjectChanges: true,
        showEvents: true
      }
    });

    console.log('   ✅ 交易執行成功！');
    console.log(`   Transaction Digest: ${result.digest}`);

    // 7. 提取創建的 AuditReportMetadata 對象 ID
    console.log('\n6️⃣ 提取 AuditReportMetadata ID...');
    const reportMetadata = result.objectChanges?.find(
      (change: any) => change.type === 'created' && change.objectType?.includes('AuditReportMetadata')
    );

    if (reportMetadata && 'objectId' in reportMetadata) {
      console.log('\n✅ 審計報告元數據創建成功！');
      console.log('='.repeat(70));
      console.log('\n🎯 關鍵輸出:');
      console.log(`   report_metadata_id = ${reportMetadata.objectId}`);
      console.log(`   Blob ID = ${WALRUS_BLOB_ID.toString()}`);
      console.log(`   Transaction: https://testnet.suivision.xyz/txblock/${result.digest}`);

      // 顯示事件
      if (result.events && result.events.length > 0) {
        console.log('\n📡 Events:');
        result.events.forEach((event: any) => {
          console.log(`   Type: ${event.type}`);
          if (event.parsedJson) {
            console.log(`   Data:`, event.parsedJson);
          }
        });
      }

      console.log('\n📝 下一步:');
      console.log('   1. 使用這個 report_metadata_id');
      console.log('   2. 結合 Walrus blob_id');
      console.log('   3. 創建訪問策略 (create_policy)');
      console.log('   4. 運行端到端 Seal 測試');
    } else {
      console.log('   ⚠️  未找到創建的 AuditReportMetadata');
      console.log('\n所有對象變更:');
      console.log(JSON.stringify(result.objectChanges, null, 2));
    }

  } catch (error: any) {
    console.error('\n❌ 交易執行失敗:');
    console.error(error.message);
    if (error.cause) {
      console.error('\n詳細錯誤:');
      console.error(JSON.stringify(error.cause, null, 2));
    }
  }
}

// 執行
submitAuditReport().catch(console.error);
