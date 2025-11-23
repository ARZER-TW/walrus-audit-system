# Seal Client - Walrus 訪問控制客戶端

TypeScript 客戶端庫，用於與 Walrus Seal 和 Sui 區塊鏈交互，實現審計報告的加密、存儲和訪問控制。

## ⚠️ 重要公告

**請使用正確的 Seal IBE 實現！**

本專案包含兩種實現：
- ✅ **SealIBEClient** (`seal-ibe-client.ts`) - **正確實現**，使用 IBE + 門檻加密
- ❌ **AuditReportSealClient** (`audit-report.ts`) - **已棄用**，錯誤的本地 AES 加密

**遷移指南**: 請參閱 [MIGRATION_GUIDE.md](./MIGRATION_GUIDE.md)

## 🌟 功能特性（正確實現 - SealIBEClient）

### 核心功能

1. **Identity-Based Encryption (IBE)** ✅
   - 使用 Sui 地址作為加密身份
   - 無需預先分發公鑰
   - BF-IBE/BLS12-381 密鑰封裝

2. **門檻加密 (3-out-of-5)** ✅
   - 5 個去中心化金鑰伺服器
   - 需要 3 個伺服器同意才能解密
   - 容錯和抗審查

3. **Sui 鏈上訪問控制** ✅
   - 金鑰伺服器強制執行 Sui 策略
   - 無法在客戶端繞過
   - 可編程的訪問條件

4. **Session Key 機制** ✅
   - 24 小時有效期
   - 時間限制的訪問權限
   - 安全的密鑰輪換

## 📦 安裝

```bash
# 安裝依賴
npm install

# 編譯 TypeScript
npm run build

# 運行 CLI
npm run cli help
```

## 🚀 快速開始

### 1. 環境配置

創建 `.env` 文件：

```bash
# Sui 網絡配置
SUI_RPC_URL=https://fullnode.testnet.sui.io:443

# Walrus 配置
WALRUS_AGGREGATOR_URL=https://aggregator.walrus-testnet.walrus.space

# 訪問策略合約
ACCESS_POLICY_PACKAGE_ID=0x...

# 私鑰（用於簽名交易）
PRIVATE_KEY=0x...
```

### 2. 使用正確的 IBE 加密（推薦）

```bash
# ✅ 使用 IBE + 門檻加密
npm run cli encrypt-ibe report.json --identity 0xAUDITOR_ADDRESS

# 創建 Session Key（24 小時有效）
npm run cli create-session-key --ttl 24h

# 使用 Session Key 解密
npm run cli decrypt-ibe <blob_id> --session-key <key_id>
```

**輸出示例**:
```json
{
  "ciphertext": "base64_encoded_data...",
  "identity": "0xAUDITOR_ADDRESS",
  "threshold": 3,
  "packageId": "0x...",
  "encryptedAt": 1700000000000,
  "kemType": 0,
  "demType": 1
}
```

### 3. 舊版加密方式（已棄用）

```bash
# ❌ 已棄用：本地 AES 加密（不推薦）
npm run cli encrypt report.json --readers 0x123... --auditors 0x789...

# ❌ 已棄用：本地解密
npm run cli decrypt <blob_id> <your_sui_address>
```

**警告**: 舊版方式使用錯誤的加密實現，僅為向後兼容保留。新代碼請使用 `encrypt-ibe` 和 `decrypt-ibe`。

### 4. 直接使用 Walrus

```bash
# 上傳文件
npm run cli upload data.bin

# 下載文件
npm run cli download abc123... --output downloaded.bin
```

### 5. 訪問策略管理

```bash
# 查詢策略
npm run cli policy get 0xpolicy_id...

# 創建策略
npm run cli policy create <blob_id> \
  --readers 0x123... \
  --auditors 0x789...

# 授予權限
npm run cli policy grant <policy_id> <recipient> <type>

# 撤銷策略
npm run cli policy revoke <policy_id>
```

## 💻 編程 API

### ✅ 正確用法（IBE + 門檻加密）

```typescript
import { SealIBEClient, createSealIBEClient } from 'seal-client';

// 1. 初始化客戶端
const client = createSealIBEClient({
  network: 'testnet',
  auditPackageId: '0x...',
  threshold: 3  // 3-out-of-5 門檻
});

// 2. 加密審計報告
const encrypted = await client.encryptAuditReport(
  report,
  'audit0x123...'  // 審計員的 Sui 地址作為 IBE 身份
);

console.log(`已加密: identity=${encrypted.identity}, threshold=${encrypted.threshold}`);

// 3. 創建 Session Key（24 小時有效）
const sessionKey = await client.createSessionKey(24);

// 4. 創建 Sui 交易證明訪問權限
const ptb = new TransactionBlock();
ptb.moveCall({
  target: `${packageId}::access_policy::prove_access`,
  arguments: [
    ptb.object(policyId),
    ptb.pure(requesterAddress)
  ]
});
const ptbBytes = await ptb.build({ client: suiClient });

// 5. 解密報告（金鑰伺服器驗證 Sui 策略）
const decrypted = await client.decryptAuditReport(
  encrypted,
  sessionKey,
  ptbBytes
);

console.log(`已解密: ${decrypted.blob_id}`);
```

### ❌ 舊版用法（已棄用）

```typescript
import { AuditReportSealClient } from 'seal-client';

// ⚠️ DEPRECATED: 不要在新代碼中使用
const client = new AuditReportSealClient(...);

// 這會輸出警告訊息
// ⚠️⚠️⚠️ DEPRECATED WARNING ⚠️⚠️⚠️
```

詳見 [MIGRATION_GUIDE.md](./MIGRATION_GUIDE.md) 了解如何遷移。

### 訪問控制

```typescript
// 檢查訪問權限
const hasAccess = await client.checkAccessPermission(
  blobId,
  userAddress
);

if (!hasAccess) {
  throw new Error('訪問被拒絕');
}

// 授予審計員訪問權限
await client.grantAuditorAccess(policyId, auditorAddress);

// 授予讀者訪問權限
await client.grantReaderAccess(policyId, readerAddress);

// 撤銷策略
await client.revokePolicy(policyId);
```

### 直接 Walrus 操作

```typescript
const operator = client.getSealOperator();

// 上傳
const result = await operator.upload(Buffer.from('data'));
console.log(`Blob ID: ${result.blobId}`);

// 下載
const data = await operator.download(blobId);

// 檢查存在性
const exists = await operator.exists(blobId);
```

## 🏗️ 架構設計（正確實現）

### 加密工作流程（IBE + 門檻加密）

```
審計報告 (JSON)
    ↓
[1. 序列化為 Bytes]
    ↓
[2. Seal IBE 加密]
    - 使用 Sui 地址作為 IBE 身份
    - BF-IBE/BLS12-381 密鑰封裝 (kemType=0)
    - AES-256-GCM 數據加密 (demType=1)
    ↓
[3. 密鑰自動分片到 5 個金鑰伺服器]
    - Mysten #1, #2
    - Ruby Nodes
    - NodeInfra
    - Studio Mirai
    ↓
[4. 上傳到 Walrus (可選)]
    ↓
[5. 創建 Sui 訪問策略]
```

### 解密工作流程（金鑰伺服器驗證）

```
請求者 + Session Key
    ↓
[1. 創建 Sui 交易證明訪問權限]
    ↓
[2. 發送解密請求到金鑰伺服器]
    - 包含 Session Key
    - 包含 Sui 交易證明
    ↓
[3. 金鑰伺服器查詢 Sui 鏈上策略]
    - 驗證請求者是否有權限
    - 驗證 Session Key 是否有效
    ↓
[4. 至少 3 個伺服器返回密鑰分片]
    ↓
[5. 客戶端重建完整密鑰並解密]
    ↓
[3. 從密鑰服務器獲取密鑰]
    ↓
[4. AES-256-GCM 解密]
    ↓
[5. PQC 簽名驗證]
    ↓
審計報告 (JSON)
```

### 安全層級

| 層級 | 技術 | 功能 |
|------|------|------|
| **應用層** | Dilithium3 PQC | 長期簽名驗證 |
| **加密層** | AES-256-GCM | 數據保密性 |
| **訪問控制** | Sui 智能合約 | 權限管理 |
| **密鑰管理** | Threshold Encryption | 去中心化密鑰 |
| **存儲層** | Walrus | 持久化存儲 |

## 🔐 安全考量

### 已實現

1. ✅ **端到端加密**: AES-256-GCM 提供機密性和完整性
2. ✅ **訪問控制**: 基於 Sui 智能合約的權限管理
3. ✅ **PQC 簽名**: Dilithium3 提供量子抗性認證
4. ✅ **時間限制**: 策略自動過期機制

### 限制（MVP 版本）

1. ⚠️ **Threshold 加密未完全實現**
   - 當前版本: 本地對稱加密
   - 生產版本: 2-out-of-3 密鑰服務器
   - 安全影響: 密鑰管理集中化

2. ⚠️ **PQC 驗證通過 FFI**
   - 需要調用 Rust auditor-node 模塊
   - 當前: 占位實現
   - TODO: 實現 WASM 或子進程調用

3. ⚠️ **訪問策略執行**
   - 依賴客戶端檢查
   - 密鑰服務器應強制執行策略
   - TODO: 服務端權限驗證

## 📝 API 參考

### AuditReportSealClient

#### 構造函數

```typescript
constructor(
  suiRpcUrl: string,
  walrusAggregatorUrl: string,
  accessPolicyPackageId: string,
  privateKey?: string
)
```

#### 方法

**`encryptAndUpload(report, publicKey?, options?)`**
- 加密並上傳審計報告
- 返回: `Promise<EncryptedReportMetadata>`

**`downloadAndDecrypt(blobId, requesterAddress, publicKey?)`**
- 下載並解密審計報告
- 返回: `Promise<AuditReport>`

**`checkAccessPermission(blobId, requesterAddress)`**
- 檢查訪問權限
- 返回: `Promise<boolean>`

**`grantAuditorAccess(policyId, auditorAddress)`**
- 授予審計員訪問權限
- 返回: `Promise<string>` (交易 digest)

**`grantReaderAccess(policyId, readerAddress)`**
- 授予讀者訪問權限
- 返回: `Promise<string>`

**`revokePolicy(policyId)`**
- 撤銷訪問策略
- 返回: `Promise<string>`

### SealOperator

**`upload(data: Buffer)`**
- 上傳數據到 Walrus
- 返回: `Promise<UploadResult>`

**`download(blobId: string)`**
- 從 Walrus 下載數據
- 返回: `Promise<Buffer>`

**`exists(blobId: string)`**
- 檢查 Blob 是否存在
- 返回: `Promise<boolean>`

### PolicyManager

**`createPolicy(blobId, allowedReaders, allowedAuditors, expiryTimestamp)`**
- 創建訪問策略
- 返回: `Promise<string>`

**`getPolicy(policyId)`**
- 查詢策略詳情
- 返回: `Promise<AccessPolicy | null>`

**`grantAccess(policyId, recipient, accessType)`**
- 授予訪問權限
- 返回: `Promise<string>`

**`revokePolicy(policyId)`**
- 撤銷策略
- 返回: `Promise<string>`

## 🧪 測試

```bash
# 運行所有測試
npm test

# 運行特定測試
npm test -- audit-report.test.ts

# 生成覆蓋率報告
npm test -- --coverage
```

## 🛠️ 開發

```bash
# 監聽模式（自動重新編譯）
npm run watch

# 代碼檢查
npm run lint

# 代碼格式化
npm run format
```

## 📂 項目結構

```
seal-client/
├── src/
│   ├── index.ts          # 主入口
│   ├── types.ts          # 類型定義
│   ├── client.ts         # Sui 客戶端
│   ├── seal.ts           # Walrus 操作
│   ├── policy.ts         # 訪問策略管理
│   ├── audit-report.ts   # 審計報告客戶端
│   └── cli.ts            # 命令行工具
├── tests/                # 測試文件
├── package.json
├── tsconfig.json
└── README.md
```

## 🔗 相關資源

- [Walrus 文檔](https://docs.walrus.site)
- [Sui 文檔](https://docs.sui.io)
- [Dilithium3 規範](https://pq-crystals.org/dilithium/)
- [NIST PQC 標準](https://csrc.nist.gov/Projects/post-quantum-cryptography)

## 📄 授權

MIT License

## 🤝 貢獻

歡迎提交 Issue 和 Pull Request！

## ⚠️ 免責聲明

這是 MVP 版本，用於演示和測試目的。生產環境使用前請：

1. 完整實現 Threshold 加密
2. 添加密鑰服務器端驗證
3. 進行完整的安全審計
4. 實現 PQC 驗證 FFI
5. 添加錯誤恢復機制

---

**開發者**: Walrus Audit Team
**最後更新**: 2025-11-16
