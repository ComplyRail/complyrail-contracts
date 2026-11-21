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

    // ---------------------------------------------------------------------
    // Shared fixtures for the security tests below (#77, #78, #79, #80)
    // ---------------------------------------------------------------------

    struct Fixture {
        env: Env,
        admin: Address,
        from_vasp: Address,
        to_vasp: Address,
        beneficiary: Address,
        asset: Address,
    }

    /// Initializes the contract and registers two active VASPs in "US".
    /// No threshold is configured.
    fn setup() -> Fixture {
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
            None,
        );
        ComplyRailContract::register_vasp(
            env.clone(),
            admin.clone(),
            to_vasp.clone(),
            name,
            jurisdiction,
            public_key,
            None,
        );

        Fixture { env, admin, from_vasp, to_vasp, beneficiary, asset }
    }

    /// Configures a threshold and submits a payment above it, so the
    /// payment starts out `Pending`.
    fn submit_pending_payment(f: &Fixture) -> BytesN<32> {
        let jurisdiction = String::from_slice(&f.env, "US");
        ComplyRailContract::set_threshold(
            f.env.clone(),
            f.admin.clone(),
            f.asset.clone(),
            jurisdiction,
            1000_0000000,
        );

        let payment_id = ComplyRailContract::submit_payment(
            f.env.clone(),
            f.from_vasp.clone(),
            f.to_vasp.clone(),
            f.beneficiary.clone(),
            f.asset.clone(),
            2000_0000000,
        );

        assert_eq!(
            ComplyRailContract::get_payment(f.env.clone(), payment_id.clone()).unwrap().status,
            PaymentStatus::Pending
        );
        payment_id
    }

    /// Runs `op` and returns `true` if it panicked (i.e. the contract call
    /// was rejected), so state can still be inspected afterwards.
    fn call_fails<F: FnOnce()>(op: F) -> bool {
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(op)).is_err()
    }

    // ---------------------------------------------------------------------
    // #77: only the beneficiary VASP may attest
    // ---------------------------------------------------------------------

    #[test]
    #[should_panic(expected = "only beneficiary vasp can attest")]
    fn test_attestation_by_originator_vasp_fails() {
        let f = setup();
        let payment_id = submit_pending_payment(&f);

        ComplyRailContract::submit_attestation(
            f.env.clone(),
            f.from_vasp.clone(),
            payment_id,
            BytesN::from_array(&f.env, &[1u8; 32]),
            String::from_slice(&f.env, "1.0"),
        );
    }

    #[test]
    #[should_panic(expected = "only beneficiary vasp can attest")]
    fn test_attestation_by_outsider_fails() {
        let f = setup();
        let payment_id = submit_pending_payment(&f);
        let outsider = Address::random(&f.env);

        ComplyRailContract::submit_attestation(
            f.env.clone(),
            outsider,
            payment_id,
            BytesN::from_array(&f.env, &[1u8; 32]),
            String::from_slice(&f.env, "1.0"),
        );
    }

    #[test]
    fn test_attestation_by_non_beneficiary_leaves_payment_pending() {
        let f = setup();
        let payment_id = submit_pending_payment(&f);
        let outsider = Address::random(&f.env);

        for caller in [f.from_vasp.clone(), outsider] {
            assert!(call_fails(|| {
                ComplyRailContract::submit_attestation(
                    f.env.clone(),
                    caller,
                    payment_id.clone(),
                    BytesN::from_array(&f.env, &[1u8; 32]),
                    String::from_slice(&f.env, "1.0"),
                );
            }));
        }

        let payment = ComplyRailContract::get_payment(f.env.clone(), payment_id).unwrap();
        assert_eq!(payment.status, PaymentStatus::Pending);
        assert_eq!(payment.attestation_hash, None);
        assert_eq!(payment.ivms_version, None);
    }

    // ---------------------------------------------------------------------
    // #78: an attestation cannot be overwritten (see #35)
    // ---------------------------------------------------------------------

    #[test]
    fn test_second_attestation_fails_and_first_hash_is_kept() {
        let f = setup();
        let payment_id = submit_pending_payment(&f);

        let first_hash = BytesN::from_array(&f.env, &[1u8; 32]);
        let first_version = String::from_slice(&f.env, "1.0");
        ComplyRailContract::submit_attestation(
            f.env.clone(),
            f.to_vasp.clone(),
            payment_id.clone(),
            first_hash.clone(),
            first_version.clone(),
        );

        let second_hash = BytesN::from_array(&f.env, &[2u8; 32]);
        let second_version = String::from_slice(&f.env, "2.0");
        assert!(
            call_fails(|| {
                ComplyRailContract::submit_attestation(
                    f.env.clone(),
                    f.to_vasp.clone(),
                    payment_id.clone(),
                    second_hash,
                    second_version,
                );
            }),
            "a second attestation on the same payment must be rejected"
        );

        let payment = ComplyRailContract::get_payment(f.env.clone(), payment_id).unwrap();
        assert_eq!(payment.status, PaymentStatus::Released);
        assert_eq!(payment.attestation_hash, Some(first_hash));
        assert_eq!(payment.ivms_version, Some(first_version));
    }

    // ---------------------------------------------------------------------
    // #79: behaviour when no threshold is configured (see #22)
    // ---------------------------------------------------------------------

    /// Documents the CURRENT fail-open behaviour: with no threshold set for
    /// the asset/jurisdiction pair, `submit_payment` releases the payment
    /// immediately and no attestation is ever recorded.
    ///
    /// When #22 lands (fail closed), flip the assertions to expect
    /// `PaymentStatus::Pending` with `resolved_at == None`, and rename the
    /// test to `test_unconfigured_threshold_fails_closed_requires_attestation`.
    #[test]
    fn test_unconfigured_threshold_fails_open_releases_without_attestation() {
        let f = setup();

        assert_eq!(
            ComplyRailContract::get_threshold(
                f.env.clone(),
                f.asset.clone(),
                String::from_slice(&f.env, "US"),
            ),
            None
        );

        let payment_id = ComplyRailContract::submit_payment(
            f.env.clone(),
            f.from_vasp.clone(),
            f.to_vasp.clone(),
            f.beneficiary.clone(),
            f.asset.clone(),
            2000_0000000,
        );

        let payment = ComplyRailContract::get_payment(f.env.clone(), payment_id).unwrap();
        assert_eq!(payment.status, PaymentStatus::Released);
        assert_eq!(payment.attestation_hash, None);
        assert_eq!(payment.ivms_version, None);
        assert!(payment.resolved_at.is_some());
    }

    // ---------------------------------------------------------------------
    // #80: rejected and released are terminal states (see #27)
    // ---------------------------------------------------------------------

    #[test]
    fn test_release_of_rejected_payment_fails() {
        let f = setup();
        let payment_id = submit_pending_payment(&f);

        ComplyRailContract::reject_payment(
            f.env.clone(),
            f.admin.clone(),
            payment_id.clone(),
            String::from_slice(&f.env, "Compliance check failed"),
        );

        assert!(
            call_fails(|| {
                ComplyRailContract::release_payment(
                    f.env.clone(),
                    f.admin.clone(),
                    payment_id.clone(),
                    String::from_slice(&f.env, "Manual override"),
                );
            }),
            "a rejected payment must not be releasable"
        );

        let payment = ComplyRailContract::get_payment(f.env.clone(), payment_id).unwrap();
        assert_eq!(payment.status, PaymentStatus::Rejected);
    }

    #[test]
    fn test_reject_of_released_payment_fails() {
        let f = setup();
        let payment_id = submit_pending_payment(&f);

        ComplyRailContract::release_payment(
            f.env.clone(),
            f.admin.clone(),
            payment_id.clone(),
            String::from_slice(&f.env, "Manual override"),
        );

        assert!(
            call_fails(|| {
                ComplyRailContract::reject_payment(
                    f.env.clone(),
                    f.admin.clone(),
                    payment_id.clone(),
                    String::from_slice(&f.env, "Compliance check failed"),
                );
            }),
            "a released payment must not be rejectable"
        );

        let payment = ComplyRailContract::get_payment(f.env.clone(), payment_id).unwrap();
        assert_eq!(payment.status, PaymentStatus::Released);
    }
}
