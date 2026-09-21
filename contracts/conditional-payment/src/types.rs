//! Tipos de contrato y errores de la operación de pago condicional.

use soroban_sdk::{contracterror, contracttype, Address};

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EscrowConfig {
    pub buyer: Address,
    pub supplier: Address,
    pub engine: Address,
    pub token: Address,
    pub amount: i128,
    pub submission_period: u64,
    pub attestation_period: u64,
    pub objection_period: u64,
    pub correction_period: u64,
    pub resolution_period: u64,
    pub fallback_outcome: FallbackOutcome,
    pub fallback_split_bps: u32,
}

#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EscrowState {
    Created,
    Funded,
    EvidenceSubmitted,
    AttestedPass,
    AttestedFail,
    Released,
    Cancelled,
    Refunded,
    Disputed,
    Split,
}

#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AttestationOutcome {
    Pass,
    Fail,
}

#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FallbackOutcome {
    Release = 1,
    Refund = 2,
    Split = 3,
}

#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FinalizeReason {
    SubmissionTimeout = 1,
    AttestationTimeout = 2,
    NoObjection = 3,
    CorrectionTimeout = 4,
    ResolutionTimeout = 5,
}

#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DataKey {
    Config,
    State,
    FundedAt,
    SubmissionDeadline,
    AttestationDeadline,
    AttestedAt,
    EvidenceBundleHash,
    ReportHash,
    DisputedAt,
    ResolutionDeadline,
    CorrectionAttempts,
}

#[contracterror]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Error {
    AlreadyInitialized = 1,
    NotInitialized = 2,
    InvalidAmount = 3,
    InvalidSubmissionPeriod = 4,
    InvalidState = 5,
    SubmissionDeadlinePassed = 7,
    AlreadyApproved = 8,
    NothingToRecover = 9,
    NotFinalizableYet = 10,
    InvalidAttestationPeriod = 11,
    InvalidObjectionPeriod = 12,
    InvalidCorrectionPeriod = 13,
    InvalidResolutionPeriod = 14,
    InvalidFallback = 15,
    AttestationDeadlinePassed = 16,
}