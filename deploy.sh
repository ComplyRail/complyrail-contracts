#!/bin/bash
set -e

# ComplyRail Contracts Deployment Script

NETWORK="${1:-testnet}"
ACCOUNT="${2:-alice}"

if [ "$NETWORK" != "testnet" ] && [ "$NETWORK" != "public" ]; then
  echo "Usage: ./deploy.sh [testnet|public] [account_name]"
  exit 1
fi

echo "🚀 Deploying ComplyRail Contracts to $NETWORK..."
echo "Account: $ACCOUNT"
echo ""

# Set network variables
if [ "$NETWORK" = "testnet" ]; then
  export SOROBAN_RPC_URL="https://soroban-testnet.stellar.org"
  export SOROBAN_NETWORK_PASSPHRASE="Test SDF Network ; September 2015"
  echo "🔗 Network: https://soroban-testnet.stellar.org"
  echo "📊 Explorer: https://testnet.stellar.expert/"
else
  export SOROBAN_RPC_URL="https://soroban.stellar.org"
  export SOROBAN_NETWORK_PASSPHRASE="Public Global Stellar Network ; September 2015"
  echo "🔗 Network: https://soroban.stellar.org"
  echo "📊 Explorer: https://stellar.expert/"
fi

echo ""

# Build contract
echo "📦 Building contract..."
if command -v docker &> /dev/null; then
  docker run --rm -v $(pwd):/workspace complyrail-build:latest
elif command -v cargo &> /dev/null; then
  cargo build --release --target wasm32-unknown-unknown
else
  echo "❌ Error: Neither Docker nor Cargo found. Please install one."
  exit 1
fi

WASM_FILE="target/wasm32-unknown-unknown/release/complyrail_contracts.wasm"

if [ ! -f "$WASM_FILE" ]; then
  echo "❌ Error: WASM file not found at $WASM_FILE"
  exit 1
fi

echo "✅ Contract built successfully"
echo "📄 WASM: $WASM_FILE"
echo "📊 Size: $(du -h $WASM_FILE | cut -f1)"
echo ""

# Deploy contract
echo "🚀 Deploying contract..."
CONTRACT_ID=$(soroban contract deploy \
  --wasm "$WASM_FILE" \
  --source "$ACCOUNT" \
  --network "$NETWORK" \
  2>&1 | grep -oP 'CXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX' || echo "")

if [ -z "$CONTRACT_ID" ]; then
  echo "⚠️  Deployment output:"
  soroban contract deploy \
    --wasm "$WASM_FILE" \
    --source "$ACCOUNT" \
    --network "$NETWORK"
  exit 1
fi

echo "✅ Contract deployed!"
echo ""
echo "📋 Contract ID: $CONTRACT_ID"
echo ""
echo "💾 Save this for future interactions:"
echo "export CONTRACT_ID=$CONTRACT_ID"
echo ""

# Initialize admin
echo "🔑 Setting up admin..."
ADMIN_ADDR=$(soroban keys address "$ACCOUNT" --network "$NETWORK")
echo "Admin address: $ADMIN_ADDR"

soroban contract invoke \
  --id "$CONTRACT_ID" \
  --source "$ACCOUNT" \
  --network "$NETWORK" \
  -- set_admin \
  --admin "$ADMIN_ADDR"

echo "✅ Admin configured"
echo ""
echo "🎉 Deployment complete!"
echo ""
echo "Next steps:"
echo "1. Verify contract: https://stellar.expert/explorer/$NETWORK/contract/$CONTRACT_ID"
echo "2. Register VASPs: ./interact.sh $NETWORK register-vasp"
echo "3. Test payments: ./interact.sh $NETWORK submit-payment"
