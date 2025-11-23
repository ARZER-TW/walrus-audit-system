/**
 * 註冊審計員
 *
 * 目標：註冊錢包為審計員（質押 1 SUI + 提供 PQC 公鑰）
 *
 * 執行方式：
 * cd seal-client && npx tsx register-auditor.ts
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

// 用戶的錢包地址（有測試網代幣）
const USER_ADDRESS = '0xab8e37e25fe9f46493c4c1ef0c548750dae56ca47ed35324c61b9bed574104d9';

// 最低質押要求（1 SUI = 1,000,000,000 MIST）
const MIN_STAKE_MIST = 1_000_000_000n;

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
 * 生成模擬 Dilithium3 公鑰（實際應用中應該使用真實的 liboqs 生成）
 */
function generateMockDilithium3PublicKey(): Uint8Array {
  // Dilithium3 公鑰大小：1952 bytes
  // 注意：這只是測試用的隨機數據
  // 真實應用中應該使用 liboqs 生成配對的公私鑰
  return crypto.randomBytes(1952);
}

async function registerAuditor() {
  console.log('🚀 註冊審計員到 Sui Testnet');
  console.log('='.repeat(70));

  // 1. 初始化 Sui 客戶端
  const client = new SuiClient({
    url: 'https://fullnode.testnet.sui.io:443'
  });

  // 2. 載入錢包
  console.log('\n1️⃣ 載入錢包...');
  const keypair = loadKeypairFromSuiKeystore(USER_ADDRESS);
  const auditor = keypair.getPublicKey().toSuiAddress();

  // 3. 檢查 Gas balance
  console.log('\n2️⃣ 檢查 Gas balance...');
  const balance = await client.getBalance({ owner: auditor });
  const balanceSui = Number(balance.totalBalance) / 1_000_000_000;
  console.log(`   餘額: ${balance.totalBalance} MIST (${balanceSui} SUI)`);

  if (BigInt(balance.totalBalance) < MIN_STAKE_MIST * 2n) {
    throw new Error(`餘額不足。需要至少 ${Number(MIN_STAKE_MIST * 2n) / 1_000_000_000} SUI（1 SUI 質押 + gas）`);
  }

  // 4. 檢查是否已註冊
  console.log('\n3️⃣ 檢查註冊狀態...');
  try {
    const result = await client.devInspectTransactionBlock({
      sender: auditor,
      transactionBlock: (() => {
        const tx = new Transaction();
        tx.moveCall({
          target: `${AUDIT_PACKAGE_ID}::auditor_registry::is_auditor_registered`,
          arguments: [
            tx.object(AUDITOR_REGISTRY_ID),
            tx.pure.address(auditor)
          ]
        });
        return tx;
      })()
    });

    // 嘗試解析返回值（可能沒有明確的返回值顯示）
    console.log('   查詢結果:', JSON.stringify(result, null, 2));
  } catch (error: any) {
    console.log('   查詢失敗（可能未註冊）:', error.message);
  }

  // 5. 生成 PQC 公鑰
  console.log('\n4️⃣ 生成 PQC 公鑰（Dilithium3 模擬）...');
  const pqcPublicKey = generateMockDilithium3PublicKey();
  console.log(`   公鑰長度: ${pqcPublicKey.length} bytes`);

  // 6. 獲取用於質押的 Gas Coin
  console.log('\n5️⃣ 準備質押 coin...');
  const coins = await client.getCoins({
    owner: auditor,
    coinType: '0x2::sui::SUI'
  });

  if (coins.data.length === 0) {
    throw new Error('未找到可用的 SUI coin');
  }

  console.log(`   找到 ${coins.data.length} 個 SUI coins`);

  // 7. 構造交易
  console.log('\n6️⃣ 構造註冊交易...');
  const tx = new Transaction();

  // 分割一個 coin 用於質押（1 SUI）
  const [stakeCoin] = tx.splitCoins(tx.gas, [MIN_STAKE_MIST]);

  // 調用 register_auditor
  // public entry fun register_auditor(
  //     registry: &mut AuditorRegistry,
  //     stake: Coin<SUI>,
  //     pqc_public_key: vector<u8>,
  //     ctx: &mut TxContext
  // )
  tx.moveCall({
    target: `${AUDIT_PACKAGE_ID}::auditor_registry::register_auditor`,
    arguments: [
      tx.object(AUDITOR_REGISTRY_ID),               // registry
      stakeCoin,                                     // stake (Coin<SUI>)
      tx.pure.vector('u8', Array.from(pqcPublicKey)) // pqc_public_key (使用 pure.vector 方法)
    ]
  });

  console.log('   ✅ 交易構造完成');

  // 8. 執行交易
  console.log('\n7️⃣ 執行交易...');
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
    console.log(`   Transaction: https://testnet.suivision.xyz/txblock/${result.digest}`);

    // 9. 顯示事件
    if (result.events && result.events.length > 0) {
      console.log('\n8️⃣ 註冊事件:');
      result.events.forEach((event: any) => {
        console.log(`   Type: ${event.type}`);
        if (event.parsedJson) {
          console.log(`   Data:`, event.parsedJson);
        }
      });
    }

    console.log('\n✅ 審計員註冊成功！');
    console.log('='.repeat(70));
    console.log('\n📝 下一步:');
    console.log('   1. 運行 submit-audit-report-simple.ts');
    console.log('   2. 提交審計報告元數據');
    console.log('   3. 獲取 audit_record_id');

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
registerAuditor().catch(console.error);
