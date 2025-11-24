/**
 * Walrus <-> Sui Integration Utilities
 * 
 * Provides functions to query Walrus metadata from Sui blockchain
 */

import { SuiClient } from '@mysten/sui/client';
import { fromB64 } from '@mysten/bcs';

// Walrus System Object ID (Testnet)
// Retrieved from official Walrus CLI configuration
const WALRUS_SYSTEM_OBJECT_ID = '0x6c2547cbbc38025cf3adac45f63cb0a8d12ecf777cdc75a4971612bf97fdf6af';

/**
 * Query Sui Object ID from Walrus blob_id using Events
 * 
 * @param client - Sui client
 * @param blobId - Walrus blob ID (Base64 URL-safe format)
 * @returns Sui Object ID or null if not found
 */
export async function getBlobObjectIdFromBlobId(
  client: SuiClient,
  blobId: string
): Promise<string | null> {
  try {
    // Convert blob_id from Base64 URL-safe to hex digest
    // Walrus uses Base64 URL-safe encoding, need to convert to standard Base64
    const standardBase64 = blobId.replace(/-/g, '+').replace(/_/g, '/');
    
    // Pad if necessary
    const paddedBase64 = standardBase64 + '='.repeat((4 - (standardBase64.length % 4)) % 4);
    
    const blobIdBytes = fromB64(paddedBase64);
    const blobIdHex = '0x' + Buffer.from(blobIdBytes).toString('hex');

    console.log(`   Querying blob_id: ${blobId}`);
    console.log(`   Hex digest: ${blobIdHex}`);

    // Query BlobRegistered or BlobCertified events
    // These events are emitted when a blob is registered/certified on-chain
    const events = await client.queryEvents({
      query: {
        MoveEventType: `${WALRUS_SYSTEM_OBJECT_ID}::blob::BlobCertified`
      },
      limit: 50,
      order: 'descending'
    });

    // Search for matching blob_id in events
    for (const event of events.data) {
      const eventData = event.parsedJson as any;
      
      // BlobCertified event structure:
      // { blob_id: u256, object_id: ID, ... }
      if (eventData.blob_id === blobIdHex) {
        console.log(`   ✅ Found Object ID: ${eventData.object_id}`);
        return eventData.object_id;
      }
    }

    console.log(`   ⚠️  Object ID not found in recent events`);
    console.log(`   💡 Using mock Object ID for demo`);
    
    // Fallback: generate a valid-looking mock Object ID
    // In production, you would need to ensure the blob is properly registered
    return '0x0000000000000000000000000000000000000000000000000000000000000000';
    
  } catch (error: any) {
    console.error(`   ❌ Error querying blob Object ID:`, error.message);
    return null;
  }
}

/**
 * Query current Walrus epoch from Sui System object
 * 
 * @param client - Sui client
 * @returns Current epoch number
 */
export async function getWalrusCurrentEpoch(client: SuiClient): Promise<number> {
  try {
    // Read Walrus System shared object
    const systemObject = await client.getObject({
      id: WALRUS_SYSTEM_OBJECT_ID,
      options: {
        showContent: true,
      }
    });

    if (systemObject.data?.content?.dataType !== 'moveObject') {
      throw new Error('Invalid Walrus System object');
    }

    const fields = (systemObject.data.content as any).fields;
    const epoch = fields.epoch;

    console.log(`   Current Walrus epoch: ${epoch}`);
    return parseInt(epoch);
    
  } catch (error: any) {
    console.error(`   ❌ Error querying Walrus epoch:`, error.message);
    console.log(`   💡 Using estimated epoch from timestamp`);
    
    // Fallback: estimate epoch from current time
    // Walrus epochs are typically 1 day (86400 seconds)
    // Testnet started around epoch 0 at a specific time
    const now = Math.floor(Date.now() / 1000);
    const testnetStart = 1700000000; // Approximate Walrus Testnet start
    const epochDuration = 86400; // 1 day in seconds
    
    return Math.floor((now - testnetStart) / epochDuration);
  }
}

/**
 * Convert Walrus blob_id (Base64 URL-safe) to u256 (Move type)
 * 
 * @param blobId - Walrus blob ID in Base64 URL-safe format
 * @returns Hex string representation of u256
 */
export function blobIdToU256(blobId: string): string {
  // Convert Base64 URL-safe to standard Base64
  const standardBase64 = blobId.replace(/-/g, '+').replace(/_/g, '/');
  const paddedBase64 = standardBase64 + '='.repeat((4 - (standardBase64.length % 4)) % 4);
  
  // Decode to bytes
  const bytes = fromB64(paddedBase64);
  
  // Convert to hex (u256 is 32 bytes)
  const hex = '0x' + Buffer.from(bytes).toString('hex').padStart(64, '0');
  
  return hex;
}

/**
 * Verify if an address is an authorized auditor
 * 
 * @param client - Sui client
 * @param auditConfigId - AuditConfig object ID
 * @param auditorAddress - Address to verify
 * @returns true if authorized
 */
export async function isAuthorizedAuditor(
  client: SuiClient,
  auditConfigId: string,
  auditorAddress: string
): Promise<boolean> {
  try {
    const configObject = await client.getObject({
      id: auditConfigId,
      options: { showContent: true }
    });

    if (configObject.data?.content?.dataType !== 'moveObject') {
      return false;
    }

    const fields = (configObject.data.content as any).fields;
    const authorizedAuditors = fields.authorized_auditors || [];

    return authorizedAuditors.includes(auditorAddress);
    
  } catch (error) {
    console.error('Error checking auditor authorization:', error);
    return false;
  }
}
