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
        env.storage().instance().set(&symbol_short!("admin"), &admin);
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

        let admin: Address = env.storage().instance().get(&symbol_short!("admin")).expect("admin not set");
        assert!(admin == caller, "unauthorized");

        let entry = VaspEntry {
            address: vasp.clone(),
            name,
            jurisdiction,
            public_key,
            status: VaspStatus::Active,
            added_at: env.ledger().timestamp(),
        };

        env.storage().instance().set(&vasp, &entry);
        env.events().publish((symbol_short!("vasp_reg"),), &vasp);
    }

    pub fn update_vasp_status(env: Env, caller: Address, vasp: Address, status: VaspStatus) {
        caller.require_auth();

        let admin: Address = env.storage().instance().get(&symbol_short!("admin")).expect("admin not set");
        assert!(admin == caller, "unauthorized");

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

        let admin: Address = env.storage().instance().get(&symbol_short!("admin")).expect("admin not set");
        assert!(admin == caller, "unauthorized");

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

        let admin: Address = env.storage().instance().get(&symbol_short!("admin")).expect("admin not set");
        assert!(admin == caller, "only admin can release");

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

        let admin: Address = env.storage().instance().get(&symbol_short!("admin")).expect("admin not set");
        assert!(admin == caller, "only admin can reject");

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

fn format_threshold_key(env: &Env, asset: &Address, jurisdiction: &String) -> String {
    let asset_str = asset.to_string();
    let combined = format!("{}:{}", asset_str, jurisdiction);
    String::from_str(env, &combined)
}

fn jurisdiction_from_vasp(vasp: &VaspEntry) -> String {
    vasp.jurisdiction.clone()
}
