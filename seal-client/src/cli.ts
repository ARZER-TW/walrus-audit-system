#!/usr/bin/env node
/**
 * Seal 客戶端命令行工具
 *
 * 用法:
 *   npm run cli encrypt <report.json> [options]
 *   npm run cli decrypt <blob_id> <requester_address> [options]
 *   npm run cli upload <file> [options]
 *   npm run cli download <blob_id> [options]
 *   npm run cli policy <action> [args...]
 */

import * as fs from 'fs';
import * as path from 'path';
import * as dotenv from 'dotenv';
import { AuditReportSealClient, AuditReport } from './audit-report';
import { SealOperator } from './seal';

// 載入環境變量
dotenv.config();

interface CliOptions {
  suiRpc?: string;
  walrusAggregator?: string;
  packageId?: string;
  privateKey?: string;
  readers?: string;
  auditors?: string;
  expiryDays?: string;
  output?: string;
}

function parseArgs(): {
  command: string;
  args: string[];
  options: CliOptions;
} {
  const argv = process.argv.slice(2);
  const command = argv[0] || 'help';
  const args: string[] = [];
  const options: CliOptions = {};

  for (let i = 1; i < argv.length; i++) {
    const arg = argv[i];
    if (arg.startsWith('--')) {
      const key = arg.slice(2);
      const value = argv[i + 1];
      options[key as keyof CliOptions] = value;
      i++;
    } else {
      args.push(arg);
    }
  }

  return { command, args, options };
}

function getConfig(options: CliOptions) {
  return {
    suiRpcUrl: options.suiRpc || process.env.SUI_RPC_URL || 'https://fullnode.testnet.sui.io:443',
    walrusAggregatorUrl: options.walrusAggregator || process.env.WALRUS_AGGREGATOR_URL || 'https://aggregator.walrus-testnet.walrus.space',
    accessPolicyPackageId: options.packageId || process.env.ACCESS_POLICY_PACKAGE_ID || '0x...',
    privateKey: options.privateKey || process.env.PRIVATE_KEY,
  };
}

async function cmdEncrypt(args: string[], options: CliOptions) {
  if (args.length === 0) {
    console.error('錯誤: 請提供報告文件路徑');
    console.log('用法: npm run cli encrypt <report.json> [options]');
    console.log('選項:');
    console.log('  --readers <addr1,addr2>   允許的讀者地址（逗號分隔）');
    console.log('  --auditors <addr1,addr2>  允許的審計員地址（逗號分隔）');
    console.log('  --expiryDays <days>       過期天數（預設 90）');
    console.log('  --output <file>           輸出元數據文件路徑');
    process.exit(1);
  }

  const reportPath = args[0];
  const config = getConfig(options);

  console.log('=== Seal 加密審計報告 ===\n');
  console.log(`報告文件: ${reportPath}`);
  console.log(`Sui RPC: ${config.suiRpcUrl}`);
  console.log(`Walrus: ${config.walrusAggregatorUrl}\n`);

  // 讀取報告
  const reportJson = fs.readFileSync(reportPath, 'utf-8');
  const report: AuditReport = JSON.parse(reportJson);

  // 解析選項
  const allowedReaders = options.readers?.split(',') || [];
  const allowedAuditors = options.auditors?.split(',') || [];
  const expiryDays = parseInt(options.expiryDays || '90', 10);

  console.log(`允許的讀者: ${allowedReaders.length > 0 ? allowedReaders.join(', ') : '(無)'}`);
  console.log(`允許的審計員: ${allowedAuditors.length > 0 ? allowedAuditors.join(', ') : '(無)'}`);
  console.log(`過期天數: ${expiryDays}\n`);

  // 創建客戶端
  const client = new AuditReportSealClient(
    config.suiRpcUrl,
    config.walrusAggregatorUrl,
    config.accessPolicyPackageId,
    config.privateKey
  );

  // 加密並上傳
  const metadata = await client.encryptAndUpload(report, undefined, {
    allowedReaders,
    allowedAuditors,
    expiryDays,
  });

  console.log('\n=== 加密完成 ===');
  console.log(`Blob ID: ${metadata.blob_id}`);
  console.log(`Seal URL: ${metadata.seal_url}`);
  console.log(`Policy ID: ${metadata.policy_id}`);
  console.log(`加密方法: ${metadata.encryption_method}`);

  // 保存元數據
  const outputPath = options.output || `${reportPath}.encrypted.json`;
  fs.writeFileSync(outputPath, JSON.stringify(metadata, null, 2));
  console.log(`\n元數據已保存: ${outputPath}`);
}

async function cmdDecrypt(args: string[], options: CliOptions) {
  if (args.length < 2) {
    console.error('錯誤: 請提供 blob_id 和 requester_address');
    console.log('用法: npm run cli decrypt <blob_id> <requester_address> [options]');
    console.log('選項:');
    console.log('  --output <file>  輸出報告文件路徑');
    process.exit(1);
  }

  const [blobId, requesterAddress] = args;
  const config = getConfig(options);

  console.log('=== Seal 解密審計報告 ===\n');
  console.log(`Blob ID: ${blobId}`);
  console.log(`請求者: ${requesterAddress}`);
  console.log(`Sui RPC: ${config.suiRpcUrl}`);
  console.log(`Walrus: ${config.walrusAggregatorUrl}\n`);

  // 創建客戶端
  const client = new AuditReportSealClient(
    config.suiRpcUrl,
    config.walrusAggregatorUrl,
    config.accessPolicyPackageId,
    config.privateKey
  );

  // 下載並解密
  const report = await client.downloadAndDecrypt(blobId, requesterAddress);

  console.log('\n=== 解密完成 ===');
  console.log(`Blob ID: ${report.blob_id}`);
  console.log(`審計員: ${report.auditor}`);
  console.log(`挑戰數量: ${report.total_challenges}`);
  console.log(`成功驗證: ${report.successful_verifications}`);
  console.log(`是否有效: ${report.is_valid}`);

  // 保存報告
  const outputPath = options.output || `decrypted_${blobId}.json`;
  fs.writeFileSync(outputPath, JSON.stringify(report, null, 2));
  console.log(`\n報告已保存: ${outputPath}`);
}

async function cmdUpload(args: string[], options: CliOptions) {
  if (args.length === 0) {
    console.error('錯誤: 請提供文件路徑');
    console.log('用法: npm run cli upload <file>');
    process.exit(1);
  }

  const filePath = args[0];
  const config = getConfig(options);

  console.log('=== 上傳到 Walrus ===\n');
  console.log(`文件: ${filePath}`);
  console.log(`Walrus: ${config.walrusAggregatorUrl}\n`);

  const data = fs.readFileSync(filePath);
  const operator = new SealOperator(config.walrusAggregatorUrl);

  const result = await operator.upload(data);

  console.log('\n=== 上傳完成 ===');
  console.log(`Blob ID: ${result.blobId}`);
  console.log(`URL: ${result.sealUrl}`);
}

async function cmdDownload(args: string[], options: CliOptions) {
  if (args.length === 0) {
    console.error('錯誤: 請提供 blob_id');
    console.log('用法: npm run cli download <blob_id> [--output <file>]');
    process.exit(1);
  }

  const blobId = args[0];
  const config = getConfig(options);

  console.log('=== 從 Walrus 下載 ===\n');
  console.log(`Blob ID: ${blobId}`);
  console.log(`Walrus: ${config.walrusAggregatorUrl}\n`);

  const operator = new SealOperator(config.walrusAggregatorUrl);

  const data = await operator.download(blobId);

  console.log(`\n已下載: ${data.length} bytes`);

  const outputPath = options.output || `downloaded_${blobId}`;
  fs.writeFileSync(outputPath, data);
  console.log(`已保存: ${outputPath}`);
}

async function cmdPolicy(args: string[], options: CliOptions) {
  if (args.length === 0) {
    console.error('錯誤: 請提供策略操作');
    console.log('用法:');
    console.log('  npm run cli policy create <blob_id> [--readers <addrs>] [--auditors <addrs>] [--expiryDays <days>]');
    console.log('  npm run cli policy grant <policy_id> <recipient> <type>');
    console.log('  npm run cli policy revoke <policy_id>');
    console.log('  npm run cli policy get <policy_id>');
    process.exit(1);
  }

  const action = args[0];
  const config = getConfig(options);

  const client = new AuditReportSealClient(
    config.suiRpcUrl,
    config.walrusAggregatorUrl,
    config.accessPolicyPackageId,
    config.privateKey
  );

  const policyManager = client.getPolicyManager();

  switch (action) {
    case 'create':
      // ... (實現策略創建)
      console.log('策略創建功能待實現');
      break;
    case 'grant':
      // ... (實現權限授予)
      console.log('權限授予功能待實現');
      break;
    case 'revoke':
      // ... (實現策略撤銷)
      console.log('策略撤銷功能待實現');
      break;
    case 'get':
      if (args.length < 2) {
        console.error('錯誤: 請提供 policy_id');
        process.exit(1);
      }
      const policyId = args[1];
      const policy = await policyManager.getPolicy(policyId);
      if (policy) {
        console.log('\n=== 策略詳情 ===');
        console.log(JSON.stringify(policy, null, 2));
      } else {
        console.log('策略不存在或無法訪問');
      }
      break;
    default:
      console.error(`未知操作: ${action}`);
      process.exit(1);
  }
}

function cmdHelp() {
  console.log(`
Seal 客戶端命令行工具

用法:
  npm run cli <command> [args...] [options]

命令:

  encrypt <report.json>
    加密審計報告並上傳到 Walrus
    選項:
      --readers <addr1,addr2>   允許的讀者地址（逗號分隔）
      --auditors <addr1,addr2>  允許的審計員地址（逗號分隔）
      --expiryDays <days>       過期天數（預設 90）
      --output <file>           輸出元數據文件路徑

  decrypt <blob_id> <requester_address>
    從 Walrus 下載並解密審計報告
    選項:
      --output <file>  輸出報告文件路徑

  upload <file>
    上傳文件到 Walrus

  download <blob_id>
    從 Walrus 下載文件
    選項:
      --output <file>  輸出文件路徑

  policy <action> [args...]
    管理訪問策略
    操作:
      create <blob_id>       創建策略
      grant <policy_id> ...  授予權限
      revoke <policy_id>     撤銷策略
      get <policy_id>        查詢策略

  help
    顯示此幫助信息

全局選項:
  --suiRpc <url>              Sui RPC URL
  --walrusAggregator <url>    Walrus Aggregator URL
  --packageId <id>            Access Policy Package ID
  --privateKey <key>          私鑰（十六進制）

環境變量:
  SUI_RPC_URL                 Sui RPC URL
  WALRUS_AGGREGATOR_URL       Walrus Aggregator URL
  ACCESS_POLICY_PACKAGE_ID    Access Policy Package ID
  PRIVATE_KEY                 私鑰

示例:
  # 加密報告
  npm run cli encrypt report.json --readers 0x123,0x456 --auditors 0x789

  # 解密報告
  npm run cli decrypt abc123 0x456 --output decrypted.json

  # 上傳文件
  npm run cli upload data.bin

  # 下載文件
  npm run cli download abc123 --output data.bin
`);
}

async function main() {
  const { command, args, options } = parseArgs();

  try {
    switch (command) {
      case 'encrypt':
        await cmdEncrypt(args, options);
        break;
      case 'decrypt':
        await cmdDecrypt(args, options);
        break;
      case 'upload':
        await cmdUpload(args, options);
        break;
      case 'download':
        await cmdDownload(args, options);
        break;
      case 'policy':
        await cmdPolicy(args, options);
        break;
      case 'help':
      default:
        cmdHelp();
        break;
    }
  } catch (error) {
    console.error('\n❌ 錯誤:', error instanceof Error ? error.message : error);
    process.exit(1);
  }
}

// 執行
if (require.main === module) {
  main().catch((error) => {
    console.error('Fatal error:', error);
    process.exit(1);
  });
}
