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

impl PaymentStatus {
    /// Whether a payment in this status may be released by `release_payment`.
    ///
    /// Only `Pending` payments are releasable. `Rejected` payments must never
    /// be released (compliance rejection is terminal), and `Released` payments
    /// must not be released twice.
    pub fn is_releasable(&self) -> bool {
        matches!(self, PaymentStatus::Pending)
    }
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

/// Which VASP's jurisdiction governs the threshold lookup for a payment.
///
/// The originating (sending) VASP's jurisdiction is authoritative: the
/// sending VASP is the entity that initiates the transfer and is responsible
/// for applying its own travel-rule threshold before funds leave. This matches
/// the sibling `complyrail-app` contract so the same payment is held or
/// released consistently across both systems.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[contracttype]
pub enum ThresholdJurisdiction {
    Originating,
    Receiving,
}

impl ThresholdJurisdiction {
    /// The jurisdiction rule used by `submit_payment` for threshold lookup.
    pub const RULE: ThresholdJurisdiction = ThresholdJurisdiction::Originating;
}
