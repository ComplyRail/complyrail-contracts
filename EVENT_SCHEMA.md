# Event and Error Schema

This document describes every Soroban event emitted by the ComplyRail contract, its topic, payload fields, and an example. It also lists the panic strings the contract uses and what triggers each.

---

## Events

Soroban events have a **topic** (a tuple of symbols) and a **data** payload. All topics below use a single `symbol_short!` key. Topics and payloads are encoded in XDR.

### `adm_add`

Emitted when a new admin is added to the admin list.

| Field | Type | Description |
|-------|------|-------------|
| topic | `(Symbol("adm_add"),)` | |
| data | `Address` | The address that was added as an admin |

**Example:**
```
topic: ["adm_add"]
data:  GBXXX...  (the new admin's Stellar address)
```

**Trigger:** `add_admin(caller, new_admin)` — succeeds when `caller` is an existing admin and `new_admin` is not already in the list.

---

### `appr_th`

Emitted when the minimum-approval threshold is updated.

| Field | Type | Description |
|-------|------|-------------|
| topic | `(Symbol("appr_th"),)` | |
| data | `i32` | The new threshold value |

**Example:**
```
topic: ["appr_th"]
data:  2
```

**Trigger:** `set_approval_threshold(caller, threshold)` — requires `caller` to be an admin and `threshold > 0`.

> **Note:** The threshold value is stored on-chain but is not currently enforced on individual operations. See README current limitations.

---

### `vasp_reg`

Emitted when a VASP is registered.

| Field | Type | Description |
|-------|------|-------------|
| topic | `(Symbol("vasp_reg"),)` | |
| data | `Address` | The VASP address that was registered |

**Example:**
```
topic: ["vasp_reg"]
data:  GCYYY...  (the registered VASP's Stellar address)
```

**Trigger:** `register_vasp(caller, vasp, name, jurisdiction, public_key, expiry_days)` — requires `caller` to be an admin.

---

### `vasp_renew`

Emitted when a VASP registration is renewed.

| Field | Type | Description |
|-------|------|-------------|
| topic | `(Symbol("vasp_renew"),)` | |
| data | `Address` | The VASP address whose registration was extended |

**Example:**
```
topic: ["vasp_renew"]
data:  GCYYY...
```

**Trigger:** `renew_vasp_registration(caller, vasp, extension_days)` — requires `caller` to authenticate. The VASP entry must already exist.

---

### `vasp_upd`

Emitted when a VASP's status is changed.

| Field | Type | Description |
|-------|------|-------------|
| topic | `(Symbol("vasp_upd"),)` | |
| data | `Address` | The VASP address whose status was updated |

**Example:**
```
topic: ["vasp_upd"]
data:  GCYYY...
```

**Trigger:** `update_vasp_status(caller, vasp, status)` — requires `caller` to be an admin. The new status is not included in the event payload; fetch the VASP entry with `get_vasp` to read the current status.

---

### `thr_set`

Emitted when a compliance threshold is set.

| Field | Type | Description |
|-------|------|-------------|
| topic | `(Symbol("thr_set"),)` | |
| data | `Address` | The asset address the threshold applies to |

**Example:**
```
topic: ["thr_set"]
data:  GDZZZ...  (the asset contract address)
```

**Trigger:** `set_threshold(caller, asset, jurisdiction, amount)` — requires `caller` to be an admin. The jurisdiction and amount are not included in the event payload; use `get_threshold(asset, jurisdiction)` to retrieve the current value.

---

### `pay_sub`

Emitted when a payment is submitted.

| Field | Type | Description |
|-------|------|-------------|
| topic | `(Symbol("pay_sub"),)` | |
| data | `BytesN<32>` | The generated payment ID (32-byte little-endian counter padded to 32 bytes) |

**Example:**
```
topic: ["pay_sub"]
data:  0000000000000000000000000000000000000000000000000000000000000000  (payment ID #0, hex)
```

**Trigger:** `submit_payment(from_vasp, to_vasp, beneficiary, asset, amount)`. Both VASPs must be registered and `Active`, and neither may be expired. The resulting status is `Released` if no threshold is configured or the amount is below the threshold; otherwise `Pending`.

---

### `att_sub`

Emitted when an IVMS101 attestation hash is submitted for a pending payment.

| Field | Type | Description |
|-------|------|-------------|
| topic | `(Symbol("att_sub"),)` | |
| data | `BytesN<32>` | The payment ID that was attested |

**Example:**
```
topic: ["att_sub"]
data:  0100000000000000000000000000000000000000000000000000000000000000  (payment ID #1, hex)
```

**Trigger:** `submit_attestation(caller_vasp, payment_id, message_hash, ivms_version)`. The caller must be the beneficiary (`to_vasp`) of the payment, the payment must be in `Pending` status, and `message_hash` must not be all-zero bytes.

---

### `pay_rel`

Emitted when an admin manually releases a payment.

| Field | Type | Description |
|-------|------|-------------|
| topic | `(Symbol("pay_rel"),)` | |
| data | `(BytesN<32>, String)` | Tuple of payment ID and the admin-supplied reason string |

**Example:**
```
topic: ["pay_rel"]
data:  ("0100000000000000...", "compliance review passed")
```

**Trigger:** `release_payment(caller, payment_id, reason)` — requires `caller` to be an admin. The payment must not already be `Released`.

---

### `pay_rej`

Emitted when an admin rejects a payment.

| Field | Type | Description |
|-------|------|-------------|
| topic | `(Symbol("pay_rej"),)` | |
| data | `(BytesN<32>, String)` | Tuple of payment ID and the admin-supplied reason string |

**Example:**
```
topic: ["pay_rej"]
data:  ("0100000000000000...", "VASP failed KYC")
```

**Trigger:** `reject_payment(caller, payment_id, reason)` — requires `caller` to be an admin. The payment must be in `Pending` status (not already `Released` or `Rejected`).

---

## Error Codes

The contract uses `assert!` panics rather than typed error codes. The table below maps each panic message to its trigger condition.

| Panic message | Function | Trigger condition |
|---|---|---|
| `"admin already exists"` | `add_admin` | `new_admin` is already in the admin list |
| `"threshold must be positive"` | `set_approval_threshold` | `threshold <= 0` |
| `"vasp not found"` | `renew_vasp_registration`, `update_vasp_status`, `submit_attestation` | No entry exists for the given VASP address |
| `"from_vasp not registered"` | `submit_payment` | `from_vasp` has no entry in storage |
| `"to_vasp not registered"` | `submit_payment` | `to_vasp` has no entry in storage |
| `"from_vasp not active"` | `submit_payment` | `from_vasp` status is not `Active` |
| `"to_vasp not active"` | `submit_payment` | `to_vasp` status is not `Active` |
| `"from_vasp registration expired"` | `submit_payment` | Current ledger timestamp ≥ `from_vasp.expires_at` |
| `"to_vasp registration expired"` | `submit_payment` | Current ledger timestamp ≥ `to_vasp.expires_at` |
| `"payment not found"` | `submit_attestation`, `release_payment`, `reject_payment` | No entry exists for the given payment ID |
| `"payment not pending"` | `submit_attestation` | Payment status is not `Pending` |
| `"to_vasp not found"` | `submit_attestation` | `to_vasp` entry missing (internal consistency check) |
| `"from_vasp not found"` | `submit_attestation` | `from_vasp` entry missing (internal consistency check) |
| `"to_vasp not active"` | `submit_attestation` | `to_vasp` status is not `Active` |
| `"from_vasp not active"` | `submit_attestation` | `from_vasp` status is not `Active` |
| `"only beneficiary vasp can attest"` | `submit_attestation` | `caller_vasp != payment.to_vasp` |
| `"message_hash cannot be empty"` | `submit_attestation` | `message_hash` is 32 zero bytes |
| `"payment already released"` | `release_payment` | Payment status is already `Released` |
| `"payment already rejected"` | `reject_payment` | Payment status is already `Rejected` |
| `"cannot reject released payment"` | `reject_payment` | Payment status is `Released` |
| `"unauthorized: not an admin"` | any admin-gated function | `caller` is not in the admin list |

---

## Payment ID Format

Payment IDs are `BytesN<32>`. The first 8 bytes are the payment counter encoded as a little-endian `u64`; the remaining 24 bytes are zero. Payment #0 is `0000...00`, payment #1 is `0100000000000000` followed by 24 zero bytes, and so on.

---

## Keeping This Document Current

This schema is derived directly from `src/lib.rs`. When adding or changing events or error messages, update the corresponding table in this file in the same commit.
