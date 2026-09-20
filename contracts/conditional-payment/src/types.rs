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
}

#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AttestationOutcome {
    Pass,
    Fail,
}

#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DataKey {
    Config,
    State,
    FundedAt,
    SubmissionDeadline,
    EvidenceBundleHash,
    ReportHash,
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
}