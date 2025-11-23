/**
 * Walrus Seal 操作
 */

import axios from 'axios';
import { UploadResult } from './types';

export class SealOperator {
  private aggregatorUrl: string;

  constructor(aggregatorUrl: string) {
    this.aggregatorUrl = aggregatorUrl;
  }

  /**
   * 上傳數據到 Walrus 並獲取 Seal URL
   */
  async upload(data: Buffer): Promise<UploadResult> {
    try {
      const response = await axios.put(
        `${this.aggregatorUrl}/v1/store`,
        data,
        {
          headers: {
            'Content-Type': 'application/octet-stream',
          },
        }
      );

      const blobId = response.data.newlyCreated?.blobObject?.blobId ||
                     response.data.alreadyCertified?.blobId;

      if (!blobId) {
        throw new Error('上傳失敗：無法獲取 blob ID');
      }

      return {
        blobId,
        sealUrl: `${this.aggregatorUrl}/v1/blob/${blobId}`,
      };
    } catch (error) {
      throw new Error(`Walrus 上傳失敗: ${error}`);
    }
  }

  /**
   * 從 Walrus 下載數據
   */
  async download(blobId: string): Promise<Buffer> {
    try {
      const response = await axios.get(
        `${this.aggregatorUrl}/v1/blob/${blobId}`,
        {
          responseType: 'arraybuffer',
        }
      );

      return Buffer.from(response.data);
    } catch (error) {
      throw new Error(`Walrus 下載失敗: ${error}`);
    }
  }

  /**
   * 獲取 Blob 元數據
   */
  async getBlobMetadata(blobId: string): Promise<any> {
    try {
      const response = await axios.head(
        `${this.aggregatorUrl}/v1/blob/${blobId}`
      );

      return {
        size: response.headers['content-length'],
        contentType: response.headers['content-type'],
        exists: response.status === 200,
      };
    } catch (error) {
      if (axios.isAxiosError(error) && error.response?.status === 404) {
        return { exists: false };
      }
      throw new Error(`獲取 Blob 元數據失敗: ${error}`);
    }
  }

  /**
   * 驗證 Blob 是否存在
   */
  async exists(blobId: string): Promise<boolean> {
    const metadata = await this.getBlobMetadata(blobId);
    return metadata.exists;
  }
}
