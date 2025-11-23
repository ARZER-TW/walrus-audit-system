/**
 * 類型定義
 */

export interface AccessPolicy {
  policyId: string;
  blobId: string;
  owner: string;
  allowedReaders: string[];
  allowedAuditors: string[];
  expiryTimestamp: number;
  isActive: boolean;
}

export interface SealToken {
  tokenId: string;
  policyId: string;
  holder: string;
  accessType: AccessType;
  grantedAt: number;
  expiresAt: number;
}

export enum AccessType {
  READ = 0,
  AUDIT = 1,
}

export interface SealConfig {
  suiRpcUrl: string;
  walrusAggregatorUrl: string;
  accessPolicyPackageId: string;
  privateKey?: string;
}

export interface UploadResult {
  blobId: string;
  sealUrl: string;
}
