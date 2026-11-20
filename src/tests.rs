#[cfg(test)]
mod tests {
    use crate::*;
    use soroban_sdk::{Address, Env, BytesN, String};
    use soroban_sdk::token::{Client as TokenClient, StellarAssetClient};

    fn setup_token<'a>(env: &Env, admin: &Address) -> (Address, TokenClient<'a>) {
        let token_id = env.register_stellar_asset_contract(admin.clone());
        let token = TokenClient::new(env, &token_id);
        (token_id, token)
    }

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
        env.mock_all_auths();
        let admin = Address::random(&env);
        let from_vasp = Address::random(&env);
        let to_vasp = Address::random(&env);
        let beneficiary = Address::random(&env);

        ComplyRailContract::initialize(env.clone(), admin.clone());

        let (asset, token) = setup_token(&env, &admin);
        let asset_admin = StellarAssetClient::new(&env, &asset);
        asset_admin.mint(&from_vasp, &10_000_0000000);

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

        // Tokens moved from from_vasp into the contract escrow
        assert_eq!(token.balance(&from_vasp), 10_000_0000000 - payment_amount);
        assert_eq!(token.balance(&env.current_contract_address()), payment_amount);
    }

    #[test]
    fn test_above_threshold_pending() {
        let env = Env::default();
        env.mock_all_auths();
        let admin = Address::random(&env);
        let from_vasp = Address::random(&env);
        let to_vasp = Address::random(&env);
        let beneficiary = Address::random(&env);

        ComplyRailContract::initialize(env.clone(), admin.clone());

        let (asset, token) = setup_token(&env, &admin);
        let asset_admin = StellarAssetClient::new(&env, &asset);
        asset_admin.mint(&from_vasp, &10_000_0000000);

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

        // Tokens are held in escrow by the contract
        assert_eq!(token.balance(&from_vasp), 10_000_0000000 - payment_amount);
        assert_eq!(token.balance(&env.current_contract_address()), payment_amount);
    }

    #[test]
    fn test_threshold_uses_originating_vasp_jurisdiction() {
        // Issue #23: the threshold lookup is governed by the ORIGINATING VASP's
        // jurisdiction (matching complyrail-app), not the receiving VASP's.
        // Here the two VASPs are in different jurisdictions with different
        // thresholds; the payment must be evaluated against the sender's.
        let env = Env::default();
        env.mock_all_auths();
        let admin = Address::random(&env);
        let from_vasp = Address::random(&env);
        let to_vasp = Address::random(&env);
        let beneficiary = Address::random(&env);

        ComplyRailContract::initialize(env.clone(), admin.clone());

        let (asset, token) = setup_token(&env, &admin);
        let asset_admin = StellarAssetClient::new(&env, &asset);
        asset_admin.mint(&from_vasp, &10_000_0000000);

        let name = String::from_slice(&env, "Test VASP");
        let from_jurisdiction = String::from_slice(&env, "US");
        let to_jurisdiction = String::from_slice(&env, "EU");
        let public_key = BytesN::from_array(&env, &[0u8; 32]);

        ComplyRailContract::register_vasp(
            env.clone(),
            admin.clone(),
            from_vasp.clone(),
            name.clone(),
            from_jurisdiction.clone(),
            public_key.clone(),
        );

        ComplyRailContract::register_vasp(
            env.clone(),
            admin.clone(),
            to_vasp.clone(),
            name,
            to_jurisdiction.clone(),
            public_key,
        );

        // Originating jurisdiction has a high threshold; receiving has a low one.
        let from_threshold = 1000_0000000;
        let to_threshold = 100_0000000;
        ComplyRailContract::set_threshold(env.clone(), admin.clone(), asset.clone(), from_jurisdiction.clone(), from_threshold);
        ComplyRailContract::set_threshold(env.clone(), admin.clone(), asset.clone(), to_jurisdiction.clone(), to_threshold);

        // Amount is above the receiving threshold but below the originating one.
        let payment_amount = 500_0000000;
        let payment_id = ComplyRailContract::submit_payment(
            env.clone(),
            from_vasp.clone(),
            to_vasp.clone(),
            beneficiary.clone(),
            asset.clone(),
            payment_amount,
        );

        let record = ComplyRailContract::get_payment(env.clone(), payment_id.clone()).unwrap();
        // Governed by originating jurisdiction -> below threshold -> released.
        assert_eq!(record.status, PaymentStatus::Released);
        assert_eq!(token.balance(&env.current_contract_address()), 0);
    }

    #[test]
    fn test_attestation_release() {
        let env = Env::default();
        env.mock_all_auths();
        let admin = Address::random(&env);
        let from_vasp = Address::random(&env);
        let to_vasp = Address::random(&env);
        let beneficiary = Address::random(&env);

        ComplyRailContract::initialize(env.clone(), admin.clone());

        let (asset, token) = setup_token(&env, &admin);
        let asset_admin = StellarAssetClient::new(&env, &asset);
        asset_admin.mint(&from_vasp, &10_000_0000000);

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

        let t

/* … truncated 5035 chars — edit only what you need near the top … */
