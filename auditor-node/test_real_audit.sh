#!/bin/bash
# Test Real Audit Logic (using IntegrityVerifier)

set -e

echo "🧪 Testing Fixed Audit System"
echo "════════════════════════════════════"
echo ""

# Use real Walrus Testnet Blob ID
BLOB_ID="eRrTusk8yshFQpkemgDnbg0f4-qDo623V2NpeVG1Zcg"

echo "📊 Test Parameters:"
echo "   Blob ID: $BLOB_ID"
echo "   Walrus Aggregator: https://aggregator.walrus-testnet.walrus.space"
echo ""

echo "🚀 Executing single audit..."
echo ""

# Set environment variables
export WALRUS_AGGREGATOR_URL="https://aggregator.walrus-testnet.walrus.space"
export PQC_KEYSTORE_PATH="./keys/pqc_keystore"

# Execute audit (no Seal API needed)
cargo run --release --bin auditor-node -- \
    --blob-id "$BLOB_ID" \
    --log-level info

echo ""
echo "✅ Test completed!"
