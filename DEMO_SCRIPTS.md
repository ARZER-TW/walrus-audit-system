# 🎬 Demo Scripts Usage Guide

Two demo scripts have been prepared for recording:

## 📝 Available Scripts

### 1. `demo.sh` - Full Demo (7-8 minutes)
**Complete demonstration with all features**

```bash
cd ~/notebook/walrus-audit-system
./demo.sh
```

**Features:**
- ✅ Project structure overview
- ✅ Sui Testnet deployment verification
- ✅ Walrus blob verification
- ✅ Merkle Tree + PQC signature test
- ✅ Audit report inspection
- ✅ Data integrity verification
- ✅ Seal configuration showcase
- ✅ Smart contract code display
- ✅ Interactive pauses (Press Enter to continue)

**Best for:** Comprehensive presentation to judges

---

### 2. `demo-quick.sh` - Quick Demo (3 minutes)
**Essential features only**

```bash
cd ~/notebook/walrus-audit-system
./demo-quick.sh
```

**Features:**
- ✅ Walrus blob verification
- ✅ Merkle Tree + PQC test
- ✅ Report viewing
- ✅ Signature verification
- ✅ Data integrity check
- ✅ On-chain deployment view
- ✅ Interactive pauses

**Best for:** Time-constrained demo or initial screening

---

## 🎥 Recording Tips

### Before Recording

1. **Clean terminal history:**
   ```bash
   clear
   history -c
   ```

2. **Increase font size** (for better visibility):
   - Terminal: Edit → Preferences → Profile → Text → Font size: 14-16

3. **Test run** (without recording):
   ```bash
   ./demo-quick.sh
   ```

4. **Ensure prerequisites:**
   ```bash
   # Check Rust
   rustc --version

   # Check Sui CLI
   sui --version

   # Check audit report exists
   ls -lh /tmp/signed_audit_report.json
   ```

---

### During Recording

1. **Start recording** (use OBS, asciinema, or screen recorder)

2. **Run the script:**
   ```bash
   # For full demo
   ./demo.sh

   # OR for quick demo
   ./demo-quick.sh
   ```

3. **At each pause:**
   - Wait 2-3 seconds to let viewers read output
   - Highlight important lines with mouse if possible
   - Press Enter to continue

4. **Key moments to emphasize:**
   - When "100% success" appears (Merkle verification)
   - PQC signature length = 6618 (Dilithium3)
   - Hash matching (SHA-256 integrity)
   - On-chain object details

---

### After Recording

1. **Add annotations** (optional):
   - Highlight "✅ Success" messages
   - Add text overlays for key metrics
   - Include project logo/title screen

2. **Export video:**
   - Format: MP4 (H.264)
   - Resolution: 1920x1080 or 1280x720
   - Framerate: 30fps

---

## 🚀 Alternative: Auto-run Mode

For continuous recording without pauses:

```bash
# Set auto-run mode
export AUTO_MODE=1

# Run demo
./demo.sh
```

This will run all steps with 2-second delays instead of waiting for Enter.

---

## 🐛 Troubleshooting

### Issue: Script fails at Merkle test
**Solution:** Ensure you're in the correct directory
```bash
cd ~/notebook/walrus-audit-system
```

### Issue: Sui client command not found
**Solution:** Check Sui CLI installation
```bash
sui --version
# If not found, install from https://docs.sui.io/guides/developer/getting-started/sui-install
```

### Issue: Blob download fails (404)
**Solution:** Walrus Testnet might be temporarily down. Use the existing report:
```bash
# Skip blob download, just show existing report
head -40 /tmp/signed_audit_report.json
```

### Issue: cargo command takes too long
**Solution:** Use pre-compiled binary (if available) or skip this step in recording

---

## 📊 Expected Output Summary

### Part 1: Walrus Verification
```
HTTP/1.1 200 OK
```
✅ Blob exists on Walrus Testnet

### Part 2: Merkle Test
```
╔════════════════════════════════════════════════════════════════╗
║           Merkle Tree Integration Test                        ║
╚════════════════════════════════════════════════════════════════╝

✅ Audit completed: 100% success
✅ Signed report saved: /tmp/signed_audit_report.json
```

### Part 3: Signature Verification
```
Signature length: 6618 characters
Expected: 6618 (Dilithium3)
✅ Correct!
```

### Part 4: Data Integrity
```
Downloaded blob hash: bd9e5380f78734bc182e4bb8c464101d...
Report hash:          bd9e5380f78734bc182e4bb8c464101d...
✅ These should match!
```

### Part 5: On-Chain Verification
```
Object ID: 0x1dcd8f8d4965cb2ab5dc61c2dc9c168f51ff79f6b43d2aef6fedb622e220872b
Type: audit_core::AuditConfig
✅ Successfully deployed on Sui Testnet
```

---

## 🎯 Key Talking Points

While recording, mention:

1. **Post-Quantum Security**
   - "Using Dilithium3, NIST-approved PQC algorithm"
   - "Future-proof against quantum computer attacks"

2. **Real Integration**
   - "Not mocks - real Walrus Testnet blob"
   - "Deployed on Sui blockchain, verifiable on-chain"

3. **Production-Ready Crypto**
   - "100% success rate on Merkle verification"
   - "Blake2b-256 matches Walrus official specification"

4. **Innovation**
   - "First quantum-resistant audit system for decentralized storage"
   - "Combines PQC + Merkle + Blockchain + Privacy (Seal)"

---

## 📞 Need Help?

If you encounter issues during demo preparation:
1. Check [DEMO_INSTRUCTIONS.md](DEMO_INSTRUCTIONS.md) for detailed manual steps
2. Review [README.md](README.md) for system requirements
3. Verify deployment with [DEPLOYMENT_SUMMARY.md](DEPLOYMENT_SUMMARY.md)

---

**Good luck with your recording! 🎬**

*Built for Walrus Haulout Hackathon - Data Security & Privacy Track*
*November 2025*
