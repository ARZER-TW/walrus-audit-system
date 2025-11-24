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

# Check balance (simplified version, doesn't parse JSON)
echo ""
echo "💰 Checking balance..."
sui client gas 2>/dev/null | head -10

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

AUDIT_DEPLOY_OUTPUT=$(sui client publish --gas-budget 200000000 --json 2>&1 | tee -a "$DEPLOY_LOG")

# Extract Package ID (using grep and sed)
AUDIT_PACKAGE_ID=$(echo "$AUDIT_DEPLOY_OUTPUT" | grep -o '"packageId":"0x[^"]*"' | head -1 | sed 's/"packageId":"//; s/"//')

if [ -z "$AUDIT_PACKAGE_ID" ]; then
    echo "❌ audit_system deployment failed!"
    echo "Please check log: $DEPLOY_LOG"
    exit 1
fi

echo "✅ audit_system deployed successfully!"
echo "   Package ID: $AUDIT_PACKAGE_ID"

# Extract AuditRegistry ID
AUDIT_REGISTRY_ID=$(echo "$AUDIT_DEPLOY_OUTPUT" | grep -o '"objectId":"0x[^"]*"' | grep -A5 'AuditRegistry' | head -1 | sed 's/"objectId":"//; s/"//' || echo "")
if [ -n "$AUDIT_REGISTRY_ID" ]; then
    echo "   AuditRegistry ID: $AUDIT_REGISTRY_ID"
fi

# Extract IncentivePool ID
INCENTIVE_POOL_ID=$(echo "$AUDIT_DEPLOY_OUTPUT" | grep -o '"objectId":"0x[^"]*"' | grep -A5 'IncentivePool' | head -1 | sed 's/"objectId":"//; s/"//' || echo "")
if [ -n "$INCENTIVE_POOL_ID" ]; then
    echo "   IncentivePool ID: $INCENTIVE_POOL_ID"
fi

echo ""

# Deploy access_policy contract
echo "2️⃣  Deploying access_policy contract..."
cd "${DEPLOY_DIR}/access_policy"

ACCESS_DEPLOY_OUTPUT=$(sui client publish --gas-budget 200000000 --json 2>&1 | tee -a "$DEPLOY_LOG")

# Extract Package ID
ACCESS_PACKAGE_ID=$(echo "$ACCESS_DEPLOY_OUTPUT" | grep -o '"packageId":"0x[^"]*"' | head -1 | sed 's/"packageId":"//; s/"//')

if [ -z "$ACCESS_PACKAGE_ID" ]; then
    echo "❌ access_policy deployment failed!"
    echo "Please check log: $DEPLOY_LOG"
    exit 1
fi

echo "✅ access_policy deployed successfully!"
echo "   Package ID: $ACCESS_PACKAGE_ID"

echo ""
echo "================================================"
echo "✅ All contracts deployed successfully!"
echo "================================================"

# Create deployment configuration file (using pure bash, no jq dependency)
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
echo "3. View on blockchain explorer:"
echo "   https://suiscan.xyz/testnet/object/$AUDIT_PACKAGE_ID"
echo "   https://suiscan.xyz/testnet/object/$ACCESS_PACKAGE_ID"
echo ""
echo "✅ Deployment complete! 🚀"
