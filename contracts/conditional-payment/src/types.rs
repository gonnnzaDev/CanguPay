//! Tipos de contrato y errores de la operación de pago condicional.

use soroban_sdk::{contracterror, contracttype, Address, BytesN};

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EscrowConfig {
    pub buyer: Address,
    pub supplier: Address,
    pub engine: Address,
    pub resolver: Address,
    pub token: Address,
    pub amount: i128,
    pub submission_period: u64,
    pub attestation_period: u64,
    pub objection_period: u64,
    pub correction_period: u64,
    pub resolution_period: u64,
    pub fallback_outcome: FallbackOutcome,
    pub fallback_split_bps: u32,
    pub max_correction_attempts: u32,
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
    DisputeReasonHash,
    DisputeEvidenceHash,
}

/// Instantanea completa para lectores externos (agente de atestacion, UI).
///
/// Todo lo que el motor necesita para decidir va aqui, con nombre: nadie tiene
/// que adivinar la posicion de un campo dentro del `ScVec` ni conocer las
/// `DataKey` privadas. Los plazos que aun no aplican son `None`, y
/// `ledger_timestamp` es el reloj que el propio contrato usa para los plazos.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EscrowSnapshot {
    pub state: EscrowState,
    pub config: EscrowConfig,
    /// Hash del bundle que el supplier subio; es la entrada del motor.
    pub evidence_bundle_hash: Option<BytesN<32>>,
    /// Hash del reporte con el que el engine atestiguo, si ya atesto.
    /// El outcome no se duplica aqui: se deduce de `state`, que es la unica
    /// fuente que `attest()` y `dispute()` mantienen coherente.
    pub report_hash: Option<BytesN<32>>,
    pub funded_at: Option<u64>,
    pub submission_deadline: Option<u64>,
    pub attestation_deadline: Option<u64>,
    pub attested_at: Option<u64>,
    /// `attested_at + objection_period`: hasta ahi el buyer puede objetar.
    pub objection_deadline: Option<u64>,
    /// `attested_at + correction_period`: hasta ahi el supplier corrige.
    pub correction_deadline: Option<u64>,
    pub disputed_at: Option<u64>,
    pub resolution_deadline: Option<u64>,
    pub correction_attempts: u32,
    pub dispute_reason_hash: Option<BytesN<32>>,
    pub dispute_evidence_hash: Option<BytesN<32>>,
    /// Reloj del ledger actual: la misma fuente de verdad de los plazos.
    pub ledger_timestamp: u64,
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
    NotFinalizableYet = 10,
    InvalidAttestationPeriod = 11,
    InvalidObjectionPeriod = 12,
    InvalidCorrectionPeriod = 13,
    InvalidResolutionPeriod = 14,
    InvalidFallback = 15,
    AttestationDeadlinePassed = 16,
    ObjectionDeadlinePassed = 17,
    InvalidMaxCorrectionAttempts = 18,
    ArithmeticOverflow = 19,
}
