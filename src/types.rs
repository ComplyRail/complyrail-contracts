use soroban_sdk::{contracttype, Address, String};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[contracttype]
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
    /// Setting the same status again is always disallowed (#4).
    pub fn can_transition_to(&self, next: &VaspStatus) -> bool {
        matches!(
            (self, next),
            (VaspStatus::Active, VaspStatus::Suspended)
                | (VaspStatus::Active, VaspStatus::Revoked)
                | (VaspStatus::Suspended, VaspStatus::Active)
                | (VaspStatus::Suspended, VaspStatus::Revoked)
        )
    }
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
    /// Ledger time of the last change to this registration (#48).
    pub updated_at: u64,
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
    /// Reason an admin gave for releasing or rejecting the payment (#40).
    pub resolution_reason: Option<String>,
}
