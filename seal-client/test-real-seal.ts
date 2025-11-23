/**
 * 真實 Seal SDK 加密-解密測試
 *
 * 測試流程:
 * 1. 使用 Seal IBE 加密報告
 * 2. 創建 Session Key (符合 Seal SDK 標準格式)
 * 3. 使用 fetchKeys + decrypt 兩步法解密
 *
 * 執行方式:
 * npx tsx test-real-seal.ts
 */

async function testRealSeal() {
  console.log('🧪 測試真實的 Seal 加密-解密流程');
  console.log('='.repeat(70));

  const BASE_URL = 'http://localhost:3001';
  const testIdentity = '0xab8e37e25fe9f46493c4c1ef0c548750dae56ca47ed35324c61b9bed574104d9';
  const packageId = '0x1bc5c277f6c0fd20f97cf555d83ea6f9df753d93fbf99b8890a97df31af21804';

  // 測試報告
  const testReport = {
    metadata: {
      reportId: '0xreal-seal-test',
      timestamp: Date.now(),
      auditor: testIdentity
    },
    records: [{
      blobId: '0xblob001',
      result: true,
      timestamp: Date.now()
    }]
  };

  try {
    // 1. 加密 (使用真實的 Seal IBE)
    console.log('\n1️⃣ 使用 Seal IBE 加密...');
    const encryptRes = await fetch(`${BASE_URL}/api/seal/encrypt`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        data: JSON.stringify(testReport),
        identity: testIdentity,
        packageId: packageId,
        threshold: 2
      })
    });

    if (!encryptRes.ok) {
      throw new Error(`加密請求失敗: ${encryptRes.status} ${encryptRes.statusText}`);
    }

    const encryptResult = await encryptRes.json() as any;

    if (!encryptResult.success) {
      throw new Error(`加密失敗: ${encryptResult.error || '未知錯誤'}`);
    }

    console.log('✅ 加密成功');
    console.log(`   Seal Object ID: ${encryptResult.identity}`);
    console.log(`   密文大小: ${encryptResult.encryptedData.length} chars`);

    // 2. 創建 Session Key (標準格式)
    console.log('\n2️⃣ 創建 Session Key (Seal SDK 標準格式)...');
    const sessionKeyRes = await fetch(`${BASE_URL}/api/seal/create-session-key`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        address: testIdentity,
        packageId: packageId,
        ttlMin: 60
      })
    });

    if (!sessionKeyRes.ok) {
      throw new Error(`Session Key 創建失敗: ${sessionKeyRes.status}`);
    }

    const sessionKeyResponse = await sessionKeyRes.json() as any;

    if (!sessionKeyResponse.success || !sessionKeyResponse.sessionKey) {
      throw new Error(`Session Key 創建失敗: ${JSON.stringify(sessionKeyResponse)}`);
    }

    const sessionKeyInfo = sessionKeyResponse.sessionKey;

    console.log('✅ Session Key 創建成功');
    console.log('   消息格式:');
    console.log(`   "${sessionKeyInfo.message}"`);
    console.log(`   Public Key: ${sessionKeyInfo.publicKey}`);

    // 模擬錢包簽名 (實際應用中需要真實錢包簽名)
    const mockSignature = 'mock-wallet-signature-' + Date.now();
    console.log(`   簽名 (mock): ${mockSignature.substring(0, 30)}...`);

    // 3. 解密 (使用真實的 fetchKeys + decrypt)
    console.log('\n3️⃣ 解密 (Seal SDK 標準流程: fetchKeys + decrypt)...');
    console.log(`   使用 Seal Object ID: ${encryptResult.identity}`);
    const decryptRes = await fetch(`${BASE_URL}/api/seal/decrypt`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        encryptedData: encryptResult.encryptedData,
        reportId: testReport.metadata.reportId,
        requesterAddress: testIdentity,
        objectId: encryptResult.identity,  // 🔥 傳遞 Seal 加密對象 ID
        sessionKey: {
          publicKey: sessionKeyInfo.publicKey,
          signature: mockSignature,
          expiresAt: sessionKeyInfo.expiresAt,
          message: sessionKeyInfo.message
        }
      })
    });

    if (!decryptRes.ok) {
      const errorText = await decryptRes.text();
      throw new Error(`解密請求失敗: ${decryptRes.status}\n${errorText}`);
    }

    const decryptResult = await decryptRes.json() as any;

    // 4. 檢查結果
    console.log('\n4️⃣ 解密結果:');

    if (decryptResult.success) {
      console.log('✅ 解密成功!');
      console.log(`   模式: ${decryptResult.mode}`);
      console.log(`   消息: ${decryptResult.message}`);

      if (decryptResult.mode === 'real-seal') {
        console.log('\n🎉 真實的 Seal 解密工作正常!');
        console.log('   ✅ fetchKeys() 成功');
        console.log('   ✅ decrypt() 成功');
      } else if (decryptResult.mode === 'fallback') {
        console.log('\n⚠️  使用了降級模式 (Seal SDK 調用失敗)');
        if (decryptResult.sealError) {
          console.log(`   Seal 錯誤: ${decryptResult.sealError}`);
        }
      }

      // 驗證解密內容
      if (decryptResult.report) {
        console.log('\n📄 解密後的報告內容:');
        console.log(JSON.stringify(decryptResult.report, null, 2));
      }
    } else {
      console.log('❌ 解密失敗');
      console.log('   錯誤:', decryptResult.error);
      if (decryptResult.details) {
        console.log('   詳情:', decryptResult.details);
      }
    }

    console.log('\n' + '='.repeat(70));
    console.log('測試完成');

  } catch (error: any) {
    console.error('\n❌ 測試失敗:');
    console.error('   ', error.message);
    if (error.stack) {
      console.error('\n堆棧追踪:');
      console.error(error.stack);
    }
    process.exit(1);
  }
}

// 執行測試
testRealSeal().catch(console.error);
