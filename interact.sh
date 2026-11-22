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
  echo "Run: export CONTRACT_ID=<your-contract-id>"
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

# Validate a Stellar strkey by its format: G... for accounts, C... for contracts.
# Usage: validate_strkey <value> <expected-kind: account|contract> <arg-name>
# Note: this checks the format only, not the checksum.
validate_strkey() {
  local value="$1"
  local kind="$2"
  local name="$3"

  if [ -z "$value" ]; then
    echo "❌ Error: missing required argument: $name"
    exit 1
  fi

  case "$kind" in
    account)
      if ! [[ "$value" =~ ^G[A-Z2-7]{55}$ ]]; then
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
    echo ""
    echo "👤 Admins:"
    run_stellar contract invoke \
      --id "$CONTRACT_ID" \
      --source "$SIGNER" \
      --network "$NETWORK" \
      -- get_admins
    echo ""
    echo "🏦 Registered VASPs:"
    run_stellar contract invoke \
      --id "$CONTRACT_ID" \
      --source "$SIGNER" \
      --network "$NETWORK" \
      -- list_vasps
    ;;

  register-vasp)
    CALLER="${POSITIONAL[2]}"
    VASP="${POSITIONAL[3]}"
    NAME="${POSITIONAL[4]}"
    JURISDICTION="${POSITIONAL[5]}"
    PUBLIC_KEY="${POSITIONAL[6]}"
    EXPIRY_DAYS="${POSITIONAL[7]}"

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

    run_stellar contract invoke \
      --id "$CONTRACT_ID" \
      --source "$SIGNER" \
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
    FROM_VASP="${POSITIONAL[2]}"
    TO_VASP="${POSITIONAL[3]}"
    BENEFICIARY="${POSITIONAL[4]}"
    ASSET="${POSITIONAL[5]}"
    AMOUNT="${POSITIONAL[6]}"

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

    run_stellar contract invoke \
      --id "$CONTRACT_ID" \
      --source "$SIGNER" \
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
    ASSET="${POSITIONAL[2]}"
    JURISDICTION="${POSITIONAL[3]}"

    require_arg "$ASSET" "asset"
    require_arg "$JURISDICTION" "jurisdiction"
    validate_strkey "$ASSET" contract asset

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
    ASSET="${POSITIONAL[2]}"
    JURISDICTION="${POSITIONAL[3]}"
    AMOUNT="${POSITIONAL[4]}"

    require_arg "$ASSET" "asset"
    require_arg "$JURISDICTION" "jurisdiction"
    require_arg "$AMOUNT" "amount"
    validate_strkey "$ASSET" contract asset

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
    echo "  --mainnet   - Required for the public network"
    echo "  --dry-run   - Print stellar commands instead of executing them"
    echo "  --yes       - Skip the confirmation prompt for public actions"
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
    echo "  ./interact.sh --dry-run public set-threshold <asset> US 100000 --mainnet"
    exit 1
    ;;
esac
