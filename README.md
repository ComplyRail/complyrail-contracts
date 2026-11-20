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

## VASP Public Key Format

Each VASP is registered with a `BytesN<32>` public key used to receive encrypted IVMS101 payloads. The key must be a valid 32-byte value and **must not be all zeros** — `register_vasp` rejects an all-zero key, since a VASP registered with a zero key cannot receive encrypted IVMS payloads.

## License

Apache License 2.0 — see LICENSE file.

## Legal Notice

ComplyRail is a technical tool, not legal advice. FATF Travel Rule obligations vary by jurisdiction (e.g., EU TFR, FinCEN rules, MAS requirements) and carry real regulatory liability. Any team deploying this to production should have the architecture and IVMS101 handling reviewed by qualified legal/compliance counsel before processing real regulated payments.
