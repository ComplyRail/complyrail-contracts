use crate::{ComplyRailContract, ComplyRailContractClient, DataKey, PaymentStatus, VaspStatus};
use soroban_sdk::{testutils::Address as _, Address, BytesN, Env, String, Vec};

const THRESHOLD: i128 = 1000_0000000;

struct Setup<'a> {
    env: Env,
    contract_id: Address,
    client: ComplyRailContractClient<'a>,
    admin: Address,
}

/// Registers the contract, mocks all auths and initializes it with one admin.
fn setup<'a>() -> Setup<'a> {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(ComplyRailContract, ());
    let client = ComplyRailContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);

    client.initialize(&admin);

    Setup {
        env,
        contract_id,
        client,
        admin,
    }
}

fn register_vasp(s: &Setup, vasp: &Address) {
    s.client.register_vasp(
        &s.admin,
        vasp,
        &String::from_str(&s.env, "Test VASP"),
        &String::from_str(&s.env, "US"),
        &BytesN::from_array(&s.env, &[0u8; 32]),
        &None,
    );
}

/// Registers two VASPs, sets a threshold for `asset` in "US" and submits a
/// payment of `amount`. Returns (from_vasp, to_vasp, payment_id).
fn submit_payment(s: &Setup, asset: &Address, amount: i128) -> (Address, Address, BytesN<32>) {
    let from_vasp = Address::generate(&s.env);
    let to_vasp = Address::generate(&s.env);
    let beneficiary = Address::generate(&s.env);

    register_vasp(s, &from_vasp);
    register_vasp(s, &to_vasp);

    s.client.set_threshold(
        &s.admin,
        asset,
        &String::from_str(&s.env, "US"),
        &THRESHOLD,
    );

    let payment_id = s
        .client
        .submit_payment(&from_vasp, &to_vasp, &beneficiary, asset, &amount);

    (from_vasp, to_vasp, payment_id)
}

#[test]
fn test_initialize() {
    let s = setup();

    let admins: Vec<Address> = s.env.as_contract(&s.contract_id, || {
        s.env.storage().instance().get(&DataKey::Admins).unwrap()
    });

    assert_eq!(admins.len(), 1);
    assert!(admins.contains(&s.admin));
}

#[test]
fn test_get_admins() {
    let s = setup();

    let admins = s.client.get_admins();
    assert_eq!(admins.len(), 1);
    assert_eq!(admins.get(0).unwrap(), s.admin);

    let second = Address::generate(&s.env);
    s.client.add_admin(&s.admin, &second);

    let admins = s.client.get_admins();
    assert_eq!(admins.len(), 2);
    assert!(admins.contains(&s.admin));
    assert!(admins.contains(&second));
}

#[test]
fn test_register_vasp() {
    let s = setup();
    let vasp = Address::generate(&s.env);

    register_vasp(&s, &vasp);

    let entry = s.client.get_vasp(&vasp).expect("vasp should be registered");
    assert_eq!(entry.address, vasp);
    assert_eq!(entry.status, VaspStatus::Active);
}

#[test]
fn test_set_threshold() {
    let s = setup();
    let asset = Address::generate(&s.env);
    let jurisdiction = String::from_str(&s.env, "US");

    s.client
        .set_threshold(&s.admin, &asset, &jurisdiction, &THRESHOLD);

    assert_eq!(
        s.client.get_threshold(&asset, &jurisdiction),
        Some(THRESHOLD)
    );
}

#[test]
fn test_below_threshold_auto_release() {
    let s = setup();
    let asset = Address::generate(&s.env);
    let amount = 500_0000000;

    let (_, _, payment_id) = submit_payment(&s, &asset, amount);

    let record = s.client.get_payment(&payment_id).unwrap();
    assert_eq!(record.status, PaymentStatus::Released);
    assert_eq!(record.amount, amount);
}

#[test]
fn test_above_threshold_pending() {
    let s = setup();
    let asset = Address::generate(&s.env);
    let amount = 2000_0000000;

    let (_, _, payment_id) = submit_payment(&s, &asset, amount);

    let record = s.client.get_payment(&payment_id).unwrap();
    assert_eq!(record.status, PaymentStatus::Pending);
    assert_eq!(record.amount, amount);
}

#[test]
fn test_attestation_release() {
    let s = setup();
    let asset = Address::generate(&s.env);

    let (_, to_vasp, payment_id) = submit_payment(&s, &asset, 2000_0000000);
    assert_eq!(
        s.client.get_payment(&payment_id).unwrap().status,
        PaymentStatus::Pending
    );

    let message_hash = BytesN::from_array(&s.env, &[1u8; 32]);
    let ivms_version = String::from_str(&s.env, "1.0");

    s.client
        .submit_attestation(&to_vasp, &payment_id, &message_hash, &ivms_version);

    let payment = s.client.get_payment(&payment_id).unwrap();
    assert_eq!(payment.status, PaymentStatus::Released);
    assert_eq!(payment.attestation_hash, Some(message_hash));
    assert_eq!(payment.ivms_version, Some(ivms_version));
}

#[test]
fn test_admin_release() {
    let s = setup();
    let asset = Address::generate(&s.env);

    let (_, _, payment_id) = submit_payment(&s, &asset, 2000_0000000);

    s.client.release_payment(
        &s.admin,
        &payment_id,
        &String::from_str(&s.env, "Manual override"),
    );

    let payment = s.client.get_payment(&payment_id).unwrap();
    assert_eq!(payment.status, PaymentStatus::Released);
    assert!(payment.resolved_at.is_some());
}

#[test]
fn test_reject_payment() {
    let s = setup();
    let asset = Address::generate(&s.env);

    let (_, _, payment_id) = submit_payment(&s, &asset, 2000_0000000);

    s.client.reject_payment(
        &s.admin,
        &payment_id,
        &String::from_str(&s.env, "Compliance check failed"),
    );

    let payment = s.client.get_payment(&payment_id).unwrap();
    assert_eq!(payment.status, PaymentStatus::Rejected);
    assert!(payment.resolved_at.is_some());
}

#[test]
fn test_payment_count() {
    let s = setup();
    let asset = Address::generate(&s.env);

    assert_eq!(s.client.get_payment_count(), 0);

    submit_payment(&s, &asset, 500_0000000);
    assert_eq!(s.client.get_payment_count(), 1);

    submit_payment(&s, &asset, 500_0000000);
    assert_eq!(s.client.get_payment_count(), 2);
}

#[test]
fn test_submit_payment_does_not_touch_admin_entry() {
    let s = setup();
    let asset = Address::generate(&s.env);

    let read_admins = || -> Vec<Address> {
        s.env.as_contract(&s.contract_id, || {
            s.env.storage().instance().get(&DataKey::Admins).unwrap()
        })
    };

    let before = read_admins();
    submit_payment(&s, &asset, 500_0000000);
    let after = read_admins();

    assert_eq!(before, after);

    // The counter lives in its own persistent entry, not in instance storage.
    s.env.as_contract(&s.contract_id, || {
        assert!(!s.env.storage().instance().has(&DataKey::PaymentCounter));
        assert_eq!(
            s.env
                .storage()
                .persistent()
                .get::<_, u64>(&DataKey::PaymentCounter),
            Some(1)
        );
    });
}

#[test]
fn test_storage_keys_do_not_collide() {
    let env = Env::default();
    let contract_id = env.register(ComplyRailContract, ());
    let addr = Address::generate(&env);

    // A payment id whose bytes match the first payment id (counter = 0) and
    // one made of zeros: neither may alias the counter or any other key.
    let zero_id = BytesN::from_array(&env, &[0u8; 32]);
    let mut counter_like = [0u8; 32];
    counter_like[..7].copy_from_slice(b"counter");
    let counter_like_id = BytesN::from_array(&env, &counter_like);

    let keys = [
        DataKey::Admins,
        DataKey::MinApprovals,
        DataKey::PaymentCounter,
        DataKey::Vasp(addr.clone()),
        DataKey::Payment(zero_id),
        DataKey::Payment(counter_like_id),
        DataKey::Threshold(addr, String::from_str(&env, "US")),
    ];

    env.as_contract(&contract_id, || {
        let instance = env.storage().instance();
        let persistent = env.storage().persistent();

        for (i, key) in keys.iter().enumerate() {
            instance.set(key, &(i as u32));
            persistent.set(key, &(i as u32));
        }

        for (i, key) in keys.iter().enumerate() {
            assert_eq!(instance.get::<_, u32>(key), Some(i as u32));
            assert_eq!(persistent.get::<_, u32>(key), Some(i as u32));
        }
    });

    for (i, a) in keys.iter().enumerate() {
        for b in keys.iter().skip(i + 1) {
            assert_ne!(a, b);
        }
    }
}
