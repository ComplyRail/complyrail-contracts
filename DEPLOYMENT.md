# ComplyRail Contracts - Stellar Deployment Guide

Deploy the ComplyRail smart contracts to Stellar testnet or mainnet.

## Prerequisites

### Install Tools

**Using Docker (Recommended):**
```bash
# Build contract in Docker
docker run --rm -v $(pwd):/workspace complyrail-build:latest
```

**Or locally:**
```bash
# Install Rust and Soroban CLI
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
rustup target add wasm32-unknown-unknown

# Install Soroban CLI
cargo install --locked soroban-cli
```

### Set Up Wallet

```bash
# Generate new keypair (save the secret key securely!)
soroban keys generate --network testnet alice
```

Or use existing secret key:
```bash
soroban keys add alice --secret-key SXXX...
```

## Deployment Steps

### 1. Build the Contract

```bash
# Docker build
docker run --rm -v $(pwd):/workspace complyrail-build:latest

# Or with Rust installed locally
cargo build --release --target wasm32-unknown-unknown
```

Output: `target/wasm32-unknown-unknown/release/complyrail_contracts.wasm`

### 2. Deploy to Testnet

```bash
# Set network
export SOROBAN_RPC_URL=https://soroban-testnet.stellar.org
export SOROBAN_NETWORK_PASSPHRASE="Test SDF Network ; September 2015"

# Deploy (keep the contract ID!)
CONTRACT_ID=$(soroban contract deploy \
  --wasm target/wasm32-unknown-unknown/release/complyrail_contracts.wasm \
  --source alice \
  --network testnet)

echo "Contract ID: $CONTRACT_ID"
```

### 3. Initialize Contract

```bash
# Set contract admin(s) - multiple admins supported
soroban contract invoke \
  --id $CONTRACT_ID \
  --source alice \
  --network testnet \
  -- set_admin \
  --admin "GXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX"
```

## Interacting with Contract

### Register a VASP

```bash
soroban contract invoke \
  --id $CONTRACT_ID \
  --source alice \
  --network testnet \
  -- register_vasp \
  --vasp_address "GXXXXXXX..." \
  --jurisdiction "US"
```

### Submit a Payment

```bash
soroban contract invoke \
  --id $CONTRACT_ID \
  --source alice \
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
soroban contract invoke \
  --id $CONTRACT_ID \
  --source alice \
  --network testnet \
  -- get_payment_status \
  --payment_id "payment_123"
```

## Contract Functions

### Admin Functions
- `set_admin(admin: Address)` - Add admin
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
- [ ] Verify contract bytecode before mainnet deployment
- [ ] Multi-sig setup for critical operations (recommended)
- [ ] Audit contract code before mainnet
- [ ] Use testnet for 7+ days to verify stability

## Mainnet Deployment

Same process as testnet, but:

```bash
export SOROBAN_RPC_URL=https://soroban.stellar.org
export SOROBAN_NETWORK_PASSPHRASE="Public Global Stellar Network ; September 2015"

# Deploy with formal audit approval
soroban contract deploy \
  --wasm target/wasm32-unknown-unknown/release/complyrail_contracts.wasm \
  --source mainnet_admin \
  --network public
```

## Troubleshooting

**"insufficient fee"**
- Account needs more XLM for fees

**"Contract compile error"**
- Check soroban-sdk version matches contract code
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
```

## Resources

- Soroban Docs: https://developers.stellar.org/learn/build/smart-contracts
- Soroban CLI: https://github.com/stellar/rs-soroban-cli
- Testnet Explorer: https://testnet.stellar.expert/
- Mainnet Explorer: https://stellar.expert/
