#!/bin/bash
set -e

# ComplyRail Contracts Interaction Script
# Usage: ./interact.sh [testnet|public] [action] [args...] [--mainnet] [--dry-run] [--yes]

# Parse flags out of the positional arguments
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
ACTION="${POSITIONAL[1]:-status}"

# Validate network argument (fail closed)
case "$NETWORK" in
  testnet)
    ;;
  public)
    if [ "$MAINNET_FLAG" != true ]; then
      echo "❌ Error: 'public' requires an explicit --mainnet flag"
      echo "Run: ./interact.sh public <action> [args...] --mainnet"
      exit 1
    fi
    ;;
  *)
    echo "❌ Error: invalid network '$NETWORK'"
    echo "Accepted values: testnet, public"
    exit 1
    ;;
esac

if [ -z "$CONTRACT_ID" ]; then
  echo "❌ Error: CONTRACT_ID not set"
  echo "Run: export CONTRACT_ID=CXXXXXXX..."
  exit 1
fi

# Require an explicit signer identity
if [ -z "$SIGNER" ]; then
  echo "❌ Error: SIGNER not set"
  echo "Run: export SIGNER=<stellar identity name>"
  exit 1
fi

if [ "$DRY_RUN" = false ]; then
  # Check Stellar CLI is available (a dry run does not call it)
  if ! command -v stellar >/dev/null 2>&1; then
    echo "❌ Error: stellar CLI not found"
    echo "Install it from https://developers.stellar.org/docs/tools/developer-tools/cli/stellar-cli"
    exit 1
  fi
  echo "🔧 $(stellar --version)"

  if ! stellar keys address "$SIGNER" >/dev/null 2>&1; then
    echo "❌ Error: signer identity '$SIGNER' does not exist"
    echo "Create it with: stellar keys generate $SIGNER"
    exit 1
  fi
fi

# Set network
if [ "$NETWORK" = "testnet" ]; then
  export SOROBAN_RPC_URL="https://soroban-testnet.stellar.org"
  export SOROBAN_NETWORK_PASSPHRASE="Test SDF Network ; September 2015"
else
  export SOROBAN_RPC_URL="https://soroban.stellar.org"
  export SOROBAN_NETWORK_PASSPHRASE="Public Global Stellar Network ; September 2015"
fi

echo "📋 Contract: $CONTRACT_ID"
echo "🌐 Network: $NETWORK"
echo "🔑 Signer: $SIGNER"
if [ "$DRY_RUN" = true ]; then
  echo "🧪 Dry run: commands will be printed, not executed"
fi
echo ""

# Confirmation prompt before irreversible public (mainnet) actions
if [ "$NETWORK" = "public" ]; then
  echo "⚠️  WARNING: You are about to perform an action on the PUBLIC (mainnet) network."
  echo "📋 Contract ID: $CONTRACT_ID"
  echo "👤 Signer: $SIGNER"
  echo "🎬 Action: $ACTION"
  echo ""
  if [ "$ASSUME_YES" = true ]; then
    echo "✅ --yes passed, skipping confirmation"
  else
    printf "Type 'public' to confirm: "
    read -r CONFIRMATION
    if [ "$CONFIRMATION" != "public" ]; then
      echo "❌ Confirmation failed (got '$CONFIRMATION'). Aborting."
      exit 1
    fi
  fi
  echo ""
fi

# Run a stellar command, or just print it in dry-run mode
run_stellar() {
  if [ "$DRY_RUN" = true ]; then
    echo "[dry-run] stellar $*"
  else
    stellar "$@"
  fi
}

case "$ACTION" in
  status)
    echo "Getting contract status..."
    run_stellar contract invoke \
      --id "$CONTRACT_ID" \
      --source "$SIGNER" \
      --network "$NETWORK" \
      -- get_contract_info
    ;;

  register-vasp)
    VASP_ADDRESS="${POSITIONAL[2]:-GXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX}"
    JURISDICTION="${POSITIONAL[3]:-US}"
    echo "Registering VASP..."
    echo "Address: $VASP_ADDRESS"
    echo "Jurisdiction: $JURISDICTION"

    run_stellar contract invoke \
      --id "$CONTRACT_ID" \
      --source "$SIGNER" \
      --network "$NETWORK" \
      -- register_vasp \
      --vasp_address "$VASP_ADDRESS" \
      --jurisdiction "$JURISDICTION"

    echo "✅ VASP registered"
    ;;

  submit-payment)
    FROM_VASP="${POSITIONAL[2]:-GXXXXXXX1}"
    TO_VASP="${POSITIONAL[3]:-GXXXXXXX2}"
    AMOUNT="${POSITIONAL[4]:-1000}"
    ASSET="${POSITIONAL[5]:-GXXXXXXX3}"
    DATA_HASH="${POSITIONAL[6]:-0000000000000000000000000000000000000000000000000000000000000000}"

    echo "Submitting payment..."
    echo "From: $FROM_VASP"
    echo "To: $TO_VASP"
    echo "Amount: $AMOUNT"

    run_stellar contract invoke \
      --id "$CONTRACT_ID" \
      --source "$SIGNER" \
      --network "$NETWORK" \
      -- submit_payment \
      --from_vasp "$FROM_VASP" \
      --to_vasp "$TO_VASP" \
      --amount "$AMOUNT" \
      --asset "$ASSET" \
      --data_hash "$DATA_HASH"

    echo "✅ Payment submitted"
    ;;

  get-threshold)
    ASSET="${POSITIONAL[2]:-GXXXXXXX}"
    JURISDICTION="${POSITIONAL[3]:-US}"
    echo "Getting threshold..."

    run_stellar contract invoke \
      --id "$CONTRACT_ID" \
      --source "$SIGNER" \
      --network "$NETWORK" \
      -- get_threshold \
      --asset "$ASSET" \
      --jurisdiction "$JURISDICTION"
    ;;

  set-threshold)
    ASSET="${POSITIONAL[2]:-GXXXXXXX}"
    JURISDICTION="${POSITIONAL[3]:-US}"
    AMOUNT="${POSITIONAL[4]:-100000}"
    echo "Setting threshold..."

    run_stellar contract invoke \
      --id "$CONTRACT_ID" \
      --source "$SIGNER" \
      --network "$NETWORK" \
      -- set_threshold \
      --asset "$ASSET" \
      --jurisdiction "$JURISDICTION" \
      --amount "$AMOUNT"

    echo "✅ Threshold set"
    ;;

  *)
    echo "Usage: ./interact.sh [testnet|public] [action] [args...] [--mainnet] [--dry-run] [--yes]"
    echo ""
    echo "Environment:"
    echo "  CONTRACT_ID - Contract ID to interact with (required)"
    echo "  SIGNER      - Stellar identity name used to sign (required)"
    echo ""
    echo "Flags:"
    echo "  --mainnet               - Required for the public network"
    echo "  --dry-run               - Print stellar commands instead of executing them"
    echo "  --yes                   - Skip the confirmation prompt for public actions"
    echo ""
    echo "Actions:"
    echo "  status                  - Get contract info"
    echo "  register-vasp [address] [jurisdiction] - Register VASP"
    echo "  submit-payment [from] [to] [amount] [asset] [hash] - Submit payment"
    echo "  get-threshold [asset] [jurisdiction] - Get threshold"
    echo "  set-threshold [asset] [jurisdiction] [amount] - Set threshold"
    echo ""
    echo "Examples:"
    echo "  ./interact.sh testnet status"
    echo "  ./interact.sh testnet register-vasp GXXXXXXX US"
    echo "  ./interact.sh testnet submit-payment GXXXXXXX1 GXXXXXXX2 5000"
    echo "  ./interact.sh --dry-run public set-threshold GXXXXXXX US 100000 --mainnet"
    echo "  ./interact.sh public status --mainnet"
    exit 1
    ;;
esac
