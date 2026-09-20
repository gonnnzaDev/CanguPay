//! Eventos P0-02.
//!
//! SDK 27: cada evento es un struct propio con `#[contractevent]`. Los campos
//! marcados `#[topic]` van a la lista de topics (indexables); el resto, a los
//! datos del evento como un `Map` según su nombre.

use soroban_sdk::{contractevent, Address, BytesN};

use crate::types::AttestationOutcome;

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EscrowCreated {
    #[topic]
    pub buyer: Address,
    #[topic]
    pub supplier: Address,
    #[topic]
    pub engine: Address,
    #[topic]
    pub token: Address,
    pub amount: i128,
    pub submission_period: u64,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Cancelled {
    #[topic]
    pub by: Address,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Funded {
    pub funded_at: u64,
    pub submission_deadline: u64,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EvidenceSubmitted {
    #[topic]
    pub attempt: u32,
    pub evidence_bundle_hash: BytesN<32>,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Attested {
    pub outcome: AttestationOutcome,
    pub report_hash: BytesN<32>,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Approved {
    #[topic]
    pub to: Address,
    pub amount: i128,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Recovered {
    #[topic]
    pub to: Address,
    pub amount: i128,
}