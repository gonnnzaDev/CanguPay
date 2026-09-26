//! Agente de atestación CanguPay (P0-07).
//!
//! Capas, de adentro hacia afuera:
//!
//! - [`canonical`] · canonicalizacion estricta y SHA-256, identica al motor Python.
//! - [`ruleset`] · reglas deterministas v1.0.0 con detalle por campo.
//! - [`report`] · reporte reproducible PASS/FAIL con `report_hash` y `detail_hash`.
//! - [`chain`] · lectura del estado real del contrato por RPC (rasgo + impl Soroban).
//! - [`signer`] · clave del engine fuera del repositorio.
//! - [`attest`] · firma y envio de `attest(outcome, report_hash)`.
//! - [`keeper`] · invocacion periodica cuando el estado lo permite.
//! - [`golden`] · paridad con `fixtures/manifest.json`, usada por tests y por `selfcheck`.
//!
//! El motor ([`ruleset`] + [`report`]) es puro y no toca la red: se puede verificar
//! contra `fixtures/manifest.json` sin credenciales. Toda la E/S de cadena pasa por
//! el rasgo [`chain::ChainClient`], para poder probarla con dobles en `tests/`.
pub mod attest;
pub mod canonical;
pub mod chain;
pub mod error;
pub mod golden;
pub mod keeper;
pub mod report;
pub mod ruleset;
pub mod signer;

pub use error::{AgentError, Result};
pub use report::{build_report, Report};

/// Version del ruleset implementada.
pub const RULESET_VERSION: &str = ruleset::RULESET_VERSION;
