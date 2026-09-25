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
use stellar_xdr::{Limits, ReadXdr, TransactionEnvelope, WriteXdr};

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
#[derive(Debug, Clone, PartialEq)]
pub struct EscrowSnapshot {
    /// Estado actual.
    pub state: EscrowState,
    /// Configuracion vigente.
    pub config: EscrowConfigView,
    /// Hash del bundle que el supplier ya subio a la cadena, si existe.
    pub evidence_bundle_hash: Option<[u8; 32]>,
    /// Plazo de atestacion absoluto, si el estado ya lo fijo.
    pub attestation_deadline: Option<u64>,
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
    /// Firma y envia `attest`, y confirma releyendo el contrato.
    ///
    /// Secuencia: simular con `authMode: record` para obtener recursos y credenciales,
    /// firmar el payload que la red espera, enviar, esperar inclusion y **comprobar que el
    /// estado quedo en `Attested*`**. Devolver exito sin esa comprobacion seria mentir.
    pub async fn submit_attestation_async(
        &self,
        outcome: AttestationOutcome,
        report_hash: &[u8; 32],
    ) -> Result<SubmitOutcome> {
        use stellar_rpc_client::AuthMode;

        let signer = EngineSigner::from_env()?;
        let client = self.client()?;
        let passphrase = client
            .get_network()
            .await
            .map_err(|e| AgentError::Network(format!("getNetwork: {e}")))?
            .passphrase;
        client
            .verify_network_passphrase(Some(&passphrase))
            .await
            .map_err(|e| AgentError::Network(format!("passphrase inconsistente: {e}")))?;

        let account = client
            .get_account(&signer.address())
            .await
            .map_err(|e| AgentError::Network(format!("getAccount de la cuenta engine: {e}")))?;
        let source = xdr::MuxedAccount::Ed25519(signer.public_key_bytes().into());
        let contract: xdr::ScAddress = self
            .contract
            .parse::<xdr::ScAddress>()
            .map_err(|e| AgentError::Config(format!("CANGUPA_CONTRACT_ID invalido: {e}")))?;

        let mut tx = crate::attest::build_invoke_tx(
            &contract,
            &source,
            account.seq_num.0,
            outcome,
            report_hash,
        )?;

        let simulation = client
            .simulate_transaction_envelope(
                &crate::attest::unsigned_envelope(&tx),
                Some(AuthMode::Record),
            )
            .await
            .map_err(|e| AgentError::Network(format!("simulateTransaction: {e}")))?;
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

        let network_id = crate::attest::network_id_hash(&passphrase);
        let envelope = crate::attest::sign_envelope(&signer, &tx, &network_id)?;

        let hash = client
            .send_transaction(&envelope)
            .await
            .map_err(|e| AgentError::Network(format!("sendTransaction: {e}")))?;
        let response = client
            .get_transaction_polling(&hash, None)
            .await
            .map_err(|e| AgentError::Network(format!("getTransaction: {e}")))?;
        if response.status != TX_SUCCESS {
            return Err(AgentError::Network(format!(
                "la transaccion de {ATTEST_FN} no fue exitosa: estado {}",
                response.status
            )));
        }

        let expected = crate::attest::expected_state_after(outcome);
        let state_after = self.snapshot_async().await?.state;
        if state_after != expected {
            return Err(AgentError::InvalidState {
                actual: state_after.to_string(),
                operacion: format!("confirmar {ATTEST_FN} (tx {hash}, se esperaba {expected})"),
            });
        }
        Ok(SubmitOutcome {
            hash: TxHash(hash.to_string()),
            state_after,
        })
    }

    /// Lectura asincrona del estado real del contrato.
    pub async fn snapshot_async(&self) -> Result<EscrowSnapshot> {
        use stellar_rpc_client::LedgerEntryResult;

        let client = self.client()?;
        let key = contract_instance_key(&self.contract_bytes);
        let entries = client
            .get_ledger_entries(&[key])
            .await
            .map_err(|e| rpc_error("getLedgerEntries", &e))?;
        let entry: LedgerEntryResult = entries
            .entries
            .and_then(|mut v| {
                if v.is_empty() {
                    None
                } else {
                    Some(v.remove(0))
                }
            })
            .ok_or_else(|| {
                AgentError::Network(format!("no se encontro el contrato {}", self.contract))
            })?;
        let ledger_entry = xdr::LedgerEntryData::from_xdr_base64(&entry.xdr, Limits::none())
            .map_err(|e| AgentError::Xdr(format!("LedgerEntryData: {e}")))?;
        let xdr::LedgerEntryData::ContractData(contract_data) = ledger_entry else {
            return Err(AgentError::Network("la entrada no es ContractData".into()));
        };

        let storage = read_instance_storage(&contract_data.val)?;
        let state = read_state(&storage)?;
        let config = read_config(&storage)?;
        let evidence_bundle_hash = read_bytes_n(&storage, "EvidenceBundleHash")?;
        let attestation_deadline = read_u64(&storage, "AttestationDeadline")?;

        let ledger_info = client
            .get_latest_ledger()
            .await
            .map_err(|e| rpc_error("getLatestLedger", &e))?;
        let ledger_seq = ledger_info.sequence;
        let ledger_ts = self.latest_ledger_timestamp(&client, ledger_seq).await?;

        Ok(EscrowSnapshot {
            state,
            config,
            evidence_bundle_hash,
            attestation_deadline,
            ledger: ledger_seq,
            ledger_timestamp: ledger_ts,
        })
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

/// Llave de la instancia completa del contrato, que es donde viven los `DataKey`.
///
/// Los `DataKey` no son entradas propias del ledger: son claves del `ScMap` de
/// almacenamiento de instancia, asi que se lee la instancia entera y se busca dentro.
pub(crate) fn contract_instance_key(contract: &[u8; 32]) -> xdr::LedgerKey {
    xdr::LedgerKey::ContractData(xdr::LedgerKeyContractData {
        contract: xdr::ScAddress::Contract(xdr::ContractId(xdr::Hash(*contract))),
        key: ScVal::LedgerKeyContractInstance,
        durability: xdr::ContractDataDurability::Persistent,
    })
}

/// `ScVal::Vec([ScVal::Symbol(name)])`, la forma que usa `#[contracttype] enum` de Soroban.
pub(crate) fn sc_key(name: &str) -> Result<ScVal> {
    let symbol: xdr::StringM<32> = name
        .try_into()
        .map_err(|_| AgentError::Config(format!("clave de almacenamiento invalida: {name}")))?;
    let vec = xdr::ScVec(vec_one(ScVal::Symbol(xdr::ScSymbol(symbol)))?);
    Ok(ScVal::Vec(Some(vec)))
}

/// `VecM` de un solo elemento, que es la forma de un `DataKey` de Soroban.
pub(crate) fn vec_one(val: ScVal) -> Result<xdr::VecM<ScVal>> {
    xdr::VecM::try_from(vec![val])
        .map_err(|e| AgentError::Xdr(format!("no se pudo construir el VecM: {e}")))
}

/// `VecM` a partir de un vector de valores.
pub(crate) fn vec_of(vals: Vec<ScVal>) -> Result<xdr::VecM<ScVal>> {
    xdr::VecM::try_from(vals)
        .map_err(|e| AgentError::Xdr(format!("no se pudo construir el VecM: {e}")))
}

/// Lee el mapa de almacenamiento de instancia del contrato.
fn read_instance_storage(val: &ScVal) -> Result<xdr::ScMap> {
    match val {
        ScVal::ContractInstance(instance) => instance.storage.clone().ok_or_else(|| {
            AgentError::Network("el contrato no tiene almacenamiento de instancia".into())
        }),
        other => Err(AgentError::Network(format!(
            "valor inesperado en la instancia: {other:?}"
        ))),
    }
}

/// Busca una clave en el mapa de instancia.
fn lookup<'a>(storage: &'a xdr::ScMap, name: &str) -> Option<&'a ScVal> {
    let key = sc_key(name).ok()?;
    storage.0.iter().find(|e| e.key == key).map(|e| &e.val)
}

/// Estado actual, como `ScVal::U32` con el indice del enum.
fn read_state(storage: &xdr::ScMap) -> Result<EscrowState> {
    let val = lookup(storage, "State")
        .ok_or_else(|| AgentError::Network("el contrato no tiene clave State".into()))?;
    let ScVal::U32(v) = val else {
        return Err(AgentError::Network(format!("State no es U32: {val:?}")));
    };
    Ok(EscrowState::from_u32(*v))
}

/// Lee un `BytesN<32>` del almacenamiento.
fn read_bytes_n(storage: &xdr::ScMap, name: &str) -> Result<Option<[u8; 32]>> {
    match lookup(storage, name) {
        None => Ok(None),
        Some(ScVal::Bytes(b)) => {
            let slice: &[u8] = b.0.as_ref();
            if slice.len() != 32 {
                return Err(AgentError::Network(format!(
                    "{name} deberia tener 32 bytes y tiene {}",
                    slice.len()
                )));
            }
            let mut out = [0u8; 32];
            out.copy_from_slice(slice);
            Ok(Some(out))
        }
        Some(other) => Err(AgentError::Network(format!(
            "{name} no es Bytes: {other:?}"
        ))),
    }
}

/// Lee un `u64` del almacenamiento.
fn read_u64(storage: &xdr::ScMap, name: &str) -> Result<Option<u64>> {
    match lookup(storage, name) {
        None => Ok(None),
        Some(ScVal::U64(v)) => Ok(Some(*v)),
        Some(ScVal::U32(v)) => Ok(Some(*v as u64)),
        Some(other) => Err(AgentError::Network(format!("{name} no es U64: {other:?}"))),
    }
}

/// Lee el `EscrowConfig` de la cadena.
///
/// El contrato lo guarda como un `ScVal::Vec` (struct de `contracttype`) con las claves
/// publicas en orden de declaracion en `types.rs`; aqui se toman solo los campos que el
/// agente necesita, por indice y con validacion de longitud.
fn read_config(storage: &xdr::ScMap) -> Result<EscrowConfigView> {
    let val = lookup(storage, "Config").ok_or_else(|| {
        AgentError::Network("el contrato no esta inicializado (sin Config)".into())
    })?;
    let ScVal::Vec(Some(items)) = val else {
        return Err(AgentError::Network(format!("Config no es Vec: {val:?}")));
    };
    let get = |i: usize| -> Result<&ScVal> {
        items
            .get(i)
            .ok_or_else(|| AgentError::Network(format!("Config incompleto: falta el campo {i}")))
    };
    let as_address = |i: usize| -> Result<String> {
        match get(i)? {
            ScVal::Address(a) => Ok(sc_address_to_str(a)),
            other => Err(AgentError::Network(format!(
                "campo {i} no es Address: {other:?}"
            ))),
        }
    };
    let as_i128 = |i: usize| -> Result<i128> {
        match get(i)? {
            ScVal::I128(v) => Ok(((v.hi as i128) << 64) | (v.lo as i128)),
            other => Err(AgentError::Network(format!(
                "campo {i} no es I128: {other:?}"
            ))),
        }
    };
    let as_u64 = |i: usize| -> Result<u64> {
        match get(i)? {
            ScVal::U64(v) => Ok(*v),
            ScVal::U32(v) => Ok(*v as u64),
            other => Err(AgentError::Network(format!(
                "campo {i} no es U64: {other:?}"
            ))),
        }
    };
    let as_u32 = |i: usize| -> Result<u32> {
        match get(i)? {
            ScVal::U32(v) => Ok(*v),
            other => Err(AgentError::Network(format!(
                "campo {i} no es U32: {other:?}"
            ))),
        }
    };

    // Orden de `EscrowConfig` en contracts/conditional-payment/src/types.rs:
    // buyer, supplier, engine, resolver, token, amount, submission_period,
    // attestation_period, objection_period, correction_period, resolution_period,
    // fallback_outcome, fallback_split_bps, max_correction_attempts.
    let engine = as_address(2)?;
    let token = as_address(4)?;
    let amount = as_i128(5)?;
    let attestation_period = as_u64(7)?;
    let fallback_outcome = as_u32(11)?;
    let fallback_split_bps = as_u32(12)?;

    Ok(EscrowConfigView {
        engine,
        token,
        amount,
        attestation_period,
        fallback_outcome,
        fallback_split_bps,
    })
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

    fn storage_with(entries: &[(&str, ScVal)]) -> xdr::ScMap {
        xdr::ScMap::sorted_from_pairs(entries.iter().map(|(k, v)| (sc_key(k).unwrap(), v.clone())))
            .unwrap()
    }

    fn bytes32(raw: [u8; 32]) -> ScVal {
        ScVal::Bytes(xdr::ScBytes(xdr::BytesM::try_from(raw.to_vec()).unwrap()))
    }

    #[test]
    fn key_is_vec_of_symbol() {
        let expected = ScVal::Vec(Some(xdr::ScVec(
            vec_one(ScVal::Symbol("State".try_into().unwrap())).unwrap(),
        )));
        assert_eq!(sc_key("State").unwrap(), expected);
    }

    #[test]
    fn state_decodes_by_discriminant() {
        let s = storage_with(&[("State", ScVal::U32(2))]);
        assert_eq!(read_state(&s).unwrap(), EscrowState::EvidenceSubmitted);
    }

    #[test]
    fn unknown_state_is_not_silently_mapped() {
        let s = storage_with(&[("State", ScVal::U32(42))]);
        assert_eq!(read_state(&s).unwrap(), EscrowState::Unknown(42));
    }

    #[test]
    fn wrong_length_hash_is_rejected() {
        let s = storage_with(&[(
            "EvidenceBundleHash",
            ScVal::Bytes(xdr::ScBytes(xdr::BytesM::try_from(vec![1u8; 31]).unwrap())),
        )]);
        assert!(read_bytes_n(&s, "EvidenceBundleHash").is_err());
    }

    #[test]
    fn missing_state_is_an_error_not_a_panic() {
        let s = storage_with(&[("Other", ScVal::U32(1))]);
        assert!(read_state(&s).is_err());
    }

    #[test]
    fn bytes_n_is_read() {
        let mut raw = [0u8; 32];
        raw[0] = 0xab;
        let s = storage_with(&[("EvidenceBundleHash", bytes32(raw))]);
        assert_eq!(read_bytes_n(&s, "EvidenceBundleHash").unwrap(), Some(raw));
        assert_eq!(read_bytes_n(&s, "ReportHash").unwrap(), None);
    }

    #[test]
    fn snapshot_reports_attestability() {
        let snap = EscrowSnapshot {
            state: EscrowState::EvidenceSubmitted,
            config: EscrowConfigView {
                engine: "G".into(),
                token: "C".into(),
                amount: 1,
                attestation_period: 60,
                fallback_outcome: 2,
                fallback_split_bps: 0,
            },
            evidence_bundle_hash: Some([1u8; 32]),
            attestation_deadline: Some(1_000),
            ledger: 5,
            ledger_timestamp: 900,
        };
        assert!(snap.is_attestable());
        assert_eq!(snap.attestation_remaining(), Some(100));
    }
}
