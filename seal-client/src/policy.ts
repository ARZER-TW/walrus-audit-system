/**
 * 訪問策略管理
 */

import { TransactionBlock } from '@mysten/sui.js/transactions';
import { SealClient } from './client';
import { AccessPolicy, AccessType } from './types';

export class PolicyManager {
  private client: SealClient;
  private packageId: string;

  constructor(client: SealClient, packageId: string) {
    this.client = client;
    this.packageId = packageId;
  }

  /**
   * 創建訪問策略
   */
  async createPolicy(
    blobId: string,
    allowedReaders: string[],
    allowedAuditors: string[],
    expiryTimestamp: number
  ): Promise<string> {
    const txb = new TransactionBlock();

    txb.moveCall({
      target: `${this.packageId}::access_policy::create_policy`,
      arguments: [
        txb.pure(Array.from(Buffer.from(blobId))),
        txb.pure(allowedReaders),
        txb.pure(allowedAuditors),
        txb.pure(expiryTimestamp),
      ],
    });

    return await this.client.executeTransaction(txb);
  }

  /**
   * 授予訪問令牌
   */
  async grantAccess(
    policyId: string,
    recipient: string,
    accessType: AccessType
  ): Promise<string> {
    const txb = new TransactionBlock();

    txb.moveCall({
      target: `${this.packageId}::access_policy::grant_access`,
      arguments: [
        txb.object(policyId),
        txb.pure(recipient),
        txb.pure(accessType),
      ],
    });

    return await this.client.executeTransaction(txb);
  }

  /**
   * 撤銷策略
   */
  async revokePolicy(policyId: string): Promise<string> {
    const txb = new TransactionBlock();

    txb.moveCall({
      target: `${this.packageId}::access_policy::revoke_policy`,
      arguments: [txb.object(policyId)],
    });

    return await this.client.executeTransaction(txb);
  }

  /**
   * 查詢策略詳情
   */
  async getPolicy(policyId: string): Promise<AccessPolicy | null> {
    try {
      const object = await this.client
        .getSuiClient()
        .getObject({ id: policyId, options: { showContent: true } });

      if (!object.data || !object.data.content || object.data.content.dataType !== 'moveObject') {
        return null;
      }

      const fields = object.data.content.fields as any;

      return {
        policyId,
        blobId: Buffer.from(fields.blob_id).toString(),
        owner: fields.owner,
        allowedReaders: fields.allowed_readers,
        allowedAuditors: fields.allowed_auditors,
        expiryTimestamp: Number(fields.expiry_timestamp),
        isActive: fields.is_active,
      };
    } catch (error) {
      console.error('查詢策略失敗:', error);
      return null;
    }
  }
}
