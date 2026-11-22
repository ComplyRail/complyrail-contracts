# Testnet Setup

## Deployment Record

The live testnet contract details are recorded in the repository at
[`deployments/testnet.json`](deployments/testnet.json). That file is the
source of truth for the deployed contract ID, admin account, WASM hash and
the commit that was deployed.

```json
{
  "network": "testnet",
  "contractId": "C...",
  "admin": "G...",
  "wasmHash": "...",
  "commit": "..."
}
```

Use the `contractId` from that file wherever a contract address is needed
(SDK, app, `.env.testnet`, etc.). The file is written automatically by
`deploy.sh` on each deployment.

## Funding Accounts

For Phase 1 testing, we need two mock VASP accounts funded on Stellar testnet:

### Mock VASPs

**VASP A (Originating Anchor)**
- Account ID: see `admin` in `deployments/testnet.json` (or your own funded keypair)
- Role: Initiates compliant payments

**VASP B (Beneficiary Anchor)**
- Account ID: your own funded keypair (not recorded in the repo)
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
  --wasm ./target/wasm32v1-none/release/complyrail_contracts.wasm \
  --source-account vasp_a \
  --network testnet)

echo "Contract ID: $CONTRACT_ID"
```

`deploy.sh` runs this step and writes the returned contract ID, admin,
WASM hash and current commit to `deployments/testnet.json`. Commit that file
so the live testnet contract can always be found from the repository.
Use `$CONTRACT_ID`, `$VASP_A_ID`, and `$VASP_B_ID` in subsequent `stellar contract invoke` calls.
