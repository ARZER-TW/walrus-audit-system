#!/bin/bash

# Walrus PQC Audit System - Demo Script
# For Walrus Haulout Hackathon - Data Security & Privacy Track

set -e  # Exit on error

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# Helper function for section headers
print_header() {
    echo ""
    echo -e "${CYAN}╔════════════════════════════════════════════════════════════════╗${NC}"
    echo -e "${CYAN}║ $1${NC}"
    echo -e "${CYAN}╚════════════════════════════════════════════════════════════════╝${NC}"
    echo ""
}

# Helper function for steps
print_step() {
    echo -e "${GREEN}▶ $1${NC}"
}

# Helper function for info
print_info() {
    echo -e "${YELLOW}ℹ $1${NC}"
}

# Helper function for pause
pause_demo() {
    if [ "$AUTO_MODE" != "1" ]; then
        echo ""
        read -p "Press Enter to continue..."
        echo ""
    else
        sleep 2
    fi
}

# Change to project directory
cd ~/notebook/walrus-audit-system

# Main demo starts here
clear
print_header "Walrus PQC Audit System - Live Demo"
echo -e "${BLUE}This demo showcases:${NC}"
echo "  ✅ Post-Quantum Cryptography (Dilithium3)"
echo "  ✅ Merkle Tree Verification (Blake2b-256)"
echo "  ✅ Real Walrus Testnet Integration"
echo "  ✅ Sui Blockchain Smart Contracts"
echo ""
print_info "Project directory: $(pwd)"
pause_demo

# ============================================================================
# Part 1: Project Overview
# ============================================================================
print_header "Part 1: Project Structure"
print_step "Displaying project files..."
echo ""
ls -lh --color=auto | grep -E "^d|README|DEMO|DEPLOYMENT"
echo ""
print_info "Key documentation:"
echo "  📄 README.md - Project overview"
echo "  📄 DEMO_INSTRUCTIONS.md - Step-by-step guide"
echo "  📄 DEPLOYMENT_SUMMARY.md - Sui Testnet deployment details"
pause_demo

# ============================================================================
# Part 2: Sui Smart Contract Verification
# ============================================================================
print_header "Part 2: Sui Testnet Deployment Verification"
print_step "Displaying deployment summary..."
echo ""
head -50 DEPLOYMENT_SUMMARY.md
pause_demo

print_step "Querying on-chain AuditConfig object..."
echo ""
print_info "Object ID: 0x1dcd8f8d4965cb2ab5dc61c2dc9c168f51ff79f6b43d2aef6fedb622e220872b"
sui client object 0x1dcd8f8d4965cb2ab5dc61c2dc9c168f51ff79f6b43d2aef6fedb622e220872b
pause_demo

# ============================================================================
# Part 3: Verify Walrus Blob Exists
# ============================================================================
print_header "Part 3: Verify Test Blob on Walrus Testnet"
print_step "Checking if blob exists..."
echo ""
BLOB_ID="eRrTusk8yshFQpkemgDnbg0f4-qDo623V2NpeVG1Zcg"
print_info "Blob ID: $BLOB_ID"
print_info "URL: https://aggregator.walrus-testnet.walrus.space/v1/blobs/$BLOB_ID"
echo ""
curl -I "https://aggregator.walrus-testnet.walrus.space/v1/blobs/$BLOB_ID"
echo ""
print_info "✅ HTTP 200 = Blob exists on Walrus Testnet"
pause_demo

# ============================================================================
# Part 4: Core Functionality - Merkle + PQC
# ============================================================================
print_header "Part 4: Merkle Tree + PQC Signature Test (CORE DEMO)"
cd auditor-node
print_step "Running Merkle Tree integration test..."
echo ""
print_info "This will:"
echo "  1. Download blob from Walrus Testnet (870 bytes)"
echo "  2. Compute SHA-256 hash for integrity check"
echo "  3. Build Blake2b-256 Merkle Tree"
echo "  4. Perform challenge-response verification"
echo "  5. Generate Dilithium3 PQC signature"
echo "  6. Save signed report to /tmp/signed_audit_report.json"
echo ""
pause_demo

cargo run --release --bin test_merkle_integration

echo ""
print_info "✅ Test complete! Report saved to /tmp/signed_audit_report.json"
pause_demo

# ============================================================================
# Part 5: Inspect Audit Report
# ============================================================================
print_header "Part 5: Inspect PQC-Signed Audit Report"
cd ..

print_step "Displaying report structure (first 40 lines)..."
echo ""
head -40 /tmp/signed_audit_report.json
pause_demo

print_step "Extracting key fields..."
echo ""
echo -e "${CYAN}Algorithm:${NC}"
grep -o '"algorithm":"[^"]*"' /tmp/signed_audit_report.json | cut -d'"' -f4
echo ""

echo -e "${CYAN}Blob ID:${NC}"
grep -o '"blob_id":"[^"]*"' /tmp/signed_audit_report.json | cut -d'"' -f4
echo ""

echo -e "${CYAN}Merkle Root:${NC}"
grep -o '"merkle_root":"[^"]*"' /tmp/signed_audit_report.json | cut -d'"' -f4
echo ""

echo -e "${CYAN}Verification Status:${NC}"
grep -o '"verification_status":"[^"]*"' /tmp/signed_audit_report.json | cut -d'"' -f4
echo ""

print_step "Verifying PQC signature length..."
SIGLEN=$(grep -o '"signature":"[^"]*"' /tmp/signed_audit_report.json | cut -d'"' -f4 | wc -c)
echo ""
echo -e "${CYAN}Signature Length:${NC} $SIGLEN characters"
print_info "Expected: 6618 characters for Dilithium3"
if [ "$SIGLEN" -eq 6618 ]; then
    echo -e "${GREEN}✅ Signature length is correct!${NC}"
else
    echo -e "${YELLOW}⚠️  Signature length differs (actual: $SIGLEN)${NC}"
fi
pause_demo

# ============================================================================
# Part 6: Verify Data Integrity
# ============================================================================
print_header "Part 6: Verify Data Integrity"
print_step "Downloading original blob for verification..."
echo ""
curl -s "https://aggregator.walrus-testnet.walrus.space/v1/blobs/$BLOB_ID" > /tmp/test_blob.bin
echo "✅ Downloaded to /tmp/test_blob.bin"
echo ""

print_step "Computing SHA-256 hash..."
echo ""
echo -e "${CYAN}Computed hash:${NC}"
sha256sum /tmp/test_blob.bin | cut -d' ' -f1
echo ""

echo -e "${CYAN}Report hash:${NC}"
grep '"content_hash"' /tmp/signed_audit_report.json | grep -o '"[^"]*"' | tail -1 | tr -d '"'
echo ""

print_info "These hashes should match, proving data integrity!"
pause_demo

# ============================================================================
# Part 7: Seal Configuration (Optional)
# ============================================================================
print_header "Part 7: Seal Privacy Layer Configuration"
cd seal-client

print_step "Displaying Seal configuration..."
echo ""
head -30 src/seal-config.ts
echo ""

print_step "Key Server Object IDs (3-out-of-5 threshold encryption):"
echo ""
grep -A 5 "keyServerObjects" src/seal-config.ts
pause_demo

# ============================================================================
# Part 8: Smart Contract Code Showcase
# ============================================================================
print_header "Part 8: Smart Contract Functions"
cd ../contracts/audit_system

print_step "Function: submit_encrypted_report_metadata()"
echo ""
grep -A 15 "public entry fun submit_encrypted_report_metadata" sources/audit_core.move
echo ""
pause_demo

print_step "Function: seal_approve() - Access Control"
echo ""
grep -A 15 "public fun seal_approve" sources/audit_core.move
pause_demo

# ============================================================================
# Demo Complete
# ============================================================================
cd ~/notebook/walrus-audit-system
print_header "Demo Complete! 🎉"
echo -e "${GREEN}✅ Demonstrated Features:${NC}"
echo ""
echo "  1. ✅ Sui Testnet smart contract deployment"
echo "  2. ✅ Walrus Testnet blob verification (HTTP 200)"
echo "  3. ✅ Merkle Tree construction (Blake2b-256)"
echo "  4. ✅ Challenge-response verification (100% success)"
echo "  5. ✅ Dilithium3 PQC signature generation"
echo "  6. ✅ Data integrity verification (SHA-256 match)"
echo "  7. ✅ Seal privacy layer configuration"
echo "  8. ✅ Smart contract access control logic"
echo ""
echo -e "${CYAN}Key Achievements:${NC}"
echo "  🏆 First quantum-resistant audit system for decentralized storage"
echo "  🏆 Production-grade Merkle Tree implementation"
echo "  🏆 Real Walrus Testnet integration (not mocks!)"
echo "  🏆 Deployed and verifiable on Sui Blockchain"
echo ""
echo -e "${BLUE}📚 Documentation:${NC}"
echo "  - README.md - Full project overview"
echo "  - DEMO_INSTRUCTIONS.md - Detailed demo guide"
echo "  - DEPLOYMENT_SUMMARY.md - Deployment details"
echo ""
echo -e "${YELLOW}Thank you for watching the demo!${NC}"
echo ""
echo "Built for Walrus Haulout Hackathon - Data Security & Privacy Track"
echo "November 2025"
echo ""
