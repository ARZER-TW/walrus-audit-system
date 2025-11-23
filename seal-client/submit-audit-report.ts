/**
 * 提交審計報告到 Sui 智能合約
 *
 * 這個腳本演示如何將 PQC 簽名的審計報告提交到鏈上
 *
 * 執行方式：
 * cd seal-client && npx tsx submit-audit-report.ts
 */

import { SuiClient, getFullnodeUrl } from '@mysten/sui/client';
import { Transaction } from '@mysten/sui/transactions';
import { Ed25519Keypair } from '@mysten/sui/keypairs/ed25519';
import { bcs } from '@mysten/sui/bcs';
import * as fs from 'fs';
import * as path from 'path';

// ============ 配置 ============

const SUI_RPC_URL = getFullnodeUrl('testnet');

// 從部署配置讀取合約地址
const DEPLOYED_CONTRACTS_PATH = path.join(__dirname, '../contracts/deployed-contracts.json');
const AUDIT_REPORT_PATH = '/tmp/audit_report.json';

// PQC 算法常量（與 Move 合約中的定義一致）
const ALGO_FALCON512 = 1;
const ALGO_DILITHIUM2 = 2;
const ALGO_DILITHIUM3 = 2; // Dilithium3 使用 Dilithium2 的標識（因為合約只區分簽名大小類別）

// ============ 類型定義 ============

interface AuditData {
  blob_id: string;           // Base64 格式的 Blob ID
  content_hash: string;      // SHA-256 哈希（Hex）
  file_size: number;
  timestamp: number;
  verification_status: string;
}

interface SignedAuditReport {
  audit_data: AuditData;
  signature: string;         // Base64 格式的 PQC 簽名
  algorithm: string;
  auditor_public_key: string;
  report_timestamp: number;
  auditor_sui_address: string;
}

interface DeployedContracts {
  network: string;
  deployer: string;
  deployedAt: string;
  contracts: {
    audit_system: {
      packageId: string;
      auditConfigId: string;
      auditorRegistryId: string;
      rewardPoolId: string;
    };
    access_policy?: {
      packageId: string;
    };
  };
}

// ============ 工具函數 ============

/**
 * 將 Base64 格式的 Blob ID 轉換為 u256
 */
function blobIdBase64ToU256(blobIdBase64: string): string {
  // 恢復標準 Base64 格式（Walrus 使用 URL-safe Base64）
  const standardBase64 = blobIdBase64
    .replace(/-/g, '+')
    .replace(/_/g, '/');

  // 補齊 padding
  const padding = (4 - (standardBase64.length % 4)) % 4;
  const paddedBase64 = standardBase64 + '='.repeat(padding);

  // 解碼為 Buffer
  const buffer = Buffer.from(paddedBase64, 'base64');

  // 轉換為 BigInt（大端序）
  let bigIntValue = 0n;
  for (let i = 0; i < buffer.length; i++) {
    bigIntValue = (bigIntValue << 8n) | BigInt(buffer[i]);
  }

  return bigIntValue.toString();
}

/**
 * 將 SHA-256 哈希（Hex）轉換為 vector<u8>
 */
function hexHashToBytes(hexHash: string): number[] {
  // 移除 '0x' 前綴（如果有）
  const cleanHex = hexHash.startsWith('0x') ? hexHash.slice(2) : hexHash;

  const bytes: number[] = [];
  for (let i = 0; i < cleanHex.length; i += 2) {
    bytes.push(parseInt(cleanHex.slice(i, i + 2), 16));
  }
  return bytes;
}

/**
 * 將 Base64 格式的簽名轉換為 vector<u8>
 */
function base64SignatureToBytes(signatureBase64: string): number[] {
  const buffer = Buffer.from(signatureBase64, 'base64');
  return Array.from(buffer);
}

/**
 * 載入部署的合約地址
 */
function loadDeployedContracts(): DeployedContracts {
  const content = fs.readFileSync(DEPLOYED_CONTRACTS_PATH, 'utf-8');
  return JSON.parse(content);
}

/**
 * 載入審計報告
 */
function loadAuditReport(): SignedAuditReport {
  const content = fs.readFileSync(AUDIT_REPORT_PATH, 'utf-8');
  return JSON.parse(content);
}

/**
 * 從環境變數或 keystore 載入私鑰
 */
function loadWalletKeypair(): Ed25519Keypair {
  // 方式 1: 從環境變數讀取私鑰（Hex 格式）
  const privateKeyHex = process.env.SUI_PRIVATE_KEY;
  if (privateKeyHex) {
    // 移除 '0x' 前綴（如果有）
    const cleanHex = privateKeyHex.startsWith('0x') ? privateKeyHex.slice(2) : privateKeyHex;
    const privateKeyBytes = Buffer.from(cleanHex, 'hex');
    return Ed25519Keypair.fromSecretKey(privateKeyBytes);
  }

  // 方式 2: 從 Sui CLI keystore 讀取
  const keystorePath = path.join(process.env.HOME || '~', '.sui/sui_config/sui.keystore');
  if (fs.existsSync(keystorePath)) {
    const keystoreContent = fs.readFileSync(keystorePath, 'utf-8');
    const keys = JSON.parse(keystoreContent);

    // 使用第一個密鑰（通常是部署者）
    if (keys.length > 0) {
      const keyBase64 = keys[0];
      const keyBytes = Buffer.from(keyBase64, 'base64');
      // Sui keystore 格式: [scheme_byte, ...privateKey, ...publicKey]
      // Ed25519: scheme=0, privateKey=32 bytes, publicKey=32 bytes
      const privateKeyBytes = keyBytes.slice(1, 33);
      return Ed25519Keypair.fromSecretKey(privateKeyBytes);
    }
  }

  throw new Error('無法載入錢包私鑰。請設置 SUI_PRIVATE_KEY 環境變數或確保 ~/.sui/sui_config/sui.keystore 存在。');
}

// ============ 主函數 ============

async function main() {
  console.log('╔════════════════════════════════════════════════════════════════╗');
  console.log('║        提交審計報告到 Sui 智能合約                            ║');
  console.log('╚════════════════════════════════════════════════════════════════╝\n');

  try {
    // 步驟 1: 載入配置
    console.log('📄 載入配置...\n');

    const deployedContracts = loadDeployedContracts();
    const auditReport = loadAuditReport();
    const keypair = loadWalletKeypair();

    const auditorAddress = keypair.getPublicKey().toSuiAddress();
    console.log(`   審計者地址: ${auditorAddress}`);
    console.log(`   網絡: ${deployedContracts.network}`);
    console.log(`   合約套件: ${deployedContracts.contracts.audit_system.packageId}\n`);

    // 步驟 2: 初始化 Sui 客戶端
    console.log('🔗 連接到 Sui Testnet...\n');
    const client = new SuiClient({ url: SUI_RPC_URL });

    // 檢查餘額
    const balance = await client.getBalance({ owner: auditorAddress });
    console.log(`   錢包餘額: ${Number(balance.totalBalance) / 1_000_000_000} SUI\n`);

    if (Number(balance.totalBalance) === 0) {
      console.error('❌ 錢包餘額不足！請先獲取測試幣：');
      console.error('   https://discord.com/channels/916379725201563759/971488439931392130\n');
      process.exit(1);
    }

    // 步驟 3: 檢查審計者授權狀態
    console.log('🔍 檢查審計者授權...\n');

    const auditConfig = await client.getObject({
      id: deployedContracts.contracts.audit_system.auditConfigId,
      options: { showContent: true },
    });

    if (!auditConfig.data || !auditConfig.data.content) {
      console.error('❌ 無法讀取 AuditConfig 對象\n');
      process.exit(1);
    }

    const configFields = (auditConfig.data.content as any).fields;
    const authorizedAuditors: string[] = configFields.authorized_auditors || [];

    console.log(`   已授權審計者數量: ${authorizedAuditors.length}`);
    console.log(`   當前審計者已授權: ${authorizedAuditors.includes(auditorAddress) ? '✅ 是' : '❌ 否'}\n`);

    if (!authorizedAuditors.includes(auditorAddress)) {
      console.log('⚠️  審計者未授權，嘗試自動授權...\n');

      // 檢查當前用戶是否是管理員
      const adminAddress = configFields.admin;
      if (auditorAddress !== adminAddress) {
        console.error(`❌ 當前地址不是管理員！`);
        console.error(`   管理員地址: ${adminAddress}`);
        console.error(`   當前地址: ${auditorAddress}\n`);
        process.exit(1);
      }

      // 授權自己為審計者
      const authTx = new Transaction();
      authTx.moveCall({
        target: `${deployedContracts.contracts.audit_system.packageId}::audit_core::authorize_auditor`,
        arguments: [
          authTx.object(deployedContracts.contracts.audit_system.auditConfigId),
          authTx.pure.address(auditorAddress),
        ],
      });

      const authResult = await client.signAndExecuteTransaction({
        transaction: authTx,
        signer: keypair,
        options: { showEffects: true },
      });

      console.log(`   授權交易: ${authResult.digest}`);

      if (authResult.effects?.status.status !== 'success') {
        console.error('❌ 授權交易失敗\n');
        process.exit(1);
      }

      console.log('   ✅ 授權成功！\n');
    }

    // 步驟 4: 準備審計數據
    console.log('📊 準備審計數據...\n');

    // 將 Blob ID 轉換為 u256
    const blobIdU256 = blobIdBase64ToU256(auditReport.audit_data.blob_id);
    console.log(`   Blob ID (Base64): ${auditReport.audit_data.blob_id}`);
    console.log(`   Blob ID (u256): ${blobIdU256}\n`);

    // 將 content_hash 轉換為 bytes
    const integrityHash = hexHashToBytes(auditReport.audit_data.content_hash);
    console.log(`   完整性哈希: 0x${auditReport.audit_data.content_hash}`);
    console.log(`   哈希長度: ${integrityHash.length} bytes\n`);

    // 將 PQC 簽名轉換為 bytes
    const pqcSignature = base64SignatureToBytes(auditReport.signature);
    console.log(`   PQC 簽名算法: ${auditReport.algorithm}`);
    console.log(`   簽名長度: ${pqcSignature.length} bytes\n`);

    // 確定 PQC 算法 ID
    let pqcAlgorithmId: number;
    if (auditReport.algorithm === 'Dilithium3' || auditReport.algorithm === 'Dilithium2') {
      pqcAlgorithmId = ALGO_DILITHIUM2;
    } else if (auditReport.algorithm === 'Falcon512') {
      pqcAlgorithmId = ALGO_FALCON512;
    } else {
      console.error(`❌ 不支持的 PQC 算法: ${auditReport.algorithm}\n`);
      process.exit(1);
    }

    // 步驟 5: 構建交易
    console.log('🔨 構建交易...\n');

    // 注意：這裡需要一個真實的 Blob Object ID
    // 因為我們的測試只是下載了 Blob 內容，沒有在 Sui 鏈上創建對應的 Blob 對象
    // 為了演示，我們使用一個已知的 Blob Object ID（從之前讀取元數據的例子）
    const BLOB_OBJECT_ID = '0x38957e0e7dbb9259b72a06b2c6d252f4f76e6adc72fe247abe381baaac699ac2';

    // 獲取當前 epoch（作為 challenge_epoch 參數）
    const latestCheckpoint = await client.getLatestCheckpointSequenceNumber();
    const checkpoint = await client.getCheckpoint({ id: latestCheckpoint });
    const currentEpoch = Number(checkpoint.epoch);

    console.log(`   當前 Epoch: ${currentEpoch}`);
    console.log(`   挑戰次數: 10 (模擬)`);
    console.log(`   成功驗證: 10 (模擬)`);
    console.log(`   Blob Object ID: ${BLOB_OBJECT_ID}\n`);

    const tx = new Transaction();

    // 獲取 Clock 對象（0x6 是系統 Clock 的地址）
    const clockObjectId = '0x6';

    tx.moveCall({
      target: `${deployedContracts.contracts.audit_system.packageId}::audit_core::submit_audit_record`,
      arguments: [
        tx.object(deployedContracts.contracts.audit_system.auditConfigId),  // config
        tx.pure.u256(blobIdU256),                                            // blob_id
        tx.pure.id(BLOB_OBJECT_ID),                                          // blob_object_id
        tx.pure.u32(currentEpoch),                                           // challenge_epoch
        tx.pure.u16(10),                                                     // total_challenges
        tx.pure.u16(10),                                                     // successful_verifications
        tx.pure(bcs.vector(bcs.u8()).serialize(integrityHash)),              // integrity_hash
        tx.pure(bcs.vector(bcs.u8()).serialize(pqcSignature)),               // pqc_signature
        tx.pure.u8(pqcAlgorithmId),                                          // pqc_algorithm
        tx.object(clockObjectId),                                            // clock
      ],
    });

    // 步驟 6: 簽名並發送交易
    console.log('📤 提交交易到 Sui...\n');

    const result = await client.signAndExecuteTransaction({
      transaction: tx,
      signer: keypair,
      options: {
        showEffects: true,
        showEvents: true,
        showObjectChanges: true,
      },
    });

    console.log(`   交易摘要: ${result.digest}`);
    console.log(`   交易狀態: ${result.effects?.status.status}\n`);

    if (result.effects?.status.status !== 'success') {
      console.error('❌ 交易失敗！');
      console.error(`   錯誤: ${result.effects?.status.error}\n`);
      process.exit(1);
    }

    // 步驟 7: 解析事件
    console.log('📋 解析事件...\n');

    if (result.events && result.events.length > 0) {
      for (const event of result.events) {
        console.log(`   事件類型: ${event.type}`);
        console.log(`   事件數據:`, JSON.stringify(event.parsedJson, null, 2));
        console.log('');
      }
    }

    // 步驟 8: 解析創建的對象
    console.log('🆕 新創建的對象...\n');

    const createdObjects = result.objectChanges?.filter(
      (change) => change.type === 'created'
    );

    if (createdObjects && createdObjects.length > 0) {
      for (const obj of createdObjects) {
        if (obj.type === 'created') {
          console.log(`   對象 ID: ${obj.objectId}`);
          console.log(`   對象類型: ${obj.objectType}`);
          console.log(`   所有者: ${JSON.stringify(obj.owner)}\n`);
        }
      }
    }

    // 步驟 9: 保存結果
    const resultPath = '/tmp/audit_submission_result.json';
    fs.writeFileSync(
      resultPath,
      JSON.stringify({
        transactionDigest: result.digest,
        status: result.effects?.status.status,
        events: result.events,
        objectChanges: result.objectChanges,
        timestamp: new Date().toISOString(),
      }, null, 2)
    );

    console.log(`💾 交易結果已保存: ${resultPath}\n`);

    console.log('╔════════════════════════════════════════════════════════════════╗');
    console.log('║                   審計報告提交成功！                           ║');
    console.log('╚════════════════════════════════════════════════════════════════╝\n');

    console.log('🔗 查看交易詳情:');
    console.log(`   https://testnet.suivision.xyz/txblock/${result.digest}\n`);

  } catch (error: any) {
    console.error('\n❌ 錯誤:', error.message);
    if (error.stack) {
      console.error('\n堆疊追蹤:');
      console.error(error.stack);
    }
    process.exit(1);
  }
}

main();
