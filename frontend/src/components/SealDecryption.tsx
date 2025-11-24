/**
 * Seal Decryption Component
 *
 * Features:
 * 1. Connect Sui Wallet
 * 2. Display encrypted report list
 * 3. Create Session Key and request wallet signature
 * 4. Call backend decryption API
 * 5. Display decrypted report content
 *
 * Flow:
 * User → Connect Wallet → Select Report → Create Session Key → Sign → Decrypt → View Report
 */

import { useState } from 'react';
import { ConnectButton, useCurrentAccount, useSignPersonalMessage } from '@mysten/dapp-kit';
import { useSuiContract } from '../hooks/useSuiContract';

// Encrypted Report Type
interface EncryptedReport {
  reportId: string;
  blobId: string;
  timestamp: number;
  auditor: string;
  encryptedData?: string; // Downloaded from Walrus in production
}

// Decrypted Report Type
interface DecryptedReport {
  blob_id: string;
  blob_object_id: string;
  auditor: string;
  timestamp: number;
  challenge_epoch: number;
  total_challenges: number;
  successful_verifications: number;
  failed_verifications: number;
  integrity_hash: number[];
  is_valid: boolean;
  pqc_signature: number[];
  pqc_algorithm: number;
  pqc_public_key: number[];
}

// Session Key Information
interface SessionKeyInfo {
  message: string;
  publicKey: string;
  expiresAt: number;
}

// Seal API Backend Address
const SEAL_API_URL = import.meta.env.VITE_SEAL_API_URL || 'http://localhost:3001';

// Audit Contract Package ID (should be read from environment variables)
const AUDIT_PACKAGE_ID = import.meta.env.VITE_AUDIT_PACKAGE_ID ||
  '0x8afa5d31dbaa0a8fb07082692940ca3d56b5e856c5126cb5a3693f0a4de63b82';

export function SealDecryption() {
  const currentAccount = useCurrentAccount();
  const { mutateAsync: signPersonalMessage } = useSignPersonalMessage();

  // Sui Contract Hook
  const {
    isConnected: isSuiConnected,
    isLoading: isSuiLoading,
    error: suiError,
    getAuditConfigFromTest
  } = useSuiContract();

  const [selectedReport, setSelectedReport] = useState<EncryptedReport | null>(null);
  const [decryptedReport, setDecryptedReport] = useState<DecryptedReport | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // Mock encrypted report list (should query from Sui chain in production)
  const mockEncryptedReports: EncryptedReport[] = [
    {
      reportId: '0xtest-report-001',
      blobId: '0xtest-blob-123',
      timestamp: Date.now() - 3600000,
      auditor: currentAccount?.address || '0x...'
    },
    {
      reportId: '0xtest-report-002',
      blobId: '0xtest-blob-456',
      timestamp: Date.now() - 7200000,
      auditor: currentAccount?.address || '0x...'
    }
  ];

  /**
   * Create Session Key and request user signature
   */
  const createSessionKey = async (): Promise<{
    publicKey: string;
    signature: string;
    expiresAt: number;
  }> => {
    if (!currentAccount) {
      throw new Error('Please connect wallet first');
    }

    console.log('🔑 Step 1: Requesting Session Key creation from backend...');

    // 1. Request Session Key creation from backend
    const response = await fetch(`${SEAL_API_URL}/api/seal/create-session-key`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        address: currentAccount.address,
        packageId: AUDIT_PACKAGE_ID,
        ttlMin: 1440 // 24 hours
      })
    });

    if (!response.ok) {
      const error = await response.json();
      throw new Error(`Session Key creation failed: ${error.error}`);
    }

    const { sessionKey }: { sessionKey: SessionKeyInfo } = await response.json();

    console.log('✅ Session Key created successfully');
    console.log('   - Public Key:', sessionKey.publicKey);
    console.log('   - Expires:', new Date(sessionKey.expiresAt).toISOString());
    console.log('\n📝 Step 2: Requesting wallet signature...');
    console.log('Message to sign:');
    console.log(sessionKey.message);

    // 2. Request user to sign Session Key authorization message with wallet
    const signatureResult = await signPersonalMessage({
      message: new TextEncoder().encode(sessionKey.message)
    });

    console.log('✅ Wallet signature successful');
    console.log('   - Signature:', signatureResult.signature);

    return {
      publicKey: sessionKey.publicKey,
      signature: signatureResult.signature,
      expiresAt: sessionKey.expiresAt
    };
  };

  /**
   * Decrypt report
   */
  const handleDecrypt = async (report: EncryptedReport) => {
    if (!currentAccount) {
      setError('Please connect wallet first');
      return;
    }

    setLoading(true);
    setError(null);
    setSelectedReport(report);
    setDecryptedReport(null);

    try {
      console.log('🔓 Starting report decryption:', report.reportId);

      // Step 1: Create Session Key (requires user wallet signature)
      const sessionKey = await createSessionKey();

      // Step 2: Download encrypted report data
      // In production, should download encrypted blob from Walrus
      // const walrusUrl = `https://aggregator.walrus-testnet.walrus.space/v1/${report.blobId}`;
      // const blobResponse = await fetch(walrusUrl);
      // const encryptedData = await blobResponse.text();

      // Mock encrypted data (for demo)
      console.log('\n📥 Step 3: Downloading encrypted report (mock)...');
      const mockEncryptedData = btoa(JSON.stringify({
        encrypted: true,
        reportId: report.reportId,
        data: 'encrypted-blob-data'
      }));

      // Step 3: Call backend decryption API
      console.log('\n🔓 Step 4: Calling decryption API...');
      const decryptResponse = await fetch(`${SEAL_API_URL}/api/seal/decrypt`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          encryptedData: mockEncryptedData,
          reportId: report.reportId,
          requesterAddress: currentAccount.address,
          sessionKey: sessionKey,
          checkAccess: true
        })
      });

      if (!decryptResponse.ok) {
        const error = await decryptResponse.json();
        throw new Error(error.error || 'Decryption failed');
      }

      const result = await decryptResponse.json();

      console.log('✅ Decryption successful!');
      console.log('   - Report ID:', result.metadata.reportId);
      console.log('   - Decrypted Size:', result.metadata.decryptedSize);
      console.log('   - Note:', result.metadata.note);

      setDecryptedReport(result.report);

    } catch (err: any) {
      console.error('❌ Decryption failed:', err.message);
      setError(err.message);
    } finally {
      setLoading(false);
    }
  };

  return (
    <div style={{ padding: '20px', maxWidth: '1200px', margin: '0 auto' }}>
      <h1>🔐 Seal Decryption System</h1>

      {/* Wallet Connection Area */}
      <div style={{
        marginBottom: '30px',
        padding: '20px',
        border: '1px solid #ccc',
        borderRadius: '8px',
        backgroundColor: '#f9f9f9'
      }}>
        <h2>1️⃣ Connect Wallet</h2>
        <ConnectButton />
        {currentAccount && (
          <div style={{ marginTop: '10px', color: '#666' }}>
            <strong>Connected:</strong> {currentAccount.address.slice(0, 10)}...{currentAccount.address.slice(-8)}
          </div>
        )}
      </div>

      {/* Sui Contract Connection Status */}
      <div style={{
        marginBottom: '30px',
        padding: '20px',
        border: '1px solid #ccc',
        borderRadius: '8px',
        backgroundColor: isSuiConnected ? '#f1f8f4' : '#fff3e0'
      }}>
        <h2>📡 Sui Contract Connection Status</h2>

        {isSuiLoading && (
          <div style={{ color: '#666', marginTop: '10px' }}>
            🔄 Connecting to Sui contract...
          </div>
        )}

        {!isSuiLoading && isSuiConnected && (
          <div>
            <div style={{ color: '#4caf50', marginTop: '10px', fontWeight: 'bold' }}>
              ✅ Sui contract connected successfully
            </div>
            {(() => {
              const config = getAuditConfigFromTest();
              return config && (
                <div style={{ marginTop: '15px', fontSize: '14px' }}>
                  <div><strong>📦 Package ID:</strong> {AUDIT_PACKAGE_ID.slice(0, 10)}...{AUDIT_PACKAGE_ID.slice(-8)}</div>
                  <div><strong>👤 Admin:</strong> {config.admin.slice(0, 10)}...{config.admin.slice(-8)}</div>
                  <div><strong>📊 Total Audits:</strong> {config.total_audits}</div>
                  <div><strong>💾 Audited Blobs:</strong> {config.total_blobs_audited}</div>
                  <div><strong>⏱️ Challenge Interval:</strong> {parseInt(config.challenge_interval_ms) / 1000 / 60} minutes</div>
                </div>
              );
            })()}
          </div>
        )}

        {!isSuiLoading && !isSuiConnected && (
          <div>
            <div style={{ color: '#f57c00', marginTop: '10px', fontWeight: 'bold' }}>
              ⚠️ Sui contract connection failed
            </div>
            {suiError && (
              <div style={{ marginTop: '10px', fontSize: '14px', color: '#d32f2f' }}>
                Error: {suiError}
              </div>
            )}
            <div style={{ marginTop: '10px', fontSize: '14px', color: '#666' }}>
              Please ensure backend server is running: <code>http://localhost:3001</code>
            </div>
          </div>
        )}
      </div>

      {/* Report List */}
      {currentAccount && (
        <div style={{
          marginBottom: '30px',
          padding: '20px',
          border: '1px solid #ccc',
          borderRadius: '8px'
        }}>
          <h2>2️⃣ Select Report to Decrypt</h2>
          <div style={{ display: 'flex', flexDirection: 'column', gap: '10px' }}>
            {mockEncryptedReports.map((report) => (
              <div
                key={report.reportId}
                style={{
                  padding: '15px',
                  border: '1px solid #ddd',
                  borderRadius: '4px',
                  cursor: 'pointer',
                  backgroundColor: selectedReport?.reportId === report.reportId ? '#e3f2fd' : 'white',
                  transition: 'background-color 0.2s'
                }}
                onClick={() => !loading && handleDecrypt(report)}
              >
                <div><strong>Report ID:</strong> {report.reportId}</div>
                <div><strong>Blob ID:</strong> {report.blobId}</div>
                <div><strong>Time:</strong> {new Date(report.timestamp).toLocaleString()}</div>
                <div><strong>Auditor:</strong> {report.auditor.slice(0, 10)}...{report.auditor.slice(-8)}</div>
                <button
                  disabled={loading}
                  style={{
                    marginTop: '10px',
                    padding: '8px 16px',
                    backgroundColor: loading ? '#ccc' : '#1976d2',
                    color: 'white',
                    border: 'none',
                    borderRadius: '4px',
                    cursor: loading ? 'not-allowed' : 'pointer'
                  }}
                >
                  {loading && selectedReport?.reportId === report.reportId ? 'Decrypting...' : '🔓 Decrypt'}
                </button>
              </div>
            ))}
          </div>
        </div>
      )}

      {/* Error Message */}
      {error && (
        <div style={{
          padding: '15px',
          backgroundColor: '#ffebee',
          color: '#c62828',
          borderRadius: '4px',
          marginBottom: '20px'
        }}>
          <strong>❌ Error:</strong> {error}
        </div>
      )}

      {/* Decryption Result */}
      {decryptedReport && (
        <div style={{
          padding: '20px',
          border: '2px solid #4caf50',
          borderRadius: '8px',
          backgroundColor: '#f1f8f4'
        }}>
          <h2>3️⃣ Decryption Result ✅</h2>

          <div style={{ marginTop: '15px' }}>
            <h3>Report Details</h3>
            <table style={{ width: '100%', borderCollapse: 'collapse' }}>
              <tbody>
                <tr>
                  <td style={{ padding: '8px', borderBottom: '1px solid #ddd', fontWeight: 'bold' }}>Blob ID</td>
                  <td style={{ padding: '8px', borderBottom: '1px solid #ddd' }}>{decryptedReport.blob_id}</td>
                </tr>
                <tr>
                  <td style={{ padding: '8px', borderBottom: '1px solid #ddd', fontWeight: 'bold' }}>Auditor</td>
                  <td style={{ padding: '8px', borderBottom: '1px solid #ddd', fontFamily: 'monospace', fontSize: '12px' }}>
                    {decryptedReport.auditor}
                  </td>
                </tr>
                <tr>
                  <td style={{ padding: '8px', borderBottom: '1px solid #ddd', fontWeight: 'bold' }}>Timestamp</td>
                  <td style={{ padding: '8px', borderBottom: '1px solid #ddd' }}>
                    {new Date(decryptedReport.timestamp).toLocaleString()}
                  </td>
                </tr>
                <tr>
                  <td style={{ padding: '8px', borderBottom: '1px solid #ddd', fontWeight: 'bold' }}>Challenge Epoch</td>
                  <td style={{ padding: '8px', borderBottom: '1px solid #ddd' }}>{decryptedReport.challenge_epoch}</td>
                </tr>
                <tr>
                  <td style={{ padding: '8px', borderBottom: '1px solid #ddd', fontWeight: 'bold' }}>Total Challenges</td>
                  <td style={{ padding: '8px', borderBottom: '1px solid #ddd' }}>{decryptedReport.total_challenges}</td>
                </tr>
                <tr>
                  <td style={{ padding: '8px', borderBottom: '1px solid #ddd', fontWeight: 'bold' }}>Successful Verifications</td>
                  <td style={{ padding: '8px', borderBottom: '1px solid #ddd', color: '#4caf50' }}>
                    {decryptedReport.successful_verifications}
                  </td>
                </tr>
                <tr>
                  <td style={{ padding: '8px', borderBottom: '1px solid #ddd', fontWeight: 'bold' }}>Failed Verifications</td>
                  <td style={{ padding: '8px', borderBottom: '1px solid #ddd', color: '#f44336' }}>
                    {decryptedReport.failed_verifications}
                  </td>
                </tr>
                <tr>
                  <td style={{ padding: '8px', borderBottom: '1px solid #ddd', fontWeight: 'bold' }}>Integrity Status</td>
                  <td style={{ padding: '8px', borderBottom: '1px solid #ddd' }}>
                    <span style={{
                      padding: '4px 12px',
                      borderRadius: '12px',
                      backgroundColor: decryptedReport.is_valid ? '#4caf50' : '#f44336',
                      color: 'white',
                      fontWeight: 'bold'
                    }}>
                      {decryptedReport.is_valid ? '✅ Valid' : '❌ Invalid'}
                    </span>
                  </td>
                </tr>
                <tr>
                  <td style={{ padding: '8px', borderBottom: '1px solid #ddd', fontWeight: 'bold' }}>PQC Algorithm</td>
                  <td style={{ padding: '8px', borderBottom: '1px solid #ddd' }}>
                    {decryptedReport.pqc_algorithm === 3 ? 'Dilithium3 (NIST FIPS 204)' : `Unknown (${decryptedReport.pqc_algorithm})`}
                  </td>
                </tr>
                <tr>
                  <td style={{ padding: '8px', borderBottom: '1px solid #ddd', fontWeight: 'bold' }}>PQC Signature Length</td>
                  <td style={{ padding: '8px', borderBottom: '1px solid #ddd' }}>
                    {decryptedReport.pqc_signature.length} bytes
                  </td>
                </tr>
              </tbody>
            </table>
          </div>

          <div style={{ marginTop: '20px', padding: '15px', backgroundColor: '#e3f2fd', borderRadius: '4px' }}>
            <h4>🔐 Security Notes</h4>
            <ul style={{ marginTop: '10px', lineHeight: '1.6' }}>
              <li><strong>Session Key Authorization:</strong> The message you signed authorized a temporary key to decrypt this report on your behalf</li>
              <li><strong>Access Control:</strong> Backend verified your wallet address has permission to access this report</li>
              <li><strong>PQC Signature:</strong> Report uses Dilithium3 post-quantum signature to ensure long-term authenticity</li>
              <li><strong>Threshold Encryption:</strong> Decryption requires obtaining 2/3 key shares from Seal Key Servers</li>
            </ul>
          </div>
        </div>
      )}

      {/* Usage Instructions */}
      {!currentAccount && (
        <div style={{
          padding: '20px',
          backgroundColor: '#fff3e0',
          borderRadius: '8px',
          marginTop: '20px'
        }}>
          <h3>📖 Usage Instructions</h3>
          <ol style={{ lineHeight: '1.8' }}>
            <li>Click the "Connect Wallet" button above to connect your Sui wallet</li>
            <li>Select an encrypted report from the list to decrypt</li>
            <li>System will request you to sign Session Key authorization message (only authorizes temporary decryption)</li>
            <li>After successful signature, system will automatically decrypt and display report content</li>
          </ol>

          <div style={{ marginTop: '15px', padding: '10px', backgroundColor: '#e3f2fd', borderRadius: '4px' }}>
            <strong>💡 Tip:</strong> Session Key is valid for 24 hours, re-signing is required after expiration
          </div>
        </div>
      )}
    </div>
  );
}
