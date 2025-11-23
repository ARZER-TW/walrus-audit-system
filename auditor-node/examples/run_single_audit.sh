#!/bin/bash
# 執行單次審計示例

set -e

echo "🧪 Walrus 審計節點 - 單次審計示例"
echo "════════════════════════════════════"
echo ""

# 檢查 Seal API 是否運行
echo "1️⃣ 檢查 Seal API 服務..."
if curl -sf http://localhost:3001/health > /dev/null 2>&1; then
    echo "   ✅ Seal API 正在運行"
else
    echo "   ❌ Seal API 未運行"
    echo "   請先啟動 Seal API 服務:"
    echo "   cd ../seal-client && npx tsx seal-api-server.ts"
    exit 1
fi

echo ""
echo "2️⃣ 執行審計..."
echo ""

# 執行審計
cargo run --bin auditor-node -- \
    --blob-id "0xtest123456789abcdef" \
    --seal-api "http://localhost:3001" \
    --auditor-address "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef" \
    --package-id "0x8afa5d31dbaa0a8fb07082692940ca3d56b5e856c5126cb5a3693f0a4de63b82" \
    --log-level info

echo ""
echo "✅ 審計完成！"
