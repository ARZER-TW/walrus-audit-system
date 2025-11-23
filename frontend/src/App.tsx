import './App.css'
import { SealDecryption } from './components/SealDecryption'

/**
 * Walrus 審計系統 - Seal 解密前端
 *
 * 整合了：
 * - Sui 錢包連接（@mysten/dapp-kit）
 * - Session Key 簽名授權
 * - Seal 加密報告解密
 * - 訪問控制驗證
 */
function App() {
  return (
    <div style={{ minHeight: '100vh', backgroundColor: '#f5f5f5' }}>
      <header style={{
        padding: '20px',
        backgroundColor: '#1976d2',
        color: 'white',
        boxShadow: '0 2px 4px rgba(0,0,0,0.1)'
      }}>
        <div style={{ maxWidth: '1200px', margin: '0 auto' }}>
          <h1 style={{ margin: 0 }}>Walrus 審計系統</h1>
          <p style={{ margin: '5px 0 0 0', opacity: 0.9 }}>
            基於 Seal 的去中心化存儲完整性審計與訪問控制系統
          </p>
        </div>
      </header>

      <main>
        <SealDecryption />
      </main>

      <footer style={{
        marginTop: '40px',
        padding: '20px',
        textAlign: 'center',
        color: '#666',
        borderTop: '1px solid #ddd'
      }}>
        <p>
          <strong>技術堆疊:</strong> Sui Move + Rust + PQC (Dilithium3) + Seal (IBE Threshold Encryption) + Walrus
        </p>
        <p style={{ fontSize: '14px', marginTop: '10px' }}>
          Hackathon Demo - Walrus Haulout (Data Security & Privacy Track)
        </p>
      </footer>
    </div>
  )
}

export default App
