# complyrail-contracts

Soroban smart contracts enforcing FATF Travel Rule compliance at settlement: threshold-gated payment escrow with on-chain VASP registry and attestation verification for Stellar anchors.

## Overview

ComplyRail contracts implement an on-chain compliance layer for Stellar anchors and VASPs. Payments above a configurable threshold are held in escrow until an attestation (a hash of an off-chain IVMS101 compliance message) is submitted by the beneficiary VASP.

## Features

- **VASP Registry**: Register Virtual Asset Service Providers with jurisdiction and public key
- **Threshold Configuration**: Set per-asset, per-jurisdiction compliance thresholds
- **Payment Escrow**: Hold payments in escrow until compliance attestation is submitted
- **Multi-Admin**: Require multiple admins for sensitive operations
- **Event Audit Trail**: Every state change emits events for off-chain indexing

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

## License

Apache License 2.0 — see LICENSE file.

## Legal Notice

ComplyRail is a technical tool, not legal advice. FATF Travel Rule obligations vary by jurisdiction (e.g., EU TFR, FinCEN rules, MAS requirements) and carry real regulatory liability. Any team deploying this to production should have the architecture and IVMS101 handling reviewed by qualified legal/compliance counsel before processing real regulated payments.