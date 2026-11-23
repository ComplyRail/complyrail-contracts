# complyrail-contracts

Soroban smart contracts enforcing FATF Travel Rule compliance at settlement: threshold-gated payment flow with an on-chain VASP registry and attestation recording for Stellar anchors.

## Overview

ComplyRail implements an on-chain compliance layer for Stellar VASPs. Payments above a configurable per-asset, per-jurisdiction threshold are held in a `Pending` state until the beneficiary VASP submits an attestation hash — a hash of an off-chain IVMS101 compliance message. Admins can also manually release or reject pending payments.

The contract records state on the Stellar ledger and emits events for each state change. It does **not** move or custody funds; fund settlement is handled by the calling application.

Payments are fail-closed: when no threshold is configured for the payment's asset and beneficiary VASP jurisdiction, the payment remains pending and requires an attestation from the beneficiary VASP. Only payments below an explicitly configured threshold are released automatically.

Threshold jurisdiction: the threshold is looked up by the **beneficiary (receiving) VASP's jurisdiction**, not the originator's. A rule set for jurisdiction `*` applies where no rule exists for that jurisdiction (#23).

## Features

- **VASP Registry** — Register VASPs with jurisdiction, public key, and optional expiry. Admins can suspend or revoke entries.
- **Compliance Thresholds** — Configurable per-asset, per-jurisdiction amount thresholds.
- **Threshold-gated Payments** — Payments at or above threshold enter `Pending` state; payments below are immediately `Released`.
- **Attestation Recording** — The beneficiary VASP submits an IVMS101 message hash to transition a payment from `Pending` to `Released`.
- **Admin Controls** — An admin list with a configurable multi-admin approval threshold. When `min_appr > 1`, sensitive governance operations (freeze/unfreeze registry, change approval threshold) require that many admin approvals before they execute. Routine per-payment operations (release, reject, register VASP) remain single-admin.
- **Event Emission** — Each state-changing call emits a Soroban event (see [EVENT\_SCHEMA.md](EVENT_SCHEMA.md)).

## Current Limitations

- **No fund escrow** — The contract records payment intent and compliance state; it does not hold or move tokens. Actual fund movement must be implemented by the caller.
- **Admin list, not full multi-sig** — `add_admin` maintains a list of authorised addresses. Sensitive governance operations (registry freeze/unfreeze and approval-threshold changes) go through a proposal-and-approval flow governed by `min_appr`. Routine per-payment operations (release, reject, register VASP, set threshold) still require only a single admin.
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

## Release Profile

The `[profile.release]` section in `Cargo.toml` is tuned for the deployed WASM artifact:

- **Optimized**: `lto = true` and `codegen-units = 1` produce a smaller, faster binary at the cost of longer build times.
- **Small**: `strip = true` removes debug symbols to keep the deployed WASM within size limits.
- **Deterministic**: fixed optimization settings ensure reproducible builds.
- **Overflow checks enabled**: `overflow-checks = true` keeps arithmetic overflow panics in release builds, so unexpected overflows fail loudly rather than silently wrapping.

> **Warning:** Changing any of these settings alters the compiled WASM artifact (size, performance, and behavior). Any such change should be reviewed and documented, since it affects what is deployed on-chain.

## Reproducible Builds

Anyone can rebuild the deployed WASM from source and verify that its SHA-256 hash matches the one published for a release. This lets users confirm that the on-chain code corresponds exactly to the source in this repository.

### Build procedure

Run the following from the repository root:

```sh
# 1. Install the WASM target (once per toolchain)
rustup target add wasm32-unknown-unknown

# 2. Build the contract for the WASM target
cargo build --release --target wasm32-unknown-unknown --locked

# 3. Compute the SHA-256 hash of the resulting WASM
sha256sum target/wasm32-unknown-unknown/release/complyrail_contracts.wasm
```

The `--locked` flag ensures the build uses exactly the dependency versions recorded in `Cargo.lock`. `Cargo.lock` is committed to this repository specifically to pin dependencies, so a build from the same commit resolves the same crate versions.

### Verification in CI and release notes

CI builds the contract **twice** from the same commit and compares the SHA-256 of the two resulting WASM artifacts. If the hashes differ, the build is not reproducible and CI fails. The verified SHA-256 is published in the release notes for each release, so you can compare it against the hash you compute locally with the procedure above. If the hashes match, the deployed WASM was built from the published source.

## Docker Build

`deploy.sh` runs the build inside a Docker image named `complyrail-build:latest`. The image name and tag are pinned, and `deploy.sh` builds the image automatically if it is not present locally, so you do not need to build it manually. To build it yourself:

```sh
docker build -t complyrail-build:latest .
```

CI also verifies that this image builds successfully.

## VASP Public Key Format

Each VASP is registered with a `BytesN<32>` public key used to receive encrypted IVMS101 payloads. The key must be a valid 32-byte value and **must not be all zeros** — `register_vasp` rejects an all-zero key, since a VASP registered with a zero key cannot receive encrypted IVMS payloads.

## Threshold Jurisdiction Rule

When `submit_payment` decides whether a payment must be held in escrow, it looks up the threshold using the **originating VASP's jurisdiction** (the `from` VASP), not the receiving VASP's.

Rationale: the Travel Rule obligation is imposed on the originating VASP, which is the party that must collect and transmit originator information before the transfer is released. Gating on the originator's jurisdiction keeps the escrow decision aligned with the obligation that triggers it, and matches the sibling `complyrail-app` contract so the same payment cannot be held in one system and released in the other.

## Multi-Admin Approval

When `min_appr` is set to more than 1 (via `set_approval_threshold`), two governance operations require that many distinct admin signatures before they execute:

- **Freeze / unfreeze the registry** (`set_registry_frozen`)
- **Change the approval threshold itself** (`set_approval_threshold`)

Routine per-payment operations — `release_payment`, `reject_payment`, `register_vasp`, `update_vasp_status`, `set_threshold` — still require only a single admin.

### Proposal flow

1. **Any admin** calls `set_registry_frozen` or `set_approval_threshold` as normal. When `min_appr > 1` the function creates a proposal instead of executing immediately, and returns a numeric proposal ID.
2. **Co-signing admins** call `approve_action(caller, proposal_id)`. Each call records one approval. The function returns `false` while the count is below `min_appr`, and `true` when the threshold is reached and the action executes.
3. Once executed the proposal is marked `executed = true`; further calls to `approve_action` on it return `ProposalAlreadyExecuted`.

You can also open a proposal explicitly with `propose_action(caller, ProposalKind)` and retrieve its current state with `get_proposal(proposal_id)`.

### Errors

| Code | Meaning |
|------|---------|
| `ProposalNotFound` (2) | No proposal exists with the given ID |
| `AlreadyApproved` (3) | Caller already voted on this proposal |
| `ProposalAlreadyExecuted` (4) | Proposal has already been executed |

## License

Apache License 2.0 — see LICENSE file.

## Security

See [THREAT_MODEL.md](THREAT_MODEL.md) for the actor definitions, trust assumptions, and attack paths covering the escrow and attestation flow.

## Legal Notice

ComplyRail is a technical tool, not legal advice. FATF Travel Rule obligations vary by jurisdiction (e.g., EU TFR, FinCEN rules, MAS requirements) and carry real regulatory liability. Any team deploying this to production should have the architecture and IVMS101 handling reviewed by qualified legal/compliance counsel before processing real regulated payments.