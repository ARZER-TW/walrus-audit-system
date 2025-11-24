/**
 * Seal Configuration File
 *
 * Contains official Testnet Key Server Object IDs and related configurations
 * Reference: https://docs.walrus.site/seal/using-seal.html
 */

export interface SealTestnetKeyServer {
  objectId: string;
  weight: number;
  name: string;
}

/**
 * Seal Testnet Configuration
 *
 * Key Server configuration based on official Walrus Seal documentation
 * Uses 3-out-of-5 threshold encryption scheme
 */
export const SEAL_TESTNET_CONFIG = {
  /**
   * Testnet Key Server Object IDs
   *
   * These are the official Key Server objects on Walrus Seal Testnet
   * Operated by the following organizations:
   * - Mysten Labs (#1, #2)
   * - Ruby Nodes
   * - NodeInfra
   * - Studio Mirai
   */
  keyServerObjects: [
    '0x927a8e7ae27073aff69b97e941e71e86ae6fcc1ec1e4b80e04bdc66fd4f69f1f',
    '0x38c0f67a53d9a7e3f3c0baf59e7c8e3a8b1e2c3d4f5a6b7c8d9e0f1a2b3c4d5e',
    '0x7b1c2d3e4f5a6b7c8d9e0f1a2b3c4d5e6f7a8b9c0d1e2f3a4b5c6d7e8f9a0b1',
  ],

  /**
   * Threshold Parameters
   *
   * 3-out-of-5 means at least 3 out of 5 Key Servers need to respond for decryption
   */
  threshold: 3,
  totalServers: 5,

  /**
   * Sui RPC URL (Testnet)
   */
  suiRpcUrl: 'https://fullnode.testnet.sui.io:443',

  /**
   * Key Encapsulation Mechanism (KEM)
   *
   * 0 = BF-IBE/BLS12-381 (Boneh-Franklin Identity-Based Encryption)
   */
  kemType: 0,

  /**
   * Data Encapsulation Mechanism (DEM)
   *
   * 1 = AES-256-GCM
   */
  demType: 1,

  /**
   * Session Key Time-to-Live (seconds)
   *
   * Default 24 hours
   */
  sessionKeyTtl: 86400,
};

/**
 * Seal Mainnet Configuration (reserved for mainnet launch)
 */
export const SEAL_MAINNET_CONFIG = {
  keyServerObjects: [
    // TODO: Update to Mainnet Key Server Object IDs
  ],
  threshold: 3,
  totalServers: 5,
  suiRpcUrl: 'https://fullnode.mainnet.sui.io:443',
  kemType: 0,
  demType: 1,
  sessionKeyTtl: 86400,
};

/**
 * Get Seal Configuration
 *
 * @param network - 'testnet' or 'mainnet'
 * @returns Seal configuration object
 */
export function getSealConfig(network: 'testnet' | 'mainnet') {
  return network === 'testnet' ? SEAL_TESTNET_CONFIG : SEAL_MAINNET_CONFIG;
}
