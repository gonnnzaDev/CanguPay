//! CanguPay · contrato de pago condicional (P0)
//!
//! Una instancia Soroban representa **una** operación.
//! Alcance P0-02: camino `CREATED → FUNDED → EVIDENCE_SUBMITTED →
//! ATTESTED_PASS → RELEASED` y `CREATED → CANCELLED`, con auth y eventos.
//!
//! El fondeo es un pull atómico: `fund()` transfiere `amount` desde el buyer
//! al contrato y actualiza el estado en la misma operación lógica. Si el pull
//! falla, la operación no queda parcialmente fondeada. Los saldos enviados
//! por error antes del fondeo los puede recuperar el buyer con `recover()`;
//! después del fondeo, los envíos directos quedan fuera del settlement como
//! limitación conocida. El resto de vencimientos y estados (finalize,
//! corrección, disputa, fallback) se incorporan en P0-03/P0-04.

#![no_std]

mod events;
mod types;

pub use types::{AttestationOutcome, EscrowConfig, EscrowState};

use soroban_sdk::{
    contract, contractimpl, panic_with_error, token::TokenClient, BytesN, Env,
};

use crate::events::{
    Approved, Attested, Cancelled, EscrowCreated, EvidenceSubmitted, Funded, Recovered,
};
use crate::types::{DataKey, Error};

fn read_config(env: &Env) -> EscrowConfig {
    env.storage().instance().get(&DataKey::Config).unwrap_or_else(|| {
        panic_with_error!(env, Error::NotInitialized);
    })
}

fn read_state(env: &Env) -> EscrowState {
    env.storage().instance().get(&DataKey::State).unwrap_or_else(|| {
        panic_with_error!(env, Error::NotInitialized);
    })
}

#[contract]
pub struct ConditionalPayment;

#[contractimpl]
impl ConditionalPayment {
    /// Crea la operación. Solo el buyer puede crearla, y solo una vez: nadie
    /// puede abrir una operación nombrando a un buyer que no la autorizó.
    pub fn initialize(env: Env, config: EscrowConfig) {
        config.buyer.require_auth();
        if env.storage().instance().has(&DataKey::Config) {
            panic_with_error!(&env, Error::AlreadyInitialized);
        }
        if config.amount <= 0 {
            panic_with_error!(&env, Error::InvalidAmount);
        }
        if config.submission_period == 0 {
            panic_with_error!(&env, Error::InvalidSubmissionPeriod);
        }

        env.storage().instance().set(&DataKey::Config, &config);
        env.storage().instance().set(&DataKey::State, &EscrowState::Created);

        EscrowCreated {
            buyer: config.buyer,
            supplier: config.supplier,
            engine: config.engine,
            token: config.token,
            amount: config.amount,
            submission_period: config.submission_period,
        }
        .publish(&env);
    }

    /// Cancela la operación. Solo el buyer, y únicamente antes de fondear.
    pub fn cancel(env: Env) {
        let config = read_config(&env);
        config.buyer.require_auth();
        if read_state(&env) != EscrowState::Created {
            panic_with_error!(&env, Error::InvalidState);
        }

        env.storage().instance().set(&DataKey::State, &EscrowState::Cancelled);
        Cancelled { by: config.buyer }.publish(&env);
    }

    /// Fondea la operación con un pull atómico: transfiere `amount` desde el
    /// buyer al contrato y pasa a `FUNDED` en la misma operación lógica. Si
    /// el pull falla (p. ej. saldo insuficiente), la llamada revierte
    /// completa y la operación no queda parcialmente fondeada.
    pub fn fund(env: Env) {
        let config = read_config(&env);
        config.buyer.require_auth();
        if read_state(&env) != EscrowState::Created {
            panic_with_error!(&env, Error::InvalidState);
        }

        let current = env.current_contract_address();
        TokenClient::new(&env, &config.token).transfer(&config.buyer, &current, &config.amount);

        let now = env.ledger().timestamp();
        let submission_deadline = now
            .checked_add(config.submission_period)
            .expect("submission_period overflow");

        env.storage().instance().set(&DataKey::State, &EscrowState::Funded);
        env.storage()
            .instance()
            .set(&DataKey::FundedAt, &now);
        env.storage()
            .instance()
            .set(&DataKey::SubmissionDeadline, &submission_deadline);

        Funded {
            funded_at: now,
            submission_deadline,
        }
        .publish(&env);
    }

    /// El supplier presenta el hash del bundle de evidencia antes del
    /// `submission_deadline` calculado en el fondeo.
    pub fn submit_evidence(env: Env, evidence_bundle_hash: BytesN<32>) {
        let config = read_config(&env);
        config.supplier.require_auth();
        if read_state(&env) != EscrowState::Funded {
            panic_with_error!(&env, Error::InvalidState);
        }
        let deadline: u64 = env
            .storage()
            .instance()
            .get(&DataKey::SubmissionDeadline)
            .unwrap();
        if env.ledger().timestamp() >= deadline {
            panic_with_error!(&env, Error::SubmissionDeadlinePassed);
        }

        env.storage()
            .instance()
            .set(&DataKey::State, &EscrowState::EvidenceSubmitted);
        env.storage()
            .instance()
            .set(&DataKey::EvidenceBundleHash, &evidence_bundle_hash);

        EvidenceSubmitted {
            attempt: 0,
            evidence_bundle_hash,
        }
        .publish(&env);
    }

    /// El engine atestigua PASS o FAIL con su `report_hash`. No mueve fondos.
    pub fn attest(env: Env, outcome: AttestationOutcome, report_hash: BytesN<32>) {
        let config = read_config(&env);
        config.engine.require_auth();
        if read_state(&env) != EscrowState::EvidenceSubmitted {
            panic_with_error!(&env, Error::InvalidState);
        }

        let state = match outcome {
            AttestationOutcome::Pass => EscrowState::AttestedPass,
            AttestationOutcome::Fail => EscrowState::AttestedFail,
        };
        env.storage().instance().set(&DataKey::State, &state);
        env.storage().instance().set(&DataKey::ReportHash, &report_hash);

        Attested {
            outcome,
            report_hash,
        }
        .publish(&env);
    }

    /// El buyer aprueba un PASS: el contrato transfiere exactamente `amount`
    /// al supplier una sola vez.
    pub fn approve(env: Env) {
        let config = read_config(&env);
        config.buyer.require_auth();
        if read_state(&env) != EscrowState::AttestedPass {
            panic_with_error!(&env, Error::AlreadyApproved);
        }

        let current = env.current_contract_address();
        TokenClient::new(&env, &config.token).transfer(&current, &config.supplier, &config.amount);

        env.storage().instance().set(&DataKey::State, &EscrowState::Released);

        Approved {
            to: config.supplier,
            amount: config.amount,
        }
        .publish(&env);
    }

    /// El buyer recupera saldos enviados por error al contrato mientras la
    /// operación sigue en `CREATED` (antes del fondeo). Después del fondeo,
    /// los envíos directos quedan fuera del settlement (limitación conocida).
    pub fn recover(env: Env) {
        let config = read_config(&env);
        config.buyer.require_auth();
        if read_state(&env) != EscrowState::Created {
            panic_with_error!(&env, Error::InvalidState);
        }

        let current = env.current_contract_address();
        let stuck = TokenClient::new(&env, &config.token).balance(&current);
        if stuck == 0 {
            panic_with_error!(&env, Error::NothingToRecover);
        }
        TokenClient::new(&env, &config.token).transfer(&current, &config.buyer, &stuck);

        Recovered {
            to: config.buyer,
            amount: stuck,
        }
        .publish(&env);
    }

    /// Lecturas para la UI: estado y configuración vigentes.
    pub fn state(env: Env) -> EscrowState {
        read_state(&env)
    }

    pub fn config(env: Env) -> EscrowConfig {
        read_config(&env)
    }
}

#[cfg(test)]
mod tests;