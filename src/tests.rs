#[cfg(test)]
mod tests {
    use crate::*;
    use soroban_sdk::{Address, Env, BytesN, String};

    #[test]
    fn test_initialize() {
        let env = Env::default();
        let admin = Address::random(&env);

        ComplyRailContract::initialize(env.clone(), admin.clone());

        let stored_admin: Address = env.storage().instance().get(&soroban_sdk::symbol_short!("admin")).unwrap();
        assert_eq!(stored_admin, admin);
    }

    #[test]
    fn test_register_vasp() {
        let env = Env::default();
        let admin = Address::random(&env);
        let vasp = Address::random(&env);

        ComplyRailContract::initialize(env.clone(), admin.clone());

        let name = String::from_slice(&env, "Anchor A");
        let jurisdiction = String::from_slice(&env, "US");
        let public_key = BytesN::from_array(&env, &[0u8; 32]);

        ComplyRailContract::register_vasp(
            env.clone(),
            admin.clone(),
            vasp.clone(),
            name,
            jurisdiction,
            public_key.clone(),
        );

        let stored_vasp = ComplyRailContract::get_vasp(env.clone(), vasp.clone());
        assert!(stored_vasp.is_some());

        let entry = stored_vasp.unwrap();
        assert_eq!(entry.address, vasp);
        assert_eq!(entry.status, VaspStatus::Active);
    }

    #[test]
    fn test_set_threshold() {
        let env = Env::default();
        let admin = Address::random(&env);
        let asset = Address::random(&env);

        ComplyRailContract::initialize(env.clone(), admin.clone());

        let jurisdiction = String::from_slice(&env, "US");
        let threshold = 1000_0000000; // 1000 in stroops

        ComplyRailContract::set_threshold(env.clone(), admin.clone(), asset.clone(), jurisdiction.clone(), threshold);

        let stored_threshold = ComplyRailContract::get_threshold(env.clone(), asset.clone(), jurisdiction.clone());
        assert_eq!(stored_threshold, Some(threshold));
    }

    #[test]
    fn test_below_threshold_auto_release() {
        let env = Env::default();
        let admin = Address::random(&env);
        let from_vasp = Address::random(&env);
        let to_vasp = Address::random(&env);
        let beneficiary = Address::random(&env);
        let asset = Address::random(&env);

        ComplyRailContract::initialize(env.clone(), admin.clone());

        // Register VASPs
        let name = String::from_slice(&env, "Test VASP");
        let jurisdiction = String::from_slice(&env, "US");
        let public_key = BytesN::from_array(&env, &[0u8; 32]);

        ComplyRailContract::register_vasp(
            env.clone(),
            admin.clone(),
            from_vasp.clone(),
            name.clone(),
            jurisdiction.clone(),
            public_key.clone(),
        );

        ComplyRailContract::register_vasp(
            env.clone(),
            admin.clone(),
            to_vasp.clone(),
            name,
            jurisdiction.clone(),
            public_key,
        );

        // Set threshold
        let threshold = 1000_0000000;
        ComplyRailContract::set_threshold(env.clone(), admin.clone(), asset.clone(), jurisdiction.clone(), threshold);

        // Submit payment below threshold
        let payment_amount = 500_0000000;
        let payment_id = ComplyRailContract::submit_payment(
            env.clone(),
            from_vasp.clone(),
            to_vasp.clone(),
            beneficiary.clone(),
            asset.clone(),
            payment_amount,
        );

        // Verify payment is released
        let payment = ComplyRailContract::get_payment(env.clone(), payment_id.clone());
        assert!(payment.is_some());

        let record = payment.unwrap();
        assert_eq!(record.status, PaymentStatus::Released);
        assert_eq!(record.amount, payment_amount);
    }

    #[test]
    fn test_above_threshold_pending() {
        let env = Env::default();
        let admin = Address::random(&env);
        let from_vasp = Address::random(&env);
        let to_vasp = Address::random(&env);
        let beneficiary = Address::random(&env);
        let asset = Address::random(&env);

        ComplyRailContract::initialize(env.clone(), admin.clone());

        // Register VASPs
        let name = String::from_slice(&env, "Test VASP");
        let jurisdiction = String::from_slice(&env, "US");
        let public_key = BytesN::from_array(&env, &[0u8; 32]);

        ComplyRailContract::register_vasp(
            env.clone(),
            admin.clone(),
            from_vasp.clone(),
            name.clone(),
            jurisdiction.clone(),
            public_key.clone(),
        );

        ComplyRailContract::register_vasp(
            env.clone(),
            admin.clone(),
            to_vasp.clone(),
            name,
            jurisdiction.clone(),
            public_key,
        );

        // Set threshold
        let threshold = 1000_0000000;
        ComplyRailContract::set_threshold(env.clone(), admin.clone(), asset.clone(), jurisdiction.clone(), threshold);

        // Submit payment above threshold
        let payment_amount = 2000_0000000;
        let payment_id = ComplyRailContract::submit_payment(
            env.clone(),
            from_vasp.clone(),
            to_vasp.clone(),
            beneficiary.clone(),
            asset.clone(),
            payment_amount,
        );

        // Verify payment is pending (not released)
        let payment = ComplyRailContract::get_payment(env.clone(), payment_id.clone());
        assert!(payment.is_some());

        let record = payment.unwrap();
        assert_eq!(record.status, PaymentStatus::Pending);
        assert_eq!(record.amount, payment_amount);
    }

    #[test]
    fn test_attestation_release() {
        let env = Env::default();
        let admin = Address::random(&env);
        let from_vasp = Address::random(&env);
        let to_vasp = Address::random(&env);
        let beneficiary = Address::random(&env);
        let asset = Address::random(&env);

        ComplyRailContract::initialize(env.clone(), admin.clone());

        let name = String::from_slice(&env, "Test VASP");
        let jurisdiction = String::from_slice(&env, "US");
        let public_key = BytesN::from_array(&env, &[0u8; 32]);

        ComplyRailContract::register_vasp(
            env.clone(),
            admin.clone(),
            from_vasp.clone(),
            name.clone(),
            jurisdiction.clone(),
            public_key.clone(),
        );

        ComplyRailContract::register_vasp(
            env.clone(),
            admin.clone(),
            to_vasp.clone(),
            name,
            jurisdiction.clone(),
            public_key,
        );

        let threshold = 1000_0000000;
        ComplyRailContract::set_threshold(env.clone(), admin.clone(), asset.clone(), jurisdiction.clone(), threshold);

        let payment_amount = 2000_0000000;
        let payment_id = ComplyRailContract::submit_payment(
            env.clone(),
            from_vasp.clone(),
            to_vasp.clone(),
            beneficiary.clone(),
            asset.clone(),
            payment_amount,
        );

        assert_eq!(
            ComplyRailContract::get_payment(env.clone(), payment_id.clone()).unwrap().status,
            PaymentStatus::Pending
        );

        let message_hash = BytesN::from_array(&env, &[1u8; 32]);
        let ivms_version = String::from_slice(&env, "1.0");

        ComplyRailContract::submit_attestation(
            env.clone(),
            to_vasp.clone(),
            payment_id.clone(),
            message_hash.clone(),
            ivms_version.clone(),
        );

        let payment = ComplyRailContract::get_payment(env.clone(), payment_id.clone()).unwrap();
        assert_eq!(payment.status, PaymentStatus::Released);
        assert_eq!(payment.attestation_hash, Some(message_hash));
        assert_eq!(payment.ivms_version, Some(ivms_version));
    }

    #[test]
    fn test_renew_active_vasp_extends_from_expires_at() {
        let env = Env::default();
        let admin = Address::random(&env);
        let vasp = Address::random(&env);

        ComplyRailContract::initialize(env.clone(), admin.clone());

        let name = String::from_slice(&env, "Test VASP");
        let jurisdiction = String::from_slice(&env, "US");
        let public_key = BytesN::from_array(&env, &[0u8; 32]);

        ComplyRailContract::register_vasp(
            env.clone(),
            admin.clone(),
            vasp.clone(),
            name,
            jurisdiction,
            public_key,
        );

        // Active VASP with a future expiry: renewal extends from expires_at.
        let now = env.ledger().timestamp();
        let future_expiry = now + 1_000;
        let extension = 500u64;

        ComplyRailContract::renew_vasp_registration(
            env.clone(),
            admin.clone(),
            vasp.clone(),
            future_expiry,
            extension,
        );

        let entry = ComplyRailContract::get_vasp(env.clone(), vasp.clone()).unwrap();
        assert_eq!(entry.expires_at, Some(future_expiry + extension));
    }

    #[test]
    fn test_renew_expired_vasp_extends_from_now() {
        let env = Env::default();
        let admin = Address::random(&env);
        let vasp = Address::random(&env);

        ComplyRailContract::initialize(env.clone(), admin.clone());

        let name = String::from_slice(&env, "Test VASP");
        let jurisdiction = String::from_slice(&env, "US");
        let public_key = BytesN::from_array(&env, &[0u8; 32]);

        ComplyRailContract::register_vasp(
            env.clone(),
            admin.clone(),
            vasp.clone(),
            name,
            jurisdiction,
            public_key,
        );

        // Expired VASP: renewal must extend from now, not from the past expiry.
        let now = env.ledger().timestamp();
        let past_expiry = now.saturating_sub(1_000);
        let extension = 500u64;

        ComplyRailContract::renew_vasp_registration(
            env.clone(),
            admin.clone(),
            vasp.clone(),
            past_expiry,
            extension,
        );

        let entry = ComplyRailContract::get_vasp(env.clone(), vasp.clone()).unwrap();
        assert_eq!(entry.expires_at, Some(now + extension));
    }

    #[test]
    fn test_renew_unlimited_vasp_stays_unlimited() {
        let env = Env::default();
        let admin = Address::random(&env);
        let vasp = Address::random(&env);

        ComplyRailContract::initialize(env.clone(), admin.clone());

        let name = String::from_slice(&env, "Test VASP");
        let jurisdiction = String::from_slice(&env, "US");
        let public_key = BytesN::from_array(&env, &[0u8; 32]);

        ComplyRailContract::register_vasp(
            env.clone(),
            admin.clone(),
            vasp.clone(),
            name,
            jurisdiction,
            public_key,
        );

        // Unlimited VASP (no expiry): renewal must not introduce an expiry.
        let extension = 500u64;

        ComplyRailContract::renew_vasp_registration(
            env.clone(),
            admin.clone(),
            vasp.clone(),
            u64::MAX,
            extension,
        );

        let entry = ComplyRailContract::get_vasp(env.clone(), vasp.clone()).unwrap();
        assert_eq!(entry.expires_at, None);
    }

    #[test]
    fn test_admin

/* … truncated 3628 chars — edit only what you need near the top … */
