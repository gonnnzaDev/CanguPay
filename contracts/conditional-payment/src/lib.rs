//! CanguPay · contrato de pago condicional (P0)
//!
//! Una instancia Soroban representa **una** operación.
//! Alcance P0-03: `CREATED → FUNDED → EVIDENCE_SUBMITTED →
//! ATTESTED_PASS → RELEASED` y `CREATED → CANCELLED`, con auth, eventos y
//! vencimientos `finalize()` para ghost supplier/engine.
//!
//! El fondeo es un pull atómico: `fund()` transfiere `amount` desde el buyer
//! al contrato y actualiza el estado en la misma operación lógica. Si el pull
//! falla, la operación no queda parcialmente fondeada. Los envíos directos al
//! contrato no forman parte del escrow y no tienen vía de recuperación en P0.
//! Después del fondeo, los envíos directos quedan fuera del settlement como
//! limitación conocida. Los vencimientos restantes (objeción, corrección,
//! disputa, fallback) se completan en P0-04.

#![no_std]

mod events;
mod types;

pub use types::{AttestationOutcome, EscrowConfig, EscrowState, FallbackOutcome, FinalizeReason};

use soroban_sdk::{contract, contractimpl, panic_with_error, token::TokenClient, BytesN, Env};

use crate::events::{
    Approved, Attested, Cancelled, EscrowCreated, EvidenceSubmitted, Finalized, Funded,
};
use crate::types::{DataKey, Error};

fn read_config(env: &Env) -> EscrowConfig {
    env.storage()
        .instance()
        .get(&DataKey::Config)
        .unwrap_or_else(|| {
            panic_with_error!(env, Error::NotInitialized);
        })
}

fn read_state(env: &Env) -> EscrowState {
    env.storage()
        .instance()
        .get(&DataKey::State)
        .unwrap_or_else(|| {
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
        if config.attestation_period == 0 {
            panic_with_error!(&env, Error::InvalidAttestationPeriod);
        }
        if config.objection_period == 0 {
            panic_with_error!(&env, Error::InvalidObjectionPeriod);
        }
        if config.correction_period == 0 {
            panic_with_error!(&env, Error::InvalidCorrectionPeriod);
        }
        if config.resolution_period == 0 {
            panic_with_error!(&env, Error::InvalidResolutionPeriod);
        }
        if config.fallback_split_bps > 10_000 {
            panic_with_error!(&env, Error::InvalidFallback);
        }
        match config.fallback_outcome {
            FallbackOutcome::Split => {
                if config.fallback_split_bps == 0 || config.fallback_split_bps > 9_999 {
                    panic_with_error!(&env, Error::InvalidFallback);
                }
            }
            _ => {
                if config.fallback_split_bps != 0 {
                    panic_with_error!(&env, Error::InvalidFallback);
                }
            }
        }
        // 6 pares: buyer/supplier/engine/resolver deben ser distintos.
        // Address valida StrKey nativamente al deserializar XDR, no hace falta check manual.
        if config.buyer == config.supplier
            || config.buyer == config.engine
            || config.buyer == config.resolver
            || config.supplier == config.engine
            || config.supplier == config.resolver
            || config.engine == config.resolver
        {
            panic_with_error!(&env, Error::InvalidState);
        }

        env.storage().instance().set(&DataKey::Config, &config);
        env.storage()
            .instance()
            .set(&DataKey::State, &EscrowState::Created);

        EscrowCreated {
            buyer: config.buyer.clone(),
            supplier: config.supplier.clone(),
            engine: config.engine.clone(),
            resolver: config.resolver.clone(),
            token: config.token.clone(),
            amount: config.amount,
            submission_period: config.submission_period,
            attestation_period: config.attestation_period,
            objection_period: config.objection_period,
            correction_period: config.correction_period,
            resolution_period: config.resolution_period,
            fallback_outcome: config.fallback_outcome,
            fallback_split_bps: config.fallback_split_bps,
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

        env.storage()
            .instance()
            .set(&DataKey::State, &EscrowState::Cancelled);
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

        env.storage()
            .instance()
            .set(&DataKey::State, &EscrowState::Funded);
        env.storage().instance().set(&DataKey::FundedAt, &now);
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

        let now = env.ledger().timestamp();
        let attestation_deadline = now
            .checked_add(config.attestation_period)
            .expect("attestation_period overflow");

        env.storage()
            .instance()
            .set(&DataKey::State, &EscrowState::EvidenceSubmitted);
        env.storage()
            .instance()
            .set(&DataKey::EvidenceBundleHash, &evidence_bundle_hash);
        env.storage()
            .instance()
            .set(&DataKey::AttestationDeadline, &attestation_deadline);

        EvidenceSubmitted {
            attempt: 0,
            evidence_bundle_hash,
        }
        .publish(&env);
    }

    /// El engine atestigua PASS o FAIL con su `report_hash`. No mueve fondos.
    /// Debe ocurrir antes de `attestation_deadline`.
    pub fn attest(env: Env, outcome: AttestationOutcome, report_hash: BytesN<32>) {
        let config = read_config(&env);
        config.engine.require_auth();
        if read_state(&env) != EscrowState::EvidenceSubmitted {
            panic_with_error!(&env, Error::InvalidState);
        }
        let deadline: u64 = env
            .storage()
            .instance()
            .get(&DataKey::AttestationDeadline)
            .unwrap();
        if env.ledger().timestamp() >= deadline {
            panic_with_error!(&env, Error::AttestationDeadlinePassed);
        }

        let state = match outcome {
            AttestationOutcome::Pass => EscrowState::AttestedPass,
            AttestationOutcome::Fail => EscrowState::AttestedFail,
        };
        let now = env.ledger().timestamp();
        env.storage().instance().set(&DataKey::State, &state);
        env.storage()
            .instance()
            .set(&DataKey::ReportHash, &report_hash);
        env.storage().instance().set(&DataKey::AttestedAt, &now);

        Attested {
            outcome,
            report_hash,
        }
        .publish(&env);
    }

    /// El buyer aprueba un PASS: el contrato transfiere exactamente `amount`
    /// al supplier una sola vez, solo antes de `attested_at+objection_period`.
    /// Desde `now >= deadline` solo `finalize()` puede ejecutar el vencimiento
    /// (`NoObjection`).
    pub fn approve(env: Env) {
        let config = read_config(&env);
        config.buyer.require_auth();
        if read_state(&env) != EscrowState::AttestedPass {
            panic_with_error!(&env, Error::AlreadyApproved);
        }
        let attested_at: u64 = env.storage().instance().get(&DataKey::AttestedAt).unwrap();
        let deadline = attested_at
            .checked_add(config.objection_period)
            .expect("objection_period overflow");
        if env.ledger().timestamp() >= deadline {
            panic_with_error!(&env, Error::ObjectionDeadlinePassed);
        }

        let current = env.current_contract_address();
        TokenClient::new(&env, &config.token).transfer(&current, &config.supplier, &config.amount);

        env.storage()
            .instance()
            .set(&DataKey::State, &EscrowState::Released);

        Approved {
            to: config.supplier,
            amount: config.amount,
        }
        .publish(&env);
    }

    /// Ejecuta vencimientos. Permissionless: cualquier cuenta puede invocarla.
    /// - `FUNDED` sin evidencia y `now >= submission_deadline` → `REFUNDED` (`SUBMISSION_TIMEOUT`), reembolsa buyer.
    /// - `EVIDENCE_SUBMITTED` sin atestación y `now >= attestation_deadline` → `REFUNDED` (`ATTESTATION_TIMEOUT`), reembolsa buyer.
    /// - `ATTESTED_PASS` sin objeción y `now >= attested_at+objection_period` → `RELEASED` (`NO_OBJECTION`), paga supplier.
    /// - `ATTESTED_FAIL` sin corrección/disputa y `now >= attested_at+correction_period` → `REFUNDED` (`CORRECTION_TIMEOUT`), reembolsa buyer.
    /// Idempotente: en estados terminales devuelve el estado sin transferir ni emitir. En `CREATED` o antes del deadline panic `NotFinalizableYet`.
    pub fn finalize(env: Env) -> EscrowState {
        let config = read_config(&env);
        let state = read_state(&env);
        let now = env.ledger().timestamp();
        let current = env.current_contract_address();

        match state {
            EscrowState::Cancelled
            | EscrowState::Released
            | EscrowState::Refunded
            | EscrowState::Split => state,
            EscrowState::Disputed => {
                // Disputed no es terminal en finalize() P0-03; el fallback por
                // resolution_period se implementa en P0-04. Hasta entonces,
                // finalize() no liquida Disputed y reporta NotFinalizableYet
                // si se invoca antes de P0-04.
                panic_with_error!(&env, Error::NotFinalizableYet);
            }
            EscrowState::Created => {
                panic_with_error!(&env, Error::NotFinalizableYet);
            }
            EscrowState::Funded => {
                let deadline: u64 = env
                    .storage()
                    .instance()
                    .get(&DataKey::SubmissionDeadline)
                    .unwrap();
                if now < deadline {
                    panic_with_error!(&env, Error::NotFinalizableYet);
                }
                TokenClient::new(&env, &config.token).transfer(
                    &current,
                    &config.buyer,
                    &config.amount,
                );
                env.storage()
                    .instance()
                    .set(&DataKey::State, &EscrowState::Refunded);
                Finalized {
                    reason: FinalizeReason::SubmissionTimeout,
                }
                .publish(&env);
                EscrowState::Refunded
            }
            EscrowState::EvidenceSubmitted => {
                let deadline: u64 = env
                    .storage()
                    .instance()
                    .get(&DataKey::AttestationDeadline)
                    .unwrap();
                if now < deadline {
                    panic_with_error!(&env, Error::NotFinalizableYet);
                }
                TokenClient::new(&env, &config.token).transfer(
                    &current,
                    &config.buyer,
                    &config.amount,
                );
                env.storage()
                    .instance()
                    .set(&DataKey::State, &EscrowState::Refunded);
                Finalized {
                    reason: FinalizeReason::AttestationTimeout,
                }
                .publish(&env);
                EscrowState::Refunded
            }
            EscrowState::AttestedPass => {
                let attested_at: u64 = env.storage().instance().get(&DataKey::AttestedAt).unwrap();
                let deadline = attested_at
                    .checked_add(config.objection_period)
                    .expect("objection_period overflow");
                if now < deadline {
                    panic_with_error!(&env, Error::NotFinalizableYet);
                }
                TokenClient::new(&env, &config.token).transfer(
                    &current,
                    &config.supplier,
                    &config.amount,
                );
                env.storage()
                    .instance()
                    .set(&DataKey::State, &EscrowState::Released);
                Finalized {
                    reason: FinalizeReason::NoObjection,
                }
                .publish(&env);
                EscrowState::Released
            }
            EscrowState::AttestedFail => {
                let attested_at: u64 = env.storage().instance().get(&DataKey::AttestedAt).unwrap();
                let deadline = attested_at
                    .checked_add(config.correction_period)
                    .expect("correction_period overflow");
                if now < deadline {
                    panic_with_error!(&env, Error::NotFinalizableYet);
                }
                TokenClient::new(&env, &config.token).transfer(
                    &current,
                    &config.buyer,
                    &config.amount,
                );
                env.storage()
                    .instance()
                    .set(&DataKey::State, &EscrowState::Refunded);
                Finalized {
                    reason: FinalizeReason::CorrectionTimeout,
                }
                .publish(&env);
                EscrowState::Refunded
            }
        }
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
