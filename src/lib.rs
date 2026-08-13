mod types;

use soroban_sdk::{contractimpl, contracttype, Address, Env, String, BytesN, Vec, symbol_short};
use types::*;

#[derive(Clone)]
#[contracttype]
pub struct ComplyRailContract;

const ADMINS: &str = "admins";
const VASPS: &str = "vasps";
const THRESHOLDS: &str = "thresholds";
const PAYMENTS: &str = "payments";
const PAYMENT_COUNTER: &str = "payment_counter";

#[contractimpl]
impl ComplyRailContract {
    pub fn initialize(env: Env, admin: Address) {
        admin.require_auth();

        let mut admins: Vec<Address> = env.storage().instance().get(&ADMINS).unwrap_or_else(|| Vec::new(&env));
        admins.push_back(admin);
        env.storage().instance().set(&ADMINS, &admins);
    }

    pub fn add_admin(env: Env, caller: Address, new_admin: Address) {
        caller.require_auth();
        Self::require_admin(&env, &caller);

        let mut admins: Vec<Address> = env.storage().instance().get(&ADMINS).unwrap_or_else(|| Vec::new(&env));
        admins.push_back(new_admin);
        env.storage().instance().set(&ADMINS, &admins);

        env.events().publish((symbol_short!("admin_add"),), new_admin);
    }

    pub fn remove_admin(env: Env, caller: Address, admin_to_remove: Address) {
        caller.require_auth();
        Self::require_admin(&env, &caller);

        let admins: Vec<Address> = env.storage().instance().get(&ADMINS).unwrap_or_else(|| Vec::new(&env));
        let mut new_admins: Vec<Address> = Vec::new(&env);

        for admin in admins.iter() {
            if admin != admin_to_remove {
                new_admins.push_back(admin);
            }
        }

        env.storage().instance().set(&ADMINS, &new_admins);
        env.events().publish((symbol_short!("admin_rem"),), admin_to_remove);
    }

    pub fn register_vasp(
        env: Env,
        caller: Address,
        vasp: Address,
        name: String,
        jurisdiction: String,
        public_key: BytesN<32>,
    ) {
        caller.require_auth();
        Self::require_admin(&env, &caller);

        let entry = VaspEntry {
            address: vasp.clone(),
            name: name.clone(),
            jurisdiction: jurisdiction.clone(),
            public_key: public_key.clone(),
            status: VaspStatus::Active,
            added_at: env.ledger().timestamp(),
        };

        let mut vasps: Vec<VaspEntry> = env.storage().instance().get(&VASPS).unwrap_or_else(|| Vec::new(&env));
        vasps.push_back(entry);
        env.storage().instance().set(&VASPS, &vasps);

        env.events().publish((symbol_short!("vasp_reg"),), (vasp, name));
    }

    pub fn update_vasp_status(env: Env, caller: Address, vasp: Address, status: VaspStatus) {
        caller.require_auth();
        Self::require_admin(&env, &caller);

        let mut vasps: Vec<VaspEntry> = env.storage().instance().get(&VASPS).unwrap_or_else(|| Vec::new(&env));

        for entry in vasps.iter_mut() {
            if entry.address == vasp {
                entry.status = status;
                break;
            }
        }

        env.storage().instance().set(&VASPS, &vasps);
        env.events().publish((symbol_short!("vasp_upd"),), (vasp.clone(), status as u32));
    }

    pub fn get_vasp(env: Env, vasp: Address) -> Option<VaspEntry> {
        let vasps: Vec<VaspEntry> = env.storage().instance().get(&VASPS).unwrap_or_else(|| Vec::new(&env));

        for entry in vasps.iter() {
            if entry.address == vasp {
                return Some(entry);
            }
        }

        None
    }

    pub fn set_threshold(env: Env, caller: Address, asset: Address, jurisdiction: String, amount: i128) {
        caller.require_auth();
        Self::require_admin(&env, &caller);

        let config = ThresholdConfig {
            asset: asset.clone(),
            jurisdiction: jurisdiction.clone(),
            threshold_amount: amount,
        };

        let mut thresholds: Vec<ThresholdConfig> = env.storage().instance().get(&THRESHOLDS).unwrap_or_else(|| Vec::new(&env));

        let mut found = false;
        for threshold in thresholds.iter_mut() {
            if threshold.asset == asset && threshold.jurisdiction == jurisdiction {
                threshold.threshold_amount = amount;
                found = true;
                break;
            }
        }

        if !found {
            thresholds.push_back(config);
        }

        env.storage().instance().set(&THRESHOLDS, &thresholds);
        env.events().publish((symbol_short!("thresh_set"),), (asset, jurisdiction));
    }

    pub fn get_threshold(env: Env, asset: Address, jurisdiction: String) -> Option<i128> {
        let thresholds: Vec<ThresholdConfig> = env.storage().instance().get(&THRESHOLDS).unwrap_or_else(|| Vec::new(&env));

        for config in thresholds.iter() {
            if config.asset == asset && config.jurisdiction == jurisdiction {
                return Some(config.threshold_amount);
            }
        }

        None
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

        let vasp_from = Self::get_vasp(env.clone(), from_vasp.clone());
        let vasp_to = Self::get_vasp(env.clone(), to_vasp.clone());

        assert!(vasp_from.is_some(), "from_vasp not registered");
        assert!(vasp_to.is_some(), "to_vasp not registered");
        assert!(vasp_from.unwrap().status == VaspStatus::Active, "from_vasp not active");
        assert!(vasp_to.unwrap().status == VaspStatus::Active, "to_vasp not active");

        let payment_id = Self::generate_payment_id(&env);

        let status = if let Some(threshold) = Self::get_threshold(env.clone(), asset.clone(), String::from_slice(&env, "US")) {
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

        let mut payments: Vec<PaymentRecord> = env.storage().instance().get(&PAYMENTS).unwrap_or_else(|| Vec::new(&env));
        payments.push_back(payment);
        env.storage().instance().set(&PAYMENTS, &payments);

        env.events().publish((symbol_short!("pay_sub"),), (payment_id.clone(), status as u32));

        payment_id
    }

    pub fn get_payment(env: Env, payment_id: BytesN<32>) -> Option<PaymentRecord> {
        let payments: Vec<PaymentRecord> = env.storage().instance().get(&PAYMENTS).unwrap_or_else(|| Vec::new(&env));

        for payment in payments.iter() {
            if payment.id == payment_id {
                return Some(payment);
            }
        }

        None
    }

    fn require_admin(env: &Env, caller: &Address) {
        let admins: Vec<Address> = env.storage().instance().get(&ADMINS).unwrap_or_else(|| Vec::new(env));

        let mut is_admin = false;
        for admin in admins.iter() {
            if admin == *caller {
                is_admin = true;
                break;
            }
        }

        assert!(is_admin, "unauthorized: caller is not an admin");
    }

    fn generate_payment_id(env: &Env) -> BytesN<32> {
        let counter: u64 = env.storage().instance().get(&PAYMENT_COUNTER).unwrap_or(0);
        env.storage().instance().set(&PAYMENT_COUNTER, &(counter + 1));

        let mut id_bytes = [0u8; 32];
        let counter_bytes = counter.to_le_bytes();
        id_bytes[..8].copy_from_slice(&counter_bytes);
        id_bytes[8..16].copy_from_slice(&env.ledger().timestamp().to_le_bytes());

        BytesN::from_array(env, &id_bytes)
    }
}
