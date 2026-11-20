mod types;

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests;

use soroban_sdk::{contract, contractimpl, Address, Env, String, BytesN, Map, symbol_short};
use types::*;

/// Extend a record once it has less than this many ledgers left (#59).
const LEDGER_TTL_THRESHOLD: u32 = 100_000;
/// Extend a record to this many ledgers from now (#59).
const LEDGER_TTL_EXTEND: u32 = 500_000;

#[contract]
pub struct ComplyRailContract;

#[contractimpl]
impl ComplyRailContract {
    pub fn initialize(env: Env, admin: Address) {
        admin.require_auth();
        let mut admins: soroban_sdk::Vec<Address> = soroban_sdk::Vec::new(&env);
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

        admins.push_back(new_admin.clone());
        env.storage().instance().set(&symbol_short!("admins"), &admins);
        env.events().publish((symbol_short!("adm_add"),), &new_admin);
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

        let expiry = expiry_days.map(|days| env.ledger().timestamp() + (days * 86400));

        let entry = VaspEntry {
            address: vasp.clone(),
            name,
            jurisdiction,
            public_key,
            status: VaspStatus::Active,
            added_at: env.ledger().timestamp(),
            expires_at: expiry,
        };

        env.storage().persistent().set(&vasp, &entry);
        // Persistent entries expire unless extended, so every write keeps the record alive (#59).
        env.storage().persistent().extend_ttl(&vasp, LEDGER_TTL_THRESHOLD, LEDGER_TTL_EXTEND);
        env.events().publish((symbol_short!("vasp_reg"),), &vasp);
    }

    pub fn renew_vasp_registration(env: Env, caller: Address, vasp: Address, extension_days: u64) {
        caller.require_auth();

        let mut entry: VaspEntry = env.storage().persistent().get(&vasp).expect("vasp not found");

        let current_expiry = entry.expires_at.unwrap_or(env.ledger().timestamp());
        entry.expires_at = Some(current_expiry + (extension_days * 86400));

        env.storage().persistent().set(&vasp, &entry);
        // Persistent entries expire unless extended, so every write keeps the record alive (#59).
        env.storage().persistent().extend_ttl(&vasp, LEDGER_TTL_THRESHOLD, LEDGER_TTL_EXTEND);
        env.events().publish((symbol_short!("vasp_renew"),), &vasp);
    }

    pub fn update_vasp_status(env: Env, caller: Address, vasp: Address, status: VaspStatus) {
        caller.require_auth();
        Self::require_admin(&env, &caller);

        let mut entry: VaspEntry = env.storage().persistent().get(&vasp).expect("vasp not found");
        entry.status = status;
        env.storage().persistent().set(&vasp, &entry);
        // Persistent entries expire unless extended, so every write keeps the record alive (#59).
        env.storage().persistent().extend_ttl(&vasp, LEDGER_TTL_THRESHOLD, LEDGER_TTL_EXTEND);

        env.events().publish((symbol_short!("vasp_upd"),), &vasp);
    }

    pub fn get_vasp(env: Env, vasp: Address) -> Option<VaspEntry> {
        env.storage().persistent().get(&vasp)
    }

    pub fn set_threshold(env: Env, caller: Address, asset: Address, jurisdiction: String, amount: i128) {
        caller.require_auth();
        Self::require_admin(&env, &caller);

        // A tuple key replaces the string built with format!, which does not compile
        // for Soroban strings (#57). Each rule is also stored on its own (#55).
        env.storage().instance().set(&ThresholdKey { asset: asset.clone(), jurisdiction }, &amount);
        env.events().publish((symbol_short!("thr_set"),), &asset);
    }

    pub fn get_threshold(env: Env, asset: Address, jurisdiction: String) -> Option<i128> {
        env.storage().instance().get(&ThresholdKey { asset, jurisdiction })
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
            PaymentStatus::Released
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
        };

        env.storage().instance().set(&payment_id, &payment);
        env.events().publish((symbol_short!("pay_sub"),), &payment_id);

        payment_id
    }

    pub fn get_payment(env: Env, payment_id: BytesN<32>) -> Option<PaymentRecord> {
        env.storage().instance().get(&payment_id)
    }

    pub fn get_payment_count(env: Env) -> u64 {
        env.storage().instance().get(&symbol_short!("counter")).unwrap_or(0)
    }

    pub fn get_payments_by_vasp(
        env: Env,
        vasp_address: Address,
        as_sender: bool,
    ) -> soroban_sdk::Vec<BytesN<32>> {
        let count: u64 = env.storage().instance().get(&symbol_short!("counter")).unwrap_or(0);
        let mut results = soroban_sdk::Vec::new(&env);

        for i in 0..count {
            let mut id_bytes = [0u8; 32];
            let counter_bytes = i.to_le_bytes();
            id_bytes[..8].copy_from_slice(&counter_bytes);
            let payment_id = BytesN::from_array(&env, &id_bytes);

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

        assert!(&message_hash.to_array() != &[0u8; 32], "message_hash cannot be empty");

        payment.status = PaymentStatus::Released;
        payment.attestation_hash = Some(message_hash.clone());
        payment.ivms_version = Some(ivms_version);
        payment.resolved_at = Some(env.ledger().timestamp());

        env.storage().instance().set(&payment_id, &payment);
        env.events().publish((symbol_short!("att_sub"),), &payment_id);
    }

    pub fn release_payment(env: Env, caller: Address, payment_id: BytesN<32>, reason: String) {
        caller.require_auth();
        Self::require_admin(&env, &caller);

        let mut payment: PaymentRecord = env.storage()
            .instance()
            .get(&payment_id)
            .expect("payment not found");

        assert!(payment.status != PaymentStatus::Released, "payment already released");

        payment.status = PaymentStatus::Released;
        payment.resolved_at = Some(env.ledger().timestamp());

        env.storage().instance().set(&payment_id, &payment);
        env.events().publish((symbol_short!("pay_rel"),), (payment_id.clone(), reason));
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

        env.storage().instance().set(&payment_id, &payment);
        env.events().publish((symbol_short!("pay_rej"),), (payment_id.clone(), reason));
    }

    fn generate_payment_id(env: &Env) -> BytesN<32> {
        let counter: u64 = env.storage().instance().get(&symbol_short!("counter")).unwrap_or(0);
        env.storage().instance().set(&symbol_short!("counter"), &(counter + 1));

        let mut id_bytes = [0u8; 32];
        let counter_bytes = counter.to_le_bytes();
        id_bytes[..8].copy_from_slice(&counter_bytes);

        BytesN::from_array(env, &id_bytes)
    }
}

fn jurisdiction_from_vasp(vasp: &VaspEntry) -> String {
    vasp.jurisdiction.clone()
}

impl ComplyRailContract {
    fn require_admin(env: &Env, caller: &Address) {
        let admins: soroban_sdk::Vec<Address> = env.storage()
            .instance()
            .get(&symbol_short!("admins"))
            .unwrap_or_else(|| soroban_sdk::Vec::new(env));

        let mut is_admin = false;
        for admin in admins.iter() {
            if admin == *caller {
                is_admin = true;
                break;
            }
        }

        assert!(is_admin, "unauthorized: not an admin");
    }
}
