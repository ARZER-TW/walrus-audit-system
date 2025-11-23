/**
 * Seal 解密組件
 *
 * 功能：
 * 1. 連接 Sui 錢包
 * 2. 顯示加密報告列表
 * 3. 創建 Session Key 並請求錢包簽名
 * 4. 調用後端解密 API
 * 5. 顯示解密後的報告內容
 *
 * 流程：
 * User → Connect Wallet → Select Report → Create Session Key → Sign → Decrypt → View Report
 */

import { useState } from 'react';
import { ConnectButton, useCurrentAccount, useSignPersonalMessage } from '@mysten/dapp-kit';
import { useSuiContract } from '../hooks/useSuiContract';

// 加密報告類型
interface EncryptedReport {
  reportId: string;
  blobId: string;
  timestamp: number;
  auditor: string;
  encryptedData?: string; // 實際應用中從 Walrus 下載
}

// 解密後的報告類型
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

// Session Key 資訊
interface SessionKeyInfo {
  message: string;
  publicKey: string;
  expiresAt: number;
}

// Seal API 後端地址
const SEAL_API_URL = import.meta.env.VITE_SEAL_API_URL || 'http://localhost:3001';

// Audit 合約 Package ID（應該從環境變數讀取）
const AUDIT_PACKAGE_ID = import.meta.env.VITE_AUDIT_PACKAGE_ID ||
  '0x8afa5d31dbaa0a8fb07082692940ca3d56b5e856c5126cb5a3693f0a4de63b82';

export function SealDecryption() {
  const currentAccount = useCurrentAccount();
  const { mutateAsync: signPersonalMessage } = useSignPersonalMessage();

  // 🆕 Sui 合約 Hook
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

  // 模擬的加密報告列表（實際應用中從 Sui 鏈上查詢）
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
   * 創建 Session Key 並請求用戶簽名
   */
  const createSessionKey = async (): Promise<{
    publicKey: string;
    signature: string;
    expiresAt: number;
  }> => {
    if (!currentAccount) {
      throw new Error('請先連接錢包');
    }

    console.log('🔑 步驟 1: 向後端請求創建 Session Key...');

    // 1. 向後端請求創建 Session Key
    const response = await fetch(`${SEAL_API_URL}/api/seal/create-session-key`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        address: currentAccount.address,
        packageId: AUDIT_PACKAGE_ID,
        ttlMin: 1440 // 24 小時
      })
    });

    if (!response.ok) {
      const error = await response.json();
      throw new Error(`Session Key 創建失敗: ${error.error}`);
    }

    const { sessionKey }: { sessionKey: SessionKeyInfo } = await response.json();

    console.log('✅ Session Key 創建成功');
    console.log('   - Public Key:', sessionKey.publicKey);
    console.log('   - Expires:', new Date(sessionKey.expiresAt).toISOString());
    console.log('\n📝 步驟 2: 請求錢包簽名...');
    console.log('需要簽名的消息:');
    console.log(sessionKey.message);

    // 2. 請求用戶用錢包簽署 Session Key 授權消息
    const signatureResult = await signPersonalMessage({
      message: new TextEncoder().encode(sessionKey.message)
    });

    console.log('✅ 錢包簽名成功');
    console.log('   - Signature:', signatureResult.signature);

    return {
      publicKey: sessionKey.publicKey,
      signature: signatureResult.signature,
      expiresAt: sessionKey.expiresAt
    };
  };

  /**
   * 解密報告
   */
  const handleDecrypt = async (report: EncryptedReport) => {
    if (!currentAccount) {
      setError('請先連接錢包');
      return;
    }

    setLoading(true);
    setError(null);
    setSelectedReport(report);
    setDecryptedReport(null);

    try {
      console.log('🔓 開始解密報告:', report.reportId);

      // 步驟 1: 創建 Session Key（需要用戶錢包簽名）
      const sessionKey = await createSessionKey();

      // 步驟 2: 下載加密報告數據
      // 實際應用中應該從 Walrus 下載加密的 blob
      // const walrusUrl = `https://aggregator.walrus-testnet.walrus.space/v1/${report.blobId}`;
      // const blobResponse = await fetch(walrusUrl);
      // const encryptedData = await blobResponse.text();

      // 模擬加密數據（演示用）
      console.log('\n📥 步驟 3: 下載加密報告（模擬）...');
      const mockEncryptedData = btoa(JSON.stringify({
        encrypted: true,
        reportId: report.reportId,
        data: 'encrypted-blob-data'
      }));

      // 步驟 3: 調用後端解密 API
      console.log('\n🔓 步驟 4: 調用解密 API...');
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
        throw new Error(error.error || '解密失敗');
      }

      const result = await decryptResponse.json();

      console.log('✅ 解密成功！');
      console.log('   - Report ID:', result.metadata.reportId);
      console.log('   - Decrypted Size:', result.metadata.decryptedSize);
      console.log('   - Note:', result.metadata.note);

      setDecryptedReport(result.report);

    } catch (err: any) {
      console.error('❌ 解密失敗:', err.message);
      setError(err.message);
    } finally {
      setLoading(false);
    }
  };

  return (
    <div style={{ padding: '20px', maxWidth: '1200px', margin: '0 auto' }}>
      <h1>🔐 Seal 解密系統</h1>

      {/* 錢包連接區域 */}
      <div style={{
        marginBottom: '30px',
        padding: '20px',
        border: '1px solid #ccc',
        borderRadius: '8px',
        backgroundColor: '#f9f9f9'
      }}>
        <h2>1️⃣ 連接錢包</h2>
        <ConnectButton />
        {currentAccount && (
          <div style={{ marginTop: '10px', color: '#666' }}>
            <strong>已連接:</strong> {currentAccount.address.slice(0, 10)}...{currentAccount.address.slice(-8)}
          </div>
        )}
      </div>

      {/* 🆕 Sui 合約連接狀態 */}
      <div style={{
        marginBottom: '30px',
        padding: '20px',
        border: '1px solid #ccc',
        borderRadius: '8px',
        backgroundColor: isSuiConnected ? '#f1f8f4' : '#fff3e0'
      }}>
        <h2>📡 Sui 合約連接狀態</h2>

        {isSuiLoading && (
          <div style={{ color: '#666', marginTop: '10px' }}>
            🔄 正在連接 Sui 合約...
          </div>
        )}

        {!isSuiLoading && isSuiConnected && (
          <div>
            <div style={{ color: '#4caf50', marginTop: '10px', fontWeight: 'bold' }}>
              ✅ Sui 合約連接成功
            </div>
            {(() => {
              const config = getAuditConfigFromTest();
              return config && (
                <div style={{ marginTop: '15px', fontSize: '14px' }}>
                  <div><strong>📦 Package ID:</strong> {AUDIT_PACKAGE_ID.slice(0, 10)}...{AUDIT_PACKAGE_ID.slice(-8)}</div>
                  <div><strong>👤 Admin:</strong> {config.admin.slice(0, 10)}...{config.admin.slice(-8)}</div>
                  <div><strong>📊 總審計次數:</strong> {config.total_audits}</div>
                  <div><strong>💾 審計過的 Blob 數:</strong> {config.total_blobs_audited}</div>
                  <div><strong>⏱️ 挑戰間隔:</strong> {parseInt(config.challenge_interval_ms) / 1000 / 60} 分鐘</div>
                </div>
              );
            })()}
          </div>
        )}

        {!isSuiLoading && !isSuiConnected && (
          <div>
            <div style={{ color: '#f57c00', marginTop: '10px', fontWeight: 'bold' }}>
              ⚠️ Sui 合約連接失敗
            </div>
            {suiError && (
              <div style={{ marginTop: '10px', fontSize: '14px', color: '#d32f2f' }}>
                錯誤: {suiError}
              </div>
            )}
            <div style={{ marginTop: '10px', fontSize: '14px', color: '#666' }}>
              請確保後端服務器正在運行: <code>http://localhost:3001</code>
            </div>
          </div>
        )}
      </div>

      {/* 報告列表 */}
      {currentAccount && (
        <div style={{
          marginBottom: '30px',
          padding: '20px',
          border: '1px solid #ccc',
          borderRadius: '8px'
        }}>
          <h2>2️⃣ 選擇要解密的報告</h2>
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
                <div><strong>時間:</strong> {new Date(report.timestamp).toLocaleString()}</div>
                <div><strong>審計員:</strong> {report.auditor.slice(0, 10)}...{report.auditor.slice(-8)}</div>
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
                  {loading && selectedReport?.reportId === report.reportId ? '解密中...' : '🔓 解密'}
                </button>
              </div>
            ))}
          </div>
        </div>
      )}

      {/* 錯誤信息 */}
      {error && (
        <div style={{
          padding: '15px',
          backgroundColor: '#ffebee',
          color: '#c62828',
          borderRadius: '4px',
          marginBottom: '20px'
        }}>
          <strong>❌ 錯誤:</strong> {error}
        </div>
      )}

      {/* 解密結果 */}
      {decryptedReport && (
        <div style={{
          padding: '20px',
          border: '2px solid #4caf50',
          borderRadius: '8px',
          backgroundColor: '#f1f8f4'
        }}>
          <h2>3️⃣ 解密結果 ✅</h2>

          <div style={{ marginTop: '15px' }}>
            <h3>報告詳情</h3>
            <table style={{ width: '100%', borderCollapse: 'collapse' }}>
              <tbody>
                <tr>
                  <td style={{ padding: '8px', borderBottom: '1px solid #ddd', fontWeight: 'bold' }}>Blob ID</td>
                  <td style={{ padding: '8px', borderBottom: '1px solid #ddd' }}>{decryptedReport.blob_id}</td>
                </tr>
                <tr>
                  <td style={{ padding: '8px', borderBottom: '1px solid #ddd', fontWeight: 'bold' }}>審計員</td>
                  <td style={{ padding: '8px', borderBottom: '1px solid #ddd', fontFamily: 'monospace', fontSize: '12px' }}>
                    {decryptedReport.auditor}
                  </td>
                </tr>
                <tr>
                  <td style={{ padding: '8px', borderBottom: '1px solid #ddd', fontWeight: 'bold' }}>時間戳</td>
                  <td style={{ padding: '8px', borderBottom: '1px solid #ddd' }}>
                    {new Date(decryptedReport.timestamp).toLocaleString()}
                  </td>
                </tr>
                <tr>
                  <td style={{ padding: '8px', borderBottom: '1px solid #ddd', fontWeight: 'bold' }}>挑戰輪次</td>
                  <td style={{ padding: '8px', borderBottom: '1px solid #ddd' }}>{decryptedReport.challenge_epoch}</td>
                </tr>
                <tr>
                  <td style={{ padding: '8px', borderBottom: '1px solid #ddd', fontWeight: 'bold' }}>總挑戰數</td>
                  <td style={{ padding: '8px', borderBottom: '1px solid #ddd' }}>{decryptedReport.total_challenges}</td>
                </tr>
                <tr>
                  <td style={{ padding: '8px', borderBottom: '1px solid #ddd', fontWeight: 'bold' }}>成功驗證</td>
                  <td style={{ padding: '8px', borderBottom: '1px solid #ddd', color: '#4caf50' }}>
                    {decryptedReport.successful_verifications}
                  </td>
                </tr>
                <tr>
                  <td style={{ padding: '8px', borderBottom: '1px solid #ddd', fontWeight: 'bold' }}>失敗驗證</td>
                  <td style={{ padding: '8px', borderBottom: '1px solid #ddd', color: '#f44336' }}>
                    {decryptedReport.failed_verifications}
                  </td>
                </tr>
                <tr>
                  <td style={{ padding: '8px', borderBottom: '1px solid #ddd', fontWeight: 'bold' }}>完整性狀態</td>
                  <td style={{ padding: '8px', borderBottom: '1px solid #ddd' }}>
                    <span style={{
                      padding: '4px 12px',
                      borderRadius: '12px',
                      backgroundColor: decryptedReport.is_valid ? '#4caf50' : '#f44336',
                      color: 'white',
                      fontWeight: 'bold'
                    }}>
                      {decryptedReport.is_valid ? '✅ 有效' : '❌ 無效'}
                    </span>
                  </td>
                </tr>
                <tr>
                  <td style={{ padding: '8px', borderBottom: '1px solid #ddd', fontWeight: 'bold' }}>PQC 算法</td>
                  <td style={{ padding: '8px', borderBottom: '1px solid #ddd' }}>
                    {decryptedReport.pqc_algorithm === 3 ? 'Dilithium3 (NIST FIPS 204)' : `Unknown (${decryptedReport.pqc_algorithm})`}
                  </td>
                </tr>
                <tr>
                  <td style={{ padding: '8px', borderBottom: '1px solid #ddd', fontWeight: 'bold' }}>PQC 簽名長度</td>
                  <td style={{ padding: '8px', borderBottom: '1px solid #ddd' }}>
                    {decryptedReport.pqc_signature.length} bytes
                  </td>
                </tr>
              </tbody>
            </table>
          </div>

          <div style={{ marginTop: '20px', padding: '15px', backgroundColor: '#e3f2fd', borderRadius: '4px' }}>
            <h4>🔐 安全性說明</h4>
            <ul style={{ marginTop: '10px', lineHeight: '1.6' }}>
              <li><strong>Session Key 授權:</strong> 您剛才簽署的消息授權了臨時密鑰代表您解密此報告</li>
              <li><strong>訪問控制:</strong> 後端驗證了您的錢包地址有權限訪問此報告</li>
              <li><strong>PQC 簽名:</strong> 報告使用 Dilithium3 後量子簽名，確保長期真實性</li>
              <li><strong>門檻加密:</strong> 解密需要從 Seal Key Servers 獲取 2/3 密鑰份額</li>
            </ul>
          </div>
        </div>
      )}

      {/* 使用說明 */}
      {!currentAccount && (
        <div style={{
          padding: '20px',
          backgroundColor: '#fff3e0',
          borderRadius: '8px',
          marginTop: '20px'
        }}>
          <h3>📖 使用說明</h3>
          <ol style={{ lineHeight: '1.8' }}>
            <li>點擊上方「Connect Wallet」按鈕連接您的 Sui 錢包</li>
            <li>從列表中選擇要解密的加密報告</li>
            <li>系統會請求您簽署 Session Key 授權消息（僅授權臨時解密）</li>
            <li>簽名成功後，系統自動解密報告並顯示內容</li>
          </ol>

          <div style={{ marginTop: '15px', padding: '10px', backgroundColor: '#e3f2fd', borderRadius: '4px' }}>
            <strong>💡 提示:</strong> Session Key 有效期為 24 小時，過期後需要重新簽名
          </div>
        </div>
      )}
    </div>
  );
}
