#!/bin/bash
set -e

# ComplyRail Contracts Interaction Script

# Check Stellar CLI is available
if ! command -v stellar >/dev/null 2>&1; then
  echo "❌ Error: stellar CLI not found"
  echo "Install it from https://developers.stellar.org/docs/tools/developer-tools/cli/stellar-cli"
  exit 1
fi
echo "🔧 $(stellar --version)"

NETWORK="${1:-testnet}"
ACTION="${2:-status}"

# Validate network argument (fail closed)
case "$NETWORK" in
  testnet)
    ;;
  public)
    if [ "$3" != "--mainnet" ]; then
      echo "❌ Error: 'public' requires an explicit --mainnet flag"
      echo "Run: ./interact.sh public --mainnet <action> [args...]"
      exit 1
    fi
    shift
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

if ! stellar keys address "$SIGNER" >/dev/null 2>&1; then
  echo "❌ Error: signer identity '$SIGNER' does not exist"
  echo "Create it with: stellar keys generate $SIGNER"
  exit 1
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
echo ""

case "$ACTION" in
  status)
    echo "Getting contract status..."
    stellar contract invoke \
      --id "$CONTRACT_ID" \
      --source "$SIGNER" \
      --network "$NETWORK" \
      -- get_contract_info
    ;;

  register-vasp)
    VASP_ADDRESS="${3:-GXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX}"
    JURISDICTION="${4:-US}"
    echo "Registering VASP..."
    echo "Address: $VASP_ADDRESS"
    echo "Jurisdiction: $JURISDICTION"

    stellar contract invoke \
      --id "$CONTRACT_ID" \
      --source "$SIGNER" \
      --network "$NETWORK" \
      -- register_vasp \
      --vasp_address "$VASP_ADDRESS" \
      --jurisdiction "$JURISDICTION"

    echo "✅ VASP registered"
    ;;

  submit-payment)
    FROM_VASP="${3:-GXXXXXXX1}"
    TO_VASP="${4:-GXXXXXXX2}"
    AMOUNT="${5:-1000}"
    ASSET="${6:-GXXXXXXX3}"
    DATA_HASH="${7:-0000000000000000000000000000000000000000000000000000000000000000}"

    echo "Submitting payment..."
    echo "From: $FROM_VASP"
    echo "To: $TO_VASP"
    echo "Amount: $AMOUNT"

    stellar contract invoke \
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
    ASSET="${3:-GXXXXXXX}"
    JURISDICTION="${4:-US}"
    echo "Getting threshold..."

    stellar contract invoke \
      --id "$CONTRACT_ID" \
      --source "$SIGNER" \
      --network "$NETWORK" \
      -- get_threshold \
      --asset "$ASSET" \
      --jurisdiction "$JURISDICTION"
    ;;

  set-threshold)
    ASSET="${3:-GXXXXXXX}"
    JURISDICTION="${4:-US}"
    AMOUNT="${5:-100000}"
    echo "Setting threshold..."

    stellar contract invoke \
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
    echo "Usage: ./interact.sh [testnet|public] [--mainnet] [action] [args...]"
    echo ""
    echo "Environment:"
    echo "  CONTRACT_ID - Contract ID to interact with (required)"
    echo "  SIGNER      - Stellar identity name used to sign (required)"
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
    echo "  ./interact.sh public --mainnet status"
    exit 1
    ;;
esac
