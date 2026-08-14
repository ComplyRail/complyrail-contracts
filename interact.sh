#!/bin/bash
set -e

# ComplyRail Contracts Interaction Script

NETWORK="${1:-testnet}"
ACTION="${2:-status}"

if [ -z "$CONTRACT_ID" ]; then
  echo "❌ Error: CONTRACT_ID not set"
  echo "Run: export CONTRACT_ID=CXXXXXXX..."
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
echo ""

case "$ACTION" in
  status)
    echo "Getting contract status..."
    soroban contract invoke \
      --id "$CONTRACT_ID" \
      --source alice \
      --network "$NETWORK" \
      -- get_contract_info
    ;;

  register-vasp)
    VASP_ADDRESS="${3:-GXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX}"
    JURISDICTION="${4:-US}"
    echo "Registering VASP..."
    echo "Address: $VASP_ADDRESS"
    echo "Jurisdiction: $JURISDICTION"

    soroban contract invoke \
      --id "$CONTRACT_ID" \
      --source alice \
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

    soroban contract invoke \
      --id "$CONTRACT_ID" \
      --source alice \
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

    soroban contract invoke \
      --id "$CONTRACT_ID" \
      --source alice \
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

    soroban contract invoke \
      --id "$CONTRACT_ID" \
      --source alice \
      --network "$NETWORK" \
      -- set_threshold \
      --asset "$ASSET" \
      --jurisdiction "$JURISDICTION" \
      --amount "$AMOUNT"

    echo "✅ Threshold set"
    ;;

  *)
    echo "Usage: ./interact.sh [testnet|public] [action] [args...]"
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
    exit 1
    ;;
esac
