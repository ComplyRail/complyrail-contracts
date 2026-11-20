//! Property-style fuzz test for payment id generation.
//!
//! Submits a large, pseudo-randomly generated sequence of payments and checks
//! that every generated id is unique and never collides with a registry key
//! (VASP addresses, admin address, or the instance-storage symbol keys).
//!
//! The run is deterministic: a fixed seed drives a small xorshift PRNG, and the
//! iteration count defaults to `DEFAULT_ITERATIONS`. CI runs the default; set
//! `PAYMENT_ID_FUZZ_ITERATIONS` locally to run a longer campaign.

use crate::{ComplyRailContract, ComplyRailContractClient, PaymentStatus};
use soroban_sdk::{
    symbol_short, testutils::Address as _, xdr::ToXdr, Address, Bytes, BytesN, Env, String,
};
use std::collections::HashSet;

const DEFAULT_ITERATIONS: u32 = 256;
const SEED: u64 = 0x5EED_C0FF_EE15_D00D;
const VASP_COUNT: usize = 5;
const THRESHOLD: i128 = 1_000_0000000;

struct XorShift64(u64);

impl XorShift64 {
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }

    fn below(&mut self, bound: usize) -> usize {
        (self.next() % bound as u64) as usize
    }
}

fn iterations() -> u32 {
    std::env::var("PAYMENT_ID_FUZZ_ITERATIONS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(DEFAULT_ITERATIONS)
}

fn bytes_to_vec(bytes: &Bytes) -> std::vec::Vec<u8> {
    bytes.iter().collect()
}

#[test]
fn fuzz_payment_ids_are_unique_and_never_collide_with_registry_keys() {
    let env = Env::default();
    env.mock_all_auths();
    env.cost_estimate().budget().reset_unlimited();

    let contract_id = env.register(ComplyRailContract, ());
    let client = ComplyRailContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.initialize(&admin);

    let jurisdiction = String::from_str(&env, "US");
    let name = String::from_str(&env, "Fuzz VASP");
    let public_key = BytesN::from_array(&env, &[7u8; 32]);

    let vasps: std::vec::Vec<Address> = (0..VASP_COUNT)
        .map(|_| {
            let vasp = Address::generate(&env);
            client.register_vasp(&admin, &vasp, &name, &jurisdiction, &public_key, &None);
            vasp
        })
        .collect();

    let asset = Address::generate(&env);
    let beneficiary = Address::generate(&env);
    client.set_threshold(&admin, &asset, &jurisdiction, &THRESHOLD);

    // Every key the registry stores under, encoded as XDR so it can be compared
    // byte-for-byte with payment ids regardless of the key's Rust type.
    let mut registry_keys: HashSet<std::vec::Vec<u8>> = HashSet::new();
    for addr in vasps.iter().chain([&admin, &asset, &contract_id]) {
        registry_keys.insert(bytes_to_vec(&addr.clone().to_xdr(&env)));
    }
    for sym in [
        symbol_short!("admins"),
        symbol_short!("min_appr"),
        symbol_short!("thresh"),
        symbol_short!("counter"),
    ] {
        registry_keys.insert(bytes_to_vec(&sym.to_xdr(&env)));
    }

    let n = iterations();
    let mut rng = XorShift64(SEED);
    let mut seen: HashSet<[u8; 32]> = HashSet::new();
    let mut sent_per_vasp = [0u32; VASP_COUNT];

    for i in 0..n {
        let from = rng.below(VASP_COUNT);
        let to = (from + 1 + rng.below(VASP_COUNT - 1)) % VASP_COUNT;
        let amount = 1 + (rng.next() % (2 * THRESHOLD as u64)) as i128;

        let id = client.submit_payment(&vasps[from], &vasps[to], &beneficiary, &asset, &amount);
        let raw = id.to_array();

        assert!(seen.insert(raw), "duplicate payment id at iteration {i}");
        assert!(
            !registry_keys.contains(raw.as_slice()),
            "payment id equals a registry key at iteration {i}"
        );
        assert!(
            !registry_keys.contains(&bytes_to_vec(&id.clone().to_xdr(&env))),
            "payment id key encoding equals a registry key at iteration {i}"
        );

        let record = client.get_payment(&id).expect("payment not stored under its id");
        assert_eq!(record.id, id);
        assert_eq!(record.amount, amount);
        let expected = if amount < THRESHOLD {
            PaymentStatus::Released
        } else {
            PaymentStatus::Pending
        };
        assert_eq!(record.status, expected);

        sent_per_vasp[from] += 1;
    }

    assert_eq!(seen.len() as u32, n);
    assert_eq!(client.get_payment_count(), n as u64);

    // Writing payments must not have clobbered any registry entry.
    assert_eq!(client.get_admins().len(), 1);
    assert_eq!(client.get_threshold(&asset, &jurisdiction), Some(THRESHOLD));
    for (idx, vasp) in vasps.iter().enumerate() {
        let entry = client.get_vasp(vasp).expect("vasp entry overwritten");
        assert_eq!(&entry.address, vasp);
        assert_eq!(client.get_payments_by_vasp(vasp, &true).len(), sent_per_vasp[idx]);
    }
}
