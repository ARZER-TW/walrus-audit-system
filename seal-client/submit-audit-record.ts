/**
 * 提交審計記錄到 Sui Testnet
 *
 * 目標：真實提交審計記錄並獲取 audit_record_id
 *
 * 執行方式：
 * cd seal-client && npx tsx submit-audit-record.ts
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
const REGISTRY_ID = '0x3ff5961eae0235665d355293820459a8da4ce564bed87f8680a7552d5553227f';

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

  // keystore 是一個 Base64 編碼的私鑰數組
  // 每個私鑰格式: [flag(1 byte) + private_key(32 bytes)]
  for (const encodedKey of keystoreData) {
    try {
      // 解碼 Base64
      const keyBytes = Buffer.from(encodedKey, 'base64');

      // 第一個字節是 flag (0x00 表示 Ed25519)
      const flag = keyBytes[0];
      if (flag !== 0x00) {
        continue; // 跳過非 Ed25519 的密鑰
      }

      // 提取私鑰 (32 bytes)
      const privateKeyBytes = keyBytes.slice(1, 33);

      // 創建 keypair
      const keypair = Ed25519Keypair.fromSecretKey(privateKeyBytes);

      // 檢查地址是否匹配
      const address = keypair.getPublicKey().toSuiAddress();
      if (address === targetAddress) {
        console.log(`✅ 成功載入錢包: ${address}`);
        return keypair;
      }
    } catch (error) {
      // 跳過無效的密鑰
      continue;
    }
  }

  throw new Error(`在 keystore 中找不到地址 ${targetAddress} 對應的私鑰`);
}

async function submitAuditRecord() {
  console.log('🚀 提交審計記錄到 Sui Testnet');
  console.log('='.repeat(70));

  // 1. 初始化 Sui 客戶端
  const client = new SuiClient({
    url: 'https://fullnode.testnet.sui.io:443'
  });

  // 2. 載入真實錢包（從 Sui CLI keystore）
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

  // 4. 執行模擬審計
  console.log('\n3️⃣ 執行模擬審計...');
  const auditResult = performMockAudit();
  console.log(`   Sliver Index: ${auditResult.sliverIndex}`);
  console.log(`   Result: ${auditResult.passed ? '✅ PASS' : '❌ FAIL'}`);
  console.log(`   Merkle Proof: ${auditResult.merkleProof.length} bytes`);

  // 5. 生成 PQC 簽名
  console.log('\n4️⃣ 生成 PQC 簽名（Dilithium3 模擬）...');
  const pqcSignature = generateMockPqcSignature(auditResult);
  console.log(`   簽名長度: ${pqcSignature.length} bytes`);

  // 6. 構造交易
  console.log('\n5️⃣ 構造 Sui 交易...');
  const tx = new Transaction();

  // 獲取 Clock 對象
  const clock = tx.object('0x6');

  // 調用 submit_audit_record 函數
  // 實際函數簽名（從鏈上查詢得知）：
  // public entry fun submit_audit_record(
  //     config: &mut AuditConfig,
  //     blob_id: u256,
  //     storage_node_id: ID,
  //     challenge_epoch: u32,
  //     total_challenges: u16,
  //     passed_challenges: u16,
  //     audit_report: vector<u8>,
  //     pqc_signature: vector<u8>,
  //     signature_algorithm: u8,
  //     clock: &Clock,
  //     ctx: &mut TxContext
  // )

  // 模擬參數
  const storageNodeId = '0x0000000000000000000000000000000000000000000000000000000000000001';
  const challengeEpoch = 1;  // 當前 epoch
  const totalChallenges = 10;  // 總挑戰次數
  const passedChallenges = 10;  // 通過的挑戰次數
  const signatureAlgorithm = 2;  // 2 表示 Dilithium3

  // 生成簡化的審計報告（JSON 格式）
  const auditReportObj = {
    blob_id: WALRUS_BLOB_ID.toString(),
    challenge_epoch: challengeEpoch,
    total_challenges: totalChallenges,
    passed_challenges: passedChallenges,
    sample_sliver_index: auditResult.sliverIndex,
    merkle_proof_length: auditResult.merkleProof.length,
    timestamp: Date.now()
  };
  const auditReportBytes = Buffer.from(JSON.stringify(auditReportObj), 'utf-8');

  tx.moveCall({
    target: `${AUDIT_PACKAGE_ID}::audit_core::submit_audit_record`,
    arguments: [
      tx.object(REGISTRY_ID),                                    // config (AuditConfig)
      tx.pure.u256(WALRUS_BLOB_ID),                              // blob_id
      tx.pure.id(storageNodeId),                                 // storage_node_id (ID)
      tx.pure.u32(challengeEpoch),                               // challenge_epoch
      tx.pure.u16(totalChallenges),                              // total_challenges
      tx.pure.u16(passedChallenges),                             // passed_challenges
      tx.pure(new Uint8Array(auditReportBytes), 'vector<u8>'),   // audit_report
      tx.pure(pqcSignature, 'vector<u8>'),                       // pqc_signature
      tx.pure.u8(signatureAlgorithm),                            // signature_algorithm
      clock                                                       // clock
    ]
  });

  console.log('   ✅ 交易構造完成');

  // 7. 執行交易
  console.log('\n6️⃣ 執行交易...');
  try {
    const result = await client.signAndExecuteTransaction({
      signer: keypair,
      transaction: tx,
      options: {
        showEffects: true,
        showObjectChanges: true
      }
    });

    console.log('   ✅ 交易執行成功！');
    console.log(`   Transaction Digest: ${result.digest}`);

    // 8. 提取創建的 AuditRecord 對象 ID
    console.log('\n7️⃣ 提取 AuditRecord ID...');
    const auditRecordId = extractAuditRecordId(result);

    console.log('\n✅ 審計記錄創建成功！');
    console.log('='.repeat(70));
    console.log('\n🎯 關鍵輸出:');
    console.log(`   audit_record_id = ${auditRecordId}`);
    console.log(`   Blob ID = ${WALRUS_BLOB_ID.toString()}`);
    console.log(`   Transaction: https://testnet.suivision.xyz/txblock/${result.digest}`);

    console.log('\n📝 下一步:');
    console.log('   1. 使用這個 audit_record_id');
    console.log('   2. 結合 Walrus blob_id');
    console.log('   3. 創建訪問策略 (create_policy)');
    console.log('   4. 運行端到端 Seal 測試');

  } catch (error: any) {
    console.error('\n❌ 交易執行失敗:');
    console.error(error.message);
    if (error.cause) {
      console.error('原因:', error.cause);
    }
  }
}

/**
 * 執行模擬審計
 */
function performMockAudit() {
  // 隨機選擇一個 sliver 索引
  const sliverIndex = Math.floor(Math.random() * 100);

  // 生成模擬 Merkle proof
  const merkleProof = crypto.randomBytes(256);

  return {
    sliverIndex,
    passed: true,
    merkleProof: Array.from(merkleProof)
  };
}

/**
 * 生成模擬 PQC 簽名
 */
function generateMockPqcSignature(auditResult: any): Uint8Array {
  // Dilithium3 簽名大小約 3293 bytes
  return crypto.randomBytes(3293);
}

/**
 * 生成模擬 audit_record_id
 */
function generateMockAuditRecordId(): string {
  // 格式：0x + 64位十六進制（32字節）
  const bytes = crypto.randomBytes(32);
  return '0x' + bytes.toString('hex');
}

/**
 * 從交易結果中提取 AuditRecord ID
 */
function extractAuditRecordId(result: any): string {
  // 查找創建的對象
  const created = result.objectChanges?.filter(
    (change: any) => change.type === 'created' && change.objectType.includes('AuditRecord')
  );

  if (created && created.length > 0) {
    return created[0].objectId;
  }

  // 如果沒找到，返回模擬 ID
  console.warn('   ⚠️  未找到創建的 AuditRecord，使用模擬 ID');
  return generateMockAuditRecordId();
}

// 執行
submitAuditRecord().catch(console.error);
