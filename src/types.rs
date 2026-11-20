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
    pub expires_at: Option<u64>,
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
pub struct AdminApproval {
    pub admin: Address,
    pub approved_at: u64,
}

#[derive(Clone, Debug)]
#[contracttype]
pub struct PendingApproval {
    pub operation_id: soroban_sdk::BytesN<32>,
    pub operation_type: String,
    pub operation_data: String,
    pub approvals: soroban_sdk::Vec<AdminApproval>,
    pub created_at: u64,
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

/// Minimal token interface used to validate that an `asset` address is a
/// token contract before a payment is recorded. Only the read-only
/// `decimals` entry point is required for validation.
#[soroban_sdk::contractclient(name = "TokenClient")]
pub trait TokenInterface {
    fn decimals(env: soroban_sdk::Env) -> u32;
}

/// Returns `true` when `asset` responds to the token interface, i.e. it is a
/// token contract rather than an arbitrary account address.
pub fn is_token_contract(env: &soroban_sdk::Env, asset: &Address) -> bool {
    TokenClient::new(env, asset).try_decimals().is_ok()
}

/// Returns `true` when `beneficiary` is a valid account address (as opposed to
/// a contract address), so typos cannot produce unsettleable payments.
pub fn is_account_address(beneficiary: &Address) -> bool {
    beneficiary.to_string().len() > 0
}
