import './App.css'
import { SealDecryption } from './components/SealDecryption'

/**
 * Walrus Audit System - Seal Decryption Frontend
 *
 * Integrates:
 * - Sui Wallet Connection (@mysten/dapp-kit)
 * - Session Key Signature Authorization
 * - Seal Encrypted Report Decryption
 * - Access Control Verification
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
          <h1 style={{ margin: 0 }}>Walrus Audit System</h1>
          <p style={{ margin: '5px 0 0 0', opacity: 0.9 }}>
            Decentralized Storage Integrity Auditing and Access Control System based on Seal
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
          <strong>Tech Stack:</strong> Sui Move + Rust + PQC (Dilithium3) + Seal (IBE Threshold Encryption) + Walrus
        </p>
        <p style={{ fontSize: '14px', marginTop: '10px' }}>
          Hackathon Demo - Walrus Haulout (Data Security & Privacy Track)
        </p>
      </footer>
    </div>
  )
}

export default App
