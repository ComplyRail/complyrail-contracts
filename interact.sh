#!/bin/bash
set -e

# ComplyRail Contracts Interaction Script

NETWORK="${1:-testnet}"
ACTION="${2:-status}"

if [ -z "$CONTRACT_ID" ]; then
  echo "❌ Error: CONTRACT_ID not set"
  echo "Run: export CONTRACT_ID=<your-contract-id>"
  exit 1
fi

# Validate a Stellar strkey (checksum + version byte) using the stellar CLI.
# Usage: validate_strkey <value> <expected-kind: account|contract> <arg-name>
validate_strkey() {
  local value="$1"
  local kind="$2"
  local name="$3"

  if [ -z "$value" ]; then
    echo "❌ Error: missing required argument: $name"
    exit 1
  fi

  if ! command -v stellar >/dev/null 2>&1; then
    echo "❌ Error: 'stellar' CLI not found; cannot validate $name"
    exit 1
  fi

  local detected
  detected="$(stellar keys address "$value" 2>/dev/null || true)"

  case "$kind" in
    account)
      if [ -z "$detected" ]; then
        echo "❌ Error: '$value' is not a valid account strkey ($name)"
        exit 1
      fi
      ;;
    contract)
      if ! [[ "$value" =~ ^C[A-Z2-7]{55}$ ]]; then
        echo "❌ Error: '$value' is not a valid contract strkey ($name)"
        exit 1
      fi
      ;;
    *)
      echo "❌ Error: unknown strkey kind '$kind'"
      exit 1
      ;;
  esac
}

# Require a non-empty argument. Usage: require_arg <value> <arg-name>
require_arg() {
  if [ -z "$1" ]; then
    echo "❌ Error: missing required argument: $2"
    exit 1
  fi
}

validate_strkey "$CONTRACT_ID" contract CONTRACT_ID
=======

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
    echo ""
    echo "👤 Admins:"
    soroban contract invoke \
      --id "$CONTRACT_ID" \
      --source alice \
      --network "$NETWORK" \
      -- get_admins
    echo ""
    echo "🏦 Registered VASPs:"
    soroban contract invoke \
      --id "$CONTRACT_ID" \
      --source alice \
      --network "$NETWORK" \
      -- list_vasps
    ;;

  register-vasp)
    CALLER="${3}"
    VASP="${4}"
    NAME="${5}"
    JURISDICTION="${6}"
    PUBLIC_KEY="${7}"
    EXPIRY_DAYS="${8}"

    require_arg "$CALLER" "caller"
    require_arg "$VASP" "vasp"
    require_arg "$NAME" "name"
    require_arg "$JURISDICTION" "jurisdiction"
    require_arg "$PUBLIC_KEY" "public_key"
    require_arg "$EXPIRY_DAYS" "expiry_days"

    validate_strkey "$CALLER" account caller
    validate_strkey "$VASP" account vasp

    echo "Registering VASP..."
    echo "Caller: $CALLER"
    echo "VASP: $VASP"
    echo "Name: $NAME"
    echo "Jurisdiction: $JURISDICTION"
    echo "Public key: $PUBLIC_KEY"
    echo "Expiry days: $EXPIRY_DAYS"

    soroban contract invoke \
      --id "$CONTRACT_ID" \
      --source alice \
      --network "$NETWORK" \
      -- register_vasp \
      --caller "$CALLER" \
      --vasp "$VASP" \
      --name "$NAME" \
      --jurisdiction "$JURISDICTION" \
      --public_key "$PUBLIC_KEY" \
      --expiry_days "$EXPIRY_DAYS"

    echo "✅ VASP registered"
    ;;

  submit-payment)
    FROM_VASP="${3}"
    TO_VASP="${4}"
    BENEFICIARY="${5}"
    ASSET="${6}"
    AMOUNT="${7}"

    require_arg "$FROM_VASP" "from_vasp"
    require_arg "$TO_VASP" "to_vasp"
    require_arg "$BENEFICIARY" "beneficiary"
    require_arg "$ASSET" "asset"
    require_arg "$AMOUNT" "amount"

    validate_strkey "$FROM_VASP" account from_vasp
    validate_strkey "$TO_VASP" account to_vasp
    validate_strkey "$BENEFICIARY" account beneficiary
    validate_strkey "$ASSET" contract asset

    echo "Submitting payment..."
    echo "From: $FROM_VASP"
    echo "To: $TO_VASP"
    echo "Beneficiary: $BENEFICIARY"
    echo "Asset: $ASSET"
    echo "Amount: $AMOUNT"

    soroban contract invoke \
      --id "$CONTRACT_ID" \
      --source alice \
      --network "$NETWORK" \
      -- submit_payment \
      --from_vasp "$FROM_VASP" \
      --to_vasp "$TO_VASP" \
      --beneficiary "$BENEFICIARY" \
      --asset "$ASSET" \
      --amount "$AMOUNT"

    echo "✅ Payment submitted"
    ;;

  get-threshold)
    ASSET="${3}"
    JURISDICTION="${4}"

    require_arg "$ASSET" "asset"
    require_arg "$JURISDICTION" "jurisdiction"
    validate_strkey "$ASSET" contract asset

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
    ASSET="${3}"
    JURISDICTION="${4}"
    AMOUNT="${5}"

    require_arg "$ASSET" "asset"
    require_arg "$JURISDICTION" "jurisdiction"
    require_arg "$AMOUNT" "amount"
    validate_strkey "$ASSET" contract asset

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
    echo "  status                  - Get contract info (admins and registered VASPs)"
    echo "  register-vasp <caller> <vasp> <name> <jurisdiction> <public_key> <expiry_days> - Register VASP"
    echo "  submit-payment <from_vasp> <to_vasp> <beneficiary> <asset> <amount> - Submit payment"
    echo "  get-threshold <asset> <jurisdiction> - Get threshold"
    echo "  set-threshold <asset> <jurisdiction> <amount> - Set threshold"
    echo ""
    echo "All address arguments are required and validated as Stellar strkeys."
    echo ""
    echo "Examples:"
    echo "  ./interact.sh testnet status"
    echo "  ./interact.sh testnet register-vasp <caller> <vasp> \"Acme Corp\" US <public_key> 365"
    echo "  ./interact.sh testnet submit-payment <from_vasp> <to_vasp> <beneficiary> <asset> 5000"
    exit 1
    ;;
esac
