import { SuiClient } from '@mysten/sui/client';

async function queryFunctionSignature() {
  const client = new SuiClient({ url: 'https://fullnode.testnet.sui.io:443' });

  const pkg = await client.getNormalizedMoveModulesByPackage({
    package: '0x1bc5c277f6c0fd20f97cf555d83ea6f9df753d93fbf99b8890a97df31af21804'
  });

  console.log('包中所有模塊:');
  console.log(Object.keys(pkg));

  console.log('\nauditor_registry 模塊（如果存在）:');
  if (pkg.auditor_registry) {
    console.log(JSON.stringify(pkg.auditor_registry, null, 2));
  } else {
    console.log('不存在');
  }
}

queryFunctionSignature().catch(console.error);
