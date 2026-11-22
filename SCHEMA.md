# ComplyRail Contracts — Schema and Design Reference

This document describes the on-chain data types, payment identifier format, and the reconciliation between this contract and the complyrail-app dashboard design.

---

## Payment Identifier Format

**Agreed format: `BytesN<32>` with the sequential `u64` counter in bytes 0–7 (little-endian), bytes 8–31 zero-padded.**

This bridges the two historic representations:

| Repository | Historic format | On-chain now |
|---|---|---|
| complyrail-contracts | `BytesN<32>` | `BytesN<32>` (unchanged) |
| complyrail-app | `u64` | Extract via `payment_id_to_u64` |

### Extracting the u64 from a BytesN<32>

The contract exposes a view function:

```
payment_id_to_u64(payment_id: BytesN<32>) -> u64
```

This reads bytes 0–7 as a little-endian `u64`. The result is the sequential counter value and is unique for the lifetime of the contract (until `u64::MAX` payments, which is not reachable in practice).

Indexers and dashboards that previously used bare `u64` IDs should:
1. Store the full `BytesN<32>` as the canonical identifier.
2. Call `payment_id_to_u64` or decode bytes 0–7 locally to obtain the numeric ID for display or legacy lookups.

### Migration for existing payments

Payments submitted before this format was documented already follow it: `generate_payment_id` has always written the counter into bytes 0–7 of a 32-byte array. No on-chain migration is needed. Off-chain indexes that stored only the `u64` counter can reconstruct the full `BytesN<32>` by zero-padding to 32 bytes with the counter in little-endian at offset 0.

---

## Reconciled Design (issues #121, #124)

This contract and complyrail-app previously diverged on several points. The table below shows the agreed current state.

| Dimension | complyrail-contracts (this repo) | complyrail-app | Resolved |
|---|---|---|---|
| Payment ID type | `BytesN<32>` | `u64` | Use `BytesN<32>` on-chain; `payment_id_to_u64` for dashboard compatibility |
| Payment status values | `Pending / Released / Held / Rejected` | `Pending / Released / Held / Rejected` | Aligned |
| Threshold logic | fail-closed: amount ≥ threshold → `Pending` | fail-closed | Aligned |
| Attestation side | Beneficiary VASP (`to_vasp`) | Originator-side in older draft | This contract uses beneficiary-side; dashboard must be updated to match |
| Escrow | No token transfer on-chain; status-only escrow | Escrowed tokens | Status-only model is current; token escrow is a future enhancement |
| Wildcard rule | Not implemented | Present in older draft | Not yet implemented here; open for a follow-up issue |

---

## Key Types

### PaymentRecord

```
PaymentRecord {
    id:               BytesN<32>       -- canonical payment ID (u64 counter in bytes 0-7, LE)
    from_vasp:        Address
    to_vasp:          Address
    beneficiary:      Address
    asset:            Address
    amount:           i128
    status:           PaymentStatus    -- Pending | Released | Held | Rejected
    attestation_hash: Option<BytesN<32>>
    ivms_version:     Option<String>
    created_at:       u64              -- ledger timestamp
    resolved_at:      Option<u64>      -- ledger timestamp, set on release/reject
}
```

### VaspEntry

```
VaspEntry {
    address:     Address
    name:        String
    jurisdiction: String              -- ISO 3166-1 alpha-2 (e.g. "US", "GB")
    public_key:  BytesN<32>
    status:      VaspStatus           -- Active | Suspended | Revoked
    added_at:    u64
    expires_at:  Option<u64>          -- ledger timestamp; None means no expiry
}
```

### ThresholdConfig

Thresholds are stored in a `Map<String, i128>` keyed by `"<asset_address>:<jurisdiction>"`. Amounts are in the asset's smallest unit (stroops for XLM-equivalent assets).

---

## Contract Version

The contract exposes:

```
version() -> &'static str    -- semver string, e.g. "1.0.0"
```

Always record the version returned by this function alongside the contract ID in your deployment log. See DEPLOYMENT.md for the query command.
