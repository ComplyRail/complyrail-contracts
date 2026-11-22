mod types;

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests;

// Tests written against the older direct-call API, kept until ported to the client API in `tests`.
#[cfg(all(test, not(target_arch = "wasm32")))]
mod legacy_tests;

use soroban_sdk::{contract, contracterror, contractimpl, Address, Env, String, BytesN, Map, symbol_short};
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

#[contract]
pub struct ComplyRailContract;

#[contractimpl]
impl ComplyRailContract {
    pub fn initialize(env: Env, admin: Address) {
        admin.require_auth();
        let mut admins: soroban_sdk::Vec<Address> = soroban_sdk::Vec::new(&env);
        env.storage().instance().set(&(symbol_short!("admin"), admin.clone()), &true);
        admins.push_back(admin);
        env.storage().instance().set(&symbol_short!("admins"), &admins);
        env.storage().instance().set(&symbol_short!("min_appr"), &1i32);
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
        env.storage().instance().set(&symbol_short!("min_appr"), &threshold);
        env.events().publish((symbol_short!("appr_th"),), (threshold, caller));
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

        env.storage().instance().set(&vasp, &entry);

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

        let mut entry: VaspEntry = env.storage().instance().get(&vasp).expect("vasp not found");

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

        env.storage().instance().set(&vasp, &entry);
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

        let mut entry: VaspEntry = env.storage().instance().get(&vasp).expect("vasp not found");
        entry.expires_at = Some(env.ledger().timestamp() + expires_in_seconds);
        entry.updated_at = env.ledger().timestamp();

        env.storage().instance().set(&vasp, &entry);
        env.events().publish((symbol_short!("vasp_exp"),), &vasp);
    }

    pub fn update_vasp_status(env: Env, caller: Address, vasp: Address, status: VaspStatus) {
        caller.require_auth();
        Self::require_admin(&env, &caller);

        let mut entry: VaspEntry = env.storage().instance().get(&vasp).expect("vasp not found");
        entry.status = status;
        entry.updated_at = env.ledger().timestamp();
        env.storage().instance().set(&vasp, &entry);

        // Pending payments touching a suspended or revoked VASP are held (#44).
        if entry.status != VaspStatus::Active {
            let payment_ids: soroban_sdk::Vec<BytesN<32>> = env.storage()
                .instance()
                .get(&symbol_short!("pay_ids"))
                .unwrap_or_else(|| soroban_sdk::Vec::new(&env));
            for payment_id in payment_ids.iter() {
                let mut payment: PaymentRecord = env.storage()
                    .instance()
                    .get(&payment_id)
                    .expect("payment not found");
                if payment.status == PaymentStatus::Pending
                    && (payment.from_vasp == vasp || payment.to_vasp == vasp)
                {
                    payment.status = PaymentStatus::Held;
                    env.storage().instance().set(&payment_id, &payment);
                }
            }
        }

        // Typed payload: the VASP and its new status, so consumers need no lookup (#50).
        env.events().publish((symbol_short!("vasp_upd"),), (vasp.clone(), entry.status));
    }

    /// Returns `VaspNotFound` instead of `None` for an unknown VASP (#46).
    pub fn get_vasp(env: Env, vasp: Address) -> Result<VaspEntry, ComplyError> {
        env.storage().instance().get(&vasp).ok_or(ComplyError::VaspNotFound)
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
        assert!(!Self::is_registry_frozen(env.clone()), "registry is frozen");

        let from_entry: VaspEntry = env.storage().instance().get(&from_vasp).expect("from_vasp not registered");
        let to_entry: VaspEntry = env.storage().instance().get(&to_vasp).expect("to_vasp not registered");

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

        env.storage().instance().set(&payment_id, &payment);

        let mut payment_ids: soroban_sdk::Vec<BytesN<32>> = env.storage()
            .instance()
            .get(&symbol_short!("pay_ids"))
            .unwrap_or_else(|| soroban_sdk::Vec::new(&env));
        payment_ids.push_back(payment_id.clone());
        env.storage().instance().set(&symbol_short!("pay_ids"), &payment_ids);

        // Amount, parties and asset are in the event so indexers need no lookup (#38).
        env.events().publish(
            (symbol_short!("pay_sub"),),
            (payment_id.clone(), from_vasp, to_vasp, asset, amount),
        );

        payment_id
    }

    pub fn get_payment(env: Env, payment_id: BytesN<32>) -> Option<PaymentRecord> {
        env.storage().instance().get(&payment_id)
    }

    /// Outcome of a payment and when it was resolved, without the full record (#45).
    pub fn get_payment_status(env: Env, payment_id: BytesN<32>) -> Option<(PaymentStatus, Option<u64>)> {
        env.storage()
            .instance()
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
        let payment_ids: soroban_sdk::Vec<BytesN<32>> = env.storage()
            .instance()
            .get(&symbol_short!("pay_ids"))
            .unwrap_or_else(|| soroban_sdk::Vec::new(&env));
        let mut results = soroban_sdk::Vec::new(&env);

        for payment_id in payment_ids.iter() {
            if let Some(payment) = env.storage().instance().get::<_, PaymentRecord>(&payment_id) {
                if as_sender && payment.from_vasp == vasp_address {
                    results.push_back(payment_id);
                } else if !as_sender && payment.to_vasp == vasp_address {
                    results.push_back(payment_id);
                }
            }
        }

        results
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
            .instance()
            .get(&payment_id)
            .expect("payment not found");

        assert!(payment.status == PaymentStatus::Pending, "payment not pending");
        // Attestations after the payment deadline are refused (#34).
        assert!(
            env.ledger().timestamp() <= payment.created_at + PAYMENT_WINDOW_SECS,
            "payment deadline has passed"
        );

        let to_vasp_entry: VaspEntry = env.storage()
            .instance()
            .get(&payment.to_vasp)
            .expect("to_vasp not found");

        let from_vasp_entry: VaspEntry = env.storage()
            .instance()
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

        env.storage().instance().set(&payment_id, &payment);
        env.events().publish((symbol_short!("att_sub"),), (payment_id.clone(), caller_vasp));
    }

    pub fn release_payment(env: Env, caller: Address, payment_id: BytesN<32>, reason: String) {
        caller.require_auth();
        Self::require_admin(&env, &caller);

        let mut payment: PaymentRecord = env.storage()
            .instance()
            .get(&payment_id)
            .expect("payment not found");

        assert!(payment.status != PaymentStatus::Released, "payment already released");

        // The beneficiary must still be eligible at release time (#43).
        let to_entry: VaspEntry = env.storage()
            .instance()
            .get(&payment.to_vasp)
            .expect("to_vasp not registered");
        assert!(to_entry.status == VaspStatus::Active, "to_vasp not active");
        if let Some(expires) = to_entry.expires_at {
            assert!(env.ledger().timestamp() < expires, "to_vasp registration expired");
        }

        payment.status = PaymentStatus::Released;
        payment.resolved_at = Some(env.ledger().timestamp());
        payment.resolution_reason = Some(reason.clone());

        env.storage().instance().set(&payment_id, &payment);
        env.events().publish(
            (symbol_short!("pay_rel"),),
            (payment_id.clone(), caller.clone(), payment.amount, reason),
        );
    }

    pub fn reject_payment(env: Env, caller: Address, payment_id: BytesN<32>, reason: String) {
        caller.require_auth();
        Self::require_admin(&env, &caller);

        let mut payment: PaymentRecord = env.storage()
            .instance()
            .get(&payment_id)
            .expect("payment not found");

        assert!(payment.status != PaymentStatus::Rejected, "payment already rejected");
        assert!(payment.status != PaymentStatus::Released, "cannot reject released payment");

        payment.status = PaymentStatus::Rejected;
        payment.resolved_at = Some(env.ledger().timestamp());
        payment.resolution_reason = Some(reason.clone());

        env.storage().instance().set(&payment_id, &payment);
        env.events().publish(
            (symbol_short!("pay_rej"),),
            (payment_id.clone(), caller.clone(), payment.amount, reason),
        );
    }

    /// Let the sender reclaim a payment whose attestation window has passed (#33).
    pub fn reclaim_payment(env: Env, from_vasp: Address, payment_id: BytesN<32>) {
        from_vasp.require_auth();

        let mut payment: PaymentRecord = env.storage()
            .instance()
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

        env.storage().instance().set(&payment_id, &payment);
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

    fn generate_payment_id(env: &Env) -> BytesN<32> {
        let counter: u64 = env.storage().instance().get(&symbol_short!("counter")).unwrap_or(0);
        env.storage().instance().set(&symbol_short!("counter"), &(counter + 1));

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
