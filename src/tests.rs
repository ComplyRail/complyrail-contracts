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
    fn test_admin_release() {
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

        let reason = String::from_slice(&env, "Manual override");
        ComplyRailContract::release_payment(env.clone(), admin.clone(), payment_id.clone(), reason);

        let payment = ComplyRailContract::get_payment(env.clone(), payment_id.clone()).unwrap();
        assert_eq!(payment.status, PaymentStatus::Released);
        assert!(payment.resolved_at.is_some());
    }

    #[test]
    fn test_reject_payment() {
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

        let reason = String::from_slice(&env, "Compliance check failed");
        ComplyRailContract::reject_payment(env.clone(), admin.clone(), payment_id.clone(), reason);

        let payment = ComplyRailContract::get_payment(env.clone(), payment_id.clone()).unwrap();
        assert_eq!(payment.status, PaymentStatus::Rejected);
        assert!(payment.resolved_at.is_some());
    }
}

#[cfg(test)]
mod issue_tests {
    use crate::*;
    use soroban_sdk::testutils::{Address as _, Ledger};
    use soroban_sdk::{Address, BytesN, Env, String};

    struct Setup {
        env: Env,
        contract: Address,
        admin: Address,
    }

    fn setup() -> Setup {
        let env = Env::default();
        env.mock_all_auths();
        let contract = env.register_contract(None, ComplyRailContract);
        let admin = Address::generate(&env);
        env.as_contract(&contract, || {
            ComplyRailContract::initialize(env.clone(), admin.clone());
        });
        Setup { env, contract, admin }
    }

    fn register(s: &Setup, vasp: &Address, expiry_days: Option<u64>) {
        s.env.as_contract(&s.contract, || {
            ComplyRailContract::register_vasp(
                s.env.clone(),
                s.admin.clone(),
                vasp.clone(),
                String::from_str(&s.env, "VASP"),
                String::from_str(&s.env, "US"),
                BytesN::from_array(&s.env, &[0u8; 32]),
                expiry_days,
            );
        });
    }

    fn pay(s: &Setup, from: &Address, to: &Address, asset: &Address, amount: i128) -> PaymentStatus {
        s.env.as_contract(&s.contract, || {
            let id = ComplyRailContract::submit_payment(
                s.env.clone(),
                from.clone(),
                to.clone(),
                Address::generate(&s.env),
                asset.clone(),
                amount,
            );
            ComplyRailContract::get_payment(s.env.clone(), id).unwrap().status
        })
    }

    fn set_time(s: &Setup, ts: u64) {
        s.env.ledger().with_mut(|l| l.timestamp = ts);
    }

    // Issue #81: VASP expiry at payment time

    #[test]
    #[should_panic(expected = "from_vasp registration expired")]
    fn test_expired_originator() {
        let s = setup();
        let (from, to, asset) = (Address::generate(&s.env), Address::generate(&s.env), Address::generate(&s.env));
        register(&s, &from, Some(1));
        register(&s, &to, None);
        set_time(&s, 86400 + 1);
        pay(&s, &from, &to, &asset, 100);
    }

    #[test]
    #[should_panic(expected = "to_vasp registration expired")]
    fn test_expired_beneficiary() {
        let s = setup();
        let (from, to, asset) = (Address::generate(&s.env), Address::generate(&s.env), Address::generate(&s.env));
        register(&s, &from, None);
        register(&s, &to, Some(1));
        set_time(&s, 86400 + 1);
        pay(&s, &from, &to, &asset, 100);
    }

    #[test]
    fn test_payment_one_second_before_expiry() {
        let s = setup();
        let (from, to, asset) = (Address::generate(&s.env), Address::generate(&s.env), Address::generate(&s.env));
        register(&s, &from, Some(1));
        register(&s, &to, Some(1));
        set_time(&s, 86400 - 1);
        assert_eq!(pay(&s, &from, &to, &asset, 100), PaymentStatus::Released);
    }

    #[test]
    #[should_panic(expected = "from_vasp registration expired")]
    fn test_payment_at_exact_expiry_second() {
        let s = setup();
        let (from, to, asset) = (Address::generate(&s.env), Address::generate(&s.env), Address::generate(&s.env));
        register(&s, &from, Some(1));
        register(&s, &to, Some(1));
        set_time(&s, 86400);
        pay(&s, &from, &to, &asset, 100);
    }

    // Issue #82: admin add and duplicate detection

    #[test]
    fn test_add_admin() {
        let s = setup();
        let new_admin = Address::generate(&s.env);
        s.env.as_contract(&s.contract, || {
            ComplyRailContract::add_admin(s.env.clone(), s.admin.clone(), new_admin.clone());
            let admins = ComplyRailContract::get_admins(s.env.clone());
            assert_eq!(admins.len(), 2);
            assert!(admins.contains(&new_admin));
        });
    }

    #[test]
    #[should_panic(expected = "admin already exists")]
    fn test_add_duplicate_admin() {
        let s = setup();
        let new_admin = Address::generate(&s.env);
        s.env.as_contract(&s.contract, || {
            ComplyRailContract::add_admin(s.env.clone(), s.admin.clone(), new_admin.clone());
            ComplyRailContract::add_admin(s.env.clone(), s.admin.clone(), new_admin.clone());
        });
    }

    #[test]
    #[should_panic(expected = "unauthorized: not an admin")]
    fn test_non_admin_cannot_add_admin() {
        let s = setup();
        let outsider = Address::generate(&s.env);
        let new_admin = Address::generate(&s.env);
        s.env.as_contract(&s.contract, || {
            ComplyRailContract::add_admin(s.env.clone(), outsider, new_admin);
        });
    }

    // Issue #83: overflow in expiry arithmetic

    #[test]
    #[should_panic(expected = "expiry overflow")]
    fn test_register_expiry_overflow() {
        let s = setup();
        let vasp = Address::generate(&s.env);
        register(&s, &vasp, Some(u64::MAX));
    }

    #[test]
    #[should_panic(expected = "expiry overflow")]
    fn test_renew_expiry_overflow() {
        let s = setup();
        let vasp = Address::generate(&s.env);
        register(&s, &vasp, Some(1));
        s.env.as_contract(&s.contract, || {
            ComplyRailContract::renew_vasp_registration(s.env.clone(), s.admin.clone(), vasp, u64::MAX);
        });
    }

    // Issue #84: threshold comparison at the boundary

    #[test]
    fn test_threshold_boundary_property() {
        let s = setup();
        let (from, to, asset) = (Address::generate(&s.env), Address::generate(&s.env), Address::generate(&s.env));
        register(&s, &from, None);
        register(&s, &to, None);

        // Simple deterministic pseudo-random generator (xorshift), no extra dependencies.
        let mut seed: u64 = 0x9E37_79B9_7F4A_7C15;
        let mut next = || {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            seed
        };

        for _ in 0..50 {
            let threshold = (next() % 1_000_000_000_000) as i128 + 1;
            let amount = (next() % 2_000_000_000_000) as i128;

            s.env.as_contract(&s.contract, || {
                ComplyRailContract::set_threshold(
                    s.env.clone(),
                    s.admin.clone(),
                    asset.clone(),
                    String::from_str(&s.env, "US"),
                    threshold,
                );
            });

            let expected = if amount < threshold { PaymentStatus::Released } else { PaymentStatus::Pending };
            assert_eq!(pay(&s, &from, &to, &asset, amount), expected);
            assert_eq!(pay(&s, &from, &to, &asset, threshold - 1), PaymentStatus::Released);
            assert_eq!(pay(&s, &from, &to, &asset, threshold), PaymentStatus::Pending);
            assert_eq!(pay(&s, &from, &to, &asset, threshold + 1), PaymentStatus::Pending);
        }
    }
}
