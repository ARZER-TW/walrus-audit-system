#!/bin/bash

set -e

echo "🚀 部署 Walrus Audit System 合約到 Sui Testnet"
echo ""

# 檢查 Sui CLI 是否安裝
if ! command -v sui &> /dev/null; then
    echo "❌ 錯誤: Sui CLI 未安裝"
    echo "請參考: https://docs.sui.io/build/install"
    exit 1
fi

echo "✅ Sui CLI 版本: $(sui --version)"

# 檢查是否有活動地址
ACTIVE_ADDRESS=$(sui client active-address 2>/dev/null || true)
if [ -z "$ACTIVE_ADDRESS" ]; then
    echo "❌ 錯誤: 未找到活動的 Sui 地址"
    echo "請執行: sui client new-address ed25519"
    exit 1
fi

echo "✅ 活動地址: $ACTIVE_ADDRESS"

# 檢查當前網路
ACTIVE_ENV=$(sui client active-env 2>/dev/null || echo "未知")
echo "📡 當前網路: $ACTIVE_ENV"

# 檢查餘額 (簡化版,不解析 JSON)
echo ""
echo "💰 檢查餘額..."
sui client gas 2>/dev/null | head -10

echo ""
echo "================================================"
echo "📦 開始部署合約..."
echo "================================================"

# 創建部署輸出目錄
DEPLOY_DIR="$(dirname "$0")"
TIMESTAMP=$(date +%Y%m%d_%H%M%S)
DEPLOY_LOG="${DEPLOY_DIR}/deployment_${TIMESTAMP}.log"

echo "📝 部署日誌將保存到: $DEPLOY_LOG"
echo ""

# 部署 audit_system 合約
echo "1️⃣  部署 audit_system 合約..."
cd "${DEPLOY_DIR}/audit_system"

AUDIT_DEPLOY_OUTPUT=$(sui client publish --gas-budget 200000000 --json 2>&1 | tee -a "$DEPLOY_LOG")

# 提取 Package ID (使用 grep 和 sed)
AUDIT_PACKAGE_ID=$(echo "$AUDIT_DEPLOY_OUTPUT" | grep -o '"packageId":"0x[^"]*"' | head -1 | sed 's/"packageId":"//; s/"//')

if [ -z "$AUDIT_PACKAGE_ID" ]; then
    echo "❌ audit_system 部署失敗!"
    echo "請檢查日誌: $DEPLOY_LOG"
    exit 1
fi

echo "✅ audit_system 部署成功!"
echo "   Package ID: $AUDIT_PACKAGE_ID"

# 提取 AuditRegistry ID
AUDIT_REGISTRY_ID=$(echo "$AUDIT_DEPLOY_OUTPUT" | grep -o '"objectId":"0x[^"]*"' | grep -A5 'AuditRegistry' | head -1 | sed 's/"objectId":"//; s/"//' || echo "")
if [ -n "$AUDIT_REGISTRY_ID" ]; then
    echo "   AuditRegistry ID: $AUDIT_REGISTRY_ID"
fi

# 提取 IncentivePool ID
INCENTIVE_POOL_ID=$(echo "$AUDIT_DEPLOY_OUTPUT" | grep -o '"objectId":"0x[^"]*"' | grep -A5 'IncentivePool' | head -1 | sed 's/"objectId":"//; s/"//' || echo "")
if [ -n "$INCENTIVE_POOL_ID" ]; then
    echo "   IncentivePool ID: $INCENTIVE_POOL_ID"
fi

echo ""

# 部署 access_policy 合約
echo "2️⃣  部署 access_policy 合約..."
cd "${DEPLOY_DIR}/access_policy"

ACCESS_DEPLOY_OUTPUT=$(sui client publish --gas-budget 200000000 --json 2>&1 | tee -a "$DEPLOY_LOG")

# 提取 Package ID
ACCESS_PACKAGE_ID=$(echo "$ACCESS_DEPLOY_OUTPUT" | grep -o '"packageId":"0x[^"]*"' | head -1 | sed 's/"packageId":"//; s/"//')

if [ -z "$ACCESS_PACKAGE_ID" ]; then
    echo "❌ access_policy 部署失敗!"
    echo "請檢查日誌: $DEPLOY_LOG"
    exit 1
fi

echo "✅ access_policy 部署成功!"
echo "   Package ID: $ACCESS_PACKAGE_ID"

echo ""
echo "================================================"
echo "✅ 所有合約部署完成!"
echo "================================================"

# 創建部署配置文件 (使用純 bash,不依賴 jq)
DEPLOY_CONFIG="${DEPLOY_DIR}/deployed-contracts.json"

cat > "$DEPLOY_CONFIG" <<EOF
{
  "network": "$ACTIVE_ENV",
  "deployer": "$ACTIVE_ADDRESS",
  "deployedAt": "$(date -u +%Y-%m-%dT%H:%M:%SZ 2>/dev/null || date +%Y-%m-%dT%H:%M:%SZ)",
  "contracts": {
    "audit_system": {
      "packageId": "$AUDIT_PACKAGE_ID",
      "auditRegistryId": "$AUDIT_REGISTRY_ID",
      "incentivePoolId": "$INCENTIVE_POOL_ID"
    },
    "access_policy": {
      "packageId": "$ACCESS_PACKAGE_ID"
    }
  },
  "systemObjects": {
    "clock": "0x6",
    "systemState": "0x5"
  }
}
EOF

echo ""
echo "📄 部署配置已保存到: $DEPLOY_CONFIG"
echo ""
cat "$DEPLOY_CONFIG"

echo ""
echo "================================================"
echo "🎯 後續步驟"
echo "================================================"
echo ""
echo "1. 更新前端配置:"
echo "   - 編輯 frontend/.env"
echo "   - 設置 VITE_AUDIT_PACKAGE_ID=$AUDIT_PACKAGE_ID"
echo ""
echo "2. 更新審計節點配置:"
echo "   - 編輯 auditor-node/.env"
echo "   - 設置 AUDIT_PACKAGE_ID=$AUDIT_PACKAGE_ID"
echo "   - 設置 AUDIT_REGISTRY_ID=$AUDIT_REGISTRY_ID"
echo ""
echo "3. 查看區塊鏈瀏覽器:"
echo "   https://suiscan.xyz/testnet/object/$AUDIT_PACKAGE_ID"
echo "   https://suiscan.xyz/testnet/object/$ACCESS_PACKAGE_ID"
echo ""
echo "✅ 部署完成! 🚀"
