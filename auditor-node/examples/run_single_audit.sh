#!/bin/bash
# Single Audit Execution Example

set -e

echo "🧪 Walrus Auditor Node - Single Audit Example"
echo "════════════════════════════════════"
echo ""

# Check if Seal API is running
echo "1️⃣ Checking Seal API service..."
if curl -sf http://localhost:3001/health > /dev/null 2>&1; then
    echo "   ✅ Seal API is running"
else
    echo "   ❌ Seal API is not running"
    echo "   Please start Seal API service first:"
    echo "   cd ../seal-client && npx tsx seal-api-server.ts"
    exit 1
fi

echo ""
echo "2️⃣ Executing audit..."
echo ""

# Execute audit
cargo run --bin auditor-node -- \
    --blob-id "0xtest123456789abcdef" \
    --seal-api "http://localhost:3001" \
    --auditor-address "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef" \
    --package-id "0x8afa5d31dbaa0a8fb07082692940ca3d56b5e856c5126cb5a3693f0a4de63b82" \
    --log-level info

echo ""
echo "✅ Audit completed!"
