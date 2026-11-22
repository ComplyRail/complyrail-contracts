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
- Setup: Fund from testnet faucet at https://developers.stellar.org/docs/build/guides/get-test-account

**VASP B (Beneficiary Anchor)**
- Account ID: your own funded keypair (not recorded in the repo)
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

3. Update `.env.testnet` with the account IDs and the deployed contract ID
   from `deployments/testnet.json`:
   ```
   VASP_A_ID=<your vasp_a public key>
   VASP_B_ID=<your vasp_b public key>
   CONTRACT_ID=<contractId from deployments/testnet.json>
   ```

## Deploying to Testnet

```bash
stellar contract deploy \
  --wasm ./target/wasm32v1-none/release/complyrail_contracts.wasm \
  --source-account vasp_a \
  --network testnet
```

`deploy.sh` runs this step and writes the returned contract ID, admin,
WASM hash and current commit to `deployments/testnet.json`. Commit that file
so the live testnet contract can always be found from the repository.