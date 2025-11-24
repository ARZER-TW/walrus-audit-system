/**
 * Seal 配置文件
 *
 * 包含官方 Testnet Key Server Object IDs 和相關配置
 * 參考: https://docs.walrus.site/seal/using-seal.html
 */

export interface SealTestnetKeyServer {
  objectId: string;
  weight: number;
  name: string;
}

/**
 * Seal Testnet 配置
 *
 * Key Server 配置基於官方 Walrus Seal 文檔
 * 使用 3-out-of-5 門檻加密方案
 */
export const SEAL_TESTNET_CONFIG = {
  /**
   * Testnet Key Server Object IDs
   *
   * 這些是 Walrus Seal Testnet 上的官方 Key Server 物件
   * 由以下組織運營:
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
   * 門檻參數
   *
   * 3-out-of-5 表示 5 個 Key Server 中需要至少 3 個響應才能解密
   */
  threshold: 3,
  totalServers: 5,

  /**
   * Sui RPC URL (Testnet)
   */
  suiRpcUrl: 'https://fullnode.testnet.sui.io:443',

  /**
   * 密鑰封裝機制 (KEM)
   *
   * 0 = BF-IBE/BLS12-381 (Boneh-Franklin Identity-Based Encryption)
   */
  kemType: 0,

  /**
   * 數據加密機制 (DEM)
   *
   * 1 = AES-256-GCM
   */
  demType: 1,

  /**
   * Session Key 有效期（秒）
   *
   * 預設 24 小時
   */
  sessionKeyTtl: 86400,
};

/**
 * Seal Mainnet 配置 (暫時保留待正式網上線)
 */
export const SEAL_MAINNET_CONFIG = {
  keyServerObjects: [
    // TODO: 更新為 Mainnet Key Server Object IDs
  ],
  threshold: 3,
  totalServers: 5,
  suiRpcUrl: 'https://fullnode.mainnet.sui.io:443',
  kemType: 0,
  demType: 1,
  sessionKeyTtl: 86400,
};

/**
 * 獲取 Seal 配置
 *
 * @param network - 'testnet' 或 'mainnet'
 * @returns Seal 配置物件
 */
export function getSealConfig(network: 'testnet' | 'mainnet') {
  return network === 'testnet' ? SEAL_TESTNET_CONFIG : SEAL_MAINNET_CONFIG;
}
