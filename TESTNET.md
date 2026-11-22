# Testnet Setup

## Funding Accounts

For Phase 1 testing, we need two mock VASP accounts funded on Stellar testnet:

### Mock VASPs

**VASP A (Originating Anchor)**
- Role: Initiates compliant payments

**VASP B (Beneficiary Anchor)**
- Role: Receives compliant payments and submits attestations

### Process

1. Generate keypairs for each VASP:
   ```bash
   stellar keys generate vasp_a
   stellar keys generate vasp_b
   ```

2. Fund each account via Friendbot (testnet faucet):
   ```bash
   stellar keys fund vasp_a --network testnet
   stellar keys fund vasp_b --network testnet
   ```

3. Export the account IDs for use in subsequent commands:
   ```bash
   export VASP_A_ID=$(stellar keys address vasp_a)
   export VASP_B_ID=$(stellar keys address vasp_b)
   echo "VASP A: $VASP_A_ID"
   echo "VASP B: $VASP_B_ID"
   ```

## Deploying to Testnet

```bash
export CONTRACT_ID=$(stellar contract deploy \
  --wasm ./target/wasm32-unknown-unknown/release/complyrail_contracts.wasm \
  --source-account vasp_a \
  --network testnet)

echo "Contract ID: $CONTRACT_ID"
```

Use `$CONTRACT_ID`, `$VASP_A_ID`, and `$VASP_B_ID` in subsequent `stellar contract invoke` calls.
