# Sui Move Contract Deployment Guide

> **Document Purpose**: Provide complete deployment process for Walrus Audit System smart contracts
>
> **Target Network**: Sui Testnet / Mainnet
>
> **Prerequisites**: Sui CLI >= 1.20.0

---

## 📑 Table of Contents

1. [Environment Setup](#1-environment-setup)
2. [Contract Compilation](#2-contract-compilation)
3. [Deployment Process](#3-deployment-process)
4. [Initialization Configuration](#4-initialization-configuration)
5. [Deployment Verification](#5-deployment-verification)
6. [Common Issues](#6-common-issues)

---

## 1. Environment Setup

### 1.1 Install Sui CLI

```bash
# Install using official script
curl --proto '=https' --tlsv1.2 -sSf https://raw.githubusercontent.com/MystenLabs/sui/main/scripts/installer.sh | sh

# Verify installation
sui --version
```

### 1.2 Configure Network

```bash
# Switch to testnet
sui client switch --env testnet

# Or connect to custom RPC
sui client new-env --alias custom --rpc https://your-rpc-url
sui client switch --env custom
```

### 1.3 Prepare Address and Gas

```bash
# View current address
sui client active-address

# Get testnet tokens
curl --location --request POST 'https://faucet.testnet.sui.io/gas' \
--header 'Content-Type: application/json' \
--data-raw '{
    "FixedAmountRequest": {
        "recipient": "YOUR_ADDRESS"
    }
}'

# Check balance
sui client gas
```

---

## 2. Contract Compilation

### 2.1 Compile access_policy Contract

```bash
cd contracts/access_policy

# Compile contract
sui move build

# Check output
# Should see: BUILDING access_policy
# No errors (warnings can be ignored)
```

**Expected Output**:
```
INCLUDING DEPENDENCY Sui
INCLUDING DEPENDENCY MoveStdlib
BUILDING access_policy
```

### 2.2 Compile audit_system Contract

```bash
cd ../audit_system

# Compile contract
sui move build
```

**Expected Output**:
```
INCLUDING DEPENDENCY Sui
INCLUDING DEPENDENCY MoveStdlib
BUILDING audit_system
```

---

## 3. Deployment Process

### 3.1 Deploy access_policy Contract

**Why deploy access_policy first?**
- `audit_system` may need to reference `access_policy` types
- Access control is an independent infrastructure layer

```bash
cd contracts/access_policy

# Deploy contract
sui client publish --gas-budget 100000000

# Wait for transaction confirmation...
```

**Important Output Explanation**:

After successful deployment, you will see output similar to:

```
╭──────────────────────────────────────────────────────────╮
│ Transaction Data                                          │
├──────────────────────────────────────────────────────────┤
│ Sender: 0xYOUR_ADDRESS                                   │
│ Gas Budget: 100000000 MIST                               │
│ Gas Price: 1000 MIST                                     │
╰──────────────────────────────────────────────────────────╯

╭────────────────────────────────────────────────────────────────────────────╮
│ Transaction Effects                                                         │
├────────────────────────────────────────────────────────────────────────────┤
│ Status: Success                                                            │
│ Created Objects:                                                           │
│  ┌──                                                                       │
│  │ ObjectID: 0xPACKAGE_ID                                                 │  ← Record this!
│  │ Version: 1                                                              │
│  │ Digest: ...                                                             │
│  │ ObjectType: 0x2::package::Publisher                                    │
│  └──                                                                       │
│ Published Objects:                                                         │
│  ┌──                                                                       │
│  │ PackageID: 0xACCESS_POLICY_PACKAGE_ID                                 │  ← Most important!
│  │ Version: 1                                                              │
│  │ Digest: ...                                                             │
│  │ Modules: report_access                                                 │
│  └──                                                                       │
╰────────────────────────────────────────────────────────────────────────────╯
```

**Record the following information**:
```bash
# Save to environment variables or configuration file
ACCESS_POLICY_PACKAGE_ID=0xACCESS_POLICY_PACKAGE_ID
```

### 3.2 Deploy audit_system Contract

```bash
cd ../audit_system

# Deploy contract
sui client publish --gas-budget 100000000
```

**Record Output**:

```bash
# Save Package ID
AUDIT_SYSTEM_PACKAGE_ID=0xAUDIT_SYSTEM_PACKAGE_ID

# Record shared object ID (AuditConfig)
AUDIT_CONFIG_OBJECT_ID=0xCONFIG_OBJECT_ID
```

**Key Object Identification**:
- `AuditConfig`: Shared object created during contract initialization
- `Publisher`: Permission object used for subsequent contract upgrades

---

## 4. Initialization Configuration

### 4.1 Authorize Auditors

After deployment, the administrator needs to authorize auditor addresses:

```bash
# Replace the following variables
AUDIT_CONFIG_ID="0xYOUR_CONFIG_OBJECT_ID"
AUDITOR_ADDRESS="0xAUDITOR_ADDRESS"

# Authorize auditor
sui client call \
  --package $AUDIT_SYSTEM_PACKAGE_ID \
  --module audit_core \
  --function authorize_auditor \
  --args $AUDIT_CONFIG_ID $AUDITOR_ADDRESS \
  --gas-budget 10000000
```

**Expected Result**:
```
Status: Success
```

### 4.2 Update Audit Parameters (Optional)

```bash
# Set audit parameters
sui client call \
  --package $AUDIT_SYSTEM_PACKAGE_ID \
  --module audit_core \
  --function update_audit_params \
  --args $AUDIT_CONFIG_ID 20 50 7200000 \
  --gas-budget 10000000

# Parameter explanation:
# 20: Minimum challenge count
# 50: Maximum challenge count
# 7200000: Audit interval (2 hours, in milliseconds)
```

### 4.3 Create Test Access Policy

```bash
# Create test policy (requires existing Blob ID)
REPORT_BLOB_ID="0x1234567890abcdef..."  # 32 bytes u256
AUDIT_RECORD_ID="0xRECORD_ID"

sui client call \
  --package $ACCESS_POLICY_PACKAGE_ID \
  --module report_access \
  --function create_policy \
  --args $REPORT_BLOB_ID $AUDIT_RECORD_ID \
  "[]" "[]" "null" "0xCLOCK_ID" \
  --gas-budget 10000000

# Parameter explanation:
# [] Empty readers list
# [] Empty auditors list
# null Never expires
# 0x6 is Clock shared object ID (fixed)
```

---

## 5. Deployment Verification

### 5.1 Query Contract Objects

```bash
# View audit_system package information
sui client object $AUDIT_SYSTEM_PACKAGE_ID

# View AuditConfig object
sui client object $AUDIT_CONFIG_OBJECT_ID --json | jq .data.content.fields
```

**Expected Output**:
```json
{
  "admin": "0xYOUR_ADDRESS",
  "authorized_auditors": [],
  "min_challenge_count": 10,
  "max_challenge_count": 100,
  "challenge_interval_ms": 3600000,
  "total_audits": 0,
  "total_blobs_audited": 0
}
```

### 5.2 Test Contract Calls

Create a test audit record:

```bash
# Prepare test data
BLOB_ID="115792089237316195423570985008687907853269984665640564039457584007913129639935"  # u256 example
BLOB_OBJECT_ID="0x0000000000000000000000000000000000000000000000000000000000000001"
INTEGRITY_HASH="0x$(echo -n 'test_hash' | sha256sum | cut -d' ' -f1)"
PQC_SIGNATURE="0x$(openssl rand -hex 128)"  # Falcon-512 signature ~666 bytes

# Submit audit record (requires authorized auditor)
sui client call \
  --package $AUDIT_SYSTEM_PACKAGE_ID \
  --module audit_core \
  --function submit_audit_record \
  --args \
    $AUDIT_CONFIG_OBJECT_ID \
    $BLOB_ID \
    $BLOB_OBJECT_ID \
    100 \
    50 \
    48 \
    "[$INTEGRITY_HASH]" \
    "[$PQC_SIGNATURE]" \
    1 \
    "0x6" \
  --gas-budget 20000000

# Parameter explanation:
# 100: challenge_epoch
# 50: total_challenges
# 48: successful_verifications
# 1: pqc_algorithm (Falcon-512)
# 0x6: Clock object ID
```

### 5.3 Query Events

```bash
# Query AuditCreated events
sui client events \
  --query "{\"MoveEventType\":\"$AUDIT_SYSTEM_PACKAGE_ID::audit_core::AuditCreated\"}" \
  --limit 10

# Query PolicyCreated events
sui client events \
  --query "{\"MoveEventType\":\"$ACCESS_POLICY_PACKAGE_ID::report_access::PolicyCreated\"}" \
  --limit 10
```

---

## 6. Common Issues

### Q1: Compilation Failed - "dependency not found"

**Problem**:
```
error: dependency 'Sui' not found
```

**Solution**:
```bash
# Clean cache
rm -rf ~/.move

# Recompile
sui move build
```

### Q2: Insufficient Gas

**Problem**:
```
InsufficientGas
```

**Solution**:
```bash
# Increase gas-budget
sui client publish --gas-budget 200000000

# Or get more testnet tokens
curl --location --request POST 'https://faucet.testnet.sui.io/gas' ...
```

### Q3: Cannot Find Package ID After Deployment

**Solution**:

Save output immediately after successful deployment:

```bash
# Redirect output during deployment
sui client publish --gas-budget 100000000 > deployment_output.txt

# Extract Package ID from output
cat deployment_output.txt | grep "PackageID:"
```

Or query transaction history:

```bash
# Query recent transactions
sui client transactions --address $(sui client active-address) --limit 1
```

### Q4: Contract Upgrade

**Important**: Contracts deployed by default are immutable.

If you need upgradeable contracts, use `UpgradeCap`:

```bash
# UpgradeCap is automatically created during deployment
# Record UpgradeCap Object ID

# Upgrade contract
sui client upgrade \
  --upgrade-capability $UPGRADE_CAP_ID \
  --gas-budget 100000000
```

### Q5: How to Connect to Mainnet?

```bash
# Switch to mainnet
sui client switch --env mainnet

# Confirm network
sui client active-env

# Check balance (mainnet requires real SUI)
sui client gas
```

---

## 📋 Deployment Checklist

After completing deployment, confirm the following items:

- [ ] `access_policy` contract successfully deployed
- [ ] `audit_system` contract successfully deployed
- [ ] Record both Package IDs
- [ ] Record AuditConfig shared object ID
- [ ] Authorize at least one auditor address
- [ ] Successfully call `submit_audit_record`
- [ ] Query `AuditCreated` events
- [ ] Update Package IDs in `.env` file
- [ ] Commit configuration to Git (excluding private keys)

---

## 📝 Environment Variables Template

Create `.env.deployment` file:

```bash
# Sui Network
SUI_NETWORK=testnet
SUI_RPC_URL=https://fullnode.testnet.sui.io:443

# Deployed Contracts
ACCESS_POLICY_PACKAGE_ID=0x...
AUDIT_SYSTEM_PACKAGE_ID=0x...

# Shared Objects
AUDIT_CONFIG_OBJECT_ID=0x...

# Admin Address
ADMIN_ADDRESS=0x...

# System Objects (Fixed)
CLOCK_OBJECT_ID=0x6
SYSTEM_STATE_OBJECT_ID=0x5
```

---

## 🔗 Related Resources

- [Sui Move Documentation](https://docs.sui.io/build/move)
- [Sui CLI Reference](https://docs.sui.io/references/cli)
- [Walrus Documentation](https://docs.walrus.site/)
- [Project Main Documentation](../README.md)
- [Walrus On-chain Integration Guide](./audit_system/docs/walrus_onchain_integration.md)

---

## 🆘 Getting Help

If you encounter issues:

1. Check Sui CLI version: `sui --version`
2. View detailed logs: Add `--verbose` flag
3. Verify network connection: `sui client objects`
4. Submit Issue: [GitHub Issues](https://github.com/your-org/walrus-audit-system/issues)

---

**After successful deployment, continue with [QUICKSTART.md](../QUICKSTART.md) to run the audit node!** 🚀
