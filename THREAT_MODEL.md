# ComplyRail Contracts — Threat Model

This document covers the actors, trust assumptions, and attack paths for the escrow and attestation flow. It maps each identified risk to a mitigation in the current code or to an open issue.

---

## Actors

### Admin
- **What it is**: One or more Stellar accounts stored in the `admins` list at initialization time.
- **What it trusts**: Only itself; all state-changing admin calls require `require_auth()`.
- **What it can change**: VASP registrations and status, compliance thresholds, manual release or rejection of any pending payment, addition of new admins, approval threshold.

### Originator VASP
- **What it is**: A registered Stellar account representing the sending Virtual Asset Service Provider.
- **What it trusts**: The contract to record and enforce the payment state correctly.
- **What it can change**: It can submit payments (`submit_payment`) from its own account. It cannot change payment status directly once submitted.

### Beneficiary VASP
- **What it is**: A registered Stellar account representing the receiving VASP.
- **What it trusts**: The contract state for payment details.
- **What it can change**: It can submit the IVMS101 attestation (`submit_attestation`) for a payment directed to it. Only the `to_vasp` address on a payment may attest.

### Beneficiary (end user)
- **What it is**: The end-user address receiving funds.
- **What it trusts**: Both VASPs and the contract to process the payment correctly.
- **What it can change**: Nothing directly on the contract.

---

## Trust Assumptions

1. The admin key(s) are kept secret and are not shared. Compromise of an admin key is an unconditional break of the model.
2. VASP public keys registered on-chain correspond to the keys used to sign off-chain IVMS101 messages. The contract does **not** verify this linkage on-chain; it is enforced off-chain by the VASPs themselves.
3. The off-chain IVMS101 message whose hash is submitted in `submit_attestation` is genuine. The contract stores only the hash; it cannot validate the content.
4. The Stellar ledger timestamp (`env.ledger().timestamp()`) is a reliable-enough proxy for real-world time for VASP registration expiry purposes.

---

## Attack Paths and Mitigations

| # | Attack | Impact | Mitigation | Status |
|---|--------|--------|------------|--------|
| 1 | **Admin key compromise** — attacker gains an admin key and releases or rejects arbitrary payments, adds a rogue VASP, or lowers the threshold to zero. | Critical | Multi-admin setup: require more than one admin approval for sensitive operations (`set_approval_threshold`). Keep keys in HSMs or multisig wallets. | Partial — multi-admin list exists; `set_approval_threshold` is implemented. The `release_payment` and `reject_payment` functions do not yet check the approval count; they accept any single admin. This is an open issue. |
| 2 | **Rogue VASP registration** — an attacker with an admin key registers a fake VASP and submits fraudulent payments. | High | Only admins can register VASPs. VASP entries carry status (`Active`/`Suspended`/`Revoked`) and optional expiry. Rotate or revoke compromised VASPs via `update_vasp_status`. | Mitigated by admin-only registration. Requires diligence on admin key hygiene (see #1). |
| 3 | **Fake attestation** — a beneficiary VASP submits a hash that does not correspond to a real IVMS101 message. | High | The contract enforces that only the designated `to_vasp` may attest a payment, and rejects an all-zero hash. Authenticity of the message content is an off-chain responsibility; deployers must verify the IVMS101 exchange out-of-band. | Partial — on-chain check is minimal. Full mitigation requires the off-chain IVMS101 protocol. |
| 4 | **Payment ID collision** — a u64 counter overflow (after 2^64 payments) reuses a payment ID. | Low | The counter is a `u64`; at any realistic payment volume, overflow is not reachable. The ID is stored as `BytesN<32>` with the counter in bytes 0–7, leaving bytes 8–31 as zero padding. | Acceptable risk. |
| 5 | **Threshold manipulation** — an admin sets a threshold to zero or a very high value, causing all payments to be auto-released or permanently held. | Medium | Only admins can set thresholds. Governance over admin membership limits this risk. | Mitigated by admin-only access. |
| 6 | **Expired VASP submits a payment** — a VASP whose registration has expired attempts to submit a payment. | Low | `submit_payment` checks `expires_at` for both VASPs and panics if expired. | Mitigated. |
| 7 | **Re-entrancy** — a contract call re-enters the ComplyRail contract during execution. | Low | Soroban's execution model does not support re-entrant contract calls within the same transaction. | Mitigated by the platform. |
| 8 | **Denial of service via storage growth** — an attacker submits a large number of payments to fill ledger storage. | Medium | Soroban charges fees per ledger entry. Payments from VASPs require `require_auth()` from the originator VASP, limiting who can spam entries. | Mitigated by fee model and auth requirements. |
| 9 | **Admin added without quorum** — a single admin calls `add_admin` to add a rogue admin without consensus. | High | `add_admin` currently requires only one admin signature. A multi-sig approval flow for `add_admin` is not yet implemented. | **Open issue** — pending multi-admin approval quorum enforcement. |
| 10 | **Off-chain IVMS101 message never delivered** — the originator VASP submits a payment but never sends the off-chain message, leaving funds held indefinitely. | Medium | Admin can manually release or reject via `release_payment` / `reject_payment`. Deployers should define a maximum hold time policy enforced by the admin. | Partial — no on-chain timeout; policy is off-chain. |

---

## Open Issues

- **Multi-admin quorum for sensitive operations** (attacks #1, #9): `release_payment`, `reject_payment`, and `add_admin` accept any single admin. The `PendingApproval` type and `min_appr` threshold exist in the contract but are not wired to these functions.
- **On-chain hold timeout**: No automatic release or escalation after a configurable period. This is an operational gap for the scenario in attack #10.

---

## Scope

This threat model covers the on-chain contract only. It does not cover the off-chain IVMS101 exchange, the dashboard, the indexer, key management infrastructure, or the deployment pipeline. Those surfaces have their own risk profiles and should be modelled separately.
