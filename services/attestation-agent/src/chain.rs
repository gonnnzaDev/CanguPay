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
            "report_hash": self.report_hash.map(hex::encode),
            "attestation_deadline": self.attestation_deadline,
            "attestation_remaining": self.attestation_remaining(),
            "correction_attempts": self.correction_attempts,
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
    /// Nombre del enum tal y como lo codifica Soroban.
    pub fn name(self) -> &'static str {
        match self {
            Self::Pass => "Pass",
            Self::Fail => "Fail",
        }
    }

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

    fn finalize(&self) -> Result<EscrowState> {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| AgentError::Network(format!("runtime: {e}")))?;
        rt.block_on(self.finalize_async())
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
        // `attest()` no devuelve nada: la confirmacion es la relectura del contrato.
        let (hash, _) = self
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
        if after.state != expected {
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
            state_after: expected,
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
        let (_hash, returned) = self
            .invoke_and_confirm(crate::attest::FINALIZE_FN, |contract, source, seq| {
                crate::attest::build_finalize_tx(contract, source, seq)
            })
            .await?;
        let state_after = returned
            .ok_or_else(|| AgentError::Xdr("finalize() no devolvio el estado resultante".into()))?;
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
    ) -> Result<(xdr::Hash, Option<EscrowState>)>
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
        check_simulation_error(&simulation, fn_name)?;
        let data = simulation.transaction_data().map_err(|e| {
            AgentError::Network(format!(
                "la simulacion de {fn_name} no devolvio sorobanData: {e}"
            ))
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

        // El RPC sirve la entrada de cuenta con una secuencia anterior a la que
        // espera la red, asi que la transaccion se rechaza con `TxBadSeq` aunque la
        // firma sea correcta. Se relee la cuenta y se reintenta una vez con la
        // siguiente; el error de la red se conserva si tambien asi falla.
        let hash = match client.send_transaction(&envelope).await {
            Ok(h) => h,
            Err(first) if format!("{first}").contains("TxBadSeq") => {
                let fresh = client
                    .get_account(&signer.address())
                    .await
                    .map_err(|e| AgentError::Network(format!("{fn_name}: relectura: {e}")))?;
                let mut retry = tx.clone();
                retry.seq_num = xdr::SequenceNumber(fresh.seq_num.0 + 1);
                client
                    .send_transaction(&crate::attest::sign_envelope(
                        &signer,
                        &retry,
                        &network_id,
                        fn_name,
                    )?)
                    .await
                    .map_err(|second| {
                        let dump = crate::attest::sign_envelope(&signer, &retry, &network_id, fn_name)
                            .ok()
                            .and_then(|e| envelope_to_base64(&e).ok())
                            .unwrap_or_default();
                        AgentError::Network(format!(
                            "sendTransaction({fn_name}): {second} (tras reintentar con seq {}) | ENV={dump}",
                            fresh.seq_num.0 + 1
                        ))
                    })?
            }
            Err(e) => {
                return Err(AgentError::Network(format!(
                    "sendTransaction({fn_name}): {e}"
                )))
            }
        };
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
        let returned = state_from_simulation(&simulation)?;
        Ok((hash, returned))
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
        check_simulation_error(&simulation, SNAPSHOT_FN)?;
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

/// Convierte un error de simulacion del RPC en un [`AgentError`] legible.
///
/// El RPC responde `200 OK` con un campo `error` cuando la simulacion falla, y sin
/// `sorobanData`. Si eso no se comprueba, el error real se pierde y el sintoma aparece
/// como "no devolvio resultados", que no dice nada. Este fallo ya costo una hora de
/// diagnostico, asi que el error de la red se propaga tal cual.
pub(crate) fn check_simulation_error(
    simulation: &stellar_rpc_client::SimulateTransactionResponse,
    fn_name: &str,
) -> Result<()> {
    match &simulation.error {
        Some(err) => Err(AgentError::Network(format!(
            "la simulacion de {fn_name} fallo en la red: {err}"
        ))),
        None => Ok(()),
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
) -> Result<Option<EscrowState>> {
    let result = simulation
        .results()
        .map_err(|e| AgentError::Network(format!("resultado de la simulacion: {e}")))?
        .into_iter()
        .next()
        .ok_or_else(|| AgentError::Network("la simulacion no devolvio resultados".into()))?;
    match &result.xdr {
        // Una funcion que no devuelve nada es valido; quien llama decide si lo esperaba.
        ScVal::Void => Ok(None),
        // El enum vuelve por nombre, igual que cuando se manda por argumento.
        other => decode_state(other).map(Some),
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

/// Lee el `ScVal` que devuelve `snapshot()`.
///
/// El contrato devuelve sus `#[contracttype] struct` como un `Map` **con claves por
/// nombre**, no como un `Vec` posicional. Leerlos por nombre es lo correcto: si el
/// contrato reordena o inserta un campo, esta decodificacion sigue siendo valida y un
/// `Vec` habria shiftsado todos los valores siguientes en silencio.
pub fn decode_snapshot_scval(val: &ScVal) -> Result<ContractSnapshot> {
    let map = as_named_map(val, "snapshot()")?;
    let config = match map.get("config") {
        Some(v) => decode_config(v)?,
        None => return Err(AgentError::Network("snapshot() sin config".into())),
    };
    let state = match map.get("state") {
        Some(v) => decode_state(v)?,
        None => return Err(AgentError::Network("snapshot() sin state".into())),
    };

    let out = ContractSnapshot {
        state,
        config,
        evidence_bundle_hash: opt_hash(&map, "evidence_bundle_hash")?,
        report_hash: opt_hash(&map, "report_hash")?,
        funded_at: opt_u64(&map, "funded_at")?,
        submission_deadline: opt_u64(&map, "submission_deadline")?,
        attestation_deadline: opt_u64(&map, "attestation_deadline")?,
        attested_at: opt_u64(&map, "attested_at")?,
        objection_deadline: opt_u64(&map, "objection_deadline")?,
        correction_deadline: opt_u64(&map, "correction_deadline")?,
        disputed_at: opt_u64(&map, "disputed_at")?,
        resolution_deadline: opt_u64(&map, "resolution_deadline")?,
        correction_attempts: match map.get("correction_attempts") {
            Some(ScVal::U32(v)) => *v,
            Some(other) => {
                return Err(AgentError::Xdr(format!(
                    "correction_attempts no es U32: {other:?}"
                )))
            }
            None => {
                return Err(AgentError::Network(
                    "snapshot() sin correction_attempts".into(),
                ))
            }
        },
        dispute_reason_hash: opt_hash(&map, "dispute_reason_hash")?,
        dispute_evidence_hash: opt_hash(&map, "dispute_evidence_hash")?,
        ledger_timestamp: match map.get("ledger_timestamp") {
            Some(ScVal::U64(v)) => *v,
            Some(other) => {
                return Err(AgentError::Xdr(format!(
                    "ledger_timestamp no es U64: {other:?}"
                )))
            }
            None => {
                return Err(AgentError::Network(
                    "snapshot() sin ledger_timestamp".into(),
                ))
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

/// Decodifica el estado, que Soroban representa como `Vec([Symbol(nombre)])`.
///
/// Se acepta tambien el `U32` con el indice, que es como lo serializa el enum cuando
/// se pasa por argumento. Un enum nuevo del contrato se ve como `Unknown`, nunca como
/// un estado conocido por parecido.
fn decode_state(val: &ScVal) -> Result<EscrowState> {
    match val {
        ScVal::U32(v) => Ok(EscrowState::from_u32(*v)),
        ScVal::Vec(Some(items)) => {
            let Some(ScVal::Symbol(name)) = items.first() else {
                return Err(AgentError::Xdr(format!(
                    "estado no empieza por Symbol: {val:?}"
                )));
            };
            let name = std::str::from_utf8(name.0.as_slice())
                .map_err(|_| AgentError::Xdr("nombre de estado no utf-8".into()))?;
            Ok(match name {
                "Created" => EscrowState::Created,
                "Funded" => EscrowState::Funded,
                "EvidenceSubmitted" => EscrowState::EvidenceSubmitted,
                "AttestedPass" => EscrowState::AttestedPass,
                "AttestedFail" => EscrowState::AttestedFail,
                "Released" => EscrowState::Released,
                "Cancelled" => EscrowState::Cancelled,
                "Refunded" => EscrowState::Refunded,
                "Disputed" => EscrowState::Disputed,
                "Split" => EscrowState::Split,
                // Un estado que el agente no conoce no se adivina: se conserva el
                // texto para que el log diga la verdad.
                _ => EscrowState::Unknown(u32::MAX),
            })
        }
        other => Err(AgentError::Xdr(format!("estado inesperado: {other:?}"))),
    }
}

/// Interpreta un `ScVal` como `Map` con claves `Symbol` y lo expone por nombre.
fn as_named_map<'a>(val: &'a ScVal, what: &str) -> Result<NamedMap<'a>> {
    let ScVal::Map(Some(entries)) = val else {
        return Err(AgentError::Xdr(format!(
            "{what} deberia devolver un Map, recibio {val:?}"
        )));
    };
    let mut out = Vec::with_capacity(entries.0.len());
    for entry in entries.0.iter() {
        let ScVal::Symbol(name) = &entry.key else {
            return Err(AgentError::Xdr(format!("clave no Symbol: {:?}", entry.key)));
        };
        let name = std::str::from_utf8(name.0.as_slice())
            .map_err(|_| AgentError::Xdr("clave de mapa no utf-8".into()))?
            .to_owned();
        out.push((name, &entry.val));
    }
    Ok(NamedMap(out))
}

/// Mapa con busqueda por nombre, tal y como lo devuelve Soroban.
struct NamedMap<'a>(Vec<(String, &'a ScVal)>);

impl<'a> NamedMap<'a> {
    fn get(&self, name: &str) -> Option<&'a ScVal> {
        self.0.iter().find(|(k, _)| k == name).map(|(_, v)| *v)
    }
}

/// Decodifica el `EscrowConfig` embebido, tambien por nombre.
fn decode_config(val: &ScVal) -> Result<EscrowConfigView> {
    let map = as_named_map(val, "config")?;
    Ok(EscrowConfigView {
        engine: field_address(&map, "engine")?,
        token: field_address(&map, "token")?,
        amount: field_i128(&map, "amount")?,
        attestation_period: field_u64(&map, "attestation_period")?,
        fallback_outcome: field_u32(&map, "fallback_outcome")?,
        fallback_split_bps: field_u32(&map, "fallback_split_bps")?,
    })
}

fn field_address(map: &NamedMap<'_>, name: &str) -> Result<String> {
    match map.get(name) {
        Some(ScVal::Address(a)) => Ok(sc_address_to_str(a)),
        Some(other) => Err(AgentError::Xdr(format!("{name} no es Address: {other:?}"))),
        None => Err(AgentError::Xdr(format!("falta el campo {name}"))),
    }
}

fn field_i128(map: &NamedMap<'_>, name: &str) -> Result<i128> {
    match map.get(name) {
        Some(ScVal::I128(v)) => Ok(((v.hi as i128) << 64) | (v.lo as i128)),
        Some(other) => Err(AgentError::Xdr(format!("{name} no es I128: {other:?}"))),
        None => Err(AgentError::Xdr(format!("falta el campo {name}"))),
    }
}

fn field_u64(map: &NamedMap<'_>, name: &str) -> Result<u64> {
    match map.get(name) {
        Some(ScVal::U64(v)) => Ok(*v),
        Some(other) => Err(AgentError::Xdr(format!("{name} no es U64: {other:?}"))),
        None => Err(AgentError::Xdr(format!("falta el campo {name}"))),
    }
}

fn field_u32(map: &NamedMap<'_>, name: &str) -> Result<u32> {
    match map.get(name) {
        Some(ScVal::U32(v)) => Ok(*v),
        Some(other) => Err(AgentError::Xdr(format!("{name} no es U32: {other:?}"))),
        None => Err(AgentError::Xdr(format!("falta el campo {name}"))),
    }
}

/// Lee un `Option<BytesN<32>>`, que Soroban codifica como `Void` o `Bytes`.
fn opt_hash(map: &NamedMap<'_>, name: &str) -> Result<Option<[u8; 32]>> {
    match map.get(name) {
        None | Some(ScVal::Void) => Ok(None),
        Some(ScVal::Bytes(b)) => {
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
        Some(other) => Err(AgentError::Xdr(format!(
            "{name} no es Bytes ni Void: {other:?}"
        ))),
    }
}

/// Lee un `Option<u64>`, que Soroban codifica como `Void` o `U64`.
fn opt_u64(map: &NamedMap<'_>, name: &str) -> Result<Option<u64>> {
    match map.get(name) {
        None | Some(ScVal::Void) => Ok(None),
        Some(ScVal::U64(v)) => Ok(Some(*v)),
        Some(other) => Err(AgentError::Xdr(format!(
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

    /// `Map` con claves `Symbol`, la forma en que Soroban devuelve los structs.
    fn named(entries: Vec<(&str, ScVal)>) -> ScVal {
        let pairs = entries
            .into_iter()
            .map(|(k, v)| (ScVal::Symbol(k.try_into().expect("clave valida")), v))
            .collect::<Vec<_>>();
        ScVal::Map(Some(
            xdr::ScMap::sorted_from_pairs(pairs.into_iter()).expect("mapa ordenado"),
        ))
    }

    /// Estado como lo devuelve el contrato: `Vec([Symbol(nombre)])`.
    fn state_val(name: &str) -> ScVal {
        ScVal::Vec(Some(xdr::ScVec(
            xdr::VecM::try_from(vec![ScVal::Symbol(name.try_into().unwrap())]).unwrap(),
        )))
    }

    fn config_map() -> ScVal {
        named(vec![
            ("buyer", account(1)),
            ("supplier", account(2)),
            ("engine", account(3)),
            ("resolver", account(4)),
            ("token", contract(5)),
            (
                "amount",
                ScVal::I128(xdr::Int128Parts {
                    hi: 0,
                    lo: 1_000_000,
                }),
            ),
            ("submission_period", ScVal::U64(1_000)),
            ("attestation_period", ScVal::U64(600)),
            ("objection_period", ScVal::U64(1_200)),
            ("correction_period", ScVal::U64(1_800)),
            ("resolution_period", ScVal::U64(2_400)),
            ("fallback_outcome", ScVal::U32(2)),
            ("fallback_split_bps", ScVal::U32(0)),
            ("max_correction_attempts", ScVal::U32(1)),
        ])
    }

    /// `snapshot()` completo con los 16 campos, por nombre.
    fn snapshot_val(
        state: &str,
        evidence: Option<[u8; 32]>,
        report: Option<[u8; 32]>,
        correction_attempts: u32,
    ) -> ScVal {
        let opt_hash = |h: Option<[u8; 32]>| match h {
            Some(raw) => bytes32(raw),
            None => ScVal::Void,
        };
        named(vec![
            ("state", state_val(state)),
            ("config", config_map()),
            ("evidence_bundle_hash", opt_hash(evidence)),
            ("report_hash", opt_hash(report)),
            ("funded_at", ScVal::U64(1_000)),
            ("submission_deadline", ScVal::U64(2_000)),
            ("attestation_deadline", ScVal::U64(3_000)),
            ("attested_at", ScVal::Void),
            ("objection_deadline", ScVal::Void),
            ("correction_deadline", ScVal::Void),
            ("disputed_at", ScVal::Void),
            ("resolution_deadline", ScVal::Void),
            ("correction_attempts", ScVal::U32(correction_attempts)),
            ("dispute_reason_hash", ScVal::Void),
            ("dispute_evidence_hash", ScVal::Void),
            ("ledger_timestamp", ScVal::U64(2_500)),
        ])
    }

    #[test]
    fn decodes_state_and_config_by_name() {
        let sc =
            decode_snapshot_scval(&snapshot_val("EvidenceSubmitted", Some([1u8; 32]), None, 0))
                .unwrap();
        assert_eq!(sc.state, EscrowState::EvidenceSubmitted);
        assert_eq!(sc.config.amount, 1_000_000);
        assert_eq!(sc.config.attestation_period, 600);
        assert_eq!(sc.config.fallback_outcome, 2);
        assert_eq!(sc.evidence_bundle_hash, Some([1u8; 32]));
        assert_eq!(sc.ledger_timestamp, 2_500);
    }

    #[test]
    fn decodes_every_state_by_its_name() {
        // El enum llega como `Vec[Symbol]`, no como el indice numerico que se usa
        // cuando se pasa por argumento. Los diez estados deben reconocerse.
        for (name, expected) in [
            ("Created", EscrowState::Created),
            ("Funded", EscrowState::Funded),
            ("EvidenceSubmitted", EscrowState::EvidenceSubmitted),
            ("AttestedPass", EscrowState::AttestedPass),
            ("AttestedFail", EscrowState::AttestedFail),
            ("Released", EscrowState::Released),
            ("Cancelled", EscrowState::Cancelled),
            ("Refunded", EscrowState::Refunded),
            ("Disputed", EscrowState::Disputed),
            ("Split", EscrowState::Split),
        ] {
            let sc = decode_snapshot_scval(&snapshot_val(
                name,
                Some([1u8; 32]),
                if matches!(expected, EscrowState::AttestedPass) {
                    Some([9u8; 32])
                } else {
                    None
                },
                0,
            ))
            .unwrap();
            assert_eq!(sc.state, expected, "estado {name}");
        }
    }

    #[test]
    fn unknown_state_is_not_silently_mapped() {
        // Un estado que el contrato anade no se confunde con uno conocido.
        let sc = decode_snapshot_scval(&snapshot_val("SomethingNew", None, None, 0)).unwrap();
        assert_ne!(sc.state, EscrowState::Created);
        assert_ne!(sc.state, EscrowState::Funded);
    }

    #[test]
    fn decodes_optionals_as_void() {
        let sc =
            decode_snapshot_scval(&snapshot_val("EvidenceSubmitted", Some([1u8; 32]), None, 0))
                .unwrap();
        assert_eq!(sc.report_hash, None);
        assert_eq!(sc.attested_at, None);
        assert_eq!(sc.objection_deadline, None);
    }

    #[test]
    fn missing_field_is_an_explicit_error() {
        // Si el contrato renombra un campo, se dice cual falta en vez de leer shifting.
        let mut entries = match snapshot_val("Funded", None, None, 0) {
            ScVal::Map(Some(m)) => m.0.to_vec(),
            _ => panic!("se esperaba Map"),
        };
        entries.retain(|e| e.key != ScVal::Symbol("ledger_timestamp".try_into().unwrap()));
        let val = ScVal::Map(Some(xdr::ScMap(entries.try_into().unwrap())));
        let err = decode_snapshot_scval(&val).unwrap_err();
        assert!(format!("{err}").contains("ledger_timestamp"), "{err}");
    }

    #[test]
    fn wrong_type_is_not_silently_zero() {
        let err = decode_snapshot_scval(&named(vec![
            ("state", state_val("Funded")),
            ("config", config_map()),
            ("correction_attempts", ScVal::U64(1)),
            ("ledger_timestamp", ScVal::U64(1)),
        ]))
        .unwrap_err();
        assert!(format!("{err}").contains("correction_attempts"), "{err}");
    }

    #[test]
    fn field_order_does_not_matter() {
        // Es la ventaja de leer por nombre: reordenar el contrato no mueve valores.
        let a = decode_snapshot_scval(&snapshot_val("Funded", Some([1u8; 32]), None, 0)).unwrap();
        let mut entries = match snapshot_val("Funded", Some([1u8; 32]), None, 0) {
            ScVal::Map(Some(m)) => m.0.to_vec(),
            _ => panic!("se esperaba Map"),
        };
        entries.reverse();
        let reversed = ScVal::Map(Some(xdr::ScMap(entries.try_into().unwrap())));
        let b = decode_snapshot_scval(&reversed).unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn attestable_with_report_hash_is_rejected_as_inconsistent() {
        let err = decode_snapshot_scval(&snapshot_val(
            "EvidenceSubmitted",
            Some([1u8; 32]),
            Some([9u8; 32]),
            0,
        ))
        .unwrap_err();
        assert!(format!("{err}").contains("inconsistente"));
    }

    #[test]
    fn attested_pass_decodes_with_report_hash() {
        let sc = decode_snapshot_scval(&snapshot_val(
            "AttestedPass",
            Some([1u8; 32]),
            Some([9u8; 32]),
            0,
        ))
        .unwrap();
        assert_eq!(sc.state, EscrowState::AttestedPass);
        assert_eq!(sc.report_hash, Some([9u8; 32]));
    }

    #[test]
    fn correction_attempt_is_visible_to_the_engine() {
        let sc =
            decode_snapshot_scval(&snapshot_val("EvidenceSubmitted", Some([2u8; 32]), None, 1))
                .unwrap();
        assert_eq!(sc.correction_attempts, 1);
    }

    #[test]
    fn non_map_is_rejected() {
        assert!(decode_snapshot_scval(&ScVal::U32(2)).is_err());
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
    fn contract_id_must_be_valid_strkey() {
        assert!(
            parse_contract_id("CA3D5KRYM6CB7OWQ6TWYRR3Z4T7GNZLKERYNZGGA5SOAOPIFY6YQGAXE").is_ok()
        );
        assert!(parse_contract_id("no-es-un-contract-id").is_err());
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
