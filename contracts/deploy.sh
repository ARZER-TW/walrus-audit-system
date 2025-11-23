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

# 檢查餘額
echo ""
echo "💰 檢查餘額..."
GAS_OBJECTS=$(sui client gas --json 2>/dev/null || echo "[]")
TOTAL_BALANCE=$(echo "$GAS_OBJECTS" | jq -r '[.[].balance] | add // 0')

# 將餘額從 MIST 轉換為 SUI (1 SUI = 10^9 MIST)
BALANCE_SUI=$(echo "scale=4; $TOTAL_BALANCE / 1000000000" | bc)

echo "   總餘額: $BALANCE_SUI SUI ($TOTAL_BALANCE MIST)"

# 檢查餘額是否足夠 (至少需要 0.1 SUI)
MIN_BALANCE=100000000  # 0.1 SUI in MIST
if [ "$TOTAL_BALANCE" -lt "$MIN_BALANCE" ]; then
    echo "⚠️  警告: 餘額可能不足以部署合約 (建議至少 0.1 SUI)"
    echo "   獲取測試網代幣: https://faucet.testnet.sui.io/"
    read -p "是否繼續部署? (y/N) " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        exit 1
    fi
fi

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

AUDIT_DEPLOY_OUTPUT=$(sui client publish --gas-budget 100000000 --json 2>&1 | tee -a "$DEPLOY_LOG")

# 檢查部署是否成功
if echo "$AUDIT_DEPLOY_OUTPUT" | grep -q '"status":"success"'; then
    echo "✅ audit_system 部署成功!"

    # 提取 Package ID
    AUDIT_PACKAGE_ID=$(echo "$AUDIT_DEPLOY_OUTPUT" | jq -r '.objectChanges[] | select(.type=="published") | .packageId' | head -n1)
    echo "   Package ID: $AUDIT_PACKAGE_ID"

    # 提取 AuditRegistry 共享對象 ID
    AUDIT_REGISTRY_ID=$(echo "$AUDIT_DEPLOY_OUTPUT" | jq -r '.objectChanges[] | select(.objectType | contains("audit_system::auditor_registry::AuditRegistry")) | .objectId' | head -n1)
    if [ -n "$AUDIT_REGISTRY_ID" ]; then
        echo "   AuditRegistry ID: $AUDIT_REGISTRY_ID"
    fi

    # 提取 IncentivePool 共享對象 ID
    INCENTIVE_POOL_ID=$(echo "$AUDIT_DEPLOY_OUTPUT" | jq -r '.objectChanges[] | select(.objectType | contains("audit_system::incentives::IncentivePool")) | .objectId' | head -n1)
    if [ -n "$INCENTIVE_POOL_ID" ]; then
        echo "   IncentivePool ID: $INCENTIVE_POOL_ID"
    fi
else
    echo "❌ audit_system 部署失敗!"
    echo "請檢查日誌: $DEPLOY_LOG"
    exit 1
fi

echo ""

# 部署 access_policy 合約
echo "2️⃣  部署 access_policy 合約..."
cd "${DEPLOY_DIR}/access_policy"

ACCESS_DEPLOY_OUTPUT=$(sui client publish --gas-budget 100000000 --json 2>&1 | tee -a "$DEPLOY_LOG")

# 檢查部署是否成功
if echo "$ACCESS_DEPLOY_OUTPUT" | grep -q '"status":"success"'; then
    echo "✅ access_policy 部署成功!"

    # 提取 Package ID
    ACCESS_PACKAGE_ID=$(echo "$ACCESS_DEPLOY_OUTPUT" | jq -r '.objectChanges[] | select(.type=="published") | .packageId' | head -n1)
    echo "   Package ID: $ACCESS_PACKAGE_ID"
else
    echo "❌ access_policy 部署失敗!"
    echo "請檢查日誌: $DEPLOY_LOG"
    exit 1
fi

echo ""
echo "================================================"
echo "✅ 所有合約部署完成!"
echo "================================================"

# 創建部署配置文件
DEPLOY_CONFIG="${DEPLOY_DIR}/deployed-contracts.json"

cat > "$DEPLOY_CONFIG" <<EOF
{
  "network": "$ACTIVE_ENV",
  "deployer": "$ACTIVE_ADDRESS",
  "deployedAt": "$(date -u +%Y-%m-%dT%H:%M:%SZ)",
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
echo "3. 授權審計者 (需要替換 AUDITOR_ADDRESS):"
echo "   sui client call \\"
echo "     --package $AUDIT_PACKAGE_ID \\"
echo "     --module auditor_registry \\"
echo "     --function register_auditor \\"
echo "     --args $AUDIT_REGISTRY_ID YOUR_AUDITOR_ADDRESS \\"
echo "     --gas-budget 10000000"
echo ""
echo "4. 查看部署詳情:"
echo "   sui client object $AUDIT_PACKAGE_ID"
echo "   sui client object $AUDIT_REGISTRY_ID"
echo ""
echo "✅ 部署完成! 祝你好運! 🚀"
