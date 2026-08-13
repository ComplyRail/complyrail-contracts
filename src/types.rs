use soroban_sdk::{contracttype, Address, String};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[contracttype]
pub enum VaspStatus {
    Active,
    Suspended,
    Revoked,
}

#[derive(Clone, Debug)]
#[contracttype]
pub struct VaspEntry {
    pub address: Address,
    pub name: String,
    pub jurisdiction: String,
    pub public_key: soroban_sdk::BytesN<32>,
    pub status: VaspStatus,
    pub added_at: u64,
}

#[derive(Clone, Debug)]
#[contracttype]
pub struct ThresholdConfig {
    pub asset: Address,
    pub jurisdiction: String,
    pub threshold_amount: i128,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[contracttype]
pub enum PaymentStatus {
    Pending,
    Released,
    Held,
    Rejected,
}

#[derive(Clone, Debug)]
#[contracttype]
pub struct PaymentRecord {
    pub id: soroban_sdk::BytesN<32>,
    pub from_vasp: Address,
    pub to_vasp: Address,
    pub beneficiary: Address,
    pub asset: Address,
    pub amount: i128,
    pub status: PaymentStatus,
    pub attestation_hash: Option<soroban_sdk::BytesN<32>>,
    pub ivms_version: Option<String>,
    pub created_at: u64,
    pub resolved_at: Option<u64>,
}
