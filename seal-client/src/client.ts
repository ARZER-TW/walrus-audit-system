/**
 * Seal 客戶端主類
 */

import { SuiClient } from '@mysten/sui.js/client';
import { Ed25519Keypair } from '@mysten/sui.js/keypairs/ed25519';
import { TransactionBlock } from '@mysten/sui.js/transactions';
import { SealConfig } from './types';

export class SealClient {
  private suiClient: SuiClient;
  private keypair?: Ed25519Keypair;
  private config: SealConfig;

  constructor(config: SealConfig) {
    this.config = config;
    this.suiClient = new SuiClient({ url: config.suiRpcUrl });

    if (config.privateKey) {
      // 從私鑰恢復密鑰對
      this.keypair = Ed25519Keypair.fromSecretKey(
        Buffer.from(config.privateKey, 'hex')
      );
    }
  }

  /**
   * 獲取 Sui 客戶端實例
   */
  getSuiClient(): SuiClient {
    return this.suiClient;
  }

  /**
   * 獲取當前地址
   */
  getAddress(): string {
    if (!this.keypair) {
      throw new Error('未設置私鑰，無法獲取地址');
    }
    return this.keypair.getPublicKey().toSuiAddress();
  }

  /**
   * 執行交易
   */
  async executeTransaction(txb: TransactionBlock): Promise<string> {
    if (!this.keypair) {
      throw new Error('未設置私鑰，無法簽名交易');
    }

    const result = await this.suiClient.signAndExecuteTransactionBlock({
      transactionBlock: txb,
      signer: this.keypair,
      options: {
        showEffects: true,
        showObjectChanges: true,
      },
    });

    return result.digest;
  }
}
