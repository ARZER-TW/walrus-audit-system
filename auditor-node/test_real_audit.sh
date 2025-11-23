#!/bin/bash
# 測試真實的審計邏輯（使用 IntegrityVerifier）

set -e

echo "🧪 測試修復後的審計系統"
echo "════════════════════════════════════"
echo ""

# 使用真實的 Walrus Testnet Blob ID
BLOB_ID="eRrTusk8yshFQpkemgDnbg0f4-qDo623V2NpeVG1Zcg"

echo "📊 測試參數:"
echo "   Blob ID: $BLOB_ID"
echo "   Walrus Aggregator: https://aggregator.walrus-testnet.walrus.space"
echo ""

echo "🚀 執行單次審計..."
echo ""

# 設置環境變量
export WALRUS_AGGREGATOR_URL="https://aggregator.walrus-testnet.walrus.space"
export PQC_KEYSTORE_PATH="./keys/pqc_keystore"

# 執行審計（不需要 Seal API）
cargo run --release --bin auditor-node -- \
    --blob-id "$BLOB_ID" \
    --log-level info

echo ""
echo "✅ 測試完成！"
