# complyrail-contracts

Soroban smart contracts enforcing FATF Travel Rule compliance at settlement: threshold-gated payment flow with an on-chain VASP registry and attestation recording for Stellar anchors.

## Overview

ComplyRail implements an on-chain compliance layer for Stellar VASPs. Payments above a configurable per-asset, per-jurisdiction threshold are held in a `Pending` state until the beneficiary VASP submits an attestation hash — a hash of an off-chain IVMS101 compliance message. Admins can also manually release or reject pending payments.

The contract records state on the Stellar ledger and emits events for each state change. It does **not** move or custody funds; fund settlement is handled by the calling application.

Payments are fail-closed: when no threshold is configured for the payment's asset and beneficiary VASP jurisdiction, the payment remains pending and requires an attestation from the beneficiary VASP. Only payments below an explicitly configured threshold are released automatically.

## Features

- **VASP Registry** — Register VASPs with jurisdiction, public key, and optional expiry. Admins can suspend or revoke entries.
- **Compliance Thresholds** — Configurable per-asset, per-jurisdiction amount thresholds.
- **Threshold-gated Payments** — Payments at or above threshold enter `Pending` state; payments below are immediately `Released`.
- **Attestation Recording** — The beneficiary VASP submits an IVMS101 message hash to transition a payment from `Pending` to `Released`.
- **Admin Controls** — An admin list (not a multi-sig quorum) allows manual release or rejection of payments and threshold management.
- **Event Emission** — Each state-changing call emits a Soroban event (see [EVENT\_SCHEMA.md](EVENT_SCHEMA.md)).

## Current Limitations

- **No fund escrow** — The contract records payment intent and compliance state; it does not hold or move tokens. Actual fund movement must be implemented by the caller.
- **Admin list, not multi-sig** — `add_admin` maintains a list of authorised addresses, but sensitive operations require only a single admin signature. The `min_appr` threshold value is stored but not enforced on individual operations.
- **Partial event coverage** — Not every field change emits a dedicated event. The `update_vasp_status` and `set_threshold` calls emit topic-only events without field-level payloads.
- **Linear payment scan** — `get_payments_by_vasp` iterates over all payment IDs; it will be slow on large ledgers.

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

## Toolchain

Builds are pinned so the same commit always produces the same WASM:

| Component | Version |
|-----------|---------|
| Rust | 1.85.0 |
| Stellar CLI | 22.8.1 |
| soroban-sdk | 22.0.8 |
| WASM target | `wasm32v1-none` |

Build output: `target/wasm32v1-none/release/complyrail_contracts.wasm`

## License

Apache License 2.0 — see LICENSE file.

## Legal Notice

ComplyRail is a technical tool, not legal advice. FATF Travel Rule obligations vary by jurisdiction (e.g., EU TFR, FinCEN rules, MAS requirements) and carry real regulatory liability. Any team deploying this to production should have the architecture and IVMS101 handling reviewed by qualified legal/compliance counsel before processing real regulated payments.