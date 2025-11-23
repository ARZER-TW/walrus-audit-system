/**
 * 創建測試用的 ReportAccessPolicy 對象
 *
 * 執行方式:
 * cd seal-client && npx tsx create-test-policy.ts
 */

import { Ed25519Keypair } from '@mysten/sui/keypairs/ed25519';
import { Transaction } from '@mysten/sui/transactions';
import { SuiClient } from '@mysten/sui/client';

const ACCESS_PACKAGE_ID = '0xe357603e287e9475ad94b9c3256d71a8e342aedd7488838cb05ee7dcacfba8c5';

async function createTestPolicy() {
  console.log('🔧 創建測試 ReportAccessPolicy...\n');

  // 初始化 Sui 客戶端
  const client = new SuiClient({
    url: 'https://fullnode.testnet.sui.io:443'
  });

  // 創建測試 keypair
  const keypair = new Ed25519Keypair();
  const creator = keypair.getPublicKey().toSuiAddress();

  console.log(`創建者地址: ${creator}`);

  // 模擬一個 blob ID (使用創建者地址作為測試 blob ID)
  // 將地址轉換為 u256 (移除 0x 前綴並轉換為 BigInt)
  const testBlobId = creator;
  const blobIdHex = testBlobId.startsWith('0x') ? testBlobId.slice(2) : testBlobId;
  const blobIdU256 = BigInt('0x' + blobIdHex);

  console.log(`測試 Blob ID (address): ${testBlobId}`);
  console.log(`測試 Blob ID (u256): ${blobIdU256.toString()}\n`);

  // 構造交易
  const tx = new Transaction();

  // 調用 create_policy
  // public entry fun create_policy(
  //     report_blob_id: u256,
  //     audit_record_id: ID,
  //     allowed_readers: vector<address>,
  //     allowed_auditors: vector<address>,
  //     expires_at_ms: Option<u64>,
  //     clock: &Clock,
  //     ctx: &mut TxContext
  // )

  // 創建一個虛擬的 audit_record_id (使用相同的 creator 地址轉換為 ID)
  const auditRecordId = testBlobId;

  tx.moveCall({
    target: `${ACCESS_PACKAGE_ID}::report_access::create_policy`,
    arguments: [
      tx.pure.u256(blobIdU256),  // report_blob_id (BigInt 格式的 u256)
      tx.pure.id(auditRecordId),  // audit_record_id (ID)
      tx.pure.vector('address', [creator]),  // allowed_readers (包含創建者自己)
      tx.pure.vector('address', []),  // allowed_auditors (空)
      tx.pure.option('u64', null),  // expires_at_ms (無過期時間)
      tx.object('0x6')  // clock (&Clock) - Sui Clock 對象
      // ctx: &mut TxContext 自動傳遞
    ]
  });

  // 簽名並執行
  try {
    console.log('📤 發送交易...');
    const result = await client.signAndExecuteTransaction({
      signer: keypair,
      transaction: tx,
      options: {
        showEffects: true,
        showObjectChanges: true
      }
    });

    console.log('\n✅ 交易成功!');
    console.log(`Transaction Digest: ${result.digest}`);

    // 找到創建的 ReportAccessPolicy 對象
    const created = result.objectChanges?.filter(
      (change: any) => change.type === 'created' && change.objectType.includes('ReportAccessPolicy')
    );

    if (created && created.length > 0) {
      const policyId = (created[0] as any).objectId;
      console.log(`\n🎯 ReportAccessPolicy 對象已創建:`);
      console.log(`   Object ID: ${policyId}`);
      console.log(`   Creator: ${creator}`);
      console.log(`   Blob ID: ${testBlobId}`);
      console.log(`\n📋 將此 Object ID 添加到 seal-api-server.ts 中的 AccessProofBuilder 配置:`);
      console.log(`   policyObjectId: '${policyId}'`);
    } else {
      console.log('\n⚠️  未找到創建的 ReportAccessPolicy 對象');
      console.log('Object Changes:');
      console.log(JSON.stringify(result.objectChanges, null, 2));
    }

  } catch (error: any) {
    console.error('\n❌ 交易失敗:');
    console.error(error.message);
    if (error.cause) {
      console.error('原因:', error.cause);
    }
    process.exit(1);
  }
}

createTestPolicy().catch(console.error);
