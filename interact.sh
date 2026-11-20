#!/bin/bash
set -e

# ComplyRail Contracts Interaction Script

# Parse flags out of the positional arguments
DRY_RUN=0
ASSUME_YES=0
POSITIONAL=()
for arg in "$@"; do
  case "$arg" in
    --dry-run)
      DRY_RUN=1
      ;;
    --yes|-y)
      ASSUME_YES=1
      ;;
    *)
      POSITIONAL+=("$arg")
      ;;
  esac
done

NETWORK="${POSITIONAL[0]:-testnet}"
ACTION="${POSITIONAL[1]:-status}"

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
if [ "$DRY_RUN" -eq 1 ]; then
  echo "🧪 Dry run: commands will be printed, not executed"
fi
echo ""

# Confirmation prompt before irreversible public (mainnet) actions
if [ "$NETWORK" = "public" ]; then
  echo "⚠️  WARNING: You are about to perform an action on the PUBLIC (mainnet) network."
  echo "📋 Contract ID: $CONTRACT_ID"
  echo "👤 Admin: ${ADMIN_ADDRESS:-${STELLAR_ACCOUNT:-alice}}"
  echo "🎬 Action: $ACTION"
  echo ""
  if [ "$ASSUME_YES" -eq 1 ]; then
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

# Run a soroban command, or just print it in dry-run mode
run_soroban() {
  if [ "$DRY_RUN" -eq 1 ]; then
    echo "[dry-run] soroban $*"
  else
    soroban "$@"
  fi
}

case "$ACTION" in
  status)
    echo "Getting contract status..."
    run_soroban contract invoke \
      --id "$CONTRACT_ID" \
      --source alice \
      --network "$NETWORK" \
      -- get_contract_info
    ;;

  register-vasp)
    VASP_ADDRESS="${POSITIONAL[2]:-GXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX}"
    JURISDICTION="${POSITIONAL[3]:-US}"
    echo "Registering VASP..."
    echo "Address: $VASP_ADDRESS"
    echo "Jurisdiction: $JURISDICTION"

    run_soroban contract invoke \
      --id "$CONTRACT_ID" \
      --source alice \
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

    run_soroban contract invoke \
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
    ASSET="${POSITIONAL[2]:-GXXXXXXX}"
    JURISDICTION="${POSITIONAL[3]:-US}"
    echo "Getting threshold..."

    run_soroban contract invoke \
      --id "$CONTRACT_ID" \
      --source alice \
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

    run_soroban contract invoke \
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
    echo "Usage: ./interact.sh [--dry-run] [--yes] [testnet|public] [action] [args...]"
    echo ""
    echo "Flags:"
    echo "  --dry-run               - Print soroban commands instead of executing them"
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
    echo "  ./interact.sh --dry-run public set-threshold GXXXXXXX US 100000"
    exit 1
    ;;
esac
