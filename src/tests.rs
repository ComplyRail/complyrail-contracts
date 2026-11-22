use crate::types::{PaymentStatus, VaspStatus};
use crate::{ComplyRailContract, ComplyRailContractClient};
use soroban_sdk::{
    testutils::{Address as _, Ledger as _},
    Address, BytesN, Env, String,
};

const DAY: u64 = 86_400;
const START: u64 = 1_000;
const THRESHOLD: i128 = 1000_0000000;

fn setup<'a>() -> (Env, ComplyRailContractClient<'a>, Address) {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().set_timestamp(START);

    let contract_id = env.register(ComplyRailContract, ());
    let client = ComplyRailContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.initialize(&admin);

    (env, client, admin)
}

fn register(
    env: &Env,
    client: &ComplyRailContractClient,
    admin: &Address,
    vasp: &Address,
    expiry_days: Option<u64>,
) {
    client.register_vasp(
        admin,
        vasp,
        &String::from_str(env, "Test VASP"),
        &String::from_str(env, "US"),
        &BytesN::from_array(env, &[1u8; 32]),
        &expiry_days,
    );
}

struct PendingPayment {
    from_vasp: Address,
    to_vasp: Address,
    payment_id: BytesN<32>,
}

/// Registers two VASPs, sets a threshold and submits a payment above it.
fn pending_payment(env: &Env, client: &ComplyRailContractClient, admin: &Address) -> PendingPayment {
    let from_vasp = Address::generate(env);
    let to_vasp = Address::generate(env);
    let beneficiary = Address::generate(env);
    let asset = Address::generate(env);

    register(env, client, admin, &from_vasp, None);
    register(env, client, admin, &to_vasp, None);
    client.set_threshold(admin, &asset, &String::from_str(env, "US"), &THRESHOLD);

    let payment_id = client.submit_payment(&from_vasp, &to_vasp, &beneficiary, &asset, &(THRESHOLD * 2));
    assert_eq!(client.get_payment(&payment_id).unwrap().status, PaymentStatus::Pending);

    PendingPayment { from_vasp, to_vasp, payment_id }
}

// ---------------------------------------------------------------------------
// Core flows
// ---------------------------------------------------------------------------

#[test]
fn test_initialize() {
    let (_env, client, admin) = setup();

    let admins = client.get_admins();
    assert_eq!(admins.len(), 1);
    assert_eq!(admins.get(0).unwrap(), admin);
}

#[test]
fn test_register_vasp() {
    let (env, client, admin) = setup();
    let vasp = Address::generate(&env);

    register(&env, &client, &admin, &vasp, None);

    let entry = client.get_vasp(&vasp);
    assert_eq!(entry.address, vasp);
    assert_eq!(entry.status, VaspStatus::Active);
    assert_eq!(entry.expires_at, None);
}

#[test]
fn test_set_threshold() {
    let (env, client, admin) = setup();
    let asset = Address::generate(&env);
    let jurisdiction = String::from_str(&env, "US");

    client.set_threshold(&admin, &asset, &jurisdiction, &THRESHOLD);

    assert_eq!(client.get_threshold(&asset, &jurisdiction), Some(THRESHOLD));
}

#[test]
fn test_below_threshold_auto_release() {
    let (env, client, admin) = setup();
    let from_vasp = Address::generate(&env);
    let to_vasp = Address::generate(&env);
    let beneficiary = Address::generate(&env);
    let asset = Address::generate(&env);

    register(&env, &client, &admin, &from_vasp, None);
    register(&env, &client, &admin, &to_vasp, None);
    client.set_threshold(&admin, &asset, &String::from_str(&env, "US"), &THRESHOLD);

    let amount = THRESHOLD / 2;
    let payment_id = client.submit_payment(&from_vasp, &to_vasp, &beneficiary, &asset, &amount);

    let record = client.get_payment(&payment_id).unwrap();
    assert_eq!(record.status, PaymentStatus::Released);
    assert_eq!(record.amount, amount);
}

#[test]
fn test_above_threshold_pending() {
    let (env, client, admin) = setup();
    let p = pending_payment(&env, &client, &admin);

    let record = client.get_payment(&p.payment_id).unwrap();
    assert_eq!(record.status, PaymentStatus::Pending);
    assert_eq!(record.amount, THRESHOLD * 2);
    assert_eq!(record.resolved_at, None);
}

#[test]
fn test_attestation_release() {
    let (env, client, admin) = setup();
    let p = pending_payment(&env, &client, &admin);

    let message_hash = BytesN::from_array(&env, &[1u8; 32]);
    let ivms_version = String::from_str(&env, "1.0");
    client.submit_attestation(&p.to_vasp, &p.payment_id, &message_hash, &ivms_version);

    let payment = client.get_payment(&p.payment_id).unwrap();
    assert_eq!(payment.status, PaymentStatus::Released);
    assert_eq!(payment.attestation_hash, Some(message_hash));
    assert_eq!(payment.ivms_version, Some(ivms_version));
}

#[test]
fn test_admin_release() {
    let (env, client, admin) = setup();
    let p = pending_payment(&env, &client, &admin);

    client.release_payment(&admin, &p.payment_id, &String::from_str(&env, "Manual override"));

    let payment = client.get_payment(&p.payment_id).unwrap();
    assert_eq!(payment.status, PaymentStatus::Released);
    assert!(payment.resolved_at.is_some());
}

#[test]
fn test_reject_payment() {
    let (env, client, admin) = setup();
    let p = pending_payment(&env, &client, &admin);

    client.reject_payment(&admin, &p.payment_id, &String::from_str(&env, "Compliance check failed"));

    let payment = client.get_payment(&p.payment_id).unwrap();
    assert_eq!(payment.status, PaymentStatus::Rejected);
    assert!(payment.resolved_at.is_some());
}

// ---------------------------------------------------------------------------
// Re-registration of an existing VASP
// ---------------------------------------------------------------------------

#[test]
fn test_reregister_active_vasp_fails() {
    let (env, client, admin) = setup();
    let vasp = Address::generate(&env);

    register(&env, &client, &admin, &vasp, Some(30));
    let before = client.get_vasp(&vasp);

    env.ledger().set_timestamp(START + DAY);
    let result = client.try_register_vasp(
        &admin,
        &vasp,
        &String::from_str(&env, "Renamed VASP"),
        &String::from_str(&env, "GB"),
        &BytesN::from_array(&env, &[7u8; 32]),
        &None,
    );
    assert!(result.is_err());

    let after = client.get_vasp(&vasp);
    assert_eq!(after.name, before.name);
    assert_eq!(after.jurisdiction, before.jurisdiction);
    assert_eq!(after.public_key, before.public_key);
    assert_eq!(after.added_at, before.added_at);
    assert_eq!(after.expires_at, before.expires_at);
}

#[test]
fn test_reregister_revoked_vasp_fails_and_status_survives() {
    let (env, client, admin) = setup();
    let vasp = Address::generate(&env);

    register(&env, &client, &admin, &vasp, Some(30));
    client.update_vasp_status(&admin, &vasp, &VaspStatus::Revoked);
    let before = client.get_vasp(&vasp);

    let result = client.try_register_vasp(
        &admin,
        &vasp,
        &String::from_str(&env, "Test VASP"),
        &String::from_str(&env, "US"),
        &BytesN::from_array(&env, &[0u8; 32]),
        &None,
    );
    assert!(result.is_err());

    let after = client.get_vasp(&vasp);
    assert_eq!(after.status, VaspStatus::Revoked);
    assert_eq!(after.expires_at, before.expires_at);
    assert_eq!(after.added_at, before.added_at);
}

#[test]
fn test_reregister_suspended_vasp_fails_and_status_survives() {
    let (env, client, admin) = setup();
    let vasp = Address::generate(&env);

    register(&env, &client, &admin, &vasp, None);
    client.update_vasp_status(&admin, &vasp, &VaspStatus::Suspended);

    let result = client.try_register_vasp(
        &admin,
        &vasp,
        &String::from_str(&env, "Test VASP"),
        &String::from_str(&env, "US"),
        &BytesN::from_array(&env, &[0u8; 32]),
        &None,
    );
    assert!(result.is_err());
    assert_eq!(client.get_vasp(&vasp).status, VaspStatus::Suspended);
}

// ---------------------------------------------------------------------------
// Renewal authorization and expiry computation
// ---------------------------------------------------------------------------

#[test]
fn test_renew_by_outsider_fails() {
    let (env, client, admin) = setup();
    let vasp = Address::generate(&env);
    let outsider = Address::generate(&env);

    register(&env, &client, &admin, &vasp, Some(30));
    let before = client.get_vasp(&vasp).expires_at;

    assert!(client.try_renew_vasp_registration(&outsider, &vasp, &365).is_err());
    assert_eq!(client.get_vasp(&vasp).expires_at, before);
}

#[test]
fn test_renew_by_other_registered_vasp_fails() {
    let (env, client, admin) = setup();
    let vasp = Address::generate(&env);
    let other_vasp = Address::generate(&env);

    register(&env, &client, &admin, &vasp, Some(30));
    register(&env, &client, &admin, &other_vasp, Some(30));
    let before = client.get_vasp(&vasp).expires_at;

    assert!(client.try_renew_vasp_registration(&other_vasp, &vasp, &365).is_err());
    assert_eq!(client.get_vasp(&vasp).expires_at, before);
}

#[test]
fn test_renew_by_admin_succeeds() {
    let (env, client, admin) = setup();
    let vasp = Address::generate(&env);

    register(&env, &client, &admin, &vasp, Some(30));
    client.renew_vasp_registration(&admin, &vasp, &10);

    assert_eq!(client.get_vasp(&vasp).expires_at, Some(START + 40 * DAY));
}

#[test]
fn test_renew_by_vasp_itself_succeeds() {
    let (env, client, admin) = setup();
    let vasp = Address::generate(&env);

    register(&env, &client, &admin, &vasp, Some(30));
    client.renew_vasp_registration(&vasp, &vasp, &10);

    assert_eq!(client.get_vasp(&vasp).expires_at, Some(START + 40 * DAY));
}

#[test]
fn test_renew_active_extends_from_current_expiry() {
    let (env, client, admin) = setup();
    let vasp = Address::generate(&env);

    register(&env, &client, &admin, &vasp, Some(30));
    env.ledger().set_timestamp(START + 10 * DAY);
    client.renew_vasp_registration(&admin, &vasp, &15);

    assert_eq!(client.get_vasp(&vasp).expires_at, Some(START + 45 * DAY));
}

#[test]
fn test_renew_expired_extends_from_now() {
    let (env, client, admin) = setup();
    let vasp = Address::generate(&env);

    register(&env, &client, &admin, &vasp, Some(1));
    let now = START + 5 * DAY;
    env.ledger().set_timestamp(now);
    client.renew_vasp_registration(&admin, &vasp, &10);

    let expires_at = client.get_vasp(&vasp).expires_at.unwrap();
    assert_eq!(expires_at, now + 10 * DAY);
    assert!(expires_at > now);
}

#[test]
fn test_renew_unlimited_stays_unlimited() {
    let (env, client, admin) = setup();
    let vasp = Address::generate(&env);

    register(&env, &client, &admin, &vasp, None);
    client.renew_vasp_registration(&admin, &vasp, &30);

    assert_eq!(client.get_vasp(&vasp).expires_at, None);
}

// ---------------------------------------------------------------------------
// Release by addresses other than an admin
// ---------------------------------------------------------------------------

fn assert_release_refused(caller_of: impl Fn(&Env, &PendingPayment) -> Address) {
    let (env, client, admin) = setup();
    let p = pending_payment(&env, &client, &admin);
    let caller = caller_of(&env, &p);

    let result = client.try_release_payment(&caller, &p.payment_id, &String::from_str(&env, "not allowed"));
    assert!(result.is_err());

    let payment = client.get_payment(&p.payment_id).unwrap();
    assert_eq!(payment.status, PaymentStatus::Pending);
    assert_eq!(payment.resolved_at, None);
}

#[test]
fn test_release_by_originator_fails() {
    assert_release_refused(|_, p| p.from_vasp.clone());
}

#[test]
fn test_release_by_beneficiary_vasp_fails() {
    assert_release_refused(|_, p| p.to_vasp.clone());
}

#[test]
fn test_release_by_third_party_vasp_fails() {
    let (env, client, admin) = setup();
    let p = pending_payment(&env, &client, &admin);
    let third_party = Address::generate(&env);
    register(&env, &client, &admin, &third_party, None);

    let result = client.try_release_payment(&third_party, &p.payment_id, &String::from_str(&env, "not allowed"));
    assert!(result.is_err());

    let payment = client.get_payment(&p.payment_id).unwrap();
    assert_eq!(payment.status, PaymentStatus::Pending);
    assert_eq!(payment.resolved_at, None);
}

#[test]
fn test_release_by_outsider_fails() {
    assert_release_refused(|env, _| Address::generate(env));
}

#[test]
fn test_version() {
    let (env, _client, _admin) = setup();
    assert_eq!(ComplyRailContract::version(env.clone()), String::from_str(&env, crate::VERSION));
}

#[test]
fn test_self_payment_rejected() {
    let (env, client, admin) = setup();
    let vasp = Address::generate(&env);
    let beneficiary = Address::generate(&env);
    let asset = Address::generate(&env);

    register(&env, &client, &admin, &vasp, None);
    client.set_threshold(&admin, &asset, &String::from_str(&env, "US"), &THRESHOLD);

    let result = client.try_submit_payment(&vasp, &vasp, &beneficiary, &asset, &(THRESHOLD / 2));
    assert!(result.is_err());
}

#[test]
fn test_initialize_twice_fails() {
    let (_env, client, admin) = setup();
    let result = client.try_initialize(&admin);
    assert!(result.is_err());
}

#[test]
fn test_status_transition_matrix() {
    let (env, client, admin) = setup();
    let vasp = Address::generate(&env);
    register(&env, &client, &admin, &vasp, None);

    // Active -> Suspended is allowed, Suspended -> Suspended is a no-op and is refused.
    client.update_vasp_status(&admin, &vasp, &VaspStatus::Suspended);
    assert!(client.try_update_vasp_status(&admin, &vasp, &VaspStatus::Suspended).is_err());

    // Revoked is terminal.
    client.update_vasp_status(&admin, &vasp, &VaspStatus::Revoked);
    assert!(client.try_update_vasp_status(&admin, &vasp, &VaspStatus::Active).is_err());
}

#[test]
fn test_admin_cannot_be_registered_as_vasp() {
    let (_env, client, admin) = setup();
    let name = String::from_str(&_env, "Admin VASP");
    let result = client.try_register_vasp(
        &admin,
        &admin,
        &name,
        &String::from_str(&_env, "US"),
        &BytesN::from_array(&_env, &[0u8; 32]),
        &None,
    );
    assert!(result.is_err());
}

#[test]
fn test_admin_cannot_be_payment_party() {
    let (env, client, admin) = setup();
    let vasp = Address::generate(&env);
    let beneficiary = Address::generate(&env);
    let asset = Address::generate(&env);
    register(&env, &client, &admin, &vasp, None);
    client.set_threshold(&admin, &asset, &String::from_str(&env, "US"), &THRESHOLD);

    let result = client.try_submit_payment(&admin, &vasp, &beneficiary, &asset, &(THRESHOLD / 2));
    assert!(result.is_err());
}

#[test]
fn test_zero_and_negative_amounts_rejected() {
    let (env, client, admin) = setup();
    let from = Address::generate(&env);
    let to = Address::generate(&env);
    let beneficiary = Address::generate(&env);
    let asset = Address::generate(&env);
    register(&env, &client, &admin, &from, None);
    register(&env, &client, &admin, &to, None);

    assert!(client.try_submit_payment(&from, &to, &beneficiary, &asset, &0).is_err());
    assert!(client.try_submit_payment(&from, &to, &beneficiary, &asset, &-1).is_err());
}

#[test]
fn test_amount_one_is_accepted() {
    let (env, client, admin) = setup();
    let from = Address::generate(&env);
    let to = Address::generate(&env);
    let beneficiary = Address::generate(&env);
    let asset = Address::generate(&env);
    register(&env, &client, &admin, &from, None);
    register(&env, &client, &admin, &to, None);

    // No threshold is configured, so the payment is released straight away.
    let payment_id = client.submit_payment(&from, &to, &beneficiary, &asset, &1);
    assert_eq!(client.get_payment(&payment_id).unwrap().amount, 1);
}

#[test]
fn test_all_zero_public_key_rejected() {
    let (env, client, admin) = setup();
    let vasp = Address::generate(&env);
    let result = client.try_register_vasp(
        &admin,
        &vasp,
        &String::from_str(&env, "Zero key VASP"),
        &String::from_str(&env, "US"),
        &BytesN::from_array(&env, &[0u8; 32]),
        &None,
    );
    assert!(result.is_err());
}

#[test]
fn test_rejected_and_released_payments_cannot_be_released() {
    let (env, client, admin) = setup();
    let from = Address::generate(&env);
    let to = Address::generate(&env);
    let beneficiary = Address::generate(&env);
    let asset = Address::generate(&env);
    register(&env, &client, &admin, &from, None);
    register(&env, &client, &admin, &to, None);

    // Below threshold: released on submission, so it cannot be released again.
    client.set_threshold(&admin, &asset, &String::from_str(&env, "US"), &THRESHOLD);
    let released = client.submit_payment(&from, &to, &beneficiary, &asset, &5);
    assert!(client.try_release_payment(&admin, &released, &String::from_str(&env, "again")).is_err());

    // Rejected payments cannot be released.
    let pending = client.submit_payment(&from, &to, &beneficiary, &asset, &(THRESHOLD * 2));
    client.reject_payment(&admin, &pending, &String::from_str(&env, "not allowed"));
    assert!(client.try_release_payment(&admin, &pending, &String::from_str(&env, "override")).is_err());
}

