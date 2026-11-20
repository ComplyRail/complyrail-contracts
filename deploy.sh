#!/bin/bash
set -e

# ComplyRail Contracts Deployment Script

# Check the Stellar CLI is available
if ! command -v stellar &> /dev/null; then
  echo "❌ Error: stellar CLI not found. Please install the Stellar CLI."
  exit 1
fi
echo "🔧 Stellar CLI: $(stellar --version)"
echo ""

NETWORK="${1:-testnet}"
ACCOUNT="${2:-alice}"
MAINNET_FLAG="${3:-}"

# Fail closed: only testnet or public are accepted
if [ "$NETWORK" != "testnet" ] && [ "$NETWORK" != "public" ]; then
  echo "❌ Error: Unknown network '$NETWORK'."
  echo "Usage: ./deploy.sh [testnet|public] [account_name] [--mainnet]"
  exit 1
fi

# Require an explicit --mainnet flag for the public network
if [ "$NETWORK" = "public" ] && [ "$MAINNET_FLAG" != "--mainnet" ]; then
  echo "❌ Error: Deploying to the public network requires the explicit --mainnet flag."
  echo "Usage: ./deploy.sh public [account_name] --mainnet"
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
CONTRACT_ID=$(stellar contract deploy \
  --wasm "$WASM_FILE" \
  --source "$ACCOUNT" \
  --network "$NETWORK" \
  2>&1 | grep -oP 'CXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX' || echo "")

if [ -z "$CONTRACT_ID" ]; then
  echo "⚠️  Deployment output:"
  stellar contract deploy \
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

# Initialize contract with admin
echo "🔑 Initializing contract with admin..."
ADMIN_ADDR=$(stellar keys address "$ACCOUNT" --network "$NETWORK")
echo "Admin address: $ADMIN_ADDR"

stellar contract invoke \
  --id "$CONTRACT_ID" \
  --source "$ACCOUNT" \
  --network "$NETWORK" \
  -- initialize \
  --admin "$ADMIN_ADDR"

echo "✅ Contract initialized"
echo ""

# Verify the admin was set
echo "🔍 Verifying admin..."
ADMINS=$(stellar contract invoke \
  --id "$CONTRACT_ID" \
  --source "$ACCOUNT" \
  --network "$NETWORK" \
  -- get_admins)

if [ -z "$ADMINS" ]; then
  echo "❌ Error: get_admins returned no admins. Contract may not be initialized."
  exit 1
fi

echo "✅ Admins: $ADMINS"
echo ""
echo "🎉 Deployment complete!"
echo ""
echo "Next steps:"
echo "1. Verify contract: https://stellar.expert/explorer/$NETWORK/contract/$CONTRACT_ID"
echo "2. Register VASPs: ./interact.sh $NETWORK register-vasp"
echo "3. Test payments: ./interact.sh $NETWORK submit-payment"
