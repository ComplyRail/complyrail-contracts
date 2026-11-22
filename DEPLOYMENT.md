# ComplyRail Contracts - Stellar Deployment Guide

Deploy the ComplyRail smart contracts to Stellar testnet or mainnet.

## Prerequisites

### Install Tools

**Using Docker (Recommended):**
```bash
# Build the pinned build image (must match the tag used by deploy.sh)
docker build -t complyrail-build:latest .

# Build contract in Docker
docker run --rm -v $(pwd):/workspace complyrail-build:latest
```

The image name and tag are pinned to `complyrail-build:latest`; `deploy.sh` runs
`docker run ... complyrail-build:latest`, so the image must be built locally (or
pulled from the registry) before running the script. CI builds this image on every
push to catch breakage.

**Or locally:**
```bash
# Install Rust and Stellar CLI
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
rustup toolchain install 1.85.0
rustup default 1.85.0
rustup target add wasm32v1-none

# Install Stellar CLI
cargo install --locked stellar-cli --version 22.8.1
```

### Set Up Wallet

```bash
# Generate new keypair (save the secret key securely!)
stellar keys generate --network testnet alice
```

Or use existing secret key:
```bash
stellar keys add alice --secret-key SXXX...
```

The deployment scripts check the Stellar CLI version at startup and abort if
`stellar` is not installed or too old. Upgrade with
`cargo install --locked stellar-cli --force` if the check fails.

## Deployment Steps

### 1. Build the Contract

```bash
# Build the pinned image first (required by deploy.sh)
docker build -t complyrail-build:latest .

# Docker build
docker run --rm -v $(pwd):/workspace complyrail-build:latest

# Or with Rust installed locally
cargo build --release --target wasm32v1-none
```

Output: `target/wasm32v1-none/release/complyrail_contracts.wasm`

### 1a. Reproducible Build Verification

Builds of the same commit must produce byte-identical WASM. CI builds the contract
twice and compares the SHA-256 of both artifacts; the resulting hash is published in
the release notes.

To verify locally:

```bash
# Build twice from a clean tree
cargo clean && cargo build --release --target wasm32v1-none
sha256sum target/wasm32v1-none/release/complyrail_contracts.wasm > /tmp/hash1

cargo clean && cargo build --release --target wasm32v1-none
sha256sum target/wasm32v1-none/release/complyrail_contracts.wasm > /tmp/hash2

# The two hashes must match
diff /tmp/hash1 /tmp/hash2 && echo "Reproducible build OK"
```

To check the deployed WASM against the published release hash:

```bash
# Fetch the on-chain WASM for the deployed contract and hash it
stellar contract fetch --id $CONTRACT_ID --network testnet \
  --out /tmp/deployed.wasm
sha256sum /tmp/deployed.wasm

# Compare with the hash published in the release notes for the same commit
```

### 2. Deploy to Testnet

```bash
# Set network
export SOROBAN_RPC_URL=https://soroban-testnet.stellar.org
export SOROBAN_NETWORK_PASSPHRASE="Test SDF Network ; September 2015"

# Deploy (keep the contract ID!)
CONTRACT_ID=$(stellar contract deploy \
  --wasm target/wasm32v1-none/release/complyrail_contracts.wasm \
  --source alice \
  --network testnet)

echo "Contract ID: $CONTRACT_ID"
```

### 3. Initialize Contract

```bash
# Set the contract admin
stellar contract invoke \
  --id $CONTRACT_ID \
  --source alice \
  --network testnet \
  -- initialize \
  --admin "GXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX"
```

### Network Argument Handling

The scripts accept only `testnet` or `public` as the network argument and
reject anything else. Deploying to `public` (mainnet) additionally requires
the `--mainnet` flag as an explicit safety confirmation:

```bash
./deploy.sh testnet
./deploy.sh public --mainnet
```

## Interacting with Contract

`interact.sh` signs every invocation with the identity named in the
`STELLAR_SIGNER` environment variable. There is no default: the script exits
with an error if `STELLAR_SIGNER` is unset, and also fails if the named
identity does not exist in the local Stellar CLI keystore.

```bash
# Required: the identity used to sign every invocation
export STELLAR_SIGNER=alice
```

### Register a VASP

```bash
stellar contract invoke \
  --id $CONTRACT_ID \
  --source $STELLAR_SIGNER \
  --network testnet \
  -- register_vasp \
  --vasp_address "GXXXXXXX..." \
  --jurisdiction "US"
```

### Submit a Payment

```bash
stellar contract invoke \
  --id $CONTRACT_ID \
  --source $STELLAR_SIGNER \
  --network testnet \
  -- submit_payment \
  --from_vasp "GXXXXXXX..." \
  --to_vasp "GYYYYYYY..." \
  --amount "1000" \
  --asset "GXXXXXXX..." \
  --data_hash "abc123..."
```

### Check Payment Status

```bash
stellar contract invoke \
  --id $CONTRACT_ID \
  --source $STELLAR_SIGNER \
  --network testnet \
  -- get_payment_status \
  --payment_id "payment_123"
```

## Contract Functions

### Admin Functions
- `initialize(admin: Address)` - Set the contract admin
- `set_threshold(asset, jurisdiction, amount)` - Set compliance threshold
- `release_payment(payment_id)` - Approve payment
- `reject_payment(payment_id, reason)` - Reject payment

### User Functions
- `register_vasp(address, jurisdiction)` - Register VASP
- `submit_payment(from, to, amount, asset, data_hash)` - Submit payment
- `submit_attestation(payment_id, ivms_hash)` - Submit IVMS101 attestation

### Query Functions
- `get_vasp_status(address)` - Get VASP details
- `get_payment_status(id)` - Get payment details
- `get_threshold(asset, jurisdiction)` - Get threshold for asset/jurisdiction

## Security Checklist

- [ ] Generate new keypair for testnet testing
- [ ] Fund testnet account: https://friendbot.stellar.org
- [ ] Test all contract functions on testnet first
- [ ] Verify contract bytecode before mainnet deployment:
  ```bash
  # Compute the SHA-256 hash of the local WASM binary
  sha256sum target/wasm32v1-none/release/complyrail_contracts.wasm

  # Fetch the on-chain WASM hash for the deployed contract
  stellar contract info --id $CONTRACT_ID --network testnet | grep wasm_hash

  # The two hashes must match before proceeding to mainnet.
  # For reproducible builds, see the Docker build instructions above (Prerequisites).
  ```
- [ ] Confirm the deployed WASM SHA-256 matches the hash published in the release notes
- [ ] Multi-sig setup for critical operations (recommended)
- [ ] Audit contract code before mainnet
- [ ] Use testnet for 7+ days to verify stability

## Mainnet Deployment

Same process as testnet, but:

```bash
export SOROBAN_RPC_URL=https://soroban.stellar.org
export SOROBAN_NETWORK_PASSPHRASE="Public Global Stellar Network ; September 2015"

# Deploy with formal audit approval
stellar contract deploy \
  --wasm target/wasm32v1-none/release/complyrail_contracts.wasm \
  --source mainnet_admin \
  --network public
```

Mainnet deployments must pass `--mainnet` to the scripts as an explicit
confirmation; without it the scripts refuse to target `public`.

## Rollback and Upgrade

The ComplyRail contract is **not upgradeable**. There is no on-chain upgrade function; once deployed, the bytecode is fixed.

### If a bug is found after deployment

Because the contract cannot be patched in place, the recovery path is a full migration to a new contract:

1. **Deploy a fixed contract** following the deployment steps above.
2. **Pause new payments** in the old contract by removing registered VASPs or by stopping the off-chain systems that call it (there is no on-chain pause function).
3. **Identify in-flight payments** that are still `Pending` in the old contract:
   ```bash
   # Inspect individual payment IDs to find any that remain pending
   stellar contract invoke \
     --id $OLD_CONTRACT_ID \
     --source alice \
     --network public \
     -- get_payment_status \
     --payment_id "<id>"
   ```
4. **Re-submit those payments** against the new contract so they can be completed there.
5. **Re-register all VASPs** in the new contract:
   ```bash
   stellar contract invoke \
     --id $NEW_CONTRACT_ID \
     --source alice \
     --network public \
     -- register_vasp \
     --vasp_address "<address>" \
     --jurisdiction "<code>"
   ```
6. **Update all integrations** (SDKs, apps, off-chain services) to point to the new contract ID.
7. **Communicate the migration** to all VASP operators and downstream consumers before switching over.

The old contract remains on-chain and read-only; it cannot be deleted or paused automatically.

### Deployment checklist addition

- [ ] Record the deployed contract ID and the commit SHA it was built from
- [ ] Keep the old contract ID available for read-only audit queries during migration

## Troubleshooting

**"insufficient fee"**
- Account needs more XLM for fees

**"Contract compile error"**
- Check stellar-sdk version matches contract code
- Rebuild with `cargo clean && cargo build`

**"Unauthorized"**
- Check admin permissions
- Verify signer account

**"Invalid jurisdiction code"**
- Use 2-letter ISO country codes (US, GB, SG, etc.)

## Environment Variables

```bash
# For automation
export SOROBAN_RPC_URL="https://soroban-testnet.stellar.org"
export SOROBAN_NETWORK_PASSPHRASE="Test SDF Network ; September 2015"
export SOROBAN_ACCOUNT_ID="GXXXXXXX..."
export CONTRACT_ID="CXXXXXXX..."

# Required by interact.sh: identity used to sign invocations (no default)
export STELLAR_SIGNER="alice"
```

## Resources

- Stellar Docs: https://developers.stellar.org/learn/build/smart-contracts
- Stellar CLI: https://github.com/stellar/stellar-cli
- Testnet Explorer: https://testnet.stellar.expert/
- Mainnet Explorer: https://stellar.expert/
