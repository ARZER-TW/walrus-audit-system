#!/bin/bash
#
# Sui 合約整合測試腳本
#
# 用途: 測試後端與 Sui 鏈上合約的連接和查詢功能
#

set -e

echo "🧪 Sui 合約整合測試"
echo "===================="
echo ""

# 檢查後端服務器是否運行
echo "1️⃣  檢查後端服務器狀態..."
if ! curl -s http://localhost:3001/health > /dev/null; then
    echo "   ❌ 後端服務器未運行"
    echo "   請執行: cd seal-client && npx tsx seal-api-server.ts"
    exit 1
fi
echo "   ✅ 後端服務器運行中"
echo ""

# 執行 Sui 合約測試
echo "2️⃣  執行 Sui 合約測試..."
RESPONSE=$(curl -s http://localhost:3001/api/sui/test)

# 檢查測試結果
SUCCESS=$(echo "$RESPONSE" | grep -o '"success":[^,]*' | cut -d':' -f2)
PASSED=$(echo "$RESPONSE" | grep -o '"passed":[0-9]*' | cut -d':' -f2)
TOTAL=$(echo "$RESPONSE" | grep -o '"total":[0-9]*' | cut -d':' -f2)

echo "   測試結果: $PASSED/$TOTAL 通過"
echo ""

# 顯示詳細結果
echo "3️⃣  測試詳情:"
echo "$RESPONSE" | python3 -m json.tool 2>/dev/null || echo "$RESPONSE"
echo ""

# 判斷測試是否成功
if [ "$SUCCESS" = "true" ] && [ "$PASSED" = "$TOTAL" ]; then
    echo "✅ 所有測試通過!"
    echo ""
    echo "測試包含:"
    echo "   - 讀取 AuditConfig (鏈上配置)"
    echo "   - 檢查審計員註冊狀態"
    echo "   - 查詢審計員聲譽分數"
    echo "   - 測試訪問策略檢查"
    exit 0
else
    echo "❌ 部分測試失敗"
    echo "   成功: $PASSED/$TOTAL"
    exit 1
fi
