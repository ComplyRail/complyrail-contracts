# Changelog

All notable changes to this project are documented here.

## [v0.2.0] — Phase 2

**Commit:** `a776cae`

### Added
- Attestation submission (`submit_attestation`): the beneficiary VASP submits an IVMS101 message hash to release a pending payment.
- Payment management: admins can manually release (`release_payment`) or reject (`reject_payment`) a payment with a reason string.
- Payment analytics: `get_payment_count` and `get_payments_by_vasp` for querying the payment ledger.
- VASP expiration: `register_vasp` accepts an optional `expiry_days` parameter; `renew_vasp_registration` extends it.
- Approval threshold setting (`set_approval_threshold`) stored on-chain and emitted as an event.

### Changed
- `submit_payment` now evaluates the per-asset/per-jurisdiction threshold and sets initial status to `Released` (below threshold) or `Pending` (at or above threshold).

## [v0.1.0] — Phase 1

**Commit:** `cdb8d04`

### Added
- VASP registry: `register_vasp`, `update_vasp_status`, `get_vasp`.
- Multi-admin list: `initialize`, `add_admin`, `get_admins`.
- Compliance thresholds: `set_threshold`, `get_threshold`.
- Initial `submit_payment` function (threshold evaluation not yet wired).
- Core data types: `VaspEntry`, `VaspStatus`, `PaymentRecord`, `PaymentStatus`.
