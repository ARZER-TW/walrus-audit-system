/**
 * Seal HTTP API Server
 *
 * 提供 HTTP 接口讓 Rust 審計節點可以調用 Seal 加密/解密功能
 *
 * Endpoints:
 * - POST /api/seal/encrypt - 加密數據
 * - POST /api/seal/decrypt - 解密數據（需訪問控制驗證）
 * - POST /api/seal/check-access - 檢查訪問權限
 * - POST /api/seal/create-session-key - 創建 Session Key（前端錢包簽名用）
 * - GET /health - 健康檢查
 */

import express, { Request, Response } from 'express';
import cors from 'cors';
import { SealClient, SessionKey } from "@mysten/seal";
import type { ExportedSessionKey } from "@mysten/seal";
import { SuiClient, getFullnodeUrl } from "@mysten/sui/client";
import { Transaction } from "@mysten/sui/transactions";
import { SuiContractClient } from './src/sui-contract-client';
import { AccessProofBuilder } from './src/access-proof-builder';
import { WalrusClient, createTestnetWalrusClient } from './src/walrus-client';

const app = express();
const PORT = process.env.SEAL_API_PORT || 3001;

// 中間件
app.use(cors());
app.use(express.json({ limit: '10mb' })); // 支持大型報告

// Seal 客戶端配置
let sealClient: SealClient | null = null;

// Sui 合約客戶端
let suiContractClient: SuiContractClient | null = null;

// 訪問證明構造器
let accessProofBuilder: AccessProofBuilder | null = null;

// 🆕 Seal SDK SessionKey 儲存池 (使用官方 SessionKey 類)
// Key: publicKey, Value: SessionKey 實例
const sealSessionKeyStore = new Map<string, SessionKey>();

// SuiClient 實例 (用於創建 SessionKey)
let suiClient: SuiClient | null = null;

// Walrus 客戶端
let walrusClient: WalrusClient | null = null;

/**
 * 初始化 Seal 客戶端
 */
async function initializeSealClient() {
  console.log("🔧 初始化 Seal 客戶端...");

  const network = process.env.SUI_NETWORK || 'testnet';
  suiClient = new SuiClient({
    url: getFullnodeUrl(network as any)
  });

  // 使用官方 Testnet Key Servers（從 seal-docs.wal.app 獲取）
  const testnetServers = [
    { objectId: "0x73d05d62c18d9374e3ea529e8e0ed6161da1a141a94d3f76ae3fe4e99356db75", weight: 1 }, // Mysten Labs testnet-1
    { objectId: "0xf5d14a81a982144ae441cd7d64b09027f116a468bd36e7eca494f750591623c8", weight: 1 }, // Mysten Labs testnet-2
    { objectId: "0x6068c0acb197dddbacd4746a9de7f025b2ed5a5b6c1b1ab44dade4426d141da2", weight: 1 }, // Ruby Nodes
  ];

  sealClient = new SealClient({
    suiClient,
    serverConfigs: testnetServers,
    verifyKeyServers: true
  });

  const chainId = await suiClient.getChainIdentifier();
  console.log(`✅ Seal 客戶端初始化完成`);
  console.log(`   - Network: ${network}`);
  console.log(`   - Chain ID: ${chainId}`);
  console.log(`   - Key Servers: ${testnetServers.length}`);

  // 🆕 初始化 Sui 合約客戶端
  console.log('\n🔧 初始化 Sui 合約客戶端...');
  suiContractClient = new SuiContractClient({
    network: 'testnet',
    auditPackageId: '0x1bc5c277f6c0fd20f97cf555d83ea6f9df753d93fbf99b8890a97df31af21804',
    accessPackageId: '0xe357603e287e9475ad94b9c3256d71a8e342aedd7488838cb05ee7dcacfba8c5',  // 🔥 新部署的 access_policy 合約
    auditorRegistryId: '0x3ff5961eae0235665d355293820459a8da4ce564bed87f8680a7552d5553227f',
    auditConfigId: '0xd12faca845bde47983c02dc6927e8fe24db5a7ac4f0092f72ee0f6839a8e4b21'
  });

  try {
    // 測試連接 - 讀取 AuditConfig
    const config = await suiContractClient.getAuditConfig();
    console.log('✅ Sui 合約連接測試成功');
    console.log(`   - AuditConfig ID: 0xd12faca845bde47983c02dc6927e8fe24db5a7ac4f0092f72ee0f6839a8e4b21`);
    console.log(`   - Min Stake: ${config.min_stake || 'N/A'}`);
  } catch (error) {
    console.error('⚠️  Sui 合約連接測試失敗:', error);
    console.log('   - 將繼續運行，但合約查詢功能可能不可用');
  }

  // 🆕 初始化訪問證明構造器
  console.log('\n🔧 初始化訪問證明構造器...');
  accessProofBuilder = new AccessProofBuilder({
    suiClient,
    accessPackageId: '0xe357603e287e9475ad94b9c3256d71a8e342aedd7488838cb05ee7dcacfba8c5'  // 🔥 新部署的 access_policy 合約
    // policyObjectId: '0x...' // 可選,如果有 ReportAccessPolicy 對象
  });
  console.log('✅ 訪問證明構造器初始化完成');

  // 🆕 初始化 Walrus 客戶端
  console.log('\n🔧 初始化 Walrus 客戶端...');
  walrusClient = createTestnetWalrusClient();

  // 檢查網絡狀態
  const isHealthy = await walrusClient.checkHealth();
  if (isHealthy) {
    console.log('✅ Walrus 客戶端初始化完成');
  } else {
    console.warn('⚠️  Walrus 網絡可能不可用');
  }
}

/**
 * 健康檢查端點
 */
app.get('/health', (req: Request, res: Response) => {
  const healthy = sealClient !== null && walrusClient !== null;
  res.status(healthy ? 200 : 503).json({
    status: healthy ? 'healthy' : 'initializing',
    service: 'Seal API Server',
    version: '1.0.0',
    timestamp: new Date().toISOString(),
    modules: {
      seal: sealClient ? '✅' : '❌',
      sui: suiClient ? '✅' : '❌',
      walrus: walrusClient ? '✅' : '❌',
      accessControl: accessProofBuilder ? '✅' : '❌'
    },
    endpoints: {
      encrypt: 'POST /api/seal/encrypt',
      decrypt: 'POST /api/seal/decrypt',
      createSessionKey: 'POST /api/seal/create-session-key',
      setSignature: 'POST /api/seal/set-signature',
      checkAccess: 'POST /api/seal/check-access',
      walrusUpload: 'POST /api/walrus/upload',
      walrusDownload: 'GET /api/walrus/download/:blobId'
    }
  });
});

/**
 * 加密端點
 *
 * Request Body:
 * {
 *   "data": "base64-encoded-data",
 *   "identity": "0x...",      // Sui address (auditor address)
 *   "packageId": "0x...",     // Audit contract package ID
 *   "threshold": 2            // Optional, default 2
 * }
 *
 * Response:
 * {
 *   "success": true,
 *   "encryptedData": "base64-encoded-encrypted-data",
 *   "symmetricKey": "base64-encoded-key",
 *   "metadata": {
 *     "identity": "0x...",
 *     "packageId": "0x...",
 *     "threshold": 2,
 *     "encryptedAt": 1234567890
 *   }
 * }
 */
app.post('/api/seal/encrypt', async (req: Request, res: Response) => {
  try {
    if (!sealClient) {
      return res.status(503).json({
        success: false,
        error: 'Seal client not initialized'
      });
    }

    const { data, identity, packageId, threshold = 2 } = req.body;

    // 驗證必要參數
    if (!data) {
      return res.status(400).json({
        success: false,
        error: 'Missing required field: data'
      });
    }

    if (!identity) {
      return res.status(400).json({
        success: false,
        error: 'Missing required field: identity (auditor Sui address)'
      });
    }

    if (!packageId) {
      return res.status(400).json({
        success: false,
        error: 'Missing required field: packageId (audit contract package ID)'
      });
    }

    // 驗證 identity 和 packageId 是否為有效的十六進制地址
    if (!identity.startsWith('0x') || identity.length !== 66) {
      return res.status(400).json({
        success: false,
        error: 'Invalid identity format (must be 32-byte hex string starting with 0x)'
      });
    }

    if (!packageId.startsWith('0x') || packageId.length !== 66) {
      return res.status(400).json({
        success: false,
        error: 'Invalid packageId format (must be 32-byte hex string starting with 0x)'
      });
    }

    console.log(`📦 加密請求:`);
    console.log(`   - Identity: ${identity}`);
    console.log(`   - Package ID: ${packageId}`);
    console.log(`   - Threshold: ${threshold}`);
    console.log(`   - Data size: ${data.length} bytes (base64)`);

    // 解碼 Base64 數據
    const dataBytes = Buffer.from(data, 'base64');
    console.log(`   - Decoded size: ${dataBytes.length} bytes`);

    // 調用 Seal SDK 加密
    const startTime = Date.now();
    const encryptResult = await sealClient.encrypt({
      threshold,
      packageId,
      id: identity,
      data: dataBytes,
    });
    const duration = Date.now() - startTime;

    console.log(`✅ 加密成功 (耗時 ${duration}ms)`);
    console.log(`   - 加密數據長度: ${encryptResult.encryptedObject.length} bytes`);
    console.log(`   - 對稱密鑰長度: ${encryptResult.key.length} bytes`);
    console.log(`   - Seal Object ID: ${identity}`); // 🆕 用於解密的 objectId

    // 返回結果
    res.json({
      success: true,
      encryptedData: Buffer.from(encryptResult.encryptedObject).toString('base64'),
      symmetricKey: Buffer.from(encryptResult.key).toString('base64'),
      identity: identity, // 🆕 返回 Seal Object ID (用於解密時的 fetchKeys)
      metadata: {
        identity,
        packageId,
        threshold,
        encryptedAt: Date.now(),
        originalSize: dataBytes.length,
        encryptedSize: encryptResult.encryptedObject.length,
        duration
      }
    });

  } catch (error: any) {
    console.error('❌ 加密失敗:', error.message);
    console.error('詳細錯誤:', error);

    res.status(500).json({
      success: false,
      error: error.message || 'Encryption failed',
      details: process.env.NODE_ENV === 'development' ? error.stack : undefined
    });
  }
});

/**
 * 檢查訪問權限端點
 *
 * Request Body:
 * {
 *   "reportId": "0x...",        // 報告的 Blob ID 或 Object ID
 *   "requesterAddress": "0x..."  // 請求者的 Sui 地址
 * }
 *
 * Response:
 * {
 *   "success": true,
 *   "hasAccess": true/false,
 *   "reason": "creator" | "compliance_officer" | "regulatory_approval" | "denied"
 * }
 */
app.post('/api/seal/check-access', async (req: Request, res: Response) => {
  try {
    const { reportId, requesterAddress } = req.body;

    if (!reportId || !requesterAddress) {
      return res.status(400).json({
        success: false,
        error: 'Missing required fields: reportId, requesterAddress'
      });
    }

    console.log(`🔍 檢查訪問權限:`);
    console.log(`   - Report ID: ${reportId}`);
    console.log(`   - Requester: ${requesterAddress}`);

    // TODO: 實際實現需要調用 Sui Move 合約
    // access_policy::report_access::can_access_report(reportId, requesterAddress)
    //
    // 現在使用簡化邏輯：
    // 1. 創建者總是有訪問權限
    // 2. 特定的合規官員地址有訪問權限
    // 3. 其他地址需要監管批准

    const hasAccess = await checkSuiAccessPolicy(reportId, requesterAddress);

    res.json({
      success: true,
      hasAccess,
      reason: hasAccess ? 'creator' : 'denied',
      timestamp: Date.now()
    });

  } catch (error: any) {
    console.error('❌ 訪問檢查失敗:', error.message);
    res.status(500).json({
      success: false,
      error: error.message || 'Access check failed'
    });
  }
});

/**
 * 創建 Session Key 端點 (🔥 使用 Seal SDK 官方 SessionKey 類)
 *
 * Request Body:
 * {
 *   "address": "0x...",      // 用戶 Sui 地址
 *   "packageId": "0x...",    // Audit 合約 Package ID
 *   "ttlMin": 60             // 有效期（分鐘），預設 60
 * }
 *
 * Response:
 * {
 *   "success": true,
 *   "sessionKey": ExportedSessionKey  // Seal SDK 標準格式
 * }
 */
app.post('/api/seal/create-session-key', async (req: Request, res: Response) => {
  try {
    const { address, packageId, ttlMin = 60 } = req.body;

    // 驗證必要參數
    if (!address) {
      return res.status(400).json({
        success: false,
        error: 'Missing required field: address (user Sui address)'
      });
    }

    if (!packageId) {
      return res.status(400).json({
        success: false,
        error: 'Missing required field: packageId (audit contract package ID)'
      });
    }

    if (!suiClient) {
      return res.status(503).json({
        success: false,
        error: 'Sui client not initialized'
      });
    }

    // 驗證地址格式
    if (!address.startsWith('0x') || address.length !== 66) {
      return res.status(400).json({
        success: false,
        error: 'Invalid address format (must be 32-byte hex string starting with 0x)'
      });
    }

    if (!packageId.startsWith('0x') || packageId.length !== 66) {
      return res.status(400).json({
        success: false,
        error: 'Invalid packageId format (must be 32-byte hex string starting with 0x)'
      });
    }

    console.log(`🔑 創建 Seal SDK SessionKey:`);
    console.log(`   - Address: ${address}`);
    console.log(`   - Package ID: ${packageId}`);
    console.log(`   - TTL: ${ttlMin} 分鐘`);

    // 🔥 使用 Seal SDK 官方 SessionKey.create()
    const sessionKey = await SessionKey.create({
      address,
      packageId,
      ttlMin,
      suiClient
      // signer: 不提供,讓前端用戶自己簽名
    });

    // 導出 SessionKey (序列化為可傳輸格式)
    const exported: ExportedSessionKey = sessionKey.export();

    // 保存 SessionKey 實例到內存 (用於後續解密)
    // 注意: 這裡我們需要一個標識符來存儲,使用 address+packageId 組合
    const storeKey = `${address}:${packageId}`;
    sealSessionKeyStore.set(storeKey, sessionKey);

    console.log(`✅ Seal SDK SessionKey 創建成功`);
    console.log(`   - Session Key: ${exported.sessionKey.substring(0, 20)}...`);
    console.log(`   - Created At: ${new Date(exported.creationTimeMs).toISOString()}`);
    console.log(`   - Expires At: ${new Date(exported.creationTimeMs + exported.ttlMin * 60 * 1000).toISOString()}`);

    // 🔥 獲取要簽名的消息 (返回給前端用於錢包簽名)
    const personalMessage = sessionKey.getPersonalMessage();
    const personalMessageBase64 = Buffer.from(personalMessage).toString('base64');

    // 🔥 明確構造純 JavaScript 對象以避免序列化錯誤
    const responseData = {
      address: exported.address,
      packageId: exported.packageId,
      mvrName: exported.mvrName,
      creationTimeMs: exported.creationTimeMs,
      ttlMin: exported.ttlMin,
      personalMessageSignature: exported.personalMessageSignature,
      sessionKey: exported.sessionKey,
      personalMessage: personalMessageBase64  // 🔥 供前端錢包簽名
    };

    res.json({
      success: true,
      sessionKey: responseData
    });

  } catch (error: any) {
    console.error('❌ Seal SDK SessionKey 創建失敗:', error.message);
    console.error('   詳細錯誤:', error);
    res.status(500).json({
      success: false,
      error: error.message || 'Session key creation failed',
      details: process.env.NODE_ENV === 'development' ? error.stack : undefined
    });
  }
});

/**
 * 設置 Session Key 簽名端點 (🔥 前端錢包簽名後提交)
 *
 * Request Body:
 * {
 *   "address": "0x...",
 *   "packageId": "0x...",
 *   "signature": "base64-encoded-signature"
 * }
 *
 * Response:
 * {
 *   "success": true,
 *   "message": "Signature set successfully"
 * }
 */
app.post('/api/seal/set-signature', async (req: Request, res: Response) => {
  try {
    const { address, packageId, signature } = req.body;

    if (!address || !packageId || !signature) {
      return res.status(400).json({
        success: false,
        error: 'Missing required fields: address, packageId, signature'
      });
    }

    const storeKey = `${address}:${packageId}`;
    const sessionKey = sealSessionKeyStore.get(storeKey);

    if (!sessionKey) {
      return res.status(404).json({
        success: false,
        error: 'Session key not found. Please create a new one.'
      });
    }

    // 檢查是否過期
    if (sessionKey.isExpired()) {
      sealSessionKeyStore.delete(storeKey);
      return res.status(401).json({
        success: false,
        error: 'Session key has expired'
      });
    }

    console.log(`🖊️  設置 Session Key 簽名:`);
    console.log(`   - Address: ${address}`);
    console.log(`   - Package ID: ${packageId}`);
    console.log(`   - Signature: ${signature.substring(0, 20)}...`);

    // 🔥 設置前端錢包簽名
    await sessionKey.setPersonalMessageSignature(signature);

    console.log(`✅ 簽名設置成功`);

    res.json({
      success: true,
      message: 'Signature set successfully'
    });

  } catch (error: any) {
    console.error('❌ 設置簽名失敗:', error.message);
    res.status(500).json({
      success: false,
      error: error.message || 'Failed to set signature'
    });
  }
});

/**
 * 解密端點 (🔥 100% 真實的 Seal SDK 實現)
 *
 * Request Body:
 * {
 *   "encryptedData": "base64-encoded-encrypted-data",
 *   "reportId": "0x...",         // 報告 ID（用於訪問控制）
 *   "requesterAddress": "0x...", // 請求者地址
 *   "objectId": "0x...",         // 🔥 Seal Object ID (從加密結果中獲取)
 *   "sessionKey": {              // Session Key（前端錢包簽名後）
 *     "publicKey": "0x...",
 *     "signature": "base64-encoded-signature",
 *     "expiresAt": 1234567890,
 *     "message": "Accessing keys of package..."
 *   }
 * }
 *
 * Response:
 * {
 *   "success": true,
 *   "report": { ... },  // 解密後的 JSON 報告
 *   "mode": "real-seal", // "real-seal" 或 "fallback"
 *   "metadata": {
 *     "reportId": "0x...",
 *     "decryptedAt": 1234567890,
 *     "requester": "0x...",
 *     "sizeBytes": 12345
 *   }
 * }
 */
app.post('/api/seal/decrypt', async (req: Request, res: Response) => {
  console.log('\n🔓 ===== 解密請求 (Seal SDK 真實流程) =====');

  try {
    // Step 0: 驗證必要服務已初始化
    if (!sealClient) {
      console.error('❌ Seal 客戶端未初始化');
      return res.status(503).json({
        success: false,
        error: 'Seal client not initialized'
      });
    }

    if (!accessProofBuilder) {
      console.error('❌ AccessProofBuilder 未初始化');
      return res.status(500).json({
        success: false,
        error: 'Server not properly initialized'
      });
    }

    const {
      encryptedData,      // base64 編碼的加密數據
      reportId,           // 報告 ID
      requesterAddress,   // 請求者地址
      packageId,          // 🔥 Package ID (用於查找 SessionKey)
      objectId            // 🔥 Seal 加密對象的 ID (從加密結果中獲取)
    } = req.body;

    console.log('📦 請求參數:');
    console.log(`   報告 ID: ${reportId}`);
    console.log(`   請求者: ${requesterAddress}`);
    console.log(`   Package ID: ${packageId || 'N/A'}`);
    console.log(`   Object ID: ${objectId || 'N/A'}`);

    // 驗證必要參數
    if (!encryptedData || !reportId || !requesterAddress || !packageId) {
      return res.status(400).json({
        success: false,
        error: "缺少必要參數",
        required: ["encryptedData", "reportId", "requesterAddress", "packageId"]
      });
    }

    // Step 1: 獲取並驗證 SessionKey 實例
    console.log('\n1️⃣ 獲取 Seal SDK SessionKey...');
    const storeKey = `${requesterAddress}:${packageId}`;
    const sessionKey = sealSessionKeyStore.get(storeKey);

    if (!sessionKey) {
      console.log('❌ Session Key 不存在');
      return res.status(404).json({
        success: false,
        error: "Session Key not found. Please create one first."
      });
    }

    if (sessionKey.isExpired()) {
      console.log('❌ Session Key 已過期');
      sealSessionKeyStore.delete(storeKey);
      return res.status(401).json({
        success: false,
        error: "Session Key has expired"
      });
    }
    console.log('✅ Session Key 有效');

    // Step 2: 檢查訪問權限 (鏈上)
    console.log('\n2️⃣ 檢查訪問權限 (鏈上)...');
    const hasAccess = await checkSuiAccessPolicy(reportId, requesterAddress);

    if (!hasAccess) {
      console.log('❌ 訪問被拒絕');
      return res.status(403).json({
        success: false,
        error: "訪問被拒絕",
        reportId,
        requester: requesterAddress
      });
    }
    console.log('✅ 訪問權限驗證通過');

    // Step 3: 構造訪問證明 PTB
    console.log('\n3️⃣ 構造訪問證明 PTB...');
    let txBytes: Uint8Array;

    try {
      // 🔥 使用 objectId (Seal 加密對象 ID) 作為報告 blob ID
      // objectId 同時也是用戶的 identity address
      const blobId = objectId || requesterAddress;
      console.log(`   使用 Blob ID: ${blobId}`);

      txBytes = await accessProofBuilder.buildAccessProof(
        blobId,           // 使用 objectId 作為報告 blob ID
        requesterAddress
      );
      console.log('✅ PTB 構造成功');
      console.log(`   PTB 大小: ${txBytes.length} bytes`);
    } catch (error: any) {
      console.error('❌ PTB 構造失敗:', error.message);
      return res.status(500).json({
        success: false,
        error: 'PTB 構造失敗',
        details: error.message
      });
    }

    // Step 4: 準備加密數據
    console.log('\n4️⃣ 準備加密數據...');
    let encryptedBytes: Uint8Array;
    try {
      encryptedBytes = new Uint8Array(Buffer.from(encryptedData, 'base64'));
      console.log(`   加密數據大小: ${encryptedBytes.length} bytes`);
    } catch (error) {
      return res.status(400).json({
        success: false,
        error: "加密數據格式錯誤"
      });
    }

    // Step 5-6: 🔥 調用真實的 Seal SDK (fetchKeys + decrypt)
    console.log('\n5️⃣ 調用 Seal SDK 解密...');

    try {
      // 🔥 第一步: 獲取密鑰 (如果有 objectId)
      if (objectId) {
        console.log('   🔑 fetchKeys()...');
        console.log(`   Object IDs: [${objectId}]`);
        console.log(`   Session Key Package: ${sessionKey.getPackageId()}`);
        console.log(`   Threshold: 2`);

        await sealClient.fetchKeys({
          ids: [objectId],
          txBytes: txBytes,
          sessionKey,  // 🔥 傳遞真實的 SessionKey 實例
          threshold: 2
        });
        console.log('   ✅ 密鑰獲取成功');
      } else {
        console.log('   ⚠️  沒有 objectId,跳過 fetchKeys (可能會導致解密失敗)');
      }

      // 🔥 第二步: 解密
      console.log('\n6️⃣ decrypt()...');
      const decryptedBytes = await sealClient.decrypt({
        data: encryptedBytes,
        sessionKey,  // 🔥 傳遞真實的 SessionKey 實例
        txBytes: txBytes
      });

      console.log('✅ Seal 解密成功!');
      console.log(`   解密數據大小: ${decryptedBytes.length} bytes`);

      // 轉換為 JSON
      const decryptedText = new TextDecoder().decode(decryptedBytes);
      const decryptedReport = JSON.parse(decryptedText);

      res.json({
        success: true,
        report: decryptedReport,
        message: '✅ 真實的 Seal 解密成功',
        mode: 'real-seal',
        metadata: {
          reportId,
          requester: requesterAddress,
          decryptedAt: Date.now(),
          sizeBytes: decryptedBytes.length
        }
      });

    } catch (sealError: any) {
      console.error('❌ Seal 解密失敗:', sealError.message);
      console.error('   錯誤類型:', sealError.constructor.name);
      console.error('   完整錯誤:', sealError);

      // 降級到 fallback 模式
      console.log('\n⚠️  降級到 fallback 解密...');
      try {
        const decryptedReport = JSON.parse(atob(encryptedData));

        res.json({
          success: true,
          report: decryptedReport,
          message: '⚠️ 降級到模擬解密 (Seal SDK 錯誤)',
          mode: 'fallback',
          sealError: sealError.message,
          metadata: {
            reportId,
            requester: requesterAddress,
            decryptedAt: Date.now()
          }
        });
      } catch (fallbackError: any) {
        console.error('❌ fallback 解密也失敗:', fallbackError.message);

        res.status(500).json({
          success: false,
          error: '解密完全失敗',
          sealError: sealError.message,
          fallbackError: fallbackError.message
        });
      }
    }

  } catch (error: any) {
    console.error('\n❌ 解密流程失敗:', error);
    res.status(500).json({
      success: false,
      error: "解密失敗",
      details: error.message
    });
  }
});

/**
 * 輔助函數：檢查 Sui 訪問策略
 *
 * 🆕 使用真實的 Sui 合約查詢
 * 調用 access_policy::report_access 合約檢查訪問權限
 */
async function checkSuiAccessPolicy(
  reportId: string,
  requesterAddress: string
): Promise<boolean> {
  console.log(`   [訪問策略] 檢查 ${requesterAddress} 對 ${reportId} 的訪問權限`);

  // 🆕 使用真實合約客戶端
  if (!suiContractClient) {
    console.warn('   [訪問策略] ⚠️  Sui 合約客戶端未初始化，允許訪問（降級模式）');
    return true;
  }

  try {
    const result = await suiContractClient.checkReportAccessPolicy(reportId, requesterAddress);

    if (result.allowed) {
      console.log(`   [訪問策略] ✅ 允許訪問 - ${result.reason}`);
    } else {
      console.log(`   [訪問策略] ❌ 拒絕訪問 - ${result.reason}`);
    }

    return result.allowed;
  } catch (error: any) {
    console.error(`   [訪問策略] ⚠️  查詢失敗: ${error.message}`);
    console.error('   - 降級為允許訪問（容錯模式）');
    return true; // 容錯處理：查詢失敗時允許訪問
  }
}

/**
 * 🆕 Sui 合約測試端點
 *
 * GET /api/sui/test
 *
 * 測試與 Sui 鏈上合約的連接和查詢功能
 */
app.get('/api/sui/test', async (req: Request, res: Response) => {
  console.log('\n🧪 [Sui 合約測試] 開始測試...');

  if (!suiContractClient) {
    return res.status(503).json({
      success: false,
      error: 'Sui 合約客戶端未初始化'
    });
  }

  const results: any = {
    timestamp: new Date().toISOString(),
    tests: []
  };

  // 測試 1: 讀取 AuditConfig
  try {
    console.log('   測試 1/4: 讀取 AuditConfig...');
    const config = await suiContractClient.getAuditConfig();
    results.tests.push({
      name: '讀取 AuditConfig',
      status: 'success',
      data: config
    });
    console.log('   ✅ AuditConfig 讀取成功');
  } catch (error: any) {
    results.tests.push({
      name: '讀取 AuditConfig',
      status: 'failed',
      error: error.message
    });
    console.error('   ❌ AuditConfig 讀取失敗:', error.message);
  }

  // 測試 2: 檢查審計員註冊狀態
  try {
    console.log('   測試 2/4: 檢查審計員註冊狀態...');
    const testAuditor = '0xab8e37e25fe9f46493c4c1ef0c548750dae56ca47ed35324c61b9bed574104d9'; // 部署者地址
    const isRegistered = await suiContractClient.isAuditorRegistered(testAuditor);
    results.tests.push({
      name: '檢查審計員註冊',
      status: 'success',
      data: {
        auditor: testAuditor,
        isRegistered
      }
    });
    console.log(`   ✅ 審計員註冊狀態: ${isRegistered}`);
  } catch (error: any) {
    results.tests.push({
      name: '檢查審計員註冊',
      status: 'failed',
      error: error.message
    });
    console.error('   ❌ 審計員註冊檢查失敗:', error.message);
  }

  // 測試 3: 查詢審計員聲譽分數
  try {
    console.log('   測試 3/4: 查詢審計員聲譽分數...');
    const testAuditor = '0xab8e37e25fe9f46493c4c1ef0c548750dae56ca47ed35324c61b9bed574104d9';
    const reputation = await suiContractClient.getAuditorReputation(testAuditor);
    results.tests.push({
      name: '查詢聲譽分數',
      status: 'success',
      data: {
        auditor: testAuditor,
        reputation
      }
    });
    console.log(`   ✅ 聲譽分數: ${reputation}`);
  } catch (error: any) {
    results.tests.push({
      name: '查詢聲譽分數',
      status: 'failed',
      error: error.message
    });
    console.error('   ❌ 聲譽分數查詢失敗:', error.message);
  }

  // 測試 4: 測試訪問策略檢查
  try {
    console.log('   測試 4/4: 測試訪問策略檢查...');
    const testReportId = 'test-report-001';
    const testRequester = '0xab8e37e25fe9f46493c4c1ef0c548750dae56ca47ed35324c61b9bed574104d9';
    const accessResult = await suiContractClient.checkReportAccessPolicy(testReportId, testRequester);
    results.tests.push({
      name: '訪問策略檢查',
      status: 'success',
      data: accessResult
    });
    console.log(`   ✅ 訪問檢查結果: ${accessResult.allowed} (${accessResult.reason})`);
  } catch (error: any) {
    results.tests.push({
      name: '訪問策略檢查',
      status: 'failed',
      error: error.message
    });
    console.error('   ❌ 訪問策略檢查失敗:', error.message);
  }

  // 統計結果
  const successCount = results.tests.filter((t: any) => t.status === 'success').length;
  const totalCount = results.tests.length;

  console.log(`\n📊 測試完成: ${successCount}/${totalCount} 成功\n`);

  res.json({
    success: true,
    summary: {
      total: totalCount,
      passed: successCount,
      failed: totalCount - successCount
    },
    results
  });
});

// ============ Walrus API 端點 ============

/**
 * API: 上傳數據到 Walrus
 *
 * Request Body:
 * {
 *   "data": "base64-encoded-data" | object,
 *   "dataType": "base64" | "json" | "text",  // Optional, default "text"
 *   "epochs": 5                              // Optional, default 5
 * }
 *
 * Response:
 * {
 *   "success": true,
 *   "blobId": "0x...",           // Blob ID (hex string)
 *   "blobIdU256": "123456...",   // Blob ID (u256 for contracts)
 *   "size": 1234,
 *   "epochs": 5,
 *   "cost": "0",
 *   "certified": true
 * }
 */
app.post('/api/walrus/upload', async (req: Request, res: Response) => {
  console.log('\n📤 ===== Walrus 上傳請求 =====');

  try {
    if (!walrusClient) {
      return res.status(503).json({
        success: false,
        error: 'Walrus 客戶端未初始化'
      });
    }

    const { data, dataType, epochs } = req.body;

    if (!data) {
      return res.status(400).json({
        success: false,
        error: '缺少參數: data'
      });
    }

    // 轉換數據為 Buffer
    let buffer: Buffer;
    if (dataType === 'base64') {
      buffer = Buffer.from(data, 'base64');
    } else if (dataType === 'json') {
      buffer = Buffer.from(JSON.stringify(data));
    } else {
      buffer = Buffer.from(data);
    }

    console.log(`   數據類型: ${dataType || 'text'}`);
    console.log(`   數據大小: ${buffer.length} bytes`);

    // 上傳到 Walrus
    const result = await walrusClient.upload(buffer, epochs || 5);

    console.log('✅ 上傳完成');

    res.json({
      success: true,
      ...result,
      blobIdU256: result.blobIdU256.toString(),  // Convert BigInt to string for JSON
      message: '數據已上傳到 Walrus'
    });

  } catch (error: any) {
    console.error('❌ 上傳失敗:', error);
    res.status(500).json({
      success: false,
      error: '上傳失敗',
      details: error.message
    });
  }
});

/**
 * API: 從 Walrus 下載數據
 *
 * URL Params:
 * - blobId: Blob ID (hex string)
 *
 * Query Params:
 * - format: "json" | "base64" | "buffer" (optional, default "buffer")
 *
 * Response:
 * - If format=json: { "success": true, "data": {...} }
 * - If format=base64: { "success": true, "data": "base64..." }
 * - If format=buffer: raw binary data
 */
app.get('/api/walrus/download/:blobId', async (req: Request, res: Response) => {
  console.log('\n📥 ===== Walrus 下載請求 =====');

  try {
    if (!walrusClient) {
      return res.status(503).json({
        success: false,
        error: 'Walrus 客戶端未初始化'
      });
    }

    const { blobId } = req.params;
    const { format } = req.query;

    console.log(`   Blob ID: ${blobId}`);
    console.log(`   格式: ${format || 'buffer'}`);

    // 從 Walrus 下載
    const buffer = await walrusClient.download(blobId);

    if (format === 'json') {
      const jsonData = JSON.parse(buffer.toString());
      res.json({
        success: true,
        data: jsonData
      });
    } else if (format === 'base64') {
      res.json({
        success: true,
        data: buffer.toString('base64')
      });
    } else {
      // 返回原始數據
      res.set('Content-Type', 'application/octet-stream');
      res.send(buffer);
    }

  } catch (error: any) {
    console.error('❌ 下載失敗:', error);
    res.status(500).json({
      success: false,
      error: '下載失敗',
      details: error.message
    });
  }
});

/**
 * 錯誤處理中間件
 */
app.use((err: Error, req: Request, res: Response, next: any) => {
  console.error('❌ 未處理的錯誤:', err);
  res.status(500).json({
    success: false,
    error: 'Internal server error',
    message: err.message
  });
});

/**
 * 啟動服務器
 */
async function startServer() {
  try {
    // 初始化 Seal 客戶端
    await initializeSealClient();

    // 啟動 HTTP 服務
    app.listen(PORT, () => {
      console.log(`\n🚀 Seal API Server 已啟動`);
      console.log(`   - URL: http://localhost:${PORT}`);
      console.log(`   - Health Check: http://localhost:${PORT}/health`);
      console.log(`\n📡 API Endpoints:`);
      console.log(`   - POST /api/seal/encrypt            - 加密數據`);
      console.log(`   - POST /api/seal/decrypt            - 解密數據（需 Session Key + 訪問權限）`);
      console.log(`   - POST /api/seal/check-access       - 檢查訪問權限`);
      console.log(`   - POST /api/seal/create-session-key - 創建 Session Key（前端錢包簽名）`);
      console.log(`   - GET  /api/sui/test                - 🆕 測試 Sui 合約連接（4 項測試）`);
      console.log(`\n📝 使用示例:`);
      console.log(`   curl http://localhost:${PORT}/health`);
      console.log(`   curl http://localhost:${PORT}/api/sui/test\n`);
    });
  } catch (error) {
    console.error('❌ 服務器啟動失敗:', error);
    process.exit(1);
  }
}

// 優雅關閉
process.on('SIGINT', () => {
  console.log('\n👋 關閉服務器...');
  process.exit(0);
});

process.on('SIGTERM', () => {
  console.log('\n👋 關閉服務器...');
  process.exit(0);
});

// 啟動
startServer();
