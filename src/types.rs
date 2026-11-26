use soroban_sdk::{contracttype, Address, String};

/// Identifies which sensitive action a proposal covers.
///
/// Only operations that change global governance state require multi-admin approval:
/// - `FreezeRegistry` / `UnfreezeRegistry` — halts or resumes new payment submissions.
/// - `SetApprovalThreshold` — changes how many admins must approve future proposals.
#[derive(Clone, Debug, PartialEq, Eq)]
#[contracttype]
pub enum ProposalKind {
    FreezeRegistry,
    UnfreezeRegistry,
    SetApprovalThreshold(i32),
}

/// An in-flight multi-admin proposal.
#[derive(Clone, Debug)]
#[contracttype]
pub struct Proposal {
    /// Discriminates what will execute when enough admins have approved.
    pub kind: ProposalKind,
    /// Ordered list of admins that have signalled approval (including the proposer).
    pub approvals: soroban_sdk::Vec<Address>,
    /// Whether the proposal has already been executed (guards against re-execution).
    pub executed: bool,
}

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
    /// Reason an admin gave for releasing or rejecting the payment (#40).
    pub resolution_reason: Option<String>,
}
