# complyrail-contracts

Soroban smart contracts enforcing FATF Travel Rule compliance at settlement: threshold-gated payment escrow with on-chain VASP registry and attestation verification for Stellar anchors.

## Overview

ComplyRail contracts implement an on-chain compliance layer for Stellar anchors and VASPs. Payments above a configurable threshold are held in escrow until an attestation (a hash of an off-chain IVMS101 compliance message) is submitted by the beneficiary VASP.

Payments are fail-closed: when no threshold is configured for the payment's asset and beneficiary VASP jurisdiction, the payment remains pending and requires an attestation from the beneficiary VASP. Only payments below an explicitly configured threshold are released automatically.

## Features

- **VASP Registry**: Register Virtual Asset Service Providers with jurisdiction and public key
- **Threshold Configuration**: Set per-asset, per-jurisdiction compliance thresholds
- **Payment Escrow**: Hold payments in escrow until compliance attestation is submitted
- **Multi-Admin**: Require multiple admins for sensitive operations
- **Event Audit Trail**: Every state change emits events for off-chain indexing

## Deployments

The canonical record of the live testnet deployment is
[`deployments/testnet.json`](deployments/testnet.json). It contains the deployed
contract ID, the admin address, the WASM hash and the commit that produced the
build. `deploy.sh` writes this file on every deployment, so it should always
reflect the current testnet contract. Do not edit it by hand.

## Code Owners

Ownership of security-sensitive paths is declared in
[`.github/CODEOWNERS`](.github/CODEOWNERS):

- `src/` — the contract source
- `deploy.sh`, `interact.sh` — deployment and administration scripts
- `.github/` — CI/CD workflows and repository configuration

Changes to these paths require review from a code owner. This is enforced by
branch protection on `main`: **Require review from Code Owners** must be enabled
alongside the required status checks. Repository administrators should verify
this setting after any change to branch protection rules.

## SDK Version Policy

A change to the Stellar/Soroban SDK version can alter contract behaviour, so CI
prints the resolved SDK version and fails the job when the SDK entry in the lock
file changes without a matching commit message.

To intentionally bump the SDK, include the marker `[sdk-bump]` in the commit
message (or PR title) that updates the lock file. Any SDK lock entry change
without that marker fails the build.

## License

Apache License 2.0 — see LICENSE file.

## Legal Notice

ComplyRail is a technical tool, not legal advice. FATF Travel Rule obligations vary by jurisdiction (e.g., EU TFR, FinCEN rules, MAS requirements) and carry real regulatory liability. Any team deploying this to production should have the architecture and IVMS101 handling reviewed by qualified legal/compliance counsel before processing real regulated payments.