mod types;

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests;

use soroban_sdk::{contract, contracterror, contractimpl, Address, Env, String, BytesN, symbol_short};
use types::*;

/// How long a payment may stay pending before it can no longer be attested (#33).
const PAYMENT_WINDOW_SECS: u64 = 7 * 86400;

/// Errors a caller can match on, instead of a panic or a bare `None` (#46).
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum ComplyError {
    VaspNotFound = 1,
}

/// Upper bound on admins, so the admin list cannot grow without limit (#67).
const MAX_ADMINS: u32 = 16;

/// Contract version. Increment on every breaking interface change.
pub const VERSION: &str = "1.0.0";
/// Extend a record once it has less than this many ledgers left (#59).
const LEDGER_TTL_THRESHOLD: u32 = 100_000;
/// Extend a record to this many ledgers from now (#59).
const LEDGER_TTL_EXTEND: u32 = 500_000;

#[contract]
pub struct ComplyRailContract;

#[contractimpl]
impl ComplyRailContract {
    pub fn initialize(env: Env, admin: Address) {
        // Only the first call may set the admin list (#1).
        assert!(!env.storage().instance().has(&symbol_short!("init")), "already initialized");
        env.storage().instance().set(&symbol_short!("init"), &true);
        admin.require_auth();
        let mut admins: soroban_sdk::Vec<Address> = soroban_sdk::Vec::new(&env);
        env.storage().instance().set(&(symbol_short!("admin"), admin.clone()), &true);
        admins.push_back(admin.clone());
        env.storage().instance().set(&symbol_short!("admins"), &admins);
        env.storage().instance().set(&symbol_short!("min_appr"), &1i32);
        env.events().publish((symbol_short!("init"),), (admin, 1i32));
    }

    pub fn add_admin(env: Env, caller: Address, new_admin: Address) {
        caller.require_auth();
        Self::require_admin(&env, &caller);

        let mut admins: soroban_sdk::Vec<Address> = env.storage()
            .instance()
            .get(&symbol_short!("admins"))
            .unwrap_or_else(|| soroban_sdk::Vec::new(&env));

        for admin in admins.iter() {
            assert!(admin != new_admin, "admin already exists");
        }

        assert!(admins.len() < MAX_ADMINS, "admin list is full");
        env.storage().instance().set(&(symbol_short!("admin"), new_admin.clone()), &true);
        admins.push_back(new_admin.clone());
        env.storage().instance().set(&symbol_short!("admins"), &admins);
        env.events().publish((symbol_short!("adm_add"),), (new_admin.clone(), caller));
    }

    /// Emergency switch: while frozen, no new payments can be submitted (#49).
    pub fn set_registry_frozen(env: Env, caller: Address, frozen: bool) {
        caller.require_auth();
        Self::require_admin(&env, &caller);

        env.storage().instance().set(&symbol_short!("frozen"), &frozen);
        env.events().publish((symbol_short!("frozen"),), &frozen);
    }

    pub fn is_registry_frozen(env: Env) -> bool {
        env.storage().instance().get(&symbol_short!("frozen")).unwrap_or(false)
    }

    pub fn get_admins(env: Env) -> soroban_sdk::Vec<Address> {
        env.storage()
            .instance()
            .get(&symbol_short!("admins"))
            .unwrap_or_else(|| soroban_sdk::Vec::new(&env))
    }

    pub fn set_approval_threshold(env: Env, caller: Address, threshold: i32) {
        caller.require_auth();
        Self::require_admin(&env, &caller);

        assert!(threshold > 0, "threshold must be positive");

        // Emit only on an actual change, and include the old value (#60).
        let previous: i32 = env.storage().instance().get(&symbol_short!("min_appr")).unwrap_or(0);
        if previous != threshold {
            env.storage().instance().set(&symbol_short!("min_appr"), &threshold);
            env.events().publish((symbol_short!("appr_th"),), (previous, threshold));
        }
    }

    pub fn register_vasp(
        env: Env,
        caller: Address,
        vasp: Address,
        name: String,
        jurisdiction: String,
        public_key: BytesN<32>,
        expiry_days: Option<u64>,
    ) {
        caller.require_auth();
        Self::require_admin(&env, &caller);

        // An all-zero key cannot be a real public key (#12).
        assert!(public_key.to_array() != [0u8; 32], "public key must not be all zeros");

        // Re-registering an existing VASP would overwrite its status and expiry (#73).
        assert!(!env.storage().persistent().has(&vasp), "vasp already registered");
        assert!(!Self::is_admin(&env, &vasp), "an admin cannot be registered as a vasp");

        let expiry = expiry_days.map(|days| {
            days.checked_mul(86400)
                .and_then(|secs| env.ledger().timestamp().checked_add(secs))
                .expect("expiry overflow")
        });

        let entry = VaspEntry {
            address: vasp.clone(),
            name,
            jurisdiction,
            public_key,
            status: VaspStatus::Active,
            added_at: env.ledger().timestamp(),
            expires_at: expiry,
            updated_at: env.ledger().timestamp(),
        };

        env.storage().persistent().set(&vasp, &entry);
        // Persistent entries expire unless extended, so every write keeps the record alive (#59).
        env.storage().persistent().extend_ttl(&vasp, LEDGER_TTL_THRESHOLD, LEDGER_TTL_EXTEND);

        // Keep an enumerable index of registered VASPs (#41).
        let mut vasps: soroban_sdk::Vec<Address> = env.storage()
            .instance()
            .get(&symbol_short!("vasps"))
            .unwrap_or_else(|| soroban_sdk::Vec::new(&env));
        let mut known = false;
        for existing in vasps.iter() {
            if existing == vasp {
                known = true;
            }
        }
        if !known {
            vasps.push_back(vasp.clone());
            env.storage().instance().set(&symbol_short!("vasps"), &vasps);
        }

        env.events().publish((symbol_short!("vasp_reg"),), (vasp.clone(), caller));
    }

    pub fn list_vasps(env: Env) -> soroban_sdk::Vec<Address> {
        env.storage()
            .instance()
            .get(&symbol_short!("vasps"))
            .unwrap_or_else(|| soroban_sdk::Vec::new(&env))
    }

    pub fn renew_vasp_registration(env: Env, caller: Address, vasp: Address, extension_days: u64) {
        caller.require_auth();
        assert!(
            Self::is_admin(&env, &caller) || caller == vasp,
            "unauthorized: only admin or the vasp may renew"
        );

        let mut entry: VaspEntry = env.storage().persistent().get(&vasp).expect("vasp not found");

        // Unlimited registrations stay unlimited. Limited ones extend from
        // max(now, expires_at) so renewing an expired VASP makes it valid again.
        if let Some(current_expiry) = entry.expires_at {
            let now = env.ledger().timestamp();
            let base = if current_expiry > now { current_expiry } else { now };
            let extension = extension_days
                .checked_mul(86400)
                .and_then(|secs| base.checked_add(secs))
                .expect("expiry overflow");
            entry.expires_at = Some(extension);
        }
        entry.updated_at = env.ledger().timestamp();

        env.storage().persistent().set(&vasp, &entry);
        // Persistent entries expire unless extended, so every write keeps the record alive (#59).
        env.storage().persistent().extend_ttl(&vasp, LEDGER_TTL_THRESHOLD, LEDGER_TTL_EXTEND);
        // `symbol_short!` allows at most 9 characters, so build this 10-character topic with `Symbol::new`.
        env.events().publish((soroban_sdk::Symbol::new(&env, "vasp_renew"),), (vasp.clone(), caller));
    }

    /// Set a registration to expire `expires_in_seconds` from now (#47).
    ///
    /// Day-based `register_vasp` and `renew_vasp_registration` are unchanged, so
    /// existing callers keep working.
    pub fn set_vasp_expiry_seconds(env: Env, caller: Address, vasp: Address, expires_in_seconds: u64) {
        caller.require_auth();
        Self::require_admin(&env, &caller);

        let mut entry: VaspEntry = env.storage().persistent().get(&vasp).expect("vasp not found");
        entry.expires_at = Some(env.ledger().timestamp() + expires_in_seconds);
        entry.updated_at = env.ledger().timestamp();

        env.storage().persistent().set(&vasp, &entry);
        // Persistent entries expire unless extended, so every write keeps the record alive (#59).
        env.storage().persistent().extend_ttl(&vasp, LEDGER_TTL_THRESHOLD, LEDGER_TTL_EXTEND);
        env.events().publish((symbol_short!("vasp_exp"),), &vasp);
    }

    pub fn update_vasp_status(env: Env, caller: Address, vasp: Address, status: VaspStatus) {
        caller.require_auth();
        Self::require_admin(&env, &caller);

        let mut entry: VaspEntry = env.storage().persistent().get(&vasp).expect("vasp not found");
        assert!(entry.status.can_transition_to(&status), "invalid vasp status transition");
        entry.status = status;
        entry.updated_at = env.ledger().timestamp();
        env.storage().persistent().set(&vasp, &entry);
        // Persistent entries expire unless extended, so every write keeps the record alive (#59).
        env.storage().persistent().extend_ttl(&vasp, LEDGER_TTL_THRESHOLD, LEDGER_TTL_EXTEND);

        // Pending payments touching a suspended or revoked VASP are held (#44).
        if entry.status != VaspStatus::Active {
            let payment_ids: soroban_sdk::Vec<BytesN<32>> = env.storage()
                .instance()
                .get(&symbol_short!("pay_ids"))
                .unwrap_or_else(|| soroban_sdk::Vec::new(&env));
            for payment_id in payment_ids.iter() {
                let mut payment: PaymentRecord = env.storage()
                    .persistent()
                    .get(&payment_id)
                    .expect("payment not found");
                if payment.status == PaymentStatus::Pending
                    && (payment.from_vasp == vasp || payment.to_vasp == vasp)
                {
                    payment.status = PaymentStatus::Held;
                    env.storage().persistent().set(&payment_id, &payment);
                    // Persistent entries expire unless extended, so every write keeps the record alive (#59).
                    env.storage().persistent().extend_ttl(&payment_id, LEDGER_TTL_THRESHOLD, LEDGER_TTL_EXTEND);
                }
            }
        }

        // Typed payload: the VASP and its new status, so consumers need no lookup (#50).
        env.events().publish((symbol_short!("vasp_upd"),), (vasp.clone(), entry.status));
    }

    /// Returns `VaspNotFound` instead of `None` for an unknown VASP (#46).
    pub fn get_vasp(env: Env, vasp: Address) -> Result<VaspEntry, ComplyError> {
        env.storage().persistent().get(&vasp).ok_or(ComplyError::VaspNotFound)
    }

    pub fn set_threshold(env: Env, caller: Address, asset: Address, jurisdiction: String, amount: i128) {
        caller.require_auth();
        Self::require_admin(&env, &caller);

        assert!(amount >= 0, "threshold must not be negative");

        // One storage entry per rule, so setting one rule does not rewrite the rest (#55).
        env.storage().instance().set(&(symbol_short!("thr"), asset.clone(), jurisdiction.clone()), &amount);
        // The event carries the rule's jurisdiction and amount, not only the asset (#51).
        env.events().publish((symbol_short!("thr_set"),), (asset.clone(), jurisdiction, amount));
    }

    pub fn get_threshold(env: Env, asset: Address, jurisdiction: String) -> Option<i128> {
        // A rule set for jurisdiction "*" applies wherever no specific rule exists (#52).
        if let Some(amount) = env.storage().instance().get(&(symbol_short!("thr"), asset.clone(), jurisdiction)) {
            return Some(amount);
        }
        let wildcard = String::from_str(&env, "*");
        env.storage().instance().get(&(symbol_short!("thr"), asset, wildcard))
    }

    /// Remove a threshold rule, so the asset and jurisdiction no longer have one (#54).
    pub fn remove_threshold(env: Env, caller: Address, asset: Address, jurisdiction: String) {
        caller.require_auth();
        Self::require_admin(&env, &caller);

        let key = (symbol_short!("thr"), asset.clone(), jurisdiction.clone());
        assert!(env.storage().instance().has(&key), "threshold not found");
        env.storage().instance().remove(&key);

        env.events().publish((symbol_short!("thr_del"),), (asset, caller, jurisdiction));
    }

    pub fn submit_payment(
        env: Env,
        from_vasp: Address,
        to_vasp: Address,
        beneficiary: Address,
        asset: Address,
        amount: i128,
    ) -> BytesN<32> {
        from_vasp.require_auth();

        // A VASP cannot pay itself (#31).
        assert!(from_vasp != to_vasp, "from_vasp and to_vasp must differ");
        assert!(amount > 0, "amount must be positive");
        assert!(
            !Self::is_admin(&env, &from_vasp) && !Self::is_admin(&env, &to_vasp),
            "an admin cannot be a payment party"
        );
        assert!(!Self::is_registry_frozen(env.clone()), "registry is frozen");

        let from_entry: VaspEntry = env.storage().persistent().get(&from_vasp).expect("from_vasp not registered");
        let to_entry: VaspEntry = env.storage().persistent().get(&to_vasp).expect("to_vasp not registered");

        assert!(from_entry.status == VaspStatus::Active, "from_vasp not active");
        assert!(to_entry.status == VaspStatus::Active, "to_vasp not active");

        if let Some(expires) = from_entry.expires_at {
            assert!(env.ledger().timestamp() < expires, "from_vasp registration expired");
        }
        if let Some(expires) = to_entry.expires_at {
            assert!(env.ledger().timestamp() < expires, "to_vasp registration expired");
        }

        let payment_id = Self::generate_payment_id(&env);

        let status = if let Some(threshold) = Self::get_threshold(env.clone(), asset.clone(), jurisdiction_from_vasp(&to_entry)) {
            if amount < threshold {
                PaymentStatus::Released
            } else {
                PaymentStatus::Pending
            }
        } else {
            PaymentStatus::Pending
        };

        let payment = PaymentRecord {
            id: payment_id.clone(),
            from_vasp: from_vasp.clone(),
            to_vasp: to_vasp.clone(),
            beneficiary: beneficiary.clone(),
            asset: asset.clone(),
            amount,
            status: status.clone(),
            attestation_hash: None,
            ivms_version: None,
            created_at: env.ledger().timestamp(),
            resolved_at: if status == PaymentStatus::Released { Some(env.ledger().timestamp()) } else { None },
            resolution_reason: None,
        };

        env.storage().persistent().set(&payment_id, &payment);
        // Persistent entries expire unless extended, so every write keeps the record alive (#59).
        env.storage().persistent().extend_ttl(&payment_id, LEDGER_TTL_THRESHOLD, LEDGER_TTL_EXTEND);

        let mut payment_ids: soroban_sdk::Vec<BytesN<32>> = env.storage()
            .instance()
            .get(&symbol_short!("pay_ids"))
            .unwrap_or_else(|| soroban_sdk::Vec::new(&env));
        payment_ids.push_back(payment_id.clone());
        env.storage().instance().set(&symbol_short!("pay_ids"), &payment_ids);
        Self::index_payment(&env, &from_vasp, true, &payment_id);
        Self::index_payment(&env, &to_vasp, false, &payment_id);

        // Amount, parties and asset are in the event so indexers need no lookup (#38).
        env.events().publish(
            (symbol_short!("pay_sub"),),
            (payment_id.clone(), from_vasp, to_vasp, asset, amount),
        );

        payment_id
    }

    pub fn get_payment(env: Env, payment_id: BytesN<32>) -> Option<PaymentRecord> {
        env.storage().persistent().get(&payment_id)
    }

    /// Outcome of a payment and when it was resolved, without the full record (#45).
    pub fn get_payment_status(env: Env, payment_id: BytesN<32>) -> Option<(PaymentStatus, Option<u64>)> {
        env.storage()
            .persistent()
            .get::<_, PaymentRecord>(&payment_id)
            .map(|payment| (payment.status, payment.resolved_at))
    }

    pub fn get_payment_count(env: Env) -> u64 {
        env.storage().instance().get(&symbol_short!("counter")).unwrap_or(0)
    }

    /// Returns the contract version string (semver).
    /// Returns a Soroban `String`, since contract functions cannot return `&str`.
    pub fn version(env: Env) -> String {
        String::from_str(&env, VERSION)
    }

    /// Returns the sequential u64 counter embedded in a payment ID.
    /// Payment IDs are 32-byte values with the counter stored in the first
    /// 8 bytes as little-endian u64. This helper lets dashboards that index
    /// payments by u64 derive the numeric ID from the on-chain BytesN<32>.
    pub fn payment_id_to_u64(_env: Env, payment_id: BytesN<32>) -> u64 {
        let arr = payment_id.to_array();
        let mut counter_bytes = [0u8; 8];
        counter_bytes.copy_from_slice(&arr[..8]);
        u64::from_le_bytes(counter_bytes)
    }

    pub fn get_payments_by_vasp(
        env: Env,
        vasp_address: Address,
        as_sender: bool,
    ) -> soroban_sdk::Vec<BytesN<32>> {
        // Read the per-VASP index instead of scanning every payment (#62).
        env.storage()
            .persistent()
            .get(&(symbol_short!("pidx"), vasp_address, as_sender))
            .unwrap_or_else(|| soroban_sdk::Vec::new(&env))
    }

    /// One page of `get_payments_by_vasp`, starting at `start` and holding at most
    /// `limit` IDs (#63). Callers walk the index with increasing `start`.
    pub fn get_payments_by_vasp_page(
        env: Env,
        vasp_address: Address,
        as_sender: bool,
        start: u32,
        limit: u32,
    ) -> soroban_sdk::Vec<BytesN<32>> {
        let ids = Self::get_payments_by_vasp(env.clone(), vasp_address, as_sender);
        let mut page = soroban_sdk::Vec::new(&env);
        let end = start.saturating_add(limit).min(ids.len());
        for i in start..end {
            page.push_back(ids.get(i).unwrap());
        }
        page
    }

    pub fn submit_attestation(
        env: Env,
        caller_vasp: Address,
        payment_id: BytesN<32>,
        message_hash: BytesN<32>,
        ivms_version: String,
    ) {
        caller_vasp.require_auth();

        let mut payment: PaymentRecord = env.storage()
            .persistent()
            .get(&payment_id)
            .expect("payment not found");

        assert!(payment.status == PaymentStatus::Pending, "payment not pending");
        // Attestations after the payment deadline are refused (#34).
        assert!(
            env.ledger().timestamp() <= payment.created_at + PAYMENT_WINDOW_SECS,
            "payment deadline has passed"
        );

        let to_vasp_entry: VaspEntry = env.storage()
            .persistent()
            .get(&payment.to_vasp)
            .expect("to_vasp not found");

        let from_vasp_entry: VaspEntry = env.storage()
            .persistent()
            .get(&payment.from_vasp)
            .expect("from_vasp not found");

        assert!(to_vasp_entry.status == VaspStatus::Active, "to_vasp not active");
        assert!(from_vasp_entry.status == VaspStatus::Active, "from_vasp not active");
        assert!(caller_vasp == payment.to_vasp, "only beneficiary vasp can attest");

        // Expired registrations cannot attest, matching submit_payment (#42).
        let now = env.ledger().timestamp();
        if let Some(expires) = to_vasp_entry.expires_at {
            assert!(now < expires, "to_vasp registration expired");
        }
        if let Some(expires) = from_vasp_entry.expires_at {
            assert!(now < expires, "from_vasp registration expired");
        }

        assert!(&message_hash.to_array() != &[0u8; 32], "message_hash cannot be empty");

        // IVMS version is a short version tag, such as "1.0" (#36).
        assert!(
            ivms_version.len() > 0 && ivms_version.len() <= 16,
            "ivms_version must be 1 to 16 bytes"
        );

        // An existing attestation is never overwritten (#35).
        assert!(payment.attestation_hash.is_none(), "payment already attested");

        payment.status = PaymentStatus::Released;
        payment.attestation_hash = Some(message_hash.clone());
        payment.ivms_version = Some(ivms_version);
        payment.resolved_at = Some(env.ledger().timestamp());

        env.storage().persistent().set(&payment_id, &payment);
        // Persistent entries expire unless extended, so every write keeps the record alive (#59).
        env.storage().persistent().extend_ttl(&payment_id, LEDGER_TTL_THRESHOLD, LEDGER_TTL_EXTEND);
        env.events().publish((symbol_short!("att_sub"),), (payment_id.clone(), caller_vasp));
    }

    pub fn release_payment(env: Env, caller: Address, payment_id: BytesN<32>, reason: String) {
        caller.require_auth();
        Self::require_admin(&env, &caller);

        let mut payment: PaymentRecord = env.storage()
            .persistent()
            .get(&payment_id)
            .expect("payment not found");

        assert!(payment.status != PaymentStatus::Released, "payment already released");

        // The beneficiary must still be eligible at release time (#43).
        let to_entry: VaspEntry = env.storage()
            .persistent()
            .get(&payment.to_vasp)
            .expect("to_vasp not registered");
        assert!(to_entry.status == VaspStatus::Active, "to_vasp not active");
        if let Some(expires) = to_entry.expires_at {
            assert!(env.ledger().timestamp() < expires, "to_vasp registration expired");
        }

        payment.status = PaymentStatus::Released;
        payment.resolved_at = Some(env.ledger().timestamp());
        payment.resolution_reason = Some(reason.clone());

        env.storage().persistent().set(&payment_id, &payment);
        // Persistent entries expire unless extended, so every write keeps the record alive (#59).
        env.storage().persistent().extend_ttl(&payment_id, LEDGER_TTL_THRESHOLD, LEDGER_TTL_EXTEND);
        env.events().publish(
            (symbol_short!("pay_rel"),),
            (payment_id.clone(), caller.clone(), payment.amount, reason),
        );
    }

    pub fn reject_payment(env: Env, caller: Address, payment_id: BytesN<32>, reason: String) {
        caller.require_auth();
        Self::require_admin(&env, &caller);

        let mut payment: PaymentRecord = env.storage()
            .persistent()
            .get(&payment_id)
            .expect("payment not found");

        assert!(payment.status != PaymentStatus::Rejected, "payment already rejected");
        assert!(payment.status != PaymentStatus::Released, "cannot reject released payment");

        payment.status = PaymentStatus::Rejected;
        payment.resolved_at = Some(env.ledger().timestamp());
        payment.resolution_reason = Some(reason.clone());

        env.storage().persistent().set(&payment_id, &payment);
        // Persistent entries expire unless extended, so every write keeps the record alive (#59).
        env.storage().persistent().extend_ttl(&payment_id, LEDGER_TTL_THRESHOLD, LEDGER_TTL_EXTEND);
        env.events().publish(
            (symbol_short!("pay_rej"),),
            (payment_id.clone(), caller.clone(), payment.amount, reason),
        );
    }

    /// Let the sender reclaim a payment whose attestation window has passed (#33).
    pub fn reclaim_payment(env: Env, from_vasp: Address, payment_id: BytesN<32>) {
        from_vasp.require_auth();

        let mut payment: PaymentRecord = env.storage()
            .persistent()
            .get(&payment_id)
            .expect("payment not found");

        assert!(payment.from_vasp == from_vasp, "only the sender can reclaim");
        assert!(payment.status == PaymentStatus::Pending, "payment not pending");
        assert!(
            env.ledger().timestamp() > payment.created_at + PAYMENT_WINDOW_SECS,
            "payment deadline has not passed"
        );

        payment.status = PaymentStatus::Rejected;
        payment.resolved_at = Some(env.ledger().timestamp());

        env.storage().persistent().set(&payment_id, &payment);
        // Persistent entries expire unless extended, so every write keeps the record alive (#59).
        env.storage().persistent().extend_ttl(&payment_id, LEDGER_TTL_THRESHOLD, LEDGER_TTL_EXTEND);
        env.events().publish((symbol_short!("pay_rec"),), &payment_id);
    }

    /// Remove resolved payments from active storage (#66).
    ///
    /// The admin lists the payment IDs. Only released or rejected payments are
    /// removed. Returns how many payments were removed.
    pub fn prune_resolved_payments(
        env: Env,
        caller: Address,
        payment_ids: soroban_sdk::Vec<BytesN<32>>,
    ) -> u32 {
        caller.require_auth();
        Self::require_admin(&env, &caller);

        let mut pruned = 0u32;
        for payment_id in payment_ids.iter() {
            let Some(payment) = env.storage().persistent().get::<_, PaymentRecord>(&payment_id) else {
                continue;
            };
            if payment.status != PaymentStatus::Released && payment.status != PaymentStatus::Rejected {
                continue;
            }

            env.storage().persistent().remove(&payment_id);
            pruned += 1;
        }
        pruned
    }

    /// Add a payment to a VASP's sender or receiver index (#62).
    fn index_payment(env: &Env, vasp: &Address, as_sender: bool, payment_id: &BytesN<32>) {
        let key = (symbol_short!("pidx"), vasp.clone(), as_sender);
        let mut ids: soroban_sdk::Vec<BytesN<32>> = env.storage()
            .persistent()
            .get(&key)
            .unwrap_or_else(|| soroban_sdk::Vec::new(env));
        ids.push_back(payment_id.clone());
        env.storage().persistent().set(&key, &ids);
        env.storage().persistent().extend_ttl(&key, LEDGER_TTL_THRESHOLD, LEDGER_TTL_EXTEND);
    }

    fn generate_payment_id(env: &Env) -> BytesN<32> {
        let counter: u64 = env.storage().instance().get(&symbol_short!("counter")).unwrap_or(0);
        env.storage().instance().set(&symbol_short!("counter"), &(counter + 1));
        env.storage().instance().extend_ttl(LEDGER_TTL_THRESHOLD, LEDGER_TTL_EXTEND);

        // Hash the counter with ledger state so IDs are not sequential (#37).
        // Callers find payments through the index, not by guessing IDs.
        let mut seed = soroban_sdk::Bytes::new(env);
        seed.extend_from_slice(&counter.to_be_bytes());
        seed.extend_from_slice(&env.ledger().timestamp().to_be_bytes());
        seed.extend_from_slice(&env.ledger().sequence().to_be_bytes());
        env.crypto().sha256(&seed).into()
    }
}

fn jurisdiction_from_vasp(vasp: &VaspEntry) -> String {
    vasp.jurisdiction.clone()
}

impl ComplyRailContract {
    fn is_admin(env: &Env, caller: &Address) -> bool {
        // A keyed lookup instead of a scan of the admin list (#68).
        env.storage()
            .instance()
            .get(&(symbol_short!("admin"), caller.clone()))
            .unwrap_or(false)
    }

    fn require_admin(env: &Env, caller: &Address) {
        assert!(Self::is_admin(env, caller), "unauthorized: not an admin");
    }
}
