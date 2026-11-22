#!/bin/bash
set -e

# ComplyRail Contracts Deployment Script
# Usage: ./deploy.sh [testnet|public] [account_name] [--mainnet] [--dry-run] [--yes]

DRY_RUN=false
ASSUME_YES=false
MAINNET_FLAG=false
POSITIONAL=()

for arg in "$@"; do
  case "$arg" in
    --dry-run)
      DRY_RUN=true
      ;;
    --yes|-y)
      ASSUME_YES=true
      ;;
    --mainnet)
      MAINNET_FLAG=true
      ;;
    *)
      POSITIONAL+=("$arg")
      ;;
  esac
done

NETWORK="${POSITIONAL[0]:-testnet}"
ACCOUNT="${POSITIONAL[1]:-alice}"

# Fail closed: only testnet or public are accepted
if [ "$NETWORK" != "testnet" ] && [ "$NETWORK" != "public" ]; then
  echo "❌ Error: Unknown network '$NETWORK'."
  echo "Usage: ./deploy.sh [testnet|public] [account_name] [--mainnet] [--dry-run] [--yes]"
  exit 1
fi

# Require an explicit --mainnet flag for the public network
if [ "$NETWORK" = "public" ] && [ "$MAINNET_FLAG" != true ]; then
  echo "❌ Error: Deploying to the public network requires the explicit --mainnet flag."
  echo "Usage: ./deploy.sh public [account_name] --mainnet"
  exit 1
fi

# Check the Stellar CLI is available (a dry run does not call it)
if [ "$DRY_RUN" = false ]; then
  if ! command -v stellar &> /dev/null; then
    echo "❌ Error: stellar CLI not found. Please install the Stellar CLI."
    exit 1
  fi
  echo "🔧 Stellar CLI: $(stellar --version)"
  echo ""
fi

# Echo a command instead of running it when in dry-run mode.
run() {
  if [ "$DRY_RUN" = true ]; then
    echo "[dry-run] $*"
  else
    "$@"
  fi
}

echo "🚀 Deploying ComplyRail Contracts to $NETWORK..."
echo "Account: $ACCOUNT"
if [ "$DRY_RUN" = true ]; then
  echo "Mode: dry-run (no commands will be executed)"
fi
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

# Mainnet safety: show what will be affected and require explicit confirmation.
if [ "$NETWORK" = "public" ]; then
  echo "⚠️  You are about to perform an action on the PUBLIC (mainnet) network."
  if [ "$DRY_RUN" = false ]; then
    ADMIN_PREVIEW=$(stellar keys address "$ACCOUNT" --network "$NETWORK" 2>/dev/null || echo "<unknown>")
  else
    ADMIN_PREVIEW="<unknown>"
  fi
  echo "Admin: $ADMIN_PREVIEW"
  if [ "$ASSUME_YES" = true ]; then
    echo "Proceeding without confirmation (--yes)."
  else
    read -r -p "Type the network name (public) to confirm: " CONFIRM
    if [ "$CONFIRM" != "public" ]; then
      echo "❌ Confirmation failed. Aborting."
      exit 1
    fi
  fi
  echo ""
fi
# Pinned build image name and tag
IMAGE="complyrail-build:latest"

# Build contract
echo "📦 Building contract..."
if command -v docker &> /dev/null; then
  if ! docker image inspect "$IMAGE" &> /dev/null; then
    echo "🐳 Build image $IMAGE not found locally, building it..."
    run docker build -t "$IMAGE" .
  fi
  run docker run --rm -v "$(pwd)":/workspace "$IMAGE"
elif command -v cargo &> /dev/null; then
  run cargo build --release --target wasm32v1-none
else
  echo "❌ Error: Neither Docker nor Cargo found. Please install one."
  exit 1
fi

WASM_FILE="target/wasm32v1-none/release/complyrail_contracts.wasm"

if [ "$DRY_RUN" = false ] && [ ! -f "$WASM_FILE" ]; then
  echo "❌ Error: WASM file not found at $WASM_FILE"
  exit 1
fi

echo "✅ Contract built successfully"
echo "📄 WASM: $WASM_FILE"
if [ "$DRY_RUN" = false ]; then
  echo "📊 Size: $(du -h "$WASM_FILE" | cut -f1)"
fi
echo ""

# Deploy contract
echo "🚀 Deploying contract..."
if [ "$DRY_RUN" = true ]; then
  echo "[dry-run] stellar contract deploy --wasm $WASM_FILE --source $ACCOUNT --network $NETWORK"
  CONTRACT_ID="<dry-run>"
else
  DEPLOY_OUTPUT=$(stellar contract deploy \
    --wasm "$WASM_FILE" \
    --source "$ACCOUNT" \
    --network "$NETWORK" \
    2>&1) || true

  # Extract the contract ID (strkey: C followed by 55 base32 chars)
  CONTRACT_ID=$(echo "$DEPLOY_OUTPUT" | grep -oE 'C[A-Z2-7]{55}' | head -n1 || echo "")

  if [ -z "$CONTRACT_ID" ]; then
    echo "❌ Error: Could not find a contract ID in the deployment output."
    echo "⚠️  Deployment output:"
    echo "$DEPLOY_OUTPUT"
    exit 1
  fi
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
if [ "$DRY_RUN" = true ]; then
  echo "[dry-run] stellar keys address $ACCOUNT --network $NETWORK"
  ADMIN_ADDR="<dry-run>"
else
  ADMIN_ADDR=$(stellar keys address "$ACCOUNT" --network "$NETWORK")
fi
echo "Admin address: $ADMIN_ADDR"

run stellar contract invoke \
  --id "$CONTRACT_ID" \
  --source "$ACCOUNT" \
  --network "$NETWORK" \
  -- initialize \
  --admin "$ADMIN_ADDR"

echo "✅ Contract initialized"
echo ""

if [ "$DRY_RUN" = false ]; then
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

  # Record deployment details so the contract ID is not lost
  echo "💾 Recording deployment..."
  DEPLOYMENTS_DIR="deployments"
  mkdir -p "$DEPLOYMENTS_DIR"
  DEPLOYMENT_FILE="$DEPLOYMENTS_DIR/$NETWORK.json"
  WASM_HASH=$(sha256sum "$WASM_FILE" | cut -d' ' -f1)
  GIT_COMMIT=$(git rev-parse HEAD 2>/dev/null || echo "unknown")
  DEPLOYED_AT=$(date -u +"%Y-%m-%dT%H:%M:%SZ")

  cat > "$DEPLOYMENT_FILE" <<JSON
{
  "network": "$NETWORK",
  "contractId": "$CONTRACT_ID",
  "admin": "$ADMIN_ADDR",
  "wasmHash": "$WASM_HASH",
  "commit": "$GIT_COMMIT",
  "deployedAt": "$DEPLOYED_AT"
}
JSON

  echo "✅ Deployment recorded in $DEPLOYMENT_FILE"
  echo ""
fi

echo "🎉 Deployment complete!"
echo ""
echo "Next steps:"
echo "1. Verify contract: https://stellar.expert/explorer/$NETWORK/contract/$CONTRACT_ID"
echo "2. Register VASPs: ./interact.sh $NETWORK register-vasp"
echo "3. Test payments: ./interact.sh $NETWORK submit-payment"
