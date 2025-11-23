#!/bin/bash

# Walrus Audit System 設置腳本

set -e

echo "======================================"
echo "Walrus Audit System 設置"
echo "======================================"

# 檢查依賴
echo ""
echo "檢查依賴..."

if ! command -v cargo &> /dev/null; then
    echo "錯誤: 未安裝 Rust/Cargo"
    echo "請訪問 https://rustup.rs/ 安裝 Rust"
    exit 1
fi

if ! command -v node &> /dev/null; then
    echo "錯誤: 未安裝 Node.js"
    echo "請訪問 https://nodejs.org/ 安裝 Node.js"
    exit 1
fi

if ! command -v sui &> /dev/null; then
    echo "警告: 未安裝 Sui CLI"
    echo "請訪問 https://docs.sui.io/build/install 安裝 Sui CLI"
fi

echo "✓ 依賴檢查完成"

# 構建 Rust 項目
echo ""
echo "構建 Rust 項目..."
cargo build --release
echo "✓ Rust 項目構建完成"

# 安裝 TypeScript 依賴
echo ""
echo "安裝 TypeScript 依賴..."
cd seal-client
npm install
npm run build
cd ..
echo "✓ TypeScript 項目構建完成"

# 構建 Move 合約
if command -v sui &> /dev/null; then
    echo ""
    echo "構建 Move 合約..."

    cd contracts/audit_system
    sui move build
    echo "✓ audit_system 合約構建完成"

    cd ../access_policy
    sui move build
    echo "✓ access_policy 合約構建完成"

    cd ../..
else
    echo ""
    echo "跳過 Move 合約構建（未安裝 Sui CLI）"
fi

# 複製環境配置文件
if [ ! -f .env ]; then
    echo ""
    echo "創建 .env 文件..."
    cp .env.example .env
    echo "✓ .env 文件已創建，請編輯並填入您的配置"
fi

echo ""
echo "======================================"
echo "設置完成！"
echo "======================================"
echo ""
echo "下一步："
echo "1. 編輯 .env 文件，填入您的私鑰和配置"
echo "2. 部署合約: cd contracts/audit_system && sui client publish --gas-budget 100000000"
echo "3. 更新 .env 中的 PACKAGE_ID"
echo "4. 運行審計節點: cargo run --bin auditor-node"
echo ""
