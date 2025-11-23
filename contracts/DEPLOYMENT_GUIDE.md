# Sui Move 合約部署指南

> **文檔目的**: 提供 Walrus Audit System 智能合約的完整部署流程
>
> **目標網路**: Sui Testnet / Mainnet
>
> **前置需求**: Sui CLI >= 1.20.0

---

## 📑 目錄

1. [環境準備](#1-環境準備)
2. [合約編譯](#2-合約編譯)
3. [部署流程](#3-部署流程)
4. [初始化配置](#4-初始化配置)
5. [驗證部署](#5-驗證部署)
6. [常見問題](#6-常見問題)

---

## 1. 環境準備

### 1.1 安裝 Sui CLI

```bash
# 使用官方腳本安裝
curl --proto '=https' --tlsv1.2 -sSf https://raw.githubusercontent.com/MystenLabs/sui/main/scripts/installer.sh | sh

# 驗證安裝
sui --version
```

### 1.2 配置網路

```bash
# 切換到 testnet
sui client switch --env testnet

# 或連接自定義 RPC
sui client new-env --alias custom --rpc https://your-rpc-url
sui client switch --env custom
```

### 1.3 準備地址和 Gas

```bash
# 查看當前地址
sui client active-address

# 獲取測試網代幣（testnet）
curl --location --request POST 'https://faucet.testnet.sui.io/gas' \
--header 'Content-Type: application/json' \
--data-raw '{
    "FixedAmountRequest": {
        "recipient": "YOUR_ADDRESS"
    }
}'

# 檢查餘額
sui client gas
```

---

## 2. 合約編譯

### 2.1 編譯 access_policy 合約

```bash
cd contracts/access_policy

# 編譯合約
sui move build

# 檢查輸出
# 應該看到：BUILDING access_policy
# 無錯誤（警告可以忽略）
```

**預期輸出**:
```
INCLUDING DEPENDENCY Sui
INCLUDING DEPENDENCY MoveStdlib
BUILDING access_policy
```

### 2.2 編譯 audit_system 合約

```bash
cd ../audit_system

# 編譯合約
sui move build
```

**預期輸出**:
```
INCLUDING DEPENDENCY Sui
INCLUDING DEPENDENCY MoveStdlib
BUILDING audit_system
```

---

## 3. 部署流程

### 3.1 部署 access_policy 合約

**為什麼先部署 access_policy？**
- `audit_system` 可能需要引用 `access_policy` 的類型
- 訪問控制是獨立的基礎設施層

```bash
cd contracts/access_policy

# 部署合約
sui client publish --gas-budget 100000000

# 等待交易確認...
```

**重要輸出解析**:

部署成功後，你會看到類似輸出：

```
╭──────────────────────────────────────────────────────────╮
│ Transaction Data                                          │
├──────────────────────────────────────────────────────────┤
│ Sender: 0xYOUR_ADDRESS                                   │
│ Gas Budget: 100000000 MIST                               │
│ Gas Price: 1000 MIST                                     │
╰──────────────────────────────────────────────────────────╯

╭────────────────────────────────────────────────────────────────────────────╮
│ Transaction Effects                                                         │
├────────────────────────────────────────────────────────────────────────────┤
│ Status: Success                                                            │
│ Created Objects:                                                           │
│  ┌──                                                                       │
│  │ ObjectID: 0xPACKAGE_ID                                                 │  ← 記錄這個！
│  │ Version: 1                                                              │
│  │ Digest: ...                                                             │
│  │ ObjectType: 0x2::package::Publisher                                    │
│  └──                                                                       │
│ Published Objects:                                                         │
│  ┌──                                                                       │
│  │ PackageID: 0xACCESS_POLICY_PACKAGE_ID                                 │  ← 最重要！
│  │ Version: 1                                                              │
│  │ Digest: ...                                                             │
│  │ Modules: report_access                                                 │
│  └──                                                                       │
╰────────────────────────────────────────────────────────────────────────────╯
```

**記錄以下信息**:
```bash
# 保存到環境變量或配置文件
ACCESS_POLICY_PACKAGE_ID=0xACCESS_POLICY_PACKAGE_ID
```

### 3.2 部署 audit_system 合約

```bash
cd ../audit_system

# 部署合約
sui client publish --gas-budget 100000000
```

**記錄輸出**:

```bash
# 保存 Package ID
AUDIT_SYSTEM_PACKAGE_ID=0xAUDIT_SYSTEM_PACKAGE_ID

# 記錄共享對象 ID（AuditConfig）
AUDIT_CONFIG_OBJECT_ID=0xCONFIG_OBJECT_ID
```

**關鍵對象識別**:
- `AuditConfig`: 合約初始化時創建的共享對象
- `Publisher`: 用於後續升級合約的權限對象

---

## 4. 初始化配置

### 4.1 授權審計者

部署後，管理員需要授權審計者地址：

```bash
# 替換以下變量
AUDIT_CONFIG_ID="0xYOUR_CONFIG_OBJECT_ID"
AUDITOR_ADDRESS="0xAUDITOR_ADDRESS"

# 授權審計者
sui client call \
  --package $AUDIT_SYSTEM_PACKAGE_ID \
  --module audit_core \
  --function authorize_auditor \
  --args $AUDIT_CONFIG_ID $AUDITOR_ADDRESS \
  --gas-budget 10000000
```

**預期結果**:
```
Status: Success
```

### 4.2 更新審計參數（可選）

```bash
# 設定審計參數
sui client call \
  --package $AUDIT_SYSTEM_PACKAGE_ID \
  --module audit_core \
  --function update_audit_params \
  --args $AUDIT_CONFIG_ID 20 50 7200000 \
  --gas-budget 10000000

# 參數說明：
# 20: 最少挑戰次數
# 50: 最多挑戰次數
# 7200000: 審計間隔（2 小時，單位：毫秒）
```

### 4.3 創建測試訪問策略

```bash
# 創建測試策略（需要已有 Blob ID）
REPORT_BLOB_ID="0x1234567890abcdef..."  # 32 bytes u256
AUDIT_RECORD_ID="0xRECORD_ID"

sui client call \
  --package $ACCESS_POLICY_PACKAGE_ID \
  --module report_access \
  --function create_policy \
  --args $REPORT_BLOB_ID $AUDIT_RECORD_ID \
  "[]" "[]" "null" "0xCLOCK_ID" \
  --gas-budget 10000000

# 參數說明：
# [] 空讀者列表
# [] 空審計者列表
# null 永不過期
# 0x6 是 Clock 共享對象 ID（固定）
```

---

## 5. 驗證部署

### 5.1 查詢合約對象

```bash
# 查看 audit_system 包信息
sui client object $AUDIT_SYSTEM_PACKAGE_ID

# 查看 AuditConfig 對象
sui client object $AUDIT_CONFIG_OBJECT_ID --json | jq .data.content.fields
```

**預期輸出**:
```json
{
  "admin": "0xYOUR_ADDRESS",
  "authorized_auditors": [],
  "min_challenge_count": 10,
  "max_challenge_count": 100,
  "challenge_interval_ms": 3600000,
  "total_audits": 0,
  "total_blobs_audited": 0
}
```

### 5.2 測試合約調用

創建一個測試審計記錄：

```bash
# 準備測試數據
BLOB_ID="115792089237316195423570985008687907853269984665640564039457584007913129639935"  # u256 示例
BLOB_OBJECT_ID="0x0000000000000000000000000000000000000000000000000000000000000001"
INTEGRITY_HASH="0x$(echo -n 'test_hash' | sha256sum | cut -d' ' -f1)"
PQC_SIGNATURE="0x$(openssl rand -hex 128)"  # Falcon-512 簽名 ~666 bytes

# 提交審計記錄（需要先授權審計者）
sui client call \
  --package $AUDIT_SYSTEM_PACKAGE_ID \
  --module audit_core \
  --function submit_audit_record \
  --args \
    $AUDIT_CONFIG_OBJECT_ID \
    $BLOB_ID \
    $BLOB_OBJECT_ID \
    100 \
    50 \
    48 \
    "[$INTEGRITY_HASH]" \
    "[$PQC_SIGNATURE]" \
    1 \
    "0x6" \
  --gas-budget 20000000

# 參數說明：
# 100: challenge_epoch
# 50: total_challenges
# 48: successful_verifications
# 1: pqc_algorithm (Falcon-512)
# 0x6: Clock 對象 ID
```

### 5.3 查詢事件

```bash
# 查詢 AuditCreated 事件
sui client events \
  --query "{\"MoveEventType\":\"$AUDIT_SYSTEM_PACKAGE_ID::audit_core::AuditCreated\"}" \
  --limit 10

# 查詢 PolicyCreated 事件
sui client events \
  --query "{\"MoveEventType\":\"$ACCESS_POLICY_PACKAGE_ID::report_access::PolicyCreated\"}" \
  --limit 10
```

---

## 6. 常見問題

### Q1: 編譯失敗 - "dependency not found"

**問題**:
```
error: dependency 'Sui' not found
```

**解決方案**:
```bash
# 清理緩存
rm -rf ~/.move

# 重新編譯
sui move build
```

### Q2: Gas 不足

**問題**:
```
InsufficientGas
```

**解決方案**:
```bash
# 增加 gas-budget
sui client publish --gas-budget 200000000

# 或獲取更多測試網代幣
curl --location --request POST 'https://faucet.testnet.sui.io/gas' ...
```

### Q3: 部署後找不到 Package ID

**解決方案**:

部署成功後立即保存輸出：

```bash
# 部署時重定向輸出
sui client publish --gas-budget 100000000 > deployment_output.txt

# 從輸出提取 Package ID
cat deployment_output.txt | grep "PackageID:"
```

或查詢歷史交易：

```bash
# 查詢最近的交易
sui client transactions --address $(sui client active-address) --limit 1
```

### Q4: 合約升級

**重要**: 默認部署的合約是不可變的（immutable）。

如果需要可升級合約，使用 `UpgradeCap`：

```bash
# 部署時會自動創建 UpgradeCap
# 記錄 UpgradeCap Object ID

# 升級合約
sui client upgrade \
  --upgrade-capability $UPGRADE_CAP_ID \
  --gas-budget 100000000
```

### Q5: 如何連接到 Mainnet？

```bash
# 切換到 mainnet
sui client switch --env mainnet

# 確認網路
sui client active-env

# 檢查餘額（mainnet 需要真實 SUI）
sui client gas
```

---

## 📋 部署檢查清單

完成部署後，確認以下項目：

- [ ] `access_policy` 合約成功部署
- [ ] `audit_system` 合約成功部署
- [ ] 記錄兩個 Package ID
- [ ] 記錄 AuditConfig 共享對象 ID
- [ ] 至少授權一個審計者地址
- [ ] 能夠成功調用 `submit_audit_record`
- [ ] 能夠查詢到 `AuditCreated` 事件
- [ ] 將 Package ID 更新到 `.env` 文件
- [ ] 將配置信息提交到 Git（不包括私鑰）

---

## 📝 環境變量模板

創建 `.env.deployment` 文件：

```bash
# Sui Network
SUI_NETWORK=testnet
SUI_RPC_URL=https://fullnode.testnet.sui.io:443

# Deployed Contracts
ACCESS_POLICY_PACKAGE_ID=0x...
AUDIT_SYSTEM_PACKAGE_ID=0x...

# Shared Objects
AUDIT_CONFIG_OBJECT_ID=0x...

# Admin Address
ADMIN_ADDRESS=0x...

# System Objects (Fixed)
CLOCK_OBJECT_ID=0x6
SYSTEM_STATE_OBJECT_ID=0x5
```

---

## 🔗 相關資源

- [Sui Move 文檔](https://docs.sui.io/build/move)
- [Sui CLI 參考](https://docs.sui.io/references/cli)
- [Walrus 文檔](https://docs.walrus.site/)
- [項目主文檔](../README.md)
- [Walrus 鏈上集成指南](./audit_system/docs/walrus_onchain_integration.md)

---

## 🆘 獲得幫助

如果遇到問題：

1. 檢查 Sui CLI 版本：`sui --version`
2. 查看詳細日誌：添加 `--verbose` 標記
3. 驗證網路連接：`sui client objects`
4. 提交 Issue：[GitHub Issues](https://github.com/your-org/walrus-audit-system/issues)

---

**部署成功後，繼續查看 [QUICKSTART.md](../QUICKSTART.md) 運行審計節點！** 🚀
