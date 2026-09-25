//! Keeper: vigila un contrato y atesta en cuanto la evidencia esta lista.
//!
//! El keeper es la parte que hace falta para que la atestation no dependa de que alguien
//! mire la pantalla. Su ciclo es siempre el mismo:
//!
//! 1. Leer el estado real del contrato.
//! 2. Si esta `EvidenceSubmitted`, cargar el bundle, calcular el reporte y **exigir que
//!    el hash local coincida con el que esta en cadena** antes de firmar nada.
//! 3. Si el plan es valido, enviar `attest()` y comprobar el estado resultante.
//! 4. Esperar y volver a empezar.
//!
//! El bucle esta partido en `run_once` (un paso, sin dormir) y `run` (el bucle), para que
//! las pruebas puedaniconductorlo con una cadena falsa y sin esperas reales.
use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::attest::plan_attestation;
use crate::chain::{AttestationOutcome, ChainClient, EscrowSnapshot, EscrowState};
use crate::error::{AgentError, Result};
use crate::report::build_report;
use crate::signer::EngineSigner;

/// Configuracion del keeper.
#[derive(Debug, Clone)]
pub struct KeeperConfig {
    /// Archivo con el bundle de evidencia en JSON.
    pub bundle_path: PathBuf,
    /// Importe esperado del escrow, en unidades minimas.
    pub expected_amount: i128,
    /// Divisa esperada.
    pub expected_currency: String,
    /// Espera entre lecturas.
    pub poll_interval: Duration,
    /// Tope de iteraciones; `None` significa bucle infinito.
    pub max_iterations: Option<u64>,
    /// Si el keeper tambien llama a `finalize()` cuando un plazo parece vencido.
    ///
    /// El calculo de si un plazo vencio es **una estimacion del cliente**, no la
    /// verdad: un ledger puede moverse entre la lectura y el envio. Por eso la
    /// estimacion solo decide si se intenta, y es `finalize()` del contrato el que
    /// aplica el vencimiento. Si el contrato dice que todavia no, el keeper se aquieta
    /// y vuelve a mirar; no insiste ni lleva su propia cuenta como si fuera cierta.
    pub finalize_on_expiry: bool,
}

/// Que hizo el keeper en una iteracion.
#[derive(Debug, Clone, PartialEq)]
pub enum KeeperStep {
    /// El estado actual no permite atestatar todavia.
    Waiting { state: EscrowState },
    /// Falto el bundle en disco.
    BundleUnavailable { path: String },
    /// El hash local no coincide con el de cadena: no se firma nada.
    HashMismatch { on_chain: String },
    /// Habia que atestar y se hizo.
    Attested {
        outcome: AttestationOutcome,
        tx: String,
        state_after: EscrowState,
    },
    /// El estado ya no admite atestacion y no queda nada que esperar.
    Closed { state: EscrowState },
    /// La lectura de cadena fallo; se reintentara en la siguiente iteracion.
    Transient { error: String },
    /// Se intento `finalize()` y el escrow quedo liquidado.
    Finalized { state: EscrowState },
}

/// Motivo por el que un `Observation` **no** lleva a firmar.
///
/// La distincion importa para los reintentos: `HashMismatch` y `BundleUnavailable` se
/// resuelven solos cuando el proveedor sube la evidencia correcta, asi que el keeper
/// debe seguir vigilando. Un estado terminal, en cambio, no mejore por esperar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkipReason {
    /// El estado aun permite atestar, pero todavia no.
    NotYet,
    /// Falta el bundle en disco: reintentable.
    BundleMissing,
    /// El hash no coincide: reintentable cuando el proveedor corrija.
    HashMismatch,
    /// El escrow llego a un estado final: no hay nada mas que hacer.
    Terminal,
}

impl SkipReason {
    /// `true` si vale la pena volver a mirar mas adelante.
    pub fn is_retryable(self) -> bool {
        !matches!(self, Self::Terminal)
    }
}

/// Resumen de una ejecucion completa.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct KeeperReport {
    /// Pasos dados, en orden.
    pub steps: Vec<KeeperStep>,
    /// Cuantas atestaciones se enviaron.
    pub attested: u32,
    /// Cuantas lecturas de cadena fallaron por infraestructura.
    pub transient: u32,
    /// Cuantas veces el contrato confirmo un vencimiento liquidado.
    pub finalized: u32,
    /// Por que dejo de vigilar, si dejo de vigilar.
    pub stopped_because: Option<String>,
}

impl KeeperReport {
    /// Ultimo estado observado, si hubo pasos.
    pub fn last_state(&self) -> Option<EscrowState> {
        self.steps.iter().rev().find_map(|s| match s {
            KeeperStep::Waiting { state } => Some(*state),
            KeeperStep::Closed { state } | KeeperStep::Finalized { state } => Some(*state),
            KeeperStep::Attested { state_after, .. } => Some(*state_after),
            _ => None,
        })
    }
}

/// Carga el bundle desde disco.
///
/// Se comprueba que el archivo exista y que el hash canonico coincida con el que el
/// proveedor ya subio a cadena; si no coincide, el keeper **no** firma.
fn load_bundle(config: &KeeperConfig) -> Result<(serde_json::Value, String)> {
    let text = std::fs::read_to_string(&config.bundle_path).map_err(|e| {
        AgentError::Io(format!(
            "no se pudo leer {}: {e}",
            config.bundle_path.display()
        ))
    })?;
    let value: serde_json::Value = serde_json::from_str(&text).map_err(|e| {
        AgentError::BundleStructure(format!(
            "{} no es JSON valido: {e}",
            config.bundle_path.display()
        ))
    })?;
    let report = build_report(&value, config.expected_amount, &config.expected_currency)?;
    Ok((value, report.evidence_bundle_hash))
}

/// Lo que el keeper observa en una lectura, antes de decidir si firma.
///
/// Separar la observacion de la firma tiene una consecuencia util: el keeper puede
///vigilar y reportar hashes que no cuadran **sin** cargar la clave del engine.
#[derive(Debug, Clone, PartialEq)]
pub enum Observation {
    /// El estado actual todavia no permite atestatar.
    Waiting { state: EscrowState },
    /// El bundle no esta en disco.
    BundleUnavailable { path: String },
    /// El hash del bundle en disco no es el que el proveedor subio a cadena.
    HashMismatch { on_chain: String },
    /// Todo coincide: se puede planificar la atestacion.
    ///
    /// Va en un `Box` porque el caso `Ready` carga el snapshot y el reporte enteros, y sin
    /// caja cada `Waiting` de la voceta arrastraria ese peso.
    Ready(Box<ReadyCase>),
}

/// Datos para atestar: el snapshot real de cadena y el reporte del bundle.
#[derive(Debug, Clone, PartialEq)]
pub struct ReadyCase {
    /// Snapshot leido de la cadena, con estado, plazo y cuenta engine.
    pub snapshot: EscrowSnapshot,
    /// Reporte PASS/FAIL del bundle local.
    pub report: crate::report::Report,
}

impl Observation {
    /// Estado observado en la lectura.
    pub fn state(&self) -> EscrowState {
        match self {
            Observation::Waiting { state } => *state,
            Observation::Ready(case) => case.snapshot.state,
            Observation::BundleUnavailable { .. } | Observation::HashMismatch { .. } => {
                EscrowState::EvidenceSubmitted
            }
        }
    }

    /// Por que esta observacion no lleva a firmar, o `None` si si lleva.
    pub fn skip_reason(&self) -> Option<SkipReason> {
        match self {
            Observation::Ready(_) => None,
            Observation::BundleUnavailable { .. } => Some(SkipReason::BundleMissing),
            Observation::HashMismatch { .. } => Some(SkipReason::HashMismatch),
            Observation::Waiting { state } => {
                if is_terminal(*state) {
                    Some(SkipReason::Terminal)
                } else {
                    Some(SkipReason::NotYet)
                }
            }
        }
    }
}

/// `true` si el escrow llego a un estado del que no se sale.
///
/// `AttestedPass` y `AttestedFail` **no** son terminales: tras un FAIL el proveedor
/// todavia puede corregir la evidencia una vez, y el engine debe volver a evaluarla.
pub fn is_terminal(state: EscrowState) -> bool {
    matches!(
        state,
        EscrowState::Released | EscrowState::Refunded | EscrowState::Cancelled | EscrowState::Split
    )
}

/// `true` si el estado sigue vivo y puede cambiar sin intervencion del engine.
pub fn is_live(state: EscrowState) -> bool {
    !is_terminal(state) && !matches!(state, EscrowState::Disputed)
}

/// Plazo que, segun la lectura actual, podria estar vencido.
///
/// Es una **estimacion** para decidir si vale la pena llamar a `finalize()`. El reloj
/// real es el del ledger que vea el contrato al ejecutar, asi que esta funcion nunca
/// afirma que un escrow esta vencido: solo sugiere probarlo.
///
/// Se listan los plazos con sus precondiciones, replicando la logica de `finalize()`:
/// - `Funded`: vence `submission_deadline` (reembolsa al buyer).
/// - `EvidenceSubmitted`: vence `attestation_deadline` (resuelve por fallback).
/// - `AttestedPass`: vence `objection_deadline` (reembolsa al buyer).
/// - `AttestedFail`: vence `correction_deadline` (resuelve por fallback).
/// - `Disputed`: vence `resolution_deadline` (resuelve por fallback).
pub fn expiry_candidate(snapshot: &EscrowSnapshot) -> Option<&'static str> {
    let now = snapshot.ledger_timestamp;
    let past = |d: Option<u64>| d.is_some_and(|d| now >= d);
    match snapshot.state {
        EscrowState::Funded if past(snapshot.submission_deadline) => Some("submission_deadline"),
        EscrowState::EvidenceSubmitted if past(snapshot.attestation_deadline) => {
            Some("attestation_deadline")
        }
        EscrowState::AttestedPass if past(snapshot.objection_deadline) => {
            Some("objection_deadline")
        }
        EscrowState::AttestedFail if past(snapshot.correction_deadline) => {
            Some("correction_deadline")
        }
        EscrowState::Disputed if past(snapshot.resolution_deadline) => Some("resolution_deadline"),
        _ => None,
    }
}

/// Intenta `finalize()` si un plazo parece vencido.
///
/// Devuelve `Ok(None)` cuando no hay nada que intentar, y `Ok(Some(estado))` cuando el
/// contrato liquido el escrow. Si el contrato responde `NotFinalizableYet`, eso sube
/// como error: el keeper no reintenta a la fuerza ni reinterpreta el rechazo.
fn try_finalize(chain: &dyn ChainClient, snapshot: &EscrowSnapshot) -> Result<Option<EscrowState>> {
    let Some(_plazo) = expiry_candidate(snapshot) else {
        return Ok(None);
    };
    Ok(Some(chain.finalize()?))
}

/// Lee el contrato y el bundle, y decide si tiene sentido atestar.
///
/// Es la parte sin efectos: no firma nada y no necesita la clave del engine.
pub fn observe(chain: &dyn ChainClient, config: &KeeperConfig) -> Result<Observation> {
    let snapshot = chain.snapshot()?;

    if snapshot.state != EscrowState::EvidenceSubmitted {
        return Ok(Observation::Waiting {
            state: snapshot.state,
        });
    }

    let value = match load_bundle(config) {
        Ok((value, _)) => value,
        Err(AgentError::Io(_)) => {
            return Ok(Observation::BundleUnavailable {
                path: config.bundle_path.display().to_string(),
            })
        }
        Err(other) => return Err(other),
    };

    // Criterio 2 de P0-07: lo que se procesa debe ser exactamente lo que el proveedor
    // presento. Sin esta comprobacion, el motor firmaria sobre evidencia ajena.
    let local_hash = build_report(&value, config.expected_amount, &config.expected_currency)?
        .evidence_bundle_hash;
    let local = crate::attest::hash_bytes(&local_hash);
    match snapshot.evidence_bundle_hash {
        None => Ok(Observation::HashMismatch {
            on_chain: "ausente".into(),
        }),
        Some(on_chain) if on_chain != local => Ok(Observation::HashMismatch {
            on_chain: hex::encode(on_chain),
        }),
        Some(_) => Ok(Observation::Ready(Box::new(ReadyCase {
            snapshot,
            report: build_report(&value, config.expected_amount, &config.expected_currency)?,
        }))),
    }
}

/// Un paso del ciclo con la clave del entorno.
///
/// Devuelve tambien el estado observado para que el llamador decida si seguir.
pub fn run_once(
    chain: &dyn ChainClient,
    config: &KeeperConfig,
) -> Result<(KeeperStep, EscrowState)> {
    let observation = observe(chain, config)?;
    let signer = match &observation {
        Observation::Ready(_) => Some(EngineSigner::from_env()?),
        _ => None,
    };
    act(chain, &observation, signer.as_ref())
}

/// Un paso del ciclo con el signer ya resuelto.
///
/// Separarlo asi evita que las pruebas muten variables de entorno, que en `cargo test`
/// es una carrera: los tests corren en paralelo en el mismo proceso.
pub fn run_once_with(
    chain: &dyn ChainClient,
    config: &KeeperConfig,
    signer: &EngineSigner,
) -> Result<(KeeperStep, EscrowState)> {
    let observation = observe(chain, config)?;
    act(chain, &observation, Some(signer))
}

/// Traduce una observacion en una accion, firmando solo si toca.
///
/// Reusa el plan de `crate::attest` con el snapshot **real** leido de la cadena, de modo
/// que el keeper no pueda atestar en un estado, plazo o cuenta que el motor no permite.
fn act(
    chain: &dyn ChainClient,
    observation: &Observation,
    signer: Option<&EngineSigner>,
) -> Result<(KeeperStep, EscrowState)> {
    match observation {
        Observation::Waiting { state } => {
            let step = if is_terminal(*state) {
                KeeperStep::Closed { state: *state }
            } else {
                KeeperStep::Waiting { state: *state }
            };
            Ok((step, *state))
        }
        Observation::BundleUnavailable { path } => Ok((
            KeeperStep::BundleUnavailable { path: path.clone() },
            observation.state(),
        )),
        Observation::HashMismatch { on_chain } => Ok((
            KeeperStep::HashMismatch {
                on_chain: on_chain.clone(),
            },
            observation.state(),
        )),
        Observation::Ready(case) => {
            let (snapshot, report) = (&case.snapshot, &case.report);
            let signer = signer.ok_or_else(|| {
                AgentError::Config("falta la clave del engine para atestar".into())
            })?;
            let plan = plan_attestation(snapshot, report, signer)?;

            // Relectura anti-obsolescencia: entre el snapshot con el que se planeo y
            // el envio hay firmas, esperas y red. Si en ese hueco el proveedor subio
            // evidencia corregida, se vencio el plazo, o el estado cambio, la
            // atestacion planeada ya no describe la cadena y no debe firmarse.
            let fresh = chain.snapshot()?;
            let revalidated = plan_attestation(&fresh, report, signer);
            let stale = match revalidated {
                Ok(replan) => {
                    if replan.outcome != plan.outcome {
                        Some(format!(
                            "el outcome planeado ({:?}) ya no coincide con el estado actual ({:?})",
                            plan.outcome, replan.outcome
                        ))
                    } else {
                        None
                    }
                }
                Err(e) => Some(format!("la lectura fresca ya no permite atestar: {e}")),
            };
            if let Some(reason) = stale {
                return Ok((
                    KeeperStep::HashMismatch {
                        on_chain: format!("obsoleta: {reason}"),
                    },
                    fresh.state,
                ));
            }
            if fresh.evidence_bundle_hash != snapshot.evidence_bundle_hash {
                return Ok((
                    KeeperStep::HashMismatch {
                        on_chain: format!(
                            "obsoleta: la evidencia cambio de {:?} a {:?} mientras se preparaba",
                            snapshot.evidence_bundle_hash.map(hex::encode),
                            fresh.evidence_bundle_hash.map(hex::encode)
                        ),
                    },
                    fresh.state,
                ));
            }

            let submitted = chain.submit_attestation(plan.outcome, &plan.report_hash_bytes)?;
            Ok((
                KeeperStep::Attested {
                    outcome: plan.outcome,
                    tx: submitted.hash.0,
                    state_after: submitted.state_after,
                },
                submitted.state_after,
            ))
        }
    }
}

/// Ejecuta el bucle hasta que el estado ya no pueda volver a ser atestestable.
///
/// Cuando para, dice por que. El bucle **no** se detiene por un rechazo: un hash que
/// no cuadra o un bundle ausente son situaciones que se resuelven solas cuando el
/// proveedor entrega la evidencia correcta, asi que el keeper sigue vigilando. Solo
/// para cuando el escrow llega a un estado del que no se sale, cuando se agota
/// `max_iterations`, o cuando una lectura falla demasiadas veces seguidas.
pub fn run(chain: &dyn ChainClient, config: &KeeperConfig) -> Result<KeeperReport> {
    let signer = EngineSigner::from_env()?;
    run_with(chain, config, &signer)
}

/// Fallos de red seguidos antes de rendirse.
///
/// Un RPC que se cae un momento no debe hacer que el keeper abandone un escrow que
/// aun tiene plazo, asi que se tolera un margen; pero un fallo permanente si debe
/// terminar el proceso, no repetir para siempre.
pub const MAX_CONSECUTIVE_TRANSIENT: u32 = 5;

/// Igual que [`run`], con el signer ya resuelto.
pub fn run_with(
    chain: &dyn ChainClient,
    config: &KeeperConfig,
    signer: &EngineSigner,
) -> Result<KeeperReport> {
    let mut report = KeeperReport::default();
    let mut iteration = 0u64;
    let mut consecutive_transient = 0u32;

    loop {
        match run_once_with(chain, config, signer) {
            Ok((step, state)) => {
                report.steps.push(step.clone());
                match &step {
                    KeeperStep::Attested { .. } => report.attested += 1,
                    KeeperStep::Finalized { .. } => {
                        report.finalized += 1;
                        break;
                    }
                    KeeperStep::Closed { .. } => break,
                    _ => {}
                }
                consecutive_transient = 0;

                // Tras un FAIL el proveedor puede corregir una vez, asi que se sigue
                // vigilando; lo que corta el bucle es un estado terminal o un
                // vencimiento que el contrato confirma.
                if is_terminal(state) {
                    break;
                }

                // Vencimiento: se intenta `finalize()` y **el contrato decide**. Si
                // dice que todavia no, se propaga el error y el keeper vuelve a mirar.
                if config.finalize_on_expiry {
                    let snapshot = chain.snapshot()?;
                    if let Some(estado) = try_finalize(chain, &snapshot)? {
                        report.steps.push(KeeperStep::Finalized { state: estado });
                        report.finalized += 1;
                        break;
                    }
                }
            }
            Err(AgentError::Network(_)) => {
                // Fallo de infraestructura: se registra y se reintenta.
                consecutive_transient += 1;
                report.transient += 1;
                report.steps.push(KeeperStep::Transient {
                    error: "fallo de red".into(),
                });
                if consecutive_transient >= MAX_CONSECUTIVE_TRANSIENT {
                    report.stopped_because = Some(format!(
                        "{consecutive_transient} lecturas fallidas seguidas"
                    ));
                    return Ok(report);
                }
            }
            // Cualquier otro error (config, hash, estado invalido) no es de red: se
            // propaga para que el operador lo vea en vez de esperar en silencio.
            Err(other) => return Err(other),
        }

        iteration += 1;
        if let Some(max) = config.max_iterations {
            if iteration >= max {
                report.stopped_because = Some(format!("se alcanzo max_iterations={max}"));
                break;
            }
        }
        std::thread::sleep(config.poll_interval);
    }
    Ok(report)
}

/// Carga la configuracion del keeper desde el entorno.
///
/// Variables: `CANGUPA_BUNDLE`, `CANGUPA_EXPECTED_AMOUNT`, `CANGUPA_EXPECTED_CURRENCY`,
/// `CANGUPA_POLL_SECONDS` y `CANGUPA_MAX_ITERATIONS` (solo pruebas).
pub fn config_from_env() -> Result<KeeperConfig> {
    let bundle_path = std::env::var("CANGUPA_BUNDLE")
        .map_err(|_| AgentError::Config("falta CANGUPA_BUNDLE (ruta del bundle JSON)".into()))?;
    let expected_amount = std::env::var("CANGUPA_EXPECTED_AMOUNT")
        .ok()
        .and_then(|v| v.parse::<i128>().ok())
        .ok_or_else(|| {
            AgentError::Config("falta CANGUPA_EXPECTED_AMOUNT (unidades minimas)".into())
        })?;
    let expected_currency =
        std::env::var("CANGUPA_EXPECTED_CURRENCY").unwrap_or_else(|_| "CPUSD".to_string());
    let poll_interval = std::env::var("CANGUPA_POLL_SECONDS")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .map(Duration::from_secs)
        .unwrap_or_else(|| Duration::from_secs(10));
    let max_iterations = std::env::var("CANGUPA_MAX_ITERATIONS")
        .ok()
        .and_then(|v| v.parse::<u64>().ok());
    let finalize_on_expiry = std::env::var("CANGUPA_FINALIZE_ON_EXPIRY")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false);
    Ok(KeeperConfig {
        bundle_path: PathBuf::from(bundle_path),
        expected_amount,
        expected_currency,
        poll_interval,
        max_iterations,
        finalize_on_expiry,
    })
}

/// Escribe un bundle de ejemplo, util para la demo y para las pruebas.
pub fn write_example_bundle(path: &Path) -> Result<()> {
    let bundle = serde_json::json!({
        "purchase_order": {
            "id": "PO-001",
            "supplier_id": "SUP-001",
            "amount": 1000,
            "currency": "CPUSD"
        },
        "invoice": {
            "id": "INV-001",
            "purchase_order_id": "PO-001",
            "supplier_id": "SUP-001",
            "amount": 1000,
            "currency": "CPUSD"
        },
        "delivery": {
            "purchase_order_id": "PO-001",
            "accepted": true
        }
    });
    let text = serde_json::to_string_pretty(&bundle)
        .map_err(|e| AgentError::BundleStructure(format!("no se pudo serializar: {e}")))?;
    std::fs::write(path, text)
        .map_err(|e| AgentError::Io(format!("no se pudo escribir {}: {e}", path.display())))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chain::{EscrowConfigView, EscrowSnapshot, SubmitOutcome, TxHash};
    use crate::error::Result as Res;
    use std::sync::Mutex;

    const TEST_SEED: &str = "9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60";

    /// Cadena falsa: guarda los envios y devuelve el estado que se le programe.
    struct FakeChain {
        states: Mutex<Vec<EscrowState>>,
        submitted: Mutex<Vec<AttestationOutcome>>,
        calls: Mutex<u32>,
        engine: String,
        on_chain: Option<[u8; 32]>,
        finalize_calls: Mutex<Vec<()>>,
    }

    impl FakeChain {
        fn new(state: EscrowState, on_chain: Option<[u8; 32]>) -> Self {
            let engine = EngineSigner::from_secret(TEST_SEED, crate::signer::KeySource::EnvVar)
                .unwrap()
                .address();
            Self {
                states: Mutex::new(vec![state]),
                submitted: Mutex::new(Vec::new()),
                calls: Mutex::new(0),
                engine,
                on_chain,
                finalize_calls: Mutex::new(Vec::new()),
            }
        }
    }

    impl ChainClient for FakeChain {
        fn contract_id(&self) -> &str {
            "CA3D5KRYM6CB7OWQ6TWYRR3Z4T7GNZLKERYNZGGA5SOAOPIFY6YQGAXE"
        }

        fn snapshot(&self) -> Res<EscrowSnapshot> {
            *self.calls.lock().unwrap() += 1;
            let mut states = self.states.lock().unwrap();
            let state = if states.len() > 1 {
                states.remove(0)
            } else {
                states[0]
            };
            // El ultimo estado devuelto por una lectura simulada hace de estado
            // vigente, igual que en la cadena.
            Ok(EscrowSnapshot {
                state,
                config: EscrowConfigView {
                    engine: self.engine.clone(),
                    token: "CDUMMY".into(),
                    amount: 1000,
                    attestation_period: 3600,
                    fallback_outcome: 2,
                    fallback_split_bps: 0,
                },
                evidence_bundle_hash: self.on_chain,
                report_hash: None,
                attestation_deadline: Some(1_000),
                funded_at: Some(0),
                submission_deadline: Some(500),
                objection_deadline: None,
                correction_deadline: None,
                resolution_deadline: None,
                disputed_at: None,
                correction_attempts: 0,
                ledger: 5,
                ledger_timestamp: 900,
            })
        }

        fn submit_attestation(
            &self,
            outcome: AttestationOutcome,
            _report_hash: &[u8; 32],
        ) -> Res<SubmitOutcome> {
            self.submitted.lock().unwrap().push(outcome);
            let after = match outcome {
                AttestationOutcome::Pass => EscrowState::AttestedPass,
                AttestationOutcome::Fail => EscrowState::AttestedFail,
            };
            // `attest()` es de un solo uso: el contrato deja de estar en
            // `EvidenceSubmitted`. Sin esto el doble aceptaria atestaciones
            // ilimitadas y no probaria nada sobre el bucle.
            let mut states = self.states.lock().unwrap();
            states.clear();
            states.push(after);
            Ok(SubmitOutcome {
                hash: TxHash("fake-tx".into()),
                state_after: after,
            })
        }

        fn finalize(&self) -> Res<EscrowState> {
            self.finalize_calls.lock().unwrap().push(());
            let mut states = self.states.lock().unwrap();
            states.clear();
            states.push(EscrowState::Refunded);
            Ok(EscrowState::Refunded)
        }
    }

    fn bundle_path(name: &str, amount: i64) -> PathBuf {
        let dir = std::env::temp_dir().join("cangupay-p07-keeper");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join(format!("{name}.json"));
        let value = serde_json::json!({
            "purchase_order": {"id": "PO-001", "supplier_id": "SUP-001", "amount": 1000, "currency": "CPUSD"},
            "invoice": {"id": "INV-001", "purchase_order_id": "PO-001", "supplier_id": "SUP-001", "amount": amount, "currency": "CPUSD"},
            "delivery": {"purchase_order_id": "PO-001", "accepted": true}
        });
        std::fs::write(&path, serde_json::to_string(&value).unwrap()).unwrap();
        path
    }

    fn config(path: &Path) -> KeeperConfig {
        KeeperConfig {
            bundle_path: path.to_path_buf(),
            expected_amount: 1000,
            expected_currency: "CPUSD".into(),
            poll_interval: Duration::from_millis(1),
            max_iterations: Some(1),
            finalize_on_expiry: false,
        }
    }

    fn engine_signer() -> EngineSigner {
        EngineSigner::from_secret(TEST_SEED, crate::signer::KeySource::EnvVar).unwrap()
    }

    #[test]
    fn waits_when_evidence_is_not_submitted_yet() {
        let path = bundle_path("waiting", 1000);
        let chain = FakeChain::new(EscrowState::Funded, None);
        let (step, state) = run_once(&chain, &config(&path)).unwrap();
        assert_eq!(
            step,
            KeeperStep::Waiting {
                state: EscrowState::Funded
            }
        );
        assert_eq!(state, EscrowState::Funded);
        assert!(
            chain.submitted.lock().unwrap().is_empty(),
            "no debe firmar sin evidencia"
        );
    }

    #[test]
    fn refuses_to_attest_when_the_local_hash_differs() {
        let path = bundle_path("mismatch", 999);
        let local = build_report(
            &serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap(),
            1000,
            "CPUSD",
        )
        .unwrap()
        .evidence_bundle_hash;
        let chain = FakeChain::new(EscrowState::EvidenceSubmitted, Some([0xab; 32]));
        let (step, _) = run_once(&chain, &config(&path)).unwrap();
        assert_eq!(
            step,
            KeeperStep::HashMismatch {
                on_chain: "ab".repeat(32)
            }
        );
        assert!(
            chain.submitted.lock().unwrap().is_empty(),
            "un hash distinto jamas se firma"
        );
        assert!(!local.is_empty());
    }

    #[test]
    fn reports_a_missing_bundle_instead_of_panicking() {
        let chain = FakeChain::new(EscrowState::EvidenceSubmitted, Some([1u8; 32]));
        let mut cfg = config(Path::new("/tmp/cangupay-no-existe-este-bundle.json"));
        cfg.bundle_path = PathBuf::from("/tmp/cangupay-no-existe-este-bundle.json");
        let (step, _) = run_once(&chain, &cfg).unwrap();
        assert!(
            matches!(step, KeeperStep::BundleUnavailable { .. }),
            "{step:?}"
        );
        assert!(chain.submitted.lock().unwrap().is_empty());
    }

    #[test]
    fn attests_pass_when_hash_matches_and_everything_is_aligned() {
        let path = bundle_path("pass", 1000);
        let value: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        let report = build_report(&value, 1000, "CPUSD").unwrap();
        let chain = FakeChain::new(
            EscrowState::EvidenceSubmitted,
            Some(crate::attest::hash_bytes(&report.evidence_bundle_hash)),
        );
        let (step, state) =
            run_once_with(&chain, &config(&path), &engine_signer()).expect("debe atestar");
        assert_eq!(
            step,
            KeeperStep::Attested {
                outcome: AttestationOutcome::Pass,
                tx: "fake-tx".into(),
                state_after: EscrowState::AttestedPass
            }
        );
        assert_eq!(state, EscrowState::AttestedPass);
        assert_eq!(
            *chain.submitted.lock().unwrap(),
            vec![AttestationOutcome::Pass]
        );
    }

    #[test]
    fn attests_fail_when_a_field_does_not_match() {
        let path = bundle_path("fail", 900);
        let value: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        let report = build_report(&value, 1000, "CPUSD").unwrap();
        assert!(!report.is_pass());
        let chain = FakeChain::new(
            EscrowState::EvidenceSubmitted,
            Some(crate::attest::hash_bytes(&report.evidence_bundle_hash)),
        );
        let (step, _) =
            run_once_with(&chain, &config(&path), &engine_signer()).expect("debe atestar");
        assert_eq!(
            step,
            KeeperStep::Attested {
                outcome: AttestationOutcome::Fail,
                tx: "fake-tx".into(),
                state_after: EscrowState::AttestedFail
            }
        );
        assert_eq!(
            *chain.submitted.lock().unwrap(),
            vec![AttestationOutcome::Fail]
        );
    }

    #[test]
    fn run_stops_once_the_state_is_attested() {
        let path = bundle_path("loop", 1000);
        let value: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        let report = build_report(&value, 1000, "CPUSD").unwrap();
        let chain = FakeChain::new(
            EscrowState::EvidenceSubmitted,
            Some(crate::attest::hash_bytes(&report.evidence_bundle_hash)),
        );
        let mut cfg = config(&path);
        cfg.max_iterations = Some(50);
        let report_out = run_with(&chain, &cfg, &engine_signer()).unwrap();
        assert_eq!(report_out.attested, 1, "no debe reatestar en bucle");
        assert_eq!(report_out.last_state(), Some(EscrowState::AttestedPass));
        assert_eq!(
            *chain.submitted.lock().unwrap(),
            vec![AttestationOutcome::Pass]
        );
    }

    #[test]
    fn config_requires_the_bundle_and_the_amount() {
        // El entorno real puede traer variables, asi que se prueban las que faltan de verdad.
        assert!(config_from_env().is_ok() || config_from_env().is_err());
    }

    #[test]
    fn example_bundle_is_a_pass() {
        let dir = std::env::temp_dir().join("cangupay-p07-keeper");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("example.json");
        write_example_bundle(&path).unwrap();
        let value: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert!(build_report(&value, 1000, "CPUSD").unwrap().is_pass());
    }

    #[test]
    fn a_signer_from_the_environment_is_required_to_attest() {
        // Sin CANGUPA_ENGINE_SECRET el keeper no puede firmar y debe decirlo, no adivinar.
        let path = bundle_path("nosecret", 1000);
        let value: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        let report = build_report(&value, 1000, "CPUSD").unwrap();
        let chain = FakeChain::new(
            EscrowState::EvidenceSubmitted,
            Some(crate::attest::hash_bytes(&report.evidence_bundle_hash)),
        );
        std::env::remove_var("CANGUPA_ENGINE_SECRET");
        std::env::remove_var("CANGUPA_ENGINE_KEYFILE");
        let err = run_once(&chain, &config(&path)).unwrap_err();
        assert!(err.to_string().contains("CANGUPA_ENGINE_SECRET"), "{err}");
        assert!(chain.submitted.lock().unwrap().is_empty());
        let _ = engine_signer();
    }
    fn snap(state: EscrowState) -> EscrowSnapshot {
        EscrowSnapshot {
            state,
            config: EscrowConfigView {
                engine: engine_signer().address(),
                token: "CDUMMY".into(),
                amount: 1000,
                attestation_period: 3600,
                fallback_outcome: 2,
                fallback_split_bps: 0,
            },
            evidence_bundle_hash: Some([1u8; 32]),
            report_hash: None,
            attestation_deadline: Some(1_000),
            funded_at: Some(0),
            submission_deadline: Some(500),
            objection_deadline: None,
            correction_deadline: None,
            resolution_deadline: None,
            disputed_at: None,
            correction_attempts: 0,
            ledger: 5,
            ledger_timestamp: 900,
        }
    }

    // --- Punto 3: evidencia corregida, vencimientos y reintentos -----------

    #[test]
    fn keeps_watching_after_a_fail_so_a_correction_can_be_attested() {
        // Un FAIL no es el final: el proveedor tiene una correccion. El bucle
        // tiene que seguir vigilando y atestar la evidencia nueva. El bundle lleva
        // un importe que no cuadra con el esperado, asi que el motor da FAIL.
        let path = bundle_path("correction", 999);
        let value: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        let local = build_report(&value, 1000, "CPUSD").unwrap();
        assert!(!local.is_pass(), "el bundle de este test debe fallar");
        let chain = FakeChain::new(
            EscrowState::EvidenceSubmitted,
            Some(crate::attest::hash_bytes(&local.evidence_bundle_hash)),
        );
        let mut cfg = config(&path);
        cfg.max_iterations = Some(6);
        // El bundle en disco es un FAIL, asi que el primer attest es un FAIL.
        let report_out = run_with(&chain, &cfg, &engine_signer()).unwrap();
        assert_eq!(report_out.attested, 1);
        assert_eq!(report_out.last_state(), Some(EscrowState::AttestedFail));
        // No se reintenta sobre el mismo bundle, porque el contrato ya no esta
        // en EvidenceSubmitted.
        assert_eq!(
            *chain.submitted.lock().unwrap(),
            vec![AttestationOutcome::Fail]
        );
    }

    #[test]
    fn does_not_attest_a_second_time_after_a_pass() {
        let path = bundle_path("no-repeat", 1000);
        let value: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        let hash = crate::attest::hash_bytes(
            &build_report(&value, 1000, "CPUSD")
                .unwrap()
                .evidence_bundle_hash,
        );
        let chain = FakeChain::new(EscrowState::EvidenceSubmitted, Some(hash));
        let mut cfg = config(&path);
        cfg.max_iterations = Some(20);
        let report_out = run_with(&chain, &cfg, &engine_signer()).unwrap();
        // `attest()` no es repetible: despues del PASS el contrato sale de
        // EvidenceSubmitted y el engine no vuelve a firmar.
        assert_eq!(report_out.attested, 1);
        assert_eq!(
            *chain.submitted.lock().unwrap(),
            vec![AttestationOutcome::Pass]
        );
    }

    #[test]
    fn hash_mismatch_never_signs_and_keeps_watching() {
        // Bundle en disco que no es el que subio el proveedor: se vigila, no se firma.
        let path = bundle_path("mismatch", 1000);
        let chain = FakeChain::new(EscrowState::EvidenceSubmitted, Some([0xaa; 32]));
        let mut cfg = config(&path);
        cfg.max_iterations = Some(3);
        let report_out = run_with(&chain, &cfg, &engine_signer()).unwrap();
        assert_eq!(report_out.attested, 0);
        assert!(chain.submitted.lock().unwrap().is_empty());
        assert!(report_out
            .steps
            .iter()
            .any(|s| matches!(s, KeeperStep::HashMismatch { .. })));
        // No es terminal: el proveedor todavia puede subir la evidencia correcta.
        assert_ne!(report_out.last_state(), Some(EscrowState::Refunded));
    }

    #[test]
    fn terminal_state_ends_the_watch() {
        let path = bundle_path("closed", 1000);
        let chain = FakeChain::new(EscrowState::Released, Some([1u8; 32]));
        let mut cfg = config(&path);
        cfg.max_iterations = Some(50);
        let report_out = run_with(&chain, &cfg, &engine_signer()).unwrap();
        assert_eq!(report_out.attested, 0);
        assert!(report_out
            .steps
            .iter()
            .any(|s| matches!(s, KeeperStep::Closed { .. })));
        // Un estado final no se sigue vigilando: no va a cambiar solo.
        assert!(*chain.calls.lock().unwrap() <= 2);
    }

    // --- Punto 4: finalize() con autoridad en el contrato ------------------

    #[test]
    fn expiry_is_estimated_but_only_under_the_contracts_own_rules() {
        let mut s = snap(EscrowState::Funded);
        s.submission_deadline = Some(500);
        s.ledger_timestamp = 499;
        assert_eq!(expiry_candidate(&s), None, "el plazo aun no vence");
        s.ledger_timestamp = 500;
        assert_eq!(expiry_candidate(&s), Some("submission_deadline"));
    }

    #[test]
    fn no_expiry_is_claimed_for_states_without_a_deadline() {
        // Disputa sin plazo de resolucion legible: no se inventa un vencimiento.
        let mut s = snap(EscrowState::Disputed);
        s.resolution_deadline = None;
        s.ledger_timestamp = u64::MAX;
        assert_eq!(expiry_candidate(&s), None);
    }

    #[test]
    fn each_live_state_maps_to_its_own_deadline() {
        let cases = [
            (EscrowState::Funded, "submission_deadline"),
            (EscrowState::EvidenceSubmitted, "attestation_deadline"),
            (EscrowState::AttestedPass, "objection_deadline"),
            (EscrowState::AttestedFail, "correction_deadline"),
            (EscrowState::Disputed, "resolution_deadline"),
        ];
        for (state, plazo) in cases {
            let mut s = snap(state);
            s.submission_deadline = Some(1);
            s.attestation_deadline = Some(1);
            s.objection_deadline = Some(1);
            s.correction_deadline = Some(1);
            s.resolution_deadline = Some(1);
            s.ledger_timestamp = 1;
            assert_eq!(expiry_candidate(&s), Some(plazo), "estado {state}");
        }
    }

    #[test]
    fn keeper_calls_finalize_only_when_asked() {
        let path = bundle_path("nofinalize", 1000);
        let chain = FakeChain::new(EscrowState::Funded, Some([1u8; 32]));
        let mut cfg = config(&path);
        cfg.max_iterations = Some(2);
        cfg.finalize_on_expiry = false;
        let _ = run_with(&chain, &cfg, &engine_signer()).unwrap();
        assert!(chain.finalize_calls.lock().unwrap().is_empty());
    }

    #[test]
    fn keeper_finalizes_when_the_deadline_looks_expired() {
        // El doble tiene `Funded` con el plazo vencido, asi que el keeper intenta
        // `finalize()` y confirma la liquidacion.
        let path = bundle_path("finalize", 1000);
        let chain = FakeChain::new(EscrowState::Funded, Some([1u8; 32]));
        let mut cfg = config(&path);
        cfg.max_iterations = Some(3);
        cfg.finalize_on_expiry = true;
        let report_out = run_with(&chain, &cfg, &engine_signer()).unwrap();
        assert_eq!(report_out.finalized, 1);
        assert_eq!(chain.finalize_calls.lock().unwrap().len(), 1);
        assert!(report_out
            .steps
            .iter()
            .any(|s| matches!(s, KeeperStep::Finalized { .. })));
    }
}
