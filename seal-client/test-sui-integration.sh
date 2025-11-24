#!/bin/bash
#
# Sui Contract Integration Test Script
#
# Purpose: Test backend connection and query functionality with Sui on-chain contracts
#

set -e

echo "🧪 Sui Contract Integration Test"
echo "===================="
echo ""

# Check if backend server is running
echo "1️⃣  Checking backend server status..."
if ! curl -s http://localhost:3001/health > /dev/null; then
    echo "   ❌ Backend server is not running"
    echo "   Please execute: cd seal-client && npx tsx seal-api-server.ts"
    exit 1
fi
echo "   ✅ Backend server is running"
echo ""

# Execute Sui contract test
echo "2️⃣  Executing Sui contract test..."
RESPONSE=$(curl -s http://localhost:3001/api/sui/test)

# Check test results
SUCCESS=$(echo "$RESPONSE" | grep -o '"success":[^,]*' | cut -d':' -f2)
PASSED=$(echo "$RESPONSE" | grep -o '"passed":[0-9]*' | cut -d':' -f2)
TOTAL=$(echo "$RESPONSE" | grep -o '"total":[0-9]*' | cut -d':' -f2)

echo "   Test results: $PASSED/$TOTAL passed"
echo ""

# Display detailed results
echo "3️⃣  Test details:"
echo "$RESPONSE" | python3 -m json.tool 2>/dev/null || echo "$RESPONSE"
echo ""

# Determine if test was successful
if [ "$SUCCESS" = "true" ] && [ "$PASSED" = "$TOTAL" ]; then
    echo "✅ All tests passed!"
    echo ""
    echo "Tests included:"
    echo "   - Read AuditConfig (on-chain configuration)"
    echo "   - Check auditor registration status"
    echo "   - Query auditor reputation score"
    echo "   - Test access policy check"
    exit 0
else
    echo "❌ Some tests failed"
    echo "   Success: $PASSED/$TOTAL"
    exit 1
fi
