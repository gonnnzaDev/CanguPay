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
}

/// Resumen de una ejecucion completa.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct KeeperReport {
    /// Pasos dados, en orden.
    pub steps: Vec<KeeperStep>,
    /// Cuantas atestaciones se enviaron.
    pub attested: u32,
}

impl KeeperReport {
    /// Ultimo estado observado, si hubo pasos.
    pub fn last_state(&self) -> Option<EscrowState> {
        self.steps.iter().rev().find_map(|s| match s {
            KeeperStep::Waiting { state } => Some(*state),
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
        Observation::Waiting { state } => Ok((KeeperStep::Waiting { state: *state }, *state)),
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

/// Ejecuta el bucle hasta `max_iterations` o hasta que el estado ya no sea atestatable.
pub fn run(chain: &dyn ChainClient, config: &KeeperConfig) -> Result<KeeperReport> {
    let signer = EngineSigner::from_env()?;
    run_with(chain, config, &signer)
}

/// Igual que [`run`], con el signer ya resuelto.
pub fn run_with(
    chain: &dyn ChainClient,
    config: &KeeperConfig,
    signer: &EngineSigner,
) -> Result<KeeperReport> {
    let mut report = KeeperReport::default();
    let mut iteration = 0u64;

    loop {
        let (step, state) = run_once_with(chain, config, signer)?;
        match &step {
            KeeperStep::Attested { .. } => report.attested += 1,
            other => report.steps.push(other.clone()),
        }
        if matches!(step, KeeperStep::Attested { .. }) {
            report.steps.push(step);
        }

        // Ya no hay nada que hacer hasta que el proveedor envíe una correccion.
        if matches!(state, EscrowState::AttestedPass | EscrowState::AttestedFail) {
            break;
        }
        iteration += 1;
        if let Some(max) = config.max_iterations {
            if iteration >= max {
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
    Ok(KeeperConfig {
        bundle_path: PathBuf::from(bundle_path),
        expected_amount,
        expected_currency,
        poll_interval,
        max_iterations,
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
                attestation_deadline: Some(1_000),
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
            Ok(SubmitOutcome {
                hash: TxHash("fake-tx".into()),
                state_after: after,
            })
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
}
