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
    #[should_panic]
    fn test_initialize_twice_fails() {
        let env = Env::default();
        let admin = Address::random(&env);
        let attacker = Address::random(&env);

        ComplyRailContract::initialize(env.clone(), admin.clone());

        // A second initialize must fail and must not overwrite the admin set.
        ComplyRailContract::initialize(env.clone(), attacker.clone());
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
            message_hash,
            ivms_version,
        );

        assert_eq!(
            ComplyRailContract::get_payment(env.clone(), payment_id.clone()).unwrap().status,
            PaymentStatus::Released
        );
    }

    // --- VASP status transition tests (issue #4) ---

    fn setup_vasp(env: &Env) -> (Address, Address) {
        let admin = Address::random(env);
        let vasp = Address::random(env);
        ComplyRailContract::initialize(env.clone(), admin.clone());
        let name = String::from_slice(env, "Test VASP");
        let jurisdiction = String::from_slice(env, "US");
        let public_key = BytesN::from_array(env, &[0u8; 32]);
        ComplyRailContract::register_vasp(
            env.clone(),
            admin.clone(),
            vasp.clone(),
            name,
            jurisdiction,
            public_key,
        );
        (admin, vasp)
    }

    #[test]
    fn test_transition_active_to_suspended() {
        let env = Env::default();
        let (admin, vasp) = setup_vasp(&env);
        ComplyRailContract::update_vasp_status(
            env.clone(),
            admin.clone(),
            vasp.clone(),
            VaspStatus::Suspended,
        );
        assert_eq!(
            ComplyRailContract::get_vasp(env.clone(), vasp.clone()).unwrap().status,
            VaspStatus::Suspended
        );
    }

    #[test]
    fn test_transition_active_to_revoked() {
        let env = Env::default();
        let (admin, vasp) = setup_vasp(&env);
        ComplyRailContract::update_vasp_status(
            env.clone(),
            admin.clone(),
            vasp.clone(),
            VaspStatus::Revoked,
        );
        assert_eq!(
            ComplyRailContract::get_vasp(env.clone(), vasp.clone()).unwrap().status,
            VaspStatus::Revoked
        );
    }

    #[test]
    fn test_transition_suspended_to_active() {
        let env = Env::default();
        let (admin, vasp) = setup_vasp(&env);
        ComplyRailContract::update_vasp_status(
            env.clone(),
            admin.clone(),
            vasp.clone(),
            VaspStatus::Suspended,
        );
        ComplyRailContract::update_vasp_status(
            env.clone(),
            admin.clone(),
            vasp.clone(),
            VaspStatus::Active,
        );
        assert_eq!(
            ComplyRailContract::get_vasp(env.clone(), vasp.clone()).unwrap().status,
            VaspStatus::Active
        );
    }

    #[test]
    fn test_transition_suspended_to_revoked() {
        let env = Env::default();
        let (admin, vasp) = setup_vasp(&env);
        ComplyRailContract::update_vasp_status(
            env.clone(),
            admin.clone(),
            vasp.clone(),
            VaspStatus::Suspended,
        );
        ComplyRailContract::update_vasp_status(
            env.clone(),
            admin.clone(),
            vasp.clone(),
            VaspStatus::Revoked,
        );
        assert_eq!(
            ComplyRailContract::get_vasp(env.clone(), vasp.clone()).unwrap().status,
            VaspStatus::Revoked
        );
    }

    #[test]
    #[should_panic]
    fn test_transition_same_status_rejected() {
        let env = Env::default();
        let (admin, vasp) = setup_vasp(&env);
        // Active -> Active must be rejected (no-op transition).
        ComplyRailContract::update_vasp_status(
            env.clone(),
            admin.clone(),
            vasp.clone(),
            VaspStatus::Active,
        );
    }

    #[test]
    #[should_panic]
    fn test_transition_revoked_to_active_rejected() {
        let env = Env::default();
        let (admin, vasp) = setup_vasp(&env);
        ComplyRailContract::update_vasp_status(
            env.clone(),
            admin.clone(),
            vasp.clone(),
            VaspStatus::Revoked,
        );
        // Revoked is terminal; moving back to Active must be rejected.
        ComplyRailContract::update_vasp_status(
            env.clone(),
            admin.clone(),
            vasp.clone(),
            VaspStatus::Active,
        );
    }

    #[test]
    #[should_panic]
    fn test_transition_revoked_to_suspended_rejected() {
        let env = Env::default();
        let (admin, vasp) = setup_vasp(&env);
        ComplyRailContract::update_vasp_status(
            env.clone(),
            admin.clone(),
            vasp.clone(),
            VaspStatus::Revoked,
        );
        ComplyRailContract::update_vasp_status(
            env.clone(),
            admin.clone(),
            vasp.clone(),
            VaspStatus::Suspended,
        );
    }

    #[test]
    #[should_panic]
    fn test_transition_revoked_to_revoked_rejected() {
        let env = Env::default();
        let (admin, vasp) = setup_vasp(&env);
        ComplyRailContract::update_vasp_status(
            env.clone(),
            admin.clone(),
            vasp.clone(),
            VaspStatus::Revoked,
        );
        ComplyRailContract::update_vasp_status(
            env.clone(),
            admin.clone(),
            vasp.clone(),
            VaspStatus::Revoked,
        );
    }
}
