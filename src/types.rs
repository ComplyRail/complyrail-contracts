use soroban_sdk::{contracttype, Address, BytesN, String};

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum VaspStatus {
    Active,
    Suspended,
    Revoked,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VaspEntry {
    pub address: Address,
    pub name: String,
    pub jurisdiction: String,
    pub public_key: BytesN<32>,
    pub status: VaspStatus,
    pub added_at: u64,
    pub expires_at: Option<u64>,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PaymentStatus {
    Pending,
    Released,
    Refunded,
    Rejected,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PaymentRecord {
    pub id: BytesN<32>,
    pub from_vasp: Address,
    pub to_vasp: Address,
    pub beneficiary: Address,
    pub asset: Address,
    pub amount: i128,
    pub status: PaymentStatus,
    pub attestation_hash: Option<BytesN<32>>,
    pub ivms_version: Option<String>,
    pub created_at: u64,
    pub resolved_at: Option<u64>,
}
