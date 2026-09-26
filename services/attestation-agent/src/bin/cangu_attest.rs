//! CLI del agente de atestacion.
//!
//! Subcomandos:
//!
//! - `selfcheck`  comprueba fixtures, red y clave sin enviar nada.
//! - `evaluate`   produce el reporte PASS/FAIL de un bundle local.
//! - `status`     lee el estado real del contrato.
//! - `attest`     firma y envia `attest()` tras comprobar estado, plazo y hash.
//! - `keeper`     vigila el contrato y atesta solo cuando toca.
//!
//! Los codigos de salida siguen a `error::exit_code()` para que el script de demo pueda
//! distinguir un FAIL de reglas (3) de un error de configuracion (2).
use std::path::PathBuf;
use std::process::ExitCode;

use attestation_agent::attest::plan_attestation;
use attestation_agent::chain::ChainClient;
use attestation_agent::chain::SorobanChain;
use attestation_agent::error::{exit, AgentError, Result};
use attestation_agent::keeper::{self, KeeperConfig, KeeperStep};
use attestation_agent::report::build_report;
use attestation_agent::signer::EngineSigner;

const USAGE: &str = "\
cangu-attest <comando> [opciones]

Comandos:
  selfcheck              verifica fixtures, red y clave del engine
  evaluate               evalua un bundle y muestra el reporte PASS/FAIL
  status                 muestra el estado real del contrato
  attest                 firma y envia attest() tras comprobar estado, plazo y hash
  keeper                 vigila el contrato y atesta cuando la evidencia esta lista
  finalize               llama a finalize(); el contrato decide si el plazo vencio

Opciones:
  --bundle <ruta>        bundle de evidencia en JSON (o CANGUPA_BUNDLE)
  --amount <entero>      importe esperado en unidades minimas (o CANGUPA_EXPECTED_AMOUNT)
  --currency <code>      divisa esperada, por defecto CPUSD
  --contract <C...>      id del contrato (o CANGUPA_CONTRACT_ID)
  --rpc <url>            endpoint RPC (o CANGUPA_RPC_URL)
  --network <pass>       passphrase de red (o CANGUPA_NETWORK)
  --poll <segundos>      espera del keeper, por defecto 10
  --max-iterations <n>   tope de vueltas del keeper; sin limite vigila para siempre
  --finalize             el keeper tambien llama a finalize() al detectar un vencimiento
  --json                 salida en JSON
  -h, --help             esta ayuda
";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match run(&args) {
        Ok(code) => code,
        Err(err) => {
            eprintln!("error: {err}");
            ExitCode::from(err.exit_code() as u8)
        }
    }
}

fn run(args: &[String]) -> Result<ExitCode> {
    if args.is_empty() || args.iter().any(|a| a == "-h" || a == "--help") {
        print!("{USAGE}");
        return Ok(ExitCode::SUCCESS);
    }
    let command = args[0].clone();
    let opts = Options::parse(&args[1..])?;

    match command.as_str() {
        "selfcheck" => selfcheck(&opts),
        "evaluate" => evaluate(&opts),
        "status" => status(&opts),
        "attest" => attest(&opts),
        "keeper" => keep(&opts),
        "finalize" => finalize(&opts),
        other => Err(AgentError::Config(format!(
            "comando desconocido: {other}; usa --help"
        ))),
    }
}

/// Opciones del CLI, con fallback a variables de entorno.
struct Options {
    bundle: Option<PathBuf>,
    amount: Option<i128>,
    currency: String,
    contract: Option<String>,
    rpc: Option<String>,
    network: Option<String>,
    poll: u64,
    max_iterations: Option<u64>,
    finalize: bool,
    json: bool,
}

impl Options {
    fn parse(args: &[String]) -> Result<Self> {
        let mut opts = Options {
            bundle: None,
            amount: None,
            currency: "CPUSD".to_string(),
            contract: None,
            rpc: None,
            network: None,
            poll: 10,
            max_iterations: None,
            finalize: false,
            json: false,
        };
        let mut i = 0;
        while i < args.len() {
            let flag = args[i].as_str();
            let mut value = || -> Result<String> {
                i += 1;
                args.get(i)
                    .cloned()
                    .ok_or_else(|| AgentError::Config(format!("{flag} necesita un valor")))
            };
            match flag {
                "--bundle" => opts.bundle = Some(PathBuf::from(value()?)),
                "--amount" => {
                    let raw = value()?;
                    opts.amount = Some(raw.parse::<i128>().map_err(|_| {
                        AgentError::Config(format!("--amount debe ser un entero, no {raw}"))
                    })?);
                }
                "--currency" => opts.currency = value()?,
                "--contract" => opts.contract = Some(value()?),
                "--rpc" => opts.rpc = Some(value()?),
                "--network" => opts.network = Some(value()?),
                "--poll" => {
                    let raw = value()?;
                    opts.poll = raw
                        .parse::<u64>()
                        .map_err(|_| AgentError::Config("--poll debe ser un entero".into()))?;
                }
                "--max-iterations" => {
                    let raw = value()?;
                    opts.max_iterations = Some(raw.parse::<u64>().map_err(|_| {
                        AgentError::Config("--max-iterations debe ser un entero".into())
                    })?);
                }
                "--finalize" => opts.finalize = true,
                "--json" => opts.json = true,
                other => return Err(AgentError::Config(format!("opcion desconocida: {other}"))),
            }
            i += 1;
        }

        if opts.bundle.is_none() {
            opts.bundle = std::env::var("CANGUPA_BUNDLE").ok().map(PathBuf::from);
        }
        if opts.amount.is_none() {
            opts.amount = std::env::var("CANGUPA_EXPECTED_AMOUNT")
                .ok()
                .and_then(|v| v.parse().ok());
        }
        if let Ok(v) = std::env::var("CANGUPA_EXPECTED_CURRENCY") {
            opts.currency = v;
        }
        if opts.contract.is_none() {
            opts.contract = std::env::var("CANGUPA_CONTRACT_ID").ok();
        }
        if opts.rpc.is_none() {
            opts.rpc = std::env::var("CANGUPA_RPC_URL").ok();
        }
        if opts.network.is_none() {
            opts.network = std::env::var("CANGUPA_NETWORK").ok();
        }
        Ok(opts)
    }

    fn require_bundle(&self) -> Result<(&PathBuf, i128)> {
        let bundle = self.bundle.as_ref().ok_or_else(|| {
            AgentError::Config("falta el bundle: usa --bundle o CANGUPA_BUNDLE".into())
        })?;
        let amount = self.amount.ok_or_else(|| {
            AgentError::Config("falta el importe: usa --amount o CANGUPA_EXPECTED_AMOUNT".into())
        })?;
        Ok((bundle, amount))
    }

    fn require_chain(&self) -> Result<SorobanChain> {
        let contract = self.contract.clone().ok_or_else(|| {
            AgentError::Config("falta el contrato: usa --contract o CANGUPA_CONTRACT_ID".into())
        })?;
        let rpc = self.rpc.clone().ok_or_else(|| {
            AgentError::Config("falta el RPC: usa --rpc o CANGUPA_RPC_URL".into())
        })?;
        let network = self.network.clone().ok_or_else(|| {
            AgentError::Config("falta la red: usa --network o CANGUPA_NETWORK".into())
        })?;
        SorobanChain::new(&rpc, &network, &contract)
    }
}

fn read_bundle(path: &PathBuf) -> Result<serde_json::Value> {
    let text = std::fs::read_to_string(path)
        .map_err(|e| AgentError::Io(format!("no se pudo leer {}: {e}", path.display())))?;
    serde_json::from_str(&text).map_err(|e| {
        AgentError::BundleStructure(format!("{} no es JSON valido: {e}", path.display()))
    })
}

fn selfcheck(opts: &Options) -> Result<ExitCode> {
    let mut problems: Vec<String> = Vec::new();

    // 1. Fixtures: el motor tiene que reproducir los hashes del manifiesto.
    let fixtures = attestation_agent::golden::fixture_dir();
    match attestation_agent::golden::verify_fixtures(&fixtures) {
        Ok(n) => println!("fixtures: {n} casos reproducen el manifiesto"),
        Err(e) => problems.push(format!("fixtures: {e}")),
    }

    // 2. Clave del engine: solo se comprueba que exista y de donde viene.
    match EngineSigner::from_env() {
        Ok(signer) => println!("clave del engine: {}", signer.describe()),
        Err(e) => problems.push(format!("clave del engine: {e}")),
    }

    // 3. Red y contrato, solo si estan configurados.
    match opts.require_chain() {
        Ok(chain) => match chain.snapshot() {
            Ok(snapshot) => println!(
                "contrato {}: estado {} en ledger {}",
                chain.contract_id(),
                snapshot.state,
                snapshot.ledger
            ),
            Err(e) => problems.push(format!("lectura del contrato: {e}")),
        },
        Err(_) => println!("red: sin configurar (se omite la comprobacion)"),
    }

    if problems.is_empty() {
        println!("selfcheck: OK");
        return Ok(ExitCode::SUCCESS);
    }
    for p in &problems {
        eprintln!("selfcheck: {p}");
    }
    Ok(ExitCode::from(exit::REJECT as u8))
}

fn evaluate(opts: &Options) -> Result<ExitCode> {
    let (path, amount) = opts.require_bundle()?;
    let value = read_bundle(path)?;
    let report = build_report(&value, amount, &opts.currency)?;

    if opts.json {
        println!("{}", report.to_json());
    } else {
        println!("bundle_hash: {}", report.evidence_bundle_hash);
        println!("report_hash: {}", report.report_hash);
        println!("detail_hash: {}", report.detail_hash);
        println!("resultado:   {}", report.result);
        for field in &report.failed_fields {
            println!(
                "  fallo: {} [{}] observado={} esperado={}",
                field.path,
                field.status.as_str(),
                field.observed.as_deref().unwrap_or("-"),
                field.expected.as_deref().unwrap_or("-")
            );
        }
    }
    Ok(if report.is_pass() {
        ExitCode::from(exit::OK as u8)
    } else {
        ExitCode::from(exit::FAIL as u8)
    })
}

fn status(opts: &Options) -> Result<ExitCode> {
    let chain = opts.require_chain()?;
    let snapshot = chain.snapshot()?;
    if opts.json {
        println!("{}", snapshot.to_json());
    } else {
        println!("contrato:   {}", chain.contract_id());
        println!("estado:     {}", snapshot.state);
        println!("ledger:     {}", snapshot.ledger);
        println!("engine:     {}", snapshot.config.engine);
        println!("importe:    {}", snapshot.config.amount);
        println!("plazo:      {}s", snapshot.config.attestation_period);
        if let Some(hash) = snapshot.evidence_bundle_hash {
            println!("evidencia:  {}", hex::encode(hash));
        } else {
            println!("evidencia:  (sin hash)");
        }
        if let Some(remaining) = snapshot.attestation_remaining() {
            println!("restante:   {remaining}s");
        }
    }
    Ok(ExitCode::SUCCESS)
}

fn attest(opts: &Options) -> Result<ExitCode> {
    let (path, amount) = opts.require_bundle()?;
    let chain = opts.require_chain()?;
    let value = read_bundle(path)?;
    let report = build_report(&value, amount, &opts.currency)?;
    let signer = EngineSigner::from_env()?;

    let snapshot = chain.snapshot()?;
    let plan = plan_attestation(&snapshot, &report, &signer)?;
    println!(
        "plan: {} con report_hash {} (ledger {})",
        match plan.outcome {
            attestation_agent::chain::AttestationOutcome::Pass => "PASS",
            attestation_agent::chain::AttestationOutcome::Fail => "FAIL",
        },
        plan.report_hash,
        plan.ledger
    );

    let submitted = chain.submit_attestation(plan.outcome, &plan.report_hash_bytes)?;
    println!("tx:         {}", submitted.hash.0);
    println!("estado:     {}", submitted.state_after);
    let link = attestation_agent::attest::explorer_tx_link(&explorer_base(&chain), &submitted.hash);
    println!("explorer:   {link}");
    Ok(ExitCode::SUCCESS)
}

fn keep(opts: &Options) -> Result<ExitCode> {
    let (path, amount) = opts.require_bundle()?;
    let chain = opts.require_chain()?;
    let config = KeeperConfig {
        bundle_path: path.clone(),
        expected_amount: amount,
        expected_currency: opts.currency.clone(),
        poll_interval: std::time::Duration::from_secs(opts.poll),
        max_iterations: opts.max_iterations,
        // El contrato conserva la autoridad sobre los vencimientos: el keeper solo
        // llama a `finalize()` cuando cree que se cumple uno, y es el contrato el
        // que decide si lo es.
        finalize_on_expiry: opts.finalize,
    };
    println!(
        "keeper: vigilando {} cada {}s{}",
        chain.contract_id(),
        opts.poll,
        if opts.finalize { " (con finalize)" } else { "" }
    );
    let report = keeper::run(&chain, &config)?;
    for step in &report.steps {
        match step {
            KeeperStep::Waiting { state } => println!("  esperando: {state}"),
            KeeperStep::BundleUnavailable { path } => {
                println!("  bundle no disponible: {path}")
            }
            KeeperStep::HashMismatch { on_chain } => {
                println!("  hash en cadena {on_chain} != hash local; no se firma")
            }
            KeeperStep::Attested {
                outcome,
                tx,
                state_after,
            } => {
                println!("  atestado {outcome:?} en {tx}; estado {state_after}")
            }
            KeeperStep::Closed { state } => println!("  cerrado: {state}"),
            KeeperStep::Finalized { state } => println!("  vencido y liquidado: {state}"),
            KeeperStep::Transient { error } => println!("  reintentando: {error}"),
        }
    }
    if let Some(why) = &report.stopped_because {
        println!("keeper: deja de vigilar ({why})");
    }
    println!(
        "keeper: {} atestaciones enviadas, {} liquidaciones, {} lecturas con fallo",
        report.attested, report.finalized, report.transient
    );
    Ok(ExitCode::SUCCESS)
}

/// Llama a `finalize()` una vez.
///
/// El contrato es quien decide si el plazo vencio: si responde `NotFinalizableYet` el
/// comando falla con ese error en vez de forzar nada. Se imprime el estado resultante.
fn finalize(opts: &Options) -> Result<ExitCode> {
    let chain = opts.require_chain()?;
    let state = chain.finalize()?;
    if opts.json {
        println!(
            "{}",
            serde_json::json!({ "finalized": true, "state": state.to_string() })
        );
    } else {
        println!("estado tras finalize(): {state}");
    }
    Ok(ExitCode::SUCCESS)
}

/// Base del explorer para el enlace de la transaccion.
fn explorer_base(chain: &SorobanChain) -> String {
    if chain.network_passphrase().contains("Test") {
        "https://stellar.expert/testnet".to_string()
    } else {
        "https://stellar.expert/explorer/public".to_string()
    }
}
