use soroban_sdk::{contracttype, Address, BytesN, String};

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum VaspStatus {
    Active,
    Suspended,
    Revoked,
}

impl VaspStatus {
    /// Returns true if transitioning from `self` to `next` is allowed.
    ///
    /// Allowed transitions:
    /// - Active    -> Suspended, Revoked
    /// - Suspended -> Active, Revoked
    /// - Revoked   -> (terminal, no transitions)
    ///
    /// Setting the same status again is always disallowed.
    pub fn can_transition_to(&self, next: &VaspStatus) -> bool {
        match (self, next) {
            (VaspStatus::Active, VaspStatus::Suspended) => true,
            (VaspStatus::Active, VaspStatus::Revoked) => true,
            (VaspStatus::Suspended, VaspStatus::Active) => true,
            (VaspStatus::Suspended, VaspStatus::Revoked) => true,
            _ => false,
        }
    }
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
