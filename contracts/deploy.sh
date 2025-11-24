#!/bin/bash

set -e

echo "🚀 Deploy Walrus Audit System Contracts to Sui Testnet"
echo ""

# Check if Sui CLI is installed
if ! command -v sui &> /dev/null; then
    echo "❌ Error: Sui CLI not installed"
    echo "Please refer to: https://docs.sui.io/build/install"
    exit 1
fi

echo "✅ Sui CLI version: $(sui --version)"

# Check if there's an active address
ACTIVE_ADDRESS=$(sui client active-address 2>/dev/null || true)
if [ -z "$ACTIVE_ADDRESS" ]; then
    echo "❌ Error: No active Sui address found"
    echo "Please execute: sui client new-address ed25519"
    exit 1
fi

echo "✅ Active address: $ACTIVE_ADDRESS"

# Check current network
ACTIVE_ENV=$(sui client active-env 2>/dev/null || echo "unknown")
echo "📡 Current network: $ACTIVE_ENV"

# Check balance
echo ""
echo "💰 Checking balance..."
GAS_OBJECTS=$(sui client gas --json 2>/dev/null || echo "[]")
TOTAL_BALANCE=$(echo "$GAS_OBJECTS" | jq -r '[.[].balance] | add // 0')

# Convert balance from MIST to SUI (1 SUI = 10^9 MIST)
BALANCE_SUI=$(echo "scale=4; $TOTAL_BALANCE / 1000000000" | bc)

echo "   Total balance: $BALANCE_SUI SUI ($TOTAL_BALANCE MIST)"

# Check if balance is sufficient (at least 0.1 SUI required)
MIN_BALANCE=100000000  # 0.1 SUI in MIST
if [ "$TOTAL_BALANCE" -lt "$MIN_BALANCE" ]; then
    echo "⚠️  Warning: Balance may be insufficient for contract deployment (recommend at least 0.1 SUI)"
    echo "   Get testnet tokens: https://faucet.testnet.sui.io/"
    read -p "Continue with deployment? (y/N) " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        exit 1
    fi
fi

echo ""
echo "================================================"
echo "📦 Starting contract deployment..."
echo "================================================"

# Create deployment output directory
DEPLOY_DIR="$(dirname "$0")"
TIMESTAMP=$(date +%Y%m%d_%H%M%S)
DEPLOY_LOG="${DEPLOY_DIR}/deployment_${TIMESTAMP}.log"

echo "📝 Deployment log will be saved to: $DEPLOY_LOG"
echo ""

# Deploy audit_system contract
echo "1️⃣  Deploying audit_system contract..."
cd "${DEPLOY_DIR}/audit_system"

AUDIT_DEPLOY_OUTPUT=$(sui client publish --gas-budget 100000000 --json 2>&1 | tee -a "$DEPLOY_LOG")

# Check if deployment was successful
if echo "$AUDIT_DEPLOY_OUTPUT" | grep -q '"status":"success"'; then
    echo "✅ audit_system deployed successfully!"

    # Extract Package ID
    AUDIT_PACKAGE_ID=$(echo "$AUDIT_DEPLOY_OUTPUT" | jq -r '.objectChanges[] | select(.type=="published") | .packageId' | head -n1)
    echo "   Package ID: $AUDIT_PACKAGE_ID"

    # Extract AuditRegistry shared object ID
    AUDIT_REGISTRY_ID=$(echo "$AUDIT_DEPLOY_OUTPUT" | jq -r '.objectChanges[] | select(.objectType | contains("audit_system::auditor_registry::AuditRegistry")) | .objectId' | head -n1)
    if [ -n "$AUDIT_REGISTRY_ID" ]; then
        echo "   AuditRegistry ID: $AUDIT_REGISTRY_ID"
    fi

    # Extract IncentivePool shared object ID
    INCENTIVE_POOL_ID=$(echo "$AUDIT_DEPLOY_OUTPUT" | jq -r '.objectChanges[] | select(.objectType | contains("audit_system::incentives::IncentivePool")) | .objectId' | head -n1)
    if [ -n "$INCENTIVE_POOL_ID" ]; then
        echo "   IncentivePool ID: $INCENTIVE_POOL_ID"
    fi
else
    echo "❌ audit_system deployment failed!"
    echo "Please check log: $DEPLOY_LOG"
    exit 1
fi

echo ""

# Deploy access_policy contract
echo "2️⃣  Deploying access_policy contract..."
cd "${DEPLOY_DIR}/access_policy"

ACCESS_DEPLOY_OUTPUT=$(sui client publish --gas-budget 100000000 --json 2>&1 | tee -a "$DEPLOY_LOG")

# Check if deployment was successful
if echo "$ACCESS_DEPLOY_OUTPUT" | grep -q '"status":"success"'; then
    echo "✅ access_policy deployed successfully!"

    # Extract Package ID
    ACCESS_PACKAGE_ID=$(echo "$ACCESS_DEPLOY_OUTPUT" | jq -r '.objectChanges[] | select(.type=="published") | .packageId' | head -n1)
    echo "   Package ID: $ACCESS_PACKAGE_ID"
else
    echo "❌ access_policy deployment failed!"
    echo "Please check log: $DEPLOY_LOG"
    exit 1
fi

echo ""
echo "================================================"
echo "✅ All contracts deployed successfully!"
echo "================================================"

# Create deployment configuration file
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
echo "📄 Deployment configuration saved to: $DEPLOY_CONFIG"
echo ""
cat "$DEPLOY_CONFIG"

echo ""
echo "================================================"
echo "🎯 Next Steps"
echo "================================================"
echo ""
echo "1. Update frontend configuration:"
echo "   - Edit frontend/.env"
echo "   - Set VITE_AUDIT_PACKAGE_ID=$AUDIT_PACKAGE_ID"
echo ""
echo "2. Update auditor node configuration:"
echo "   - Edit auditor-node/.env"
echo "   - Set AUDIT_PACKAGE_ID=$AUDIT_PACKAGE_ID"
echo "   - Set AUDIT_REGISTRY_ID=$AUDIT_REGISTRY_ID"
echo ""
echo "3. Authorize auditor (replace AUDITOR_ADDRESS):"
echo "   sui client call \\"
echo "     --package $AUDIT_PACKAGE_ID \\"
echo "     --module auditor_registry \\"
echo "     --function register_auditor \\"
echo "     --args $AUDIT_REGISTRY_ID YOUR_AUDITOR_ADDRESS \\"
echo "     --gas-budget 10000000"
echo ""
echo "4. View deployment details:"
echo "   sui client object $AUDIT_PACKAGE_ID"
echo "   sui client object $AUDIT_REGISTRY_ID"
echo ""
echo "✅ Deployment complete! Good luck! 🚀"
