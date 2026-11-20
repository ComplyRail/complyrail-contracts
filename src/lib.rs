mod types;

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests;

use soroban_sdk::{contract, contractimpl, Address, Env, String, BytesN, Map, symbol_short};
use types::*;

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
        env.storage().instance().set(&symbol_short!("min_appr"), &threshold);
        env.events().publish((symbol_short!("appr_th"),), &threshold);
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

        env.storage().instance().set(&vasp, &entry);
        env.events().publish((symbol_short!("vasp_reg"),), &vasp);
    }

    pub fn renew_vasp_registration(env: Env, caller: Address, vasp: Address, extension_days: u64) {
        caller.require_auth();

        let mut entry: VaspEntry = env.storage().instance().get(&vasp).expect("vasp not found");

        let current_expiry = entry.expires_at.unwrap_or(env.ledger().timestamp());
        entry.expires_at = Some(current_expiry + (extension_days * 86400));

        env.storage().instance().set(&vasp, &entry);
        env.events().publish((symbol_short!("vasp_renew"),), &vasp);
    }

    pub fn update_vasp_status(env: Env, caller: Address, vasp: Address, status: VaspStatus) {
        caller.require_auth();
        Self::require_admin(&env, &caller);

        let mut entry: VaspEntry = env.storage().instance().get(&vasp).expect("vasp not found");
        entry.status = status;
        env.storage().instance().set(&vasp, &entry);

        env.events().publish((symbol_short!("vasp_upd"),), &vasp);
    }

    pub fn get_vasp(env: Env, vasp: Address) -> Option<VaspEntry> {
        env.storage().instance().get(&vasp)
    }

    pub fn set_threshold(env: Env, caller: Address, asset: Address, jurisdiction: String, amount: i128) {
        caller.require_auth();
        Self::require_admin(&env, &caller);

        let mut thresholds: Map<String, i128> = env.storage()
            .instance()
            .get(&symbol_short!("thresh"))
            .unwrap_or_else(|| Map::new(&env));

        let key = format_threshold_key(&env, &asset, &jurisdiction);
        thresholds.set(key, amount);

        env.storage().instance().set(&symbol_short!("thresh"), &thresholds);
        env.events().publish((symbol_short!("thr_set"),), &asset);
    }

    pub fn get_threshold(env: Env, asset: Address, jurisdiction: String) -> Option<i128> {
        let thresholds: Map<String, i128> = env.storage()
            .instance()
            .get(&symbol_short!("thresh"))
            .unwrap_or_else(|| Map::new(&env));

        let key = format_threshold_key(&env, &asset, &jurisdiction);
        thresholds.get(key)
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
        attestation_hash: BytesN<32>,
        ivms_version: String,
    ) {
        caller_vasp.require_auth();

        let mut payment: PaymentRecord = env.storage().instance().get(&payment_id).expect("payment not found");

        assert!(payment.status == PaymentStatus::Pending, "payment not pending");
        assert!(caller_vasp == payment.from_vasp, "only sender vasp can attest");

        payment.attestation_hash = Some(attestation_hash.clone());
        payment.ivms_version = Some(ivms_version.clone());

        env.storage().instance().set(&payment_id, &payment);
        env.events().publish((symbol_short!("attest"),), &payment_id);
    }

    pub fn release_payment(
        env: Env,
        caller: Address,
        payment_id: BytesN<32>,
        override_reason: Option<String>,
    ) {
        caller.require_auth();

        let mut payment: PaymentRecord = env.storage().instance().get(&payment_id).expect("payment not found");

        assert!(payment.status != PaymentStatus::Released, "payment already released");

        let is_admin = Self::is_admin(&env, &caller);

        if is_admin {
            // Admin override requires an explicit, non-empty reason.
            let reason = override_reason.expect("admin override requires a reason");
            assert!(reason.len() > 0, "admin override requires a reason");
            env.events().publish((symbol_short!("pay_ovr"),), &(payment_id.clone(), reason));
        } else {
            assert!(caller == payment.from_vasp, "only sender vasp or admin can release");
            // A payment requiring attestation must have an attestation hash before release.
            assert!(payment.attestation_hash.is_some(), "attestation required before release");
        }

        payment.status = PaymentStatus::Released;
        payment.resolved_at = Some(env.ledger().timestamp());

        env.storage().instance().set(&payment_id, &payment);
        env.events().publish((symbol_short!("pay_rel"),), &payment_id);
    }

    fn require_admin(env: &Env, caller: &Address) {
        assert!(Self::is_admin(env, caller), "caller is not an admin");
    }

    fn is_admin(env: &Env, caller: &Address) -> bool {
        let admins: soroban_sdk::Vec<Address> = env.storage()
            .instance()
            .get(&symbol_short!("admins"))
            .unwrap_or_else(|| soroban_sdk::Vec::new(env));

        for admin in admins.iter() {
            if &admin == caller {
                return true;
            }
        }
        false
    }

    fn generate_payment_id(env: &Env) -> BytesN<32> {
        let counter: u64 = env.storage().instance().get(&symbol_short!("counter")).unwrap_or(0);
        let next = counter + 1;
        env.storage().instance().set(&symbol_short!("counter"), &next);

        let mut id_bytes = [0u8; 32];
        let counter_bytes = counter.to_le_bytes();
        id_bytes[..8].copy_from_slice(&counter_bytes);
        BytesN::from_array(env, &id_bytes)
    }
}

fn format_threshold_key(env: &Env, asset: &Address, jurisdiction: &String) -> String {
    let mut key = String::from_str(env, "thr:");
    key = key.concat(&String::from_str(env, &asset.to_string()));
    key = key.concat(&String::from_str(env, ":"));
    key = key.concat(jurisdiction);
    key
}

fn jurisdiction_from_vasp(entry: &VaspEntry) -> String {
    entry.jurisdiction.clone()
}
