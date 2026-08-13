# Testnet Setup

## Funding Accounts

For Phase 1 testing, we need two mock VASP accounts funded on Stellar testnet:

### Mock VASPs

**VASP A (Originating Anchor)**
- Account ID: `GVASP_A_ADDRESS` (to be funded)
- Role: Initiates compliant payments
- Setup: Fund from testnet faucet at https://developers.stellar.org/docs/build/guides/get-test-account

**VASP B (Beneficiary Anchor)**
- Account ID: `GVASP_B_ADDRESS` (to be funded)  
- Role: Receives compliant payments and submits attestations
- Setup: Fund from testnet faucet

### Process

1. Generate keypairs for each VASP:
   ```bash
   stellar keys generate vasp_a
   stellar keys generate vasp_b
   ```

2. Fund each account via Friendbot (testnet faucet):
   ```bash
   stellar keys fund vasp_a --testnet
   stellar keys fund vasp_b --testnet
   ```

3. Update `.env.testnet` with the account IDs:
   ```
   VASP_A_ID=GVASP_A_ADDRESS
   VASP_B_ID=GVASP_B_ADDRESS
   CONTRACT_ID=CCONTRACT_ADDRESS
   ```

## Deploying to Testnet

```bash
stellar contract deploy \
  --wasm ./target/wasm32-unknown-unknown/release/complyrail_contracts.wasm \
  --source-account vasp_a \
  --network testnet
```

Store the returned contract ID for use in the SDK and app.
