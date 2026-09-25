//! Lectura del estado real del contrato.
//!
//! [`ChainClient`] aisla toda la E/S de cadena para que el motor se pueda probar con
//! dobles; [`SorobanChain`] es la implementacion contra el RPC oficial
//! (`stellar-rpc-client`, protocolo 27, el mismo que el `soroban-sdk` del contrato).
//!
//! El agente **nunca** firma ni envia desde este modulo: solo lee. La firma vive en
//! [`crate::signer`] y el envio en [`crate::attest`].
use std::sync::Arc;

use stellar_xdr as xdr;
use stellar_xdr::ScVal;
use stellar_xdr::{Limits, TransactionEnvelope, WriteXdr};

use crate::attest::ATTEST_FN;
use crate::error::{AgentError, Result};
use crate::signer::EngineSigner;

/// Estado que devuelve `getTransaction` cuando la transaccion fue incluida con exito.
const TX_SUCCESS: &str = "SUCCESS";

/// Estados del contrato, en el mismo orden que el enum Rust del contrato.
/// El orden importa: `#[contracttype]` los serializa como `ScVal::U32` con el indice.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
    /// Discriminante desconocido: el contrato fue actualizado o la lectura es corrupta.
    Unknown(u32),
}

impl EscrowSnapshot {
    /// Vista JSON para el CLI, sin tipos de XDR que no se serialicen solos.
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "state": self.state.to_string(),
            "ledger": self.ledger,
            "ledger_timestamp": self.ledger_timestamp,
            "engine": self.config.engine,
            "token": self.config.token,
            "amount": self.config.amount,
            "attestation_period": self.config.attestation_period,
            "evidence_bundle_hash": self.evidence_bundle_hash.map(hex::encode),
            "attestation_deadline": self.attestation_deadline,
            "attestation_remaining": self.attestation_remaining(),
        })
    }
}

impl EscrowState {
    /// Nombre estable para logs, CLI y reportes.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Created => "Created",
            Self::Funded => "Funded",
            Self::EvidenceSubmitted => "EvidenceSubmitted",
            Self::AttestedPass => "AttestedPass",
            Self::AttestedFail => "AttestedFail",
            Self::Released => "Released",
            Self::Cancelled => "Cancelled",
            Self::Refunded => "Refunded",
            Self::Disputed => "Disputed",
            Self::Split => "Split",
            Self::Unknown(_) => "Unknown",
        }
    }

    fn from_u32(v: u32) -> Self {
        match v {
            0 => Self::Created,
            1 => Self::Funded,
            2 => Self::EvidenceSubmitted,
            3 => Self::AttestedPass,
            4 => Self::AttestedFail,
            5 => Self::Released,
            6 => Self::Cancelled,
            7 => Self::Refunded,
            8 => Self::Disputed,
            9 => Self::Split,
            other => Self::Unknown(other),
        }
    }
}

impl std::fmt::Display for EscrowState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unknown(v) => write!(f, "Unknown({v})"),
            other => f.write_str(other.as_str()),
        }
    }
}

/// Configuracion del escrow, limited a lo que el agente necesita para decidir.
#[derive(Debug, Clone, PartialEq)]
pub struct EscrowConfigView {
    /// Cuenta engine: debe coincidir con la clave que firma.
    pub engine: String,
    /// Direccion del token (SAC) del escrow.
    pub token: String,
    /// Monto en unidades minimas.
    pub amount: i128,
    /// Ventana de atestacion, en segundos de ledger.
    pub attestation_period: u64,
    /// Outcome de fallback pactado.
    pub fallback_outcome: u32,
    /// Comision de fallback en basis points.
    pub fallback_split_bps: u32,
}

/// Instantanea del escrow, tal y como esta en la cadena.
///
/// Los datos vienen del getter `snapshot()` del contrato, no de leer su
/// almacenamiento: el getter es la ABI estable y nombra cada campo, asi que
/// reordenar `EscrowConfig` no rompe al agente. El orden de los campos dentro
/// del `ScVec` es un detalle de serializacion que solo debe conocer el contrato.
#[derive(Debug, Clone, PartialEq)]
pub struct EscrowSnapshot {
    /// Estado actual.
    pub state: EscrowState,
    /// Configuracion vigente.
    pub config: EscrowConfigView,
    /// Hash del bundle que el supplier ya subio a la cadena, si existe.
    pub evidence_bundle_hash: Option<[u8; 32]>,
    /// Hash del reporte con el que el engine atestiguo, si ya atesto.
    pub report_hash: Option<[u8; 32]>,
    /// Plazo de atestacion absoluto, si el estado ya lo fijo.
    pub attestation_deadline: Option<u64>,
    /// Momento de la fundicion, si ocurrio.
    pub funded_at: Option<u64>,
    /// Plazo para que el buyer cancele o el supplier entregue evidencia.
    pub submission_deadline: Option<u64>,
    /// `attested_at + objection_period`, si el engine ya atesto.
    pub objection_deadline: Option<u64>,
    /// `attested_at + correction_period`, si el engine ya atesto.
    pub correction_deadline: Option<u64>,
    /// Plazo de resolucion de una disputa, si la hubo.
    pub resolution_deadline: Option<u64>,
    /// Momento de la disputa, si la hubo.
    pub disputed_at: Option<u64>,
    /// Veces que el supplier ya corrigio la evidencia (0 o 1 en P0-07).
    pub correction_attempts: u32,
    /// Ledger actual, para decisiones de plazo.
    pub ledger: u32,
    /// Timestamp del ledger actual.
    pub ledger_timestamp: u64,
}

impl EscrowSnapshot {
    /// `true` si el contrato esta listo para que el engine ateste.
    pub fn is_attestable(&self) -> bool {
        self.state == EscrowState::EvidenceSubmitted
    }

    /// Segundos que faltan para el plazo de atestacion, si aplica.
    pub fn attestation_remaining(&self) -> Option<i64> {
        let deadline = self.attestation_deadline?;
        Some(deadline as i64 - self.ledger_timestamp as i64)
    }

    /// `true` si la evidencia fue corregida tras un FAIL: el motor debe volver a
    /// evaluarla y el hash que evalua tiene que ser el nuevo.
    pub fn is_correction(&self) -> bool {
        self.correction_attempts > 0
    }
}

/// Hash de transaccion devuelto por el envio.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TxHash(pub String);

impl std::fmt::Display for TxHash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// Resultado de un envio confirmado.
#[derive(Debug, Clone, PartialEq)]
pub struct SubmitOutcome {
    /// Hash de la transaccion, para el enlace al explorer.
    pub hash: TxHash,
    /// Estado observado despues de confirmar.
    pub state_after: EscrowState,
}

/// Operaciones de cadena que necesita el agente.
///
/// Se implementa con `&self` para que un doble en las pruebas sea trivial.
pub trait ChainClient: Send + Sync {
    /// Identificador del contrato, en `C...` (strkey).
    fn contract_id(&self) -> &str;
    /// Lee el estado, la configuracion y el hash de evidencia.
    fn snapshot(&self) -> Result<EscrowSnapshot>;
    /// Firma y envia `attest(outcome, report_hash)`; debe confirmar el estado resultante.
    fn submit_attestation(
        &self,
        outcome: AttestationOutcome,
        report_hash: &[u8; 32],
    ) -> Result<SubmitOutcome>;
    /// Firma y envia `finalize()`; el contrato decide si el escrow ya vencio.
    ///
    /// Se deja en `Err` cuando el propio contrato dice `NotFinalizableYet`. Esa
    /// respuesta es la autoridad: el keeper **no** lleva su propia cuenta de plazos ni
    /// decide por su cuenta que un escrow vencio, porque duplicar esa aritmetica en el
    /// cliente es exactamente la forma de liquidar fondos antes de tiempo o de
    /// quedarse bloqueado esperando un plazo que el contrato no aplica.
    fn finalize(&self) -> Result<EscrowState> {
        Err(AgentError::Config(
            "el cliente no implementa finalize()".into(),
        ))
    }
}

/// Outcome que se manda a `attest()`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttestationOutcome {
    Pass,
    Fail,
}

impl AttestationOutcome {
    /// Discriminante del enum del contrato.
    pub fn as_u32(self) -> u32 {
        match self {
            Self::Pass => 0,
            Self::Fail => 1,
        }
    }
}

/// Implementacion contra el RPC de Soroban.
pub struct SorobanChain {
    rpc_url: String,
    contract: String,
    /// Solo se usa para validar el strkey al construir: un `CANGUPA_CONTRACT_ID`
    /// mal escrito falla al arrancar y no a mitad de una atestacion.
    #[allow(dead_code)]
    contract_bytes: [u8; 32],
    network_passphrase: String,
}

impl SorobanChain {
    /// Crea un cliente a partir del id strkey del contrato.
    pub fn new(rpc_url: &str, network_passphrase: &str, contract_id: &str) -> Result<Self> {
        let contract_bytes = parse_contract_id(contract_id)?;
        Ok(Self {
            rpc_url: rpc_url.to_owned(),
            contract: contract_id.to_owned(),
            contract_bytes,
            network_passphrase: network_passphrase.to_owned(),
        })
    }

    /// URL del RPC configurada.
    pub fn rpc_url(&self) -> &str {
        &self.rpc_url
    }

    /// Passphrase de red configurada; se usa para el enlace al explorer y para la
    /// verificacion de que el RPC al que se habla es la red que se cree estar usando.
    pub fn network_passphrase(&self) -> &str {
        &self.network_passphrase
    }

    /// Instancia del cliente RPC oficial.
    fn client(&self) -> Result<stellar_rpc_client::Client> {
        stellar_rpc_client::Client::new(&self.rpc_url)
            .map_err(|e| AgentError::Network(format!("no se pudo abrir el cliente RPC: {e}")))
    }
}

impl ChainClient for SorobanChain {
    fn contract_id(&self) -> &str {
        &self.contract
    }

    fn snapshot(&self) -> Result<EscrowSnapshot> {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| AgentError::Network(format!("runtime: {e}")))?;
        rt.block_on(self.snapshot_async())
    }

    fn submit_attestation(
        &self,
        outcome: AttestationOutcome,
        report_hash: &[u8; 32],
    ) -> Result<SubmitOutcome> {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| AgentError::Network(format!("runtime: {e}")))?;
        rt.block_on(self.submit_attestation_async(outcome, report_hash))
    }
}

impl SorobanChain {
    /// Firma y envia `attest`, y confirma por evento, estado y `report_hash`.
    ///
    /// El envio va por [`Self::invoke_and_confirm`], el mismo camino que usa
    /// `finalize()`: simular con `authMode: record`, firmar el payload que la red
    /// espera, enviar y esperar inclusion. Lo que distingue a una atestacion es la
    /// confirmacion posterior, que son tres comprobaciones independientes.
    pub async fn submit_attestation_async(
        &self,
        outcome: AttestationOutcome,
        report_hash: &[u8; 32],
    ) -> Result<SubmitOutcome> {
        let (hash, state_after) = self
            .invoke_and_confirm(crate::attest::ATTEST_FN, |contract, source, seq| {
                crate::attest::build_invoke_tx(contract, source, seq, outcome, report_hash)
            })
            .await?;

        // Tres confirmaciones, y las tres tienen que concordar:
        // 1. el evento `Attested`: que atestiguo esta cuenta y con que reporte.
        //    Un tx en SUCCESS y un estado `Attested*` podrian venir de otra via.
        // 2. el estado final esperado.
        // 3. el `report_hash` que quedo registrado, que es el lazo con el motor.
        let emitted = self.events_of(&hash).await?;
        crate::attest::find_attested_event(&emitted, &self.contract_bytes, outcome, report_hash)?;

        let expected = crate::attest::expected_state_after(outcome);
        let after = self.snapshot_async().await?;
        if after.state != expected || state_after != expected {
            return Err(AgentError::InvalidState {
                actual: after.state.to_string(),
                operacion: format!("confirmar {ATTEST_FN} (tx {hash}, se esperaba {expected})"),
            });
        }
        if after.report_hash != Some(*report_hash) {
            return Err(AgentError::BundleHashMismatch {
                computed: hex::encode(report_hash),
                on_chain: after
                    .report_hash
                    .map(hex::encode)
                    .unwrap_or_else(|| "<ninguno>".into()),
            });
        }
        Ok(SubmitOutcome {
            hash: TxHash(hash.to_string()),
            state_after,
        })
    }

    /// Firma y envia `finalize()`, y devuelve el estado en el que quedo el escrow.
    ///
    /// `finalize()` no exige `auth`: cualquiera puede ejecutarlo y el propio contrato
    /// decide si el plazo vencio. El keeper **no** lleva su propia cuenta de plazos:
    /// duplicar esa aritmetica en el cliente es la forma de liquidar fondos antes de
    /// tiempo, o de quedarse esperando un plazo que el contrato no aplica. Si el
    /// contrato responde `NotFinalizableYet`, ese error sube tal cual para que el
    /// keeper lo distinga de un fallo de red.
    pub async fn finalize_async(&self) -> Result<EscrowState> {
        let (_hash, state_after) = self
            .invoke_and_confirm(crate::attest::FINALIZE_FN, |contract, source, seq| {
                crate::attest::build_finalize_tx(contract, source, seq)
            })
            .await?;
        // La respuesta inmediata se contrasta con una relectura: si no coinciden,
        // alguien toco el contrato entre medias y no se afirma un estado sin fundamento.
        let after = self.snapshot_async().await?;
        if after.state != state_after {
            return Err(AgentError::InvalidState {
                actual: after.state.to_string(),
                operacion: format!(
                    "confirmar {} (la tx dio {state_after}, la cadena muestra {})",
                    crate::attest::FINALIZE_FN,
                    after.state
                ),
            });
        }
        Ok(state_after)
    }

    /// Camino comun de invocacion: simular con `record`, firmar, enviar y esperar.
    ///
    /// `build` recibe contrato, cuenta fuente y numero de secuencia para montar la
    /// transaccion; el resto es identico para cualquier funcion.
    async fn invoke_and_confirm<F>(
        &self,
        fn_name: &str,
        build: F,
    ) -> Result<(xdr::Hash, EscrowState)>
    where
        F: Fn(&xdr::ScAddress, &xdr::MuxedAccount, i64) -> Result<xdr::Transaction>,
    {
        use stellar_rpc_client::AuthMode;

        let signer = EngineSigner::from_env()?;
        let client = self.client()?;
        // Se verifica **el passphrase configurado** contra la red. Comparar el
        // passphrase del RPC consigo mismo no comprueba nada: firmaria con el id de
        // red de otra red y la transaccion caeria con BAD_AUTH, o peor, se creeria
        // estar en testnet estando en otra cosa.
        client
            .verify_network_passphrase(Some(&self.network_passphrase))
            .await
            .map_err(|e| {
                AgentError::Network(format!(
                    "el RPC no es la red esperada (passphrase configurada no coincide): {e}"
                ))
            })?;

        let account = client
            .get_account(&signer.address())
            .await
            .map_err(|e| AgentError::Network(format!("getAccount de la cuenta engine: {e}")))?;
        let source = xdr::MuxedAccount::Ed25519(signer.public_key_bytes().into());
        let contract: xdr::ScAddress = self
            .contract
            .parse::<xdr::ScAddress>()
            .map_err(|e| AgentError::Config(format!("CANGUPA_CONTRACT_ID invalido: {e}")))?;

        let mut tx = build(&contract, &source, account.seq_num.0)?;

        let simulation = client
            .simulate_transaction_envelope(
                &crate::attest::unsigned_envelope(&tx),
                Some(AuthMode::Record),
            )
            .await
            .map_err(|e| AgentError::Network(format!("simulateTransaction({fn_name}): {e}")))?;
        let data = simulation.transaction_data().map_err(|e| {
            AgentError::Network(format!("la simulacion no devolvio sorobanData: {e}"))
        })?;
        // Las credenciales llegan aparte de `sorobanData` y van en la operacion.
        let auth = simulation
            .results()
            .map_err(|e| AgentError::Network(format!("credenciales de la simulacion: {e}")))?
            .into_iter()
            .flat_map(|r| r.auth)
            .collect();
        crate::attest::apply_simulation(&mut tx, data, auth)?;

        let network_id = crate::attest::network_id_hash(&self.network_passphrase);
        let envelope = crate::attest::sign_envelope(&signer, &tx, &network_id, fn_name)?;

        let hash = client
            .send_transaction(&envelope)
            .await
            .map_err(|e| AgentError::Network(format!("sendTransaction({fn_name}): {e}")))?;
        let response = client
            .get_transaction_polling(&hash, None)
            .await
            .map_err(|e| AgentError::Network(format!("getTransaction({hash}): {e}")))?;
        if response.status != TX_SUCCESS {
            return Err(AgentError::Network(format!(
                "la transaccion de {fn_name} no fue exitosa: estado {}",
                response.status
            )));
        }
        let state_after = state_from_simulation(&simulation)?;
        Ok((hash, state_after))
    }

    /// Recupera los eventos de contrato de una transaccion ya incluida.
    async fn events_of(&self, hash: &xdr::Hash) -> Result<Vec<xdr::ContractEvent>> {
        let client = self.client()?;
        let response = client
            .get_transaction(hash)
            .await
            .map_err(|e| AgentError::Network(format!("getTransaction({hash}): {e}")))?;
        if response.status != TX_SUCCESS {
            return Err(AgentError::Network(format!(
                "la transaccion {hash} no fue exitosa: estado {}",
                response.status
            )));
        }
        Ok(response
            .events
            .contract_events
            .iter()
            .flatten()
            .cloned()
            .collect())
    }

    /// Lectura asincrona del estado real del contrato.
    ///
    /// Invoca el getter `snapshot()` por simulacion, que devuelve una sola
    /// `EscrowSnapshot` con todos los campos con nombre. No se lee el
    /// almacenamiento de instancia: ese `ScMap` es privado y su orden depende
    /// de como el contrato declare sus `DataKey`.
    pub async fn snapshot_async(&self) -> Result<EscrowSnapshot> {
        let client = self.client()?;
        let sc = self.decode_snapshot(&client).await?;

        let ledger_info = client
            .get_latest_ledger()
            .await
            .map_err(|e| rpc_error("getLatestLedger", &e))?;
        let ledger_seq = ledger_info.sequence;
        // El getter devuelve su propio `ledger_timestamp`; el de la RPC se usa
        // solo para tener el numero de ledger, que el getter no expone.
        let _close_time = self.latest_ledger_timestamp(&client, ledger_seq).await?;

        Ok(EscrowSnapshot {
            state: sc.state,
            config: sc.config,
            evidence_bundle_hash: sc.evidence_bundle_hash,
            report_hash: sc.report_hash,
            attestation_deadline: sc.attestation_deadline,
            funded_at: sc.funded_at,
            submission_deadline: sc.submission_deadline,
            objection_deadline: sc.objection_deadline,
            correction_deadline: sc.correction_deadline,
            resolution_deadline: sc.resolution_deadline,
            disputed_at: sc.disputed_at,
            correction_attempts: sc.correction_attempts,
            ledger: ledger_seq,
            ledger_timestamp: sc.ledger_timestamp,
        })
    }

    /// Llama a `snapshot()` en el contrato y decodifica el `ScVal` devuelto.
    async fn decode_snapshot(
        &self,
        client: &stellar_rpc_client::Client,
    ) -> Result<ContractSnapshot> {
        let contract: xdr::ScAddress = self
            .contract
            .parse::<xdr::ScAddress>()
            .map_err(|e| AgentError::Config(format!("CANGUPA_CONTRACT_ID invalido: {e}")))?;
        // Se simula con una cuenta de relleno: `snapshot()` es una vista, no pide
        // `auth`, asi que la identidad de la fuente es irrelevante.
        let filler = xdr::MuxedAccount::Ed25519([7u8; 32].into());
        let op = xdr::Operation {
            body: xdr::OperationBody::InvokeHostFunction(xdr::InvokeHostFunctionOp {
                host_function: xdr::HostFunction::InvokeContract(xdr::InvokeContractArgs {
                    contract_address: contract,
                    function_name: SNAPSHOT_FN
                        .try_into()
                        .map_err(|_| AgentError::Xdr("snapshot no es un simbolo valido".into()))?,
                    args: xdr::VecM::default(),
                }),
                auth: xdr::VecM::default(),
            }),
            source_account: None,
        };
        let tx = xdr::Transaction {
            source_account: filler,
            fee: crate::attest::BASE_FEE_STROOPS,
            seq_num: xdr::SequenceNumber(0),
            cond: xdr::Preconditions::default(),
            memo: xdr::Memo::None,
            operations: xdr::VecM::try_from(vec![op])
                .map_err(|e| AgentError::Xdr(format!("operaciones: {e}")))?,
            ext: xdr::TransactionExt::V0,
        };

        let simulation = client
            .simulate_transaction_envelope(&crate::attest::unsigned_envelope(&tx), None)
            .await
            .map_err(|e| AgentError::Network(format!("simulateTransaction({SNAPSHOT_FN}): {e}")))?;
        let result = simulation
            .results()
            .map_err(|e| AgentError::Network(format!("resultado de {SNAPSHOT_FN}: {e}")))?
            .into_iter()
            .next()
            .ok_or_else(|| {
                AgentError::Network(format!("{SNAPSHOT_FN} no devolvio ningun resultado"))
            })?;
        decode_snapshot_scval(&result.xdr)
    }

    /// Timestamp de cierre del ledger mas reciente.
    ///
    /// `getLatestLedger` no incluye la hora de cierre, asi que se lee el header del
    /// ultimo ledger, que es la unica fuente de `ledgerCloseTime` en el RPC.
    async fn latest_ledger_timestamp(
        &self,
        client: &stellar_rpc_client::Client,
        sequence: u32,
    ) -> Result<u64> {
        let response = client
            .get_ledgers(
                stellar_rpc_client::LedgerStart::Ledger(sequence.saturating_sub(1)),
                Some(1),
                None,
            )
            .await
            .map_err(|e| rpc_error("getLedgers", &e))?;
        u64::try_from(response.latest_ledger_close_time).map_err(|_| {
            AgentError::Network(format!(
                "latestLedgerCloseTime negativo: {}",
                response.latest_ledger_close_time
            ))
        })
    }
}

/// Convierte `Err` del cliente RPC en [`AgentError::Network`].
pub(crate) fn rpc_error(op: &str, e: &stellar_rpc_client::Error) -> AgentError {
    AgentError::Network(format!("{op}: {e}"))
}

/// Lee el estado que devuelve una funcion del contrato, tal y como lo reporta la
/// simulacion.
///
/// Se usa como primera referencia y luego se contrasta con una relectura: la
/// simulacion corre en el estado de la red en ese instante, que ya pudo quedar viejo
/// para cuando la transaccion se incluya.
pub(crate) fn state_from_simulation(
    simulation: &stellar_rpc_client::SimulateTransactionResponse,
) -> Result<EscrowState> {
    let result = simulation
        .results()
        .map_err(|e| AgentError::Network(format!("resultado de la simulacion: {e}")))?
        .into_iter()
        .next()
        .ok_or_else(|| AgentError::Network("la simulacion no devolvio resultados".into()))?;
    match &result.xdr {
        ScVal::U32(v) => Ok(EscrowState::from_u32(*v)),
        ScVal::Void => Err(AgentError::Network("la funcion no devolvio estado".into())),
        other => Err(AgentError::Xdr(format!("estado inesperado: {other:?}"))),
    }
}

/// `VecM` de un solo elemento, que es la forma de una lista de argumentos de Soroban.
pub(crate) fn vec_one(val: ScVal) -> Result<xdr::VecM<ScVal>> {
    xdr::VecM::try_from(vec![val])
        .map_err(|e| AgentError::Xdr(format!("no se pudo construir el VecM: {e}")))
}

/// `VecM` a partir de un vector de valores.
pub(crate) fn vec_of(vals: Vec<ScVal>) -> Result<xdr::VecM<ScVal>> {
    xdr::VecM::try_from(vals)
        .map_err(|e| AgentError::Xdr(format!("no se pudo construir el VecM: {e}")))
}

/// Nombre del getter de lectura que consume el agente.
pub const SNAPSHOT_FN: &str = "snapshot";

/// Instantanea tal y como la devuelve el getter `snapshot()` del contrato.
#[derive(Debug, Clone, PartialEq)]
pub struct ContractSnapshot {
    pub state: EscrowState,
    pub config: EscrowConfigView,
    pub evidence_bundle_hash: Option<[u8; 32]>,
    pub report_hash: Option<[u8; 32]>,
    pub funded_at: Option<u64>,
    pub submission_deadline: Option<u64>,
    pub attestation_deadline: Option<u64>,
    pub attested_at: Option<u64>,
    pub objection_deadline: Option<u64>,
    pub correction_deadline: Option<u64>,
    pub disputed_at: Option<u64>,
    pub resolution_deadline: Option<u64>,
    pub correction_attempts: u32,
    pub dispute_reason_hash: Option<[u8; 32]>,
    pub dispute_evidence_hash: Option<[u8; 32]>,
    pub ledger_timestamp: u64,
}

/// Indices de los campos de `EscrowSnapshot` en el `types.rs` del contrato.
///
/// Un `#[contracttype] struct` se serializa como `ScVal::Vec` con los campos en el orden
/// de declaracion. Aqui si hay posiciones, pero **del tipo que el contrato declara y
/// publica**: si el contrato reordena sus campos, el propio contrato y este agente
/// compilan juntos, y el test `wrong_field_count_is_an_explicit_error` mas la validacion
/// de longitud hacen que el desajuste sea un error explicito. Lo que ya no se depende es
/// del almacenamiento privado ni de las `DataKey`.
const F_STATE: usize = 0;
const F_CONFIG: usize = 1;
const F_EVIDENCE_HASH: usize = 2;
const F_REPORT_HASH: usize = 3;
const F_FUNDED_AT: usize = 4;
const F_SUBMISSION_DEADLINE: usize = 5;
const F_ATTESTATION_DEADLINE: usize = 6;
const F_ATTESTED_AT: usize = 7;
const F_OBJECTION_DEADLINE: usize = 8;
const F_CORRECTION_DEADLINE: usize = 9;
const F_DISPUTED_AT: usize = 10;
const F_RESOLUTION_DEADLINE: usize = 11;
const F_CORRECTION_ATTEMPTS: usize = 12;
const F_DISPUTE_REASON_HASH: usize = 13;
const F_DISPUTE_EVIDENCE_HASH: usize = 14;
const F_LEDGER_TIMESTAMP: usize = 15;
/// Cantidad de campos de `EscrowSnapshot`; si cambia, la decodificacion debe cambiar.
const SNAPSHOT_FIELDS: usize = 16;

/// Decodifica el `ScVal` que devuelve `snapshot()`.
///
/// Cada campo se valida: un tipo inesperado es un error explicito, nunca un valor por
/// defecto silencioso. `EscrowState::Unknown` si se conserva porque un enum nuevo en el
/// contrato debe verse como desconocido y no confundirse con `Created`.
pub fn decode_snapshot_scval(val: &ScVal) -> Result<ContractSnapshot> {
    let ScVal::Vec(Some(items)) = val else {
        return Err(AgentError::Xdr(format!(
            "snapshot() deberia devolver un Vec, recibio {val:?}"
        )));
    };
    if items.len() != SNAPSHOT_FIELDS {
        return Err(AgentError::Xdr(format!(
            "snapshot() devolvio {} campos y el agente espera {SNAPSHOT_FIELDS}; \
             el contrato cambio y el agente debe actualizarse",
            items.len()
        )));
    }
    let get = |i: usize| -> &ScVal { &items[i] };

    let out = ContractSnapshot {
        state: match get(F_STATE) {
            ScVal::U32(v) => EscrowState::from_u32(*v),
            other => return Err(AgentError::Xdr(format!("state no es U32: {other:?}"))),
        },
        config: decode_config(get(F_CONFIG))?,
        evidence_bundle_hash: decode_bytes32_opt(get(F_EVIDENCE_HASH), "evidence_bundle_hash")?,
        report_hash: decode_bytes32_opt(get(F_REPORT_HASH), "report_hash")?,
        funded_at: decode_u64_opt(get(F_FUNDED_AT), "funded_at")?,
        submission_deadline: decode_u64_opt(get(F_SUBMISSION_DEADLINE), "submission_deadline")?,
        attestation_deadline: decode_u64_opt(get(F_ATTESTATION_DEADLINE), "attestation_deadline")?,
        attested_at: decode_u64_opt(get(F_ATTESTED_AT), "attested_at")?,
        objection_deadline: decode_u64_opt(get(F_OBJECTION_DEADLINE), "objection_deadline")?,
        correction_deadline: decode_u64_opt(get(F_CORRECTION_DEADLINE), "correction_deadline")?,
        disputed_at: decode_u64_opt(get(F_DISPUTED_AT), "disputed_at")?,
        resolution_deadline: decode_u64_opt(get(F_RESOLUTION_DEADLINE), "resolution_deadline")?,
        correction_attempts: match get(F_CORRECTION_ATTEMPTS) {
            ScVal::U32(v) => *v,
            other => {
                return Err(AgentError::Xdr(format!(
                    "correction_attempts no es U32: {other:?}"
                )))
            }
        },
        dispute_reason_hash: decode_bytes32_opt(get(F_DISPUTE_REASON_HASH), "dispute_reason_hash")?,
        dispute_evidence_hash: decode_bytes32_opt(
            get(F_DISPUTE_EVIDENCE_HASH),
            "dispute_evidence_hash",
        )?,
        ledger_timestamp: match get(F_LEDGER_TIMESTAMP) {
            ScVal::U64(v) => *v,
            other => {
                return Err(AgentError::Xdr(format!(
                    "ledger_timestamp no es U64: {other:?}"
                )))
            }
        },
    };

    // Invariante del contrato: `attest()` solo avanza a `Attested*` y un escrow aun
    // atestetable no puede tener `report_hash`. Si se cumple al reves, la lectura no
    // viene de un contrato P0-07 y el motor no debe decidir sobre ella.
    if out.state == EscrowState::EvidenceSubmitted && out.report_hash.is_some() {
        return Err(AgentError::Network(format!(
            "lectura inconsistente: EvidenceSubmitted con report_hash {:?}",
            out.report_hash.map(hex::encode)
        )));
    }
    Ok(out)
}

/// Cantidad de campos de `EscrowConfig`; si cambia, hay que revisar los indices de abajo.
const CONFIG_FIELDS: usize = 14;

/// Decodifica el `EscrowConfig` embebido.
///
/// Indices segun `EscrowConfig` en el `types.rs` del contrato.
fn decode_config(val: &ScVal) -> Result<EscrowConfigView> {
    let ScVal::Vec(Some(items)) = val else {
        return Err(AgentError::Xdr(format!("config no es Vec: {val:?}")));
    };
    // Se exige la longitud completa aunque solo se lean seis campos: un Config
    // truncado significa que el contrato cambio, y aceptarlo seria decidir sobre
    // una lectura que no se sabe que es.
    if items.len() != CONFIG_FIELDS {
        return Err(AgentError::Xdr(format!(
            "config devolvio {} campos y el agente espera {CONFIG_FIELDS}",
            items.len()
        )));
    }
    let get = |i: usize| -> Result<&ScVal> {
        items
            .get(i)
            .ok_or_else(|| AgentError::Xdr(format!("config incompleto: falta el campo {i}")))
    };
    let as_address = |i: usize| -> Result<String> {
        match get(i)? {
            ScVal::Address(a) => Ok(sc_address_to_str(a)),
            other => Err(AgentError::Xdr(format!(
                "config.{i} no es Address: {other:?}"
            ))),
        }
    };
    let as_i128 = |i: usize| -> Result<i128> {
        match get(i)? {
            ScVal::I128(v) => Ok(((v.hi as i128) << 64) | (v.lo as i128)),
            other => Err(AgentError::Xdr(format!("config.{i} no es I128: {other:?}"))),
        }
    };
    let as_u64 = |i: usize| -> Result<u64> {
        match get(i)? {
            ScVal::U64(v) => Ok(*v),
            other => Err(AgentError::Xdr(format!("config.{i} no es U64: {other:?}"))),
        }
    };
    let as_u32 = |i: usize| -> Result<u32> {
        match get(i)? {
            ScVal::U32(v) => Ok(*v),
            other => Err(AgentError::Xdr(format!("config.{i} no es U32: {other:?}"))),
        }
    };

    // buyer, supplier, engine, resolver, token, amount, submission_period,
    // attestation_period, objection_period, correction_period, resolution_period,
    // fallback_outcome, fallback_split_bps, max_correction_attempts.
    Ok(EscrowConfigView {
        engine: as_address(2)?,
        token: as_address(4)?,
        amount: as_i128(5)?,
        attestation_period: as_u64(7)?,
        fallback_outcome: as_u32(11)?,
        fallback_split_bps: as_u32(12)?,
    })
}

/// Lee un `Option<BytesN<32>>`, que Soroban codifica como `Void` o `Bytes`.
fn decode_bytes32_opt(val: &ScVal, name: &str) -> Result<Option<[u8; 32]>> {
    match val {
        ScVal::Void => Ok(None),
        ScVal::Bytes(b) => {
            let slice: &[u8] = b.0.as_ref();
            if slice.len() != 32 {
                return Err(AgentError::Xdr(format!(
                    "{name} deberia tener 32 bytes y tiene {}",
                    slice.len()
                )));
            }
            let mut out = [0u8; 32];
            out.copy_from_slice(slice);
            Ok(Some(out))
        }
        other => Err(AgentError::Xdr(format!(
            "{name} no es Bytes ni Void: {other:?}"
        ))),
    }
}

/// Lee un `Option<u64>`, que Soroban codifica como `Void` o `U64`.
fn decode_u64_opt(val: &ScVal, name: &str) -> Result<Option<u64>> {
    match val {
        ScVal::Void => Ok(None),
        ScVal::U64(v) => Ok(Some(*v)),
        other => Err(AgentError::Xdr(format!(
            "{name} no es U64 ni Void: {other:?}"
        ))),
    }
}

/// Codifica un `ScAddress` a strkey `G...` o `C...`.
pub fn sc_address_to_str(addr: &xdr::ScAddress) -> String {
    match addr {
        xdr::ScAddress::Account(a) => match &a.0 {
            xdr::PublicKey::PublicKeyTypeEd25519(u) => stellar_strkey::ed25519::PublicKey(u.0)
                .to_string()
                .to_string(),
        },
        xdr::ScAddress::Contract(c) => stellar_strkey::Contract(c.0 .0).to_string().to_string(),
        // Un escrow no usa estas variantes; se representations textual para no perder informacion.
        other => format!("{other:?}"),
    }
}

/// Convierte un id de contrato strkey `C...` a sus 32 bytes.
pub fn parse_contract_id(id: &str) -> Result<[u8; 32]> {
    id.parse::<stellar_strkey::Contract>()
        .map(|c| c.0)
        .map_err(|e| AgentError::Config(format!("contract id invalido '{id}': {e}")))
}

/// `VecM` reexportado para no repetir la ruta del crate en el resto del agente.
pub use stellar_xdr::VecM;

/// Envuelve un envelope en XDR base64, util para depurar y para el log del keeper.
pub fn envelope_to_base64(envelope: &TransactionEnvelope) -> Result<String> {
    envelope
        .to_xdr_base64(Limits::none())
        .map_err(|e| AgentError::Xdr(format!("TransactionEnvelope: {e}")))
}

/// Cliente compartido por el CLI y el keeper.
pub type SharedChain = Arc<dyn ChainClient>;

#[cfg(test)]
mod tests {
    use super::*;

    /// Direccion de cuenta valida en XDR, para construir el `EscrowConfig` de prueba.
    fn account(seed: u8) -> ScVal {
        ScVal::Address(xdr::ScAddress::Account(xdr::AccountId(
            xdr::PublicKey::PublicKeyTypeEd25519(xdr::Uint256([seed; 32])),
        )))
    }

    fn contract(seed: u8) -> ScVal {
        ScVal::Address(xdr::ScAddress::Contract(xdr::ContractId(xdr::Hash(
            [seed; 32],
        ))))
    }

    fn bytes32(raw: [u8; 32]) -> ScVal {
        ScVal::Bytes(xdr::ScBytes(xdr::BytesM::try_from(raw.to_vec()).unwrap()))
    }

    fn u64v(v: u64) -> ScVal {
        ScVal::U64(v)
    }

    /// `VecM` de test: no es mutable, asi que se reconstruye desde un `Vec`.
    fn vec_of(vals: Vec<ScVal>) -> ScVal {
        ScVal::Vec(Some(xdr::ScVec(xdr::VecM::try_from(vals).unwrap())))
    }

    /// Desarma un `ScVal::Vec` a `Vec<ScVal>` para poder alterarlo en un test.
    fn as_vec(val: &ScVal) -> Vec<ScVal> {
        let ScVal::Vec(Some(items)) = val else {
            panic!("se esperaba ScVal::Vec")
        };
        items.0.to_vec()
    }

    /// `EscrowConfig` completo, con los 14 campos en el orden de `types.rs`.
    fn config_vec() -> ScVal {
        vec_of(vec![
            account(1),  // 0 buyer
            account(2),  // 1 supplier
            account(3),  // 2 engine
            account(4),  // 3 resolver
            contract(5), // 4 token
            ScVal::I128(xdr::Int128Parts {
                hi: 0,
                lo: 1_000_000,
            }), // 5 amount
            u64v(1_000), // 6 submission_period
            u64v(600),   // 7 attestation_period
            u64v(1_200), // 8 objection_period
            u64v(1_800), // 9 correction_period
            u64v(2_400), // 10 resolution_period
            ScVal::U32(2), // 11 fallback_outcome = Refund
            ScVal::U32(0), // 12 fallback_split_bps
            ScVal::U32(1), // 13 max_correction_attempts
        ])
    }

    /// `EscrowSnapshot` completo, con los 16 campos en el orden de `types.rs`.
    fn snapshot_val(
        state: u32,
        evidence: Option<[u8; 32]>,
        report: Option<[u8; 32]>,
        correction_attempts: u32,
    ) -> ScVal {
        let opt_hash = |h: Option<[u8; 32]>| match h {
            Some(raw) => bytes32(raw),
            None => ScVal::Void,
        };
        let opt_u64 = |v: Option<u64>| match v {
            Some(n) => u64v(n),
            None => ScVal::Void,
        };
        vec_of(vec![
            ScVal::U32(state),               // 0 state
            config_vec(),                    // 1 config
            opt_hash(evidence),              // 2 evidence_bundle_hash
            opt_hash(report),                // 3 report_hash
            opt_u64(Some(1_000)),            // 4 funded_at
            opt_u64(Some(2_000)),            // 5 submission_deadline
            opt_u64(Some(3_000)),            // 6 attestation_deadline
            opt_u64(None),                   // 7 attested_at
            opt_u64(None),                   // 8 objection_deadline
            opt_u64(None),                   // 9 correction_deadline
            opt_u64(None),                   // 10 disputed_at
            opt_u64(None),                   // 11 resolution_deadline
            ScVal::U32(correction_attempts), // 12 correction_attempts
            opt_hash(None),                  // 13 dispute_reason_hash
            opt_hash(None),                  // 14 dispute_evidence_hash
            u64v(2_500),                     // 15 ledger_timestamp
        ])
    }

    #[test]
    fn decodes_state_and_config_by_name_position() {
        let sc = decode_snapshot_scval(&snapshot_val(2, Some([1u8; 32]), None, 0)).unwrap();
        assert_eq!(sc.state, EscrowState::EvidenceSubmitted);
        assert_eq!(sc.config.amount, 1_000_000);
        assert_eq!(sc.config.attestation_period, 600);
        assert_eq!(sc.config.fallback_outcome, 2);
        assert_eq!(sc.evidence_bundle_hash, Some([1u8; 32]));
        assert_eq!(sc.ledger_timestamp, 2_500);
    }

    #[test]
    fn decodes_optionals_as_void() {
        let sc = decode_snapshot_scval(&snapshot_val(2, Some([1u8; 32]), None, 0)).unwrap();
        // Un escrow atestectable todavia no tiene report_hash ni plazos derivados.
        assert_eq!(sc.report_hash, None);
        assert_eq!(sc.attested_at, None);
        assert_eq!(sc.objection_deadline, None);
    }

    #[test]
    fn wrong_field_count_is_an_explicit_error() {
        // Un contrato con un campo mas debe fallar con un mensaje que diga que
        // hay que actualizar el agente, no leerse como si nada.
        let mut items = as_vec(&snapshot_val(2, Some([1u8; 32]), None, 0));
        items.push(ScVal::U32(0));
        let err = decode_snapshot_scval(&vec_of(items)).unwrap_err();
        let msg = format!("{err}");
        assert!(msg.contains("17"), "esperaba contar los campos, dio: {msg}");
        assert!(
            msg.contains("actualizarse"),
            "esperaba pedir actualizacion: {msg}"
        );
    }

    #[test]
    fn wrong_type_is_not_silently_zero() {
        let mut items = as_vec(&snapshot_val(2, Some([1u8; 32]), None, 0));
        items[F_CORRECTION_ATTEMPTS] = ScVal::U64(1);
        let err = decode_snapshot_scval(&vec_of(items)).unwrap_err();
        assert!(format!("{err}").contains("correction_attempts"));
    }

    #[test]
    fn attestable_with_report_hash_is_rejected_as_inconsistent() {
        // EvidenceSubmitted + report_hash no lo puede producir el contrato P0-07:
        // es una lectura de una version distinta, y el motor no debe decidir sobre ella.
        let err = decode_snapshot_scval(&snapshot_val(2, Some([1u8; 32]), Some([9u8; 32]), 0))
            .unwrap_err();
        assert!(format!("{err}").contains("inconsistente"));
    }

    #[test]
    fn attested_pass_decodes_with_report_hash() {
        let sc =
            decode_snapshot_scval(&snapshot_val(3, Some([1u8; 32]), Some([9u8; 32]), 0)).unwrap();
        assert_eq!(sc.state, EscrowState::AttestedPass);
        assert_eq!(sc.report_hash, Some([9u8; 32]));
    }

    #[test]
    fn correction_attempt_is_visible_to_the_engine() {
        let sc = decode_snapshot_scval(&snapshot_val(2, Some([2u8; 32]), None, 1)).unwrap();
        assert_eq!(sc.correction_attempts, 1);
    }

    #[test]
    fn unknown_state_survives_decoding() {
        let sc = decode_snapshot_scval(&snapshot_val(42, None, None, 0)).unwrap();
        assert_eq!(sc.state, EscrowState::Unknown(42));
        assert_eq!(sc.state.to_string(), "Unknown(42)");
    }

    #[test]
    fn non_vec_is_rejected() {
        assert!(decode_snapshot_scval(&ScVal::U32(2)).is_err());
    }

    #[test]
    fn config_with_missing_field_is_rejected() {
        let mut items = as_vec(&snapshot_val(2, Some([1u8; 32]), None, 0));
        let mut cfg = as_vec(&items[F_CONFIG]);
        cfg.pop();
        items[F_CONFIG] = vec_of(cfg);
        let err = decode_snapshot_scval(&vec_of(items)).unwrap_err();
        assert!(format!("{err}").contains("config"));
    }

    fn snap(state: EscrowState) -> EscrowSnapshot {
        EscrowSnapshot {
            state,
            config: EscrowConfigView {
                engine: "G".into(),
                token: "C".into(),
                amount: 1,
                attestation_period: 60,
                fallback_outcome: 2,
                fallback_split_bps: 0,
            },
            evidence_bundle_hash: Some([1u8; 32]),
            report_hash: None,
            attestation_deadline: Some(1_000),
            funded_at: Some(100),
            submission_deadline: Some(200),
            objection_deadline: None,
            correction_deadline: None,
            resolution_deadline: None,
            disputed_at: None,
            correction_attempts: 0,
            ledger: 5,
            ledger_timestamp: 900,
        }
    }

    #[test]
    fn snapshot_reports_attestability() {
        let s = snap(EscrowState::EvidenceSubmitted);
        assert!(s.is_attestable());
        assert_eq!(s.attestation_remaining(), Some(100));
        assert!(!s.is_correction());
    }

    #[test]
    fn expired_deadline_is_reported_as_remaining_zero_or_less() {
        let mut s = snap(EscrowState::EvidenceSubmitted);
        s.ledger_timestamp = 1_000;
        assert_eq!(s.attestation_remaining(), Some(0));
        s.ledger_timestamp = 1_001;
        assert_eq!(s.attestation_remaining(), Some(-1));
    }

    #[test]
    fn correction_is_detected_from_attempts() {
        let mut s = snap(EscrowState::EvidenceSubmitted);
        s.correction_attempts = 1;
        assert!(s.is_correction());
    }

    #[test]
    fn contract_id_must_be_valid_strkey() {
        // CRC y version verificados a mano; un id mal escrito debe fallar al
        // construir, no a mitad de una atestacion.
        assert!(
            parse_contract_id("CA3D5KRYM6CB7OWQ6TWYRR3Z4T7GNZLKERYNZGGA5SOAOPIFY6YQGAXE").is_ok()
        );
        assert!(parse_contract_id("no-es-un-contract-id").is_err());
        // Version 6 (ed25519) no es un id de contrato, aunque tenga longuitud correcta.
        assert!(
            parse_contract_id("GDLVVGAB5ZWOW6NOZUHRBCEQI54OAJ5GJLDIKDDUAP6B5HDGXIEFDOAM").is_err()
        );
    }

    #[test]
    fn to_json_exposes_the_reading_contract() {
        let json = snap(EscrowState::EvidenceSubmitted).to_json();
        assert_eq!(json["state"], "EvidenceSubmitted");
        assert_eq!(json["amount"], 1);
        assert_eq!(json["attestation_remaining"], 100);
        assert!(json["report_hash"].is_null());
    }
}
