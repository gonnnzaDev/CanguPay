//! Firma y envio de `attest(outcome, report_hash)`.
//!
//! El flujo es deliberadamente estricto y en este orden, para que una atestacion nunca
//! se firme sobre evidencia que el chain todavia no refleja:
//!
//! 1. [`plan_attestation`] decide **sin tocar la red** si la atestacion procede: estado,
//!    plazo, hash del bundle y coincidencia con la cuenta engine. Es la parte cubierta por
//!    pruebas con dobles.
//! 2. [`submit_via_rpc`] arma la transaccion, la firma con la clave del engine y la envia,
//!    y despues **vuelve a leer la cadena** para confirmar que el estado quedo `Attested*`.
//!
//! La clave nunca sale de [`crate::signer`] y nunca se imprime.
use stellar_xdr as xdr;
use stellar_xdr::ScVal;
use stellar_xdr::{Limits, ReadXdr, WriteXdr};

use crate::chain::{vec_of, vec_one, AttestationOutcome, EscrowState, SubmitOutcome, TxHash};
use crate::error::{AgentError, Result};
use crate::report::Report;
use crate::signer::EngineSigner;

/// Decision de atestacion, calculada sin red.
#[derive(Debug, Clone, PartialEq)]
pub struct AttestationPlan {
    /// Outcome a enviar al contrato.
    pub outcome: AttestationOutcome,
    /// `report_hash` que se firma y se manda.
    pub report_hash: String,
    /// Bytes del `report_hash` listo para `BytesN<32>`.
    pub report_hash_bytes: [u8; 32],
    /// Estado que se espera ver despues de confirmar.
    pub expected_state_after: EscrowState,
    /// Ledger actual en el momento de la decision.
    pub ledger: u32,
}

/// Estados en los que el contrato acepta `attest()`.
const ATTESTABLE: EscrowState = EscrowState::EvidenceSubmitted;

/// Valida que la atestacion procede y devuelve el plan.
///
/// Rechaza, en orden, las cuatro razones por las que el engine no debe atestar:
/// estado invalido, plazo vencido, bundle ausente o distinto, y clave que no es la del
/// contrato. Cada rechazo es explicito para que el keeper registre el motivo.
pub fn plan_attestation(
    snapshot: &crate::chain::EscrowSnapshot,
    report: &Report,
    signer: &EngineSigner,
) -> Result<AttestationPlan> {
    if snapshot.state != ATTESTABLE {
        return Err(AgentError::InvalidState {
            actual: snapshot.state.to_string(),
            operacion: "attest".into(),
        });
    }
    if let Some(remaining) = snapshot.attestation_remaining() {
        if remaining <= 0 {
            return Err(AgentError::DeadlinePassed {
                plazo: "attestation_period".into(),
                remaining,
            });
        }
    }
    let on_chain = snapshot
        .evidence_bundle_hash
        .ok_or_else(|| AgentError::InvalidState {
            actual: snapshot.state.to_string(),
            operacion: "attest sin EvidenceBundleHash en cadena".into(),
        })?;
    let local = hash_bytes(&report.evidence_bundle_hash);
    if local != on_chain {
        return Err(AgentError::BundleHashMismatch {
            computed: report.evidence_bundle_hash.clone(),
            on_chain: hex::encode(on_chain),
        });
    }
    signer.ensure_matches_contract_engine(&snapshot.config.engine)?;

    Ok(AttestationPlan {
        outcome: if report.is_pass() {
            AttestationOutcome::Pass
        } else {
            AttestationOutcome::Fail
        },
        report_hash: report.report_hash.clone(),
        report_hash_bytes: hash_bytes(&report.report_hash),
        expected_state_after: if report.is_pass() {
            EscrowState::AttestedPass
        } else {
            EscrowState::AttestedFail
        },
        ledger: snapshot.ledger,
    })
}

/// Convierte un hash hex de 64 caracteres en los 32 bytes que espera `BytesN<32>`.
pub fn hash_bytes(hex_hash: &str) -> [u8; 32] {
    let mut out = [0u8; 32];
    let decoded = hex::decode(hex_hash).unwrap_or_default();
    let len = decoded.len().min(32);
    out[..len].copy_from_slice(&decoded[..len]);
    out
}

/// Argumentos de `attest()` en XDR, tal y como los espera el contrato.
///
/// El host de Soroban recibe **un `ScVal` por argumento**, asi que
/// `attest(outcome: u32, report_hash: BytesN<32>)` son dos entradas y no una sola
/// lista: envolverlas en un `Vec` haria que el host no encontrara el simbolo.
pub fn attest_args(outcome: AttestationOutcome, report_hash: &[u8; 32]) -> Result<Vec<ScVal>> {
    // El enum llega como `Vec([Symbol(nombre)])`, no como el indice numerico: el host
    // decodifica esa forma y un U32 deixa la llamada sin convertir, con lo que el
    // contrato hace trap al leer `outcome`.
    let outcome_val = ScVal::Vec(Some(xdr::ScVec(vec_one(ScVal::Symbol(
        outcome
            .name()
            .try_into()
            .map_err(|_| AgentError::Xdr("nombre de outcome invalido".into()))?,
    ))?)));
    let hash_val = ScVal::Bytes(xdr::ScBytes(
        xdr::BytesM::try_from(report_hash.to_vec())
            .map_err(|e| AgentError::Xdr(format!("report_hash no cabe en BytesM: {e}")))?,
    ));
    Ok(vec![outcome_val, hash_val])
}

/// Nombre de la funcion del contrato.
pub const ATTEST_FN: &str = "attest";

/// Nombre de la funcion de vencimiento del contrato.
pub const FINALIZE_FN: &str = "finalize";

/// Funciones del contrato que exigen `auth` de Soroban.
///
/// `attest` llama a `config.engine.require_auth()`, asi que sin la firma correspondiente
/// la transaccion se rechaza. `finalize` es deliberadamente publica: cualquiera puede
/// ejecutarla porque el contrato es quien aplica el vencimiento, no quien lo comprueba.
const REQUIRES_SOROBAN_AUTH: &[&str] = &[ATTEST_FN];

/// Comision por operacion, en stroops, usada como base antes de sumar el `resource_fee`.
pub const BASE_FEE_STROOPS: u32 = 100;

/// Identificador de red para firmar: `sha256` del passphrase de la red.
///
/// Es exactamente lo que la red usa para el payload de firma; si se calculara con otra
/// variante, la transaccion seria rechazada con `BAD_AUTH`.
pub fn network_id_hash(passphrase: &str) -> xdr::Hash {
    use sha2::{Digest, Sha256};
    let mut out = [0u8; 32];
    out.copy_from_slice(&Sha256::digest(passphrase.as_bytes()));
    xdr::Hash(out)
}

/// Construye la transaccion `InvokeHostFunctionOp` que llama a `attest`.
///
/// Todavia **no** lleva recursos ni autorizacion: esos los devuelve la simulacion.
pub fn build_invoke_tx(
    contract: &xdr::ScAddress,
    source: &xdr::MuxedAccount,
    seq_num: i64,
    outcome: AttestationOutcome,
    report_hash: &[u8; 32],
) -> Result<xdr::Transaction> {
    let args = attest_args(outcome, report_hash)?;
    let op = xdr::Operation {
        body: xdr::OperationBody::InvokeHostFunction(xdr::InvokeHostFunctionOp {
            host_function: xdr::HostFunction::InvokeContract(xdr::InvokeContractArgs {
                contract_address: contract.clone(),
                function_name: ATTEST_FN
                    .try_into()
                    .map_err(|_| AgentError::Xdr(format!("{ATTEST_FN} no es un simbolo valido")))?,
                args: vec_of(args)?,
            }),
            auth: xdr::VecM::default(),
        }),
        source_account: None,
    };
    Ok(xdr::Transaction {
        source_account: source.clone(),
        fee: BASE_FEE_STROOPS,
        seq_num: xdr::SequenceNumber(seq_num),
        cond: xdr::Preconditions::default(),
        memo: xdr::Memo::None,
        operations: xdr::VecM::try_from(vec![op])
            .map_err(|e| AgentError::Xdr(format!("operaciones: {e}")))?,
        ext: xdr::TransactionExt::V0,
    })
}

/// Construye la transaccion `InvokeHostFunctionOp` que llama a `finalize`.
///
/// No lleva argumentos y **no** exige `auth`: el contrato decide por su cuenta si el
/// plazo se vencio. Se firma igualmente con la cuenta engine para que la transaccion
/// tenga una secuencia conocida y que el keeper quede como un pagador identificable.
pub fn build_finalize_tx(
    contract: &xdr::ScAddress,
    source: &xdr::MuxedAccount,
    seq_num: i64,
) -> Result<xdr::Transaction> {
    let op = xdr::Operation {
        body: xdr::OperationBody::InvokeHostFunction(xdr::InvokeHostFunctionOp {
            host_function: xdr::HostFunction::InvokeContract(xdr::InvokeContractArgs {
                contract_address: contract.clone(),
                function_name: FINALIZE_FN.try_into().map_err(|_| {
                    AgentError::Xdr(format!("{FINALIZE_FN} no es un simbolo valido"))
                })?,
                args: xdr::VecM::default(),
            }),
            auth: xdr::VecM::default(),
        }),
        source_account: None,
    };
    Ok(xdr::Transaction {
        source_account: source.clone(),
        fee: BASE_FEE_STROOPS,
        seq_num: xdr::SequenceNumber(seq_num),
        cond: xdr::Preconditions::default(),
        memo: xdr::Memo::None,
        operations: xdr::VecM::try_from(vec![op])
            .map_err(|e| AgentError::Xdr(format!("operaciones: {e}")))?,
        ext: xdr::TransactionExt::V0,
    })
}

/// Envoltura sin firmas, que es lo que se manda a `simulateTransaction`.
pub fn unsigned_envelope(tx: &xdr::Transaction) -> xdr::TransactionEnvelope {
    xdr::TransactionEnvelope::Tx(xdr::TransactionV1Envelope {
        tx: tx.clone(),
        signatures: xdr::VecM::default(),
    })
}

/// Aplica el resultado de la simulacion: recursos, comision y autorizacion.
///
/// Desde el protocolo 23 las credenciales **no** vienen dentro de `SorobanTransactionData`:
/// la simulacion las devuelve aparte y van en `InvokeHostFunctionOp.auth`. Por eso hay que
/// instalar ambas cosas, y la comision debe cubrir el `resource_fee` para que la
/// transaccion sea incluida.
pub fn apply_simulation(
    tx: &mut xdr::Transaction,
    data: xdr::SorobanTransactionData,
    auth: Vec<xdr::SorobanAuthorizationEntry>,
) -> Result<()> {
    tx.fee = tx
        .fee
        .saturating_add(u32::try_from(data.resource_fee).unwrap_or(u32::MAX));
    tx.ext = xdr::TransactionExt::V1(data);

    let mut installed = false;
    for op in tx.operations.iter_mut() {
        let xdr::OperationBody::InvokeHostFunction(invoke) = &mut op.body else {
            continue;
        };
        invoke.auth = xdr::VecM::try_from(auth.clone())
            .map_err(|e| AgentError::Xdr(format!("autorizaciones de la simulacion: {e}")))?;
        installed = true;
    }
    if !installed {
        return Err(AgentError::Xdr(
            "la transaccion no tiene InvokeHostFunctionOp donde poner las credenciales".into(),
        ));
    }
    Ok(())
}

/// Firma la autorizacion de la cuenta engine y devuelve el envelope listo para enviar.
///
/// Busca la entrada de auth que corresponde a la direccion del engine. Si la simulacion
/// no devolvio ninguna credencial y la funcion **exige** `auth` (como `attest`), es un
/// error explicito y no un fallo silencioso. Si en cambio la funcion es publica
/// (`finalize`), la ausencia de credenciales es lo esperado y solo se firma el
/// sobre de la transaccion.
pub fn sign_envelope(
    signer: &EngineSigner,
    tx: &xdr::Transaction,
    network_id: &xdr::Hash,
    fn_name: &str,
) -> Result<xdr::TransactionEnvelope> {
    let engine: xdr::ScAddress = signer
        .address()
        .parse::<xdr::ScAddress>()
        .map_err(|e| AgentError::Xdr(format!("direccion del engine invalida: {e}")))?;
    let payload_hash = signature_payload_hash(network_id, tx)?;
    let mut tx = tx.clone();
    let mut found = false;
    for op in tx.operations.iter_mut() {
        let xdr::OperationBody::InvokeHostFunction(invoke) = &mut op.body else {
            continue;
        };
        for entry in invoke.auth.iter_mut() {
            // Cada tipo de credenciales tiene su propio payload de firma. Confundirlos
            // produce una transicion que la red rechaza con `txBAD_AUTH`, asi que se
            // resuelve explicitamente por tipo.
            match &mut entry.credentials {
                xdr::SorobanCredentials::Address(creds) => {
                    if creds.address != engine {
                        continue;
                    }
                    creds.signature = legacy_auth_signature(signer, &payload_hash)?;
                    found = true;
                }
                xdr::SorobanCredentials::AddressV2(creds) => {
                    if creds.address != engine {
                        continue;
                    }
                    let sig = v2_auth_signature(signer, network_id, creds, &entry.root_invocation)?;
                    creds.signature = sig;
                    found = true;
                }
                // La cuenta que exige `auth` es la propia fuente de la transaccion:
                // Soroban la representa con credenciales `SourceAccount`, que no
                // llevan firma propia porque la firma del sobre ya la respalda. Buscar
                // solo `Address` haria pensar que la simulacion no devolvio nada.
                xdr::SorobanCredentials::SourceAccount => {
                    found = true;
                }
                _ => continue,
            }
        }
    }
    if !found && REQUIRES_SOROBAN_AUTH.contains(&fn_name) {
        return Err(AgentError::Network(format!(
            "la simulacion no devolvio credenciales para el engine {}; \
             no se puede firmar {fn_name}",
            signer.address()
        )));
    }

    // La red indexa la firma por el hint para saber que clave buscar. En el nodo de
    // protocolo 28 el hint son los **ultimos** 4 bytes de la pubkey: poner los
    // primeros hace que la red busque una clave que no existe y responda
    // `TxBadAuth` aunque la firma sea correcta.
    let pk = signer.public_key_bytes();
    let hint = xdr::SignatureHint(pk[pk.len() - 4..].try_into().unwrap());
    let decorated = xdr::DecoratedSignature {
        hint,
        signature: xdr::Signature(
            xdr::BytesM::try_from(signer.sign(&payload_hash).to_vec())
                .map_err(|e| AgentError::Xdr(format!("firma fuera de rango: {e}")))?,
        ),
    };
    Ok(xdr::TransactionEnvelope::Tx(xdr::TransactionV1Envelope {
        tx,
        signatures: xdr::VecM::try_from(vec![decorated])
            .map_err(|e| AgentError::Xdr(format!("firmas: {e}")))?,
    }))
}

/// Estado que el contrato debe mostrar tras una atestacion con exito.
pub fn expected_state_after(outcome: AttestationOutcome) -> EscrowState {
    match outcome {
        AttestationOutcome::Pass => EscrowState::AttestedPass,
        AttestationOutcome::Fail => EscrowState::AttestedFail,
    }
}

/// Firma un payload con el estandar de Soroban y devuelve el `ScVal` de la firma.
///
/// `ScVal::Vec([ScVal::Bytes(sig64)])`, que es la forma que el host espera en
/// `SorobanAddressCredentials::signature`.
pub fn signature_scval(signer: &EngineSigner, payload: &[u8]) -> Result<ScVal> {
    let signature = signer.sign(payload);
    let bytes = ScVal::Bytes(xdr::ScBytes(
        xdr::BytesM::try_from(signature.to_vec())
            .map_err(|e| AgentError::Xdr(format!("la firma no cabe en BytesM: {e}")))?,
    ));
    Ok(ScVal::Vec(Some(xdr::ScVec(vec_one(bytes)?))))
}

/// Hash del payload de firma de una transaccion, tal como lo define el protocolo.
///
/// Desde el protocolo 23 la red firma el `Transaction` envuelto en
/// `TransactionSignaturePayload::Tx`, no el envelope; se calcula aqui para poder
/// comprobar que el agente hashea exactamente los bytes que verifica la red.
pub fn signature_payload_hash(
    network_id: &xdr::Hash,
    transaction: &xdr::Transaction,
) -> Result<[u8; 32]> {
    use sha2::{Digest, Sha256};
    let payload = xdr::TransactionSignaturePayload {
        network_id: network_id.clone(),
        tagged_transaction: xdr::TransactionSignaturePayloadTaggedTransaction::Tx(
            transaction.clone(),
        ),
    };
    let bytes = payload
        .to_xdr(Limits::none())
        .map_err(|e| AgentError::Xdr(format!("TransactionSignaturePayload: {e}")))?;
    let digest = Sha256::digest(&bytes);
    let mut out = [0u8; 32];
    out.copy_from_slice(&digest);
    Ok(out)
}

/// Simbolo del topic del evento `Attested`.
///
/// Un `#[contractevent] struct` publica el nombre del evento como primer topic y deja
/// los campos `#[topic]` en los siguientes; `Attested` no tiene campos topic, asi que
/// lleva solo el nombre. Los campos sin `#[topic]` van en el `Map` de datos.
pub const ATTESTED_TOPIC: &str = "attested";

/// Confirma que una transacion emitio `Attested` con el outcome y el `report_hash` que
/// el engine pretendia.
///
/// No basta con que la tx tenga estado `SUCCESS` ni con que el contrato acabe en
/// `Attested*`: ambos pueden ocurrir sin que este engine haya sido quien atestiguo.
/// El evento es la prueba de que la atestacion salio de esta cuenta, y el
/// `report_hash` del evento es el lazo que une la cadena con el reporte determinista.
pub fn find_attested_event(
    events: &[xdr::ContractEvent],
    contract_id: &[u8; 32],
    outcome: AttestationOutcome,
    report_hash: &[u8; 32],
) -> Result<()> {
    let want_topic = ScVal::Symbol(
        ATTESTED_TOPIC
            .try_into()
            .map_err(|_| AgentError::Xdr("el simbolo del evento no es valido".into()))?,
    );
    let expected = xdr::Hash(*report_hash);

    for event in events {
        // Un evento de otro contrato no cuenta aunque traiga el mismo simbolo.
        let xdr::ContractEventBody::V0(body) = &event.body;
        let Some(xdr::ContractId(id)) = event.contract_id.as_ref() else {
            continue;
        };
        if id.0 != *contract_id {
            continue;
        }
        if body.topics.first() != Some(&want_topic) {
            continue;
        }
        // Topics extra indicarian que el evento cambio de forma.
        if body.topics.len() != 1 {
            return Err(AgentError::Xdr(format!(
                "{ATTESTED_TOPIC} deberia tener 1 topic y tiene {}",
                body.topics.len()
            )));
        }
        let data = &body.data;
        if event_outcome(data)? != outcome {
            return Err(AgentError::Network(format!(
                "el evento {ATTESTED_TOPIC} declara un outcome distinto al enviado"
            )));
        }
        if event_report_hash(data)? != expected {
            return Err(AgentError::Network(format!(
                "el evento {ATTESTED_TOPIC} lleva un report_hash distinto al firmado"
            )));
        }
        return Ok(());
    }

    Err(AgentError::Network(format!(
        "la transaccion no emitio {ATTESTED_TOPIC} para el contrato; no se puede \
         afirmar que la atestacion se registro"
    )))
}

/// Lee el campo `outcome` del `Map` de datos del evento.
///
/// Un enum de `contracttype` se serializa como `Vec([Symbol(nombre)])`.
fn event_outcome(data: &ScVal) -> Result<AttestationOutcome> {
    let val = event_field(data, "outcome")?;
    let ScVal::Vec(Some(items)) = val else {
        return Err(AgentError::Xdr(format!("outcome no es Vec: {val:?}")));
    };
    let Some(ScVal::Symbol(name)) = items.first() else {
        return Err(AgentError::Xdr("outcome no empieza por Symbol".into()));
    };
    let name = std::str::from_utf8(name.0.as_slice())
        .map_err(|_| AgentError::Xdr("el outcome del evento no es utf-8".into()))?;
    match name {
        "Pass" => Ok(AttestationOutcome::Pass),
        "Fail" => Ok(AttestationOutcome::Fail),
        other => Err(AgentError::Xdr(format!("outcome desconocido: {other}"))),
    }
}

/// Lee el campo `report_hash` del `Map` de datos del evento.
fn event_report_hash(data: &ScVal) -> Result<xdr::Hash> {
    let val = event_field(data, "report_hash")?;
    let ScVal::Bytes(b) = val else {
        return Err(AgentError::Xdr(format!("report_hash no es Bytes: {val:?}")));
    };
    let slice: &[u8] = b.0.as_ref();
    if slice.len() != 32 {
        return Err(AgentError::Xdr(format!(
            "report_hash del evento deberia tener 32 bytes y tiene {}",
            slice.len()
        )));
    }
    let mut out = [0u8; 32];
    out.copy_from_slice(slice);
    Ok(xdr::Hash(out))
}

/// Busca un campo por nombre en el `Map` de datos del evento.
fn event_field<'a>(data: &'a ScVal, name: &str) -> Result<&'a ScVal> {
    let ScVal::Map(Some(entries)) = data else {
        return Err(AgentError::Xdr(format!(
            "los datos del evento no son Map: {data:?}"
        )));
    };
    let key: xdr::ScSymbol = name
        .try_into()
        .map_err(|_| AgentError::Xdr(format!("campo de evento invalido: {name}")))?;
    entries
        .0
        .iter()
        .find(|e| e.key == ScVal::Symbol(key.clone()))
        .map(|e| &e.val)
        .ok_or_else(|| AgentError::Xdr(format!("el evento no trae el campo {name}")))
}

/// Firma de las credenciales `SOROBAN_CREDENTIALS_ADDRESS` (v1).
///
/// El payload es el de la transaccion, sin mas: es el comportamiento historico y lo
/// que el host espera de este tipo de credenciales.
fn legacy_auth_signature(signer: &EngineSigner, payload_hash: &[u8; 32]) -> Result<ScVal> {
    signature_scval(signer, payload_hash)
}

/// Firma de las credenciales `SOROBAN_CREDENTIALS_ADDRESS_V2`.
///
/// A diferencia de la v1, estas credenciales **no** se firman con el payload de la
/// transaccion: el protocolo 27 (CAP-71-02) exige el preimage
/// `ENVELOPE_TYPE_SOROBAN_AUTHORIZATION_WITH_ADDRESS`, que ademas ata la direccion a la
/// firma para impedir reutilizarla entre cuentas que compartan clave.
///
/// Firmarlas con el payload de la transaccion, como hace la v1, produce una
/// transicion que la red rechaza con `txBAD_AUTH` en cuanto el RPC entrega credenciales
/// v2, que es el comportamiento por defecto desde el protocolo 28.
fn v2_auth_signature(
    signer: &EngineSigner,
    network_id: &xdr::Hash,
    creds: &xdr::SorobanAddressCredentials,
    invocation: &xdr::SorobanAuthorizedInvocation,
) -> Result<ScVal> {
    let preimage = xdr::HashIdPreimage::SorobanAuthorizationWithAddress(
        xdr::HashIdPreimageSorobanAuthorizationWithAddress {
            network_id: network_id.clone(),
            nonce: creds.nonce,
            signature_expiration_ledger: creds.signature_expiration_ledger,
            address: creds.address.clone(),
            invocation: invocation.clone(),
        },
    );
    let bytes = preimage
        .to_xdr(Limits::none())
        .map_err(|e| AgentError::Xdr(format!("HashIDPreimage: {e}")))?;
    use sha2::{Digest, Sha256};
    let digest = Sha256::digest(&bytes);
    signature_scval(signer, &digest)
}

/// Enlace al explorer de la transaccion, para el log del keeper y la demo.
pub fn explorer_tx_link(base: &str, hash: &TxHash) -> String {
    format!("{}/tx/{}", base.trim_end_matches('/'), hash.0)
}

/// Reporta el resultado de un envio para el log del keeper.
pub fn describe_outcome(outcome: &SubmitOutcome) -> String {
    format!("tx={} estado={}", outcome.hash, outcome.state_after)
}

/// Lee el hash de un `BytesN<32>` devuelto por la red, util en pruebas de integracion.
pub fn bytes_n_from_scval(val: &ScVal) -> Result<[u8; 32]> {
    let ScVal::Bytes(b) = val else {
        return Err(AgentError::Xdr(format!("no es Bytes: {val:?}")));
    };
    let slice: &[u8] = b.0.as_ref();
    if slice.len() != 32 {
        return Err(AgentError::Xdr(format!(
            "se esperaban 32 bytes y hay {}",
            slice.len()
        )));
    }
    let mut out = [0u8; 32];
    out.copy_from_slice(slice);
    Ok(out)
}

/// Decodifica un envelope desde base64, para depurar el camino de envio.
pub fn envelope_from_base64(text: &str) -> Result<xdr::TransactionEnvelope> {
    xdr::TransactionEnvelope::from_xdr_base64(text, Limits::none())
        .map_err(|e| AgentError::Xdr(format!("TransactionEnvelope: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chain::EscrowConfigView;
    use crate::report::build_report;
    use serde_json::json;

    const TEST_SEED: &str = "9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60";

    fn signer() -> EngineSigner {
        EngineSigner::from_secret(TEST_SEED, crate::signer::KeySource::EnvVar).unwrap()
    }

    fn bundle() -> serde_json::Value {
        json!({
            "purchase_order": {"id": "PO-001", "supplier_id": "SUP-001", "amount": 1000, "currency": "CPUSD"},
            "invoice": {"id": "INV-001", "purchase_order_id": "PO-001", "supplier_id": "SUP-001", "amount": 1000, "currency": "CPUSD"},
            "delivery": {"purchase_order_id": "PO-001", "accepted": true}
        })
    }

    fn report() -> Report {
        build_report(&bundle(), 1000, "CPUSD").unwrap()
    }

    fn snapshot(
        state: EscrowState,
        on_chain_hash: Option<[u8; 32]>,
        ts: u64,
    ) -> crate::chain::EscrowSnapshot {
        crate::chain::EscrowSnapshot {
            state,
            config: EscrowConfigView {
                engine: signer().address(),
                token: "CDUMMY".into(),
                amount: 1000,
                attestation_period: 3600,
                fallback_outcome: 2,
                fallback_split_bps: 0,
            },
            evidence_bundle_hash: on_chain_hash,
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
            ledger_timestamp: ts,
        }
    }

    #[test]
    fn plans_attestation_when_everything_matches() {
        let r = report();
        let snap = snapshot(
            EscrowState::EvidenceSubmitted,
            Some(hash_bytes(&r.evidence_bundle_hash)),
            900,
        );
        let plan = plan_attestation(&snap, &r, &signer()).unwrap();
        assert_eq!(plan.outcome, AttestationOutcome::Pass);
        assert_eq!(plan.expected_state_after, EscrowState::AttestedPass);
        assert_eq!(plan.report_hash, r.report_hash);
    }

    #[test]
    fn refuses_wrong_state() {
        let r = report();
        for state in [
            EscrowState::Created,
            EscrowState::Funded,
            EscrowState::AttestedPass,
            EscrowState::Disputed,
            EscrowState::Refunded,
        ] {
            let snap = snapshot(state, Some(hash_bytes(&r.evidence_bundle_hash)), 900);
            let err = plan_attestation(&snap, &r, &signer()).unwrap_err();
            assert!(
                matches!(err, AgentError::InvalidState { .. }),
                "{state}: {err}"
            );
        }
    }

    #[test]
    fn refuses_expired_deadline() {
        let r = report();
        let snap = snapshot(
            EscrowState::EvidenceSubmitted,
            Some(hash_bytes(&r.evidence_bundle_hash)),
            1_000,
        );
        let err = plan_attestation(&snap, &r, &signer()).unwrap_err();
        assert!(matches!(err, AgentError::DeadlinePassed { .. }), "{err}");
    }

    #[test]
    fn refuses_when_bundle_hash_differs_from_chain() {
        let r = report();
        let snap = snapshot(EscrowState::EvidenceSubmitted, Some([0xab; 32]), 900);
        let err = plan_attestation(&snap, &r, &signer()).unwrap_err();
        match err {
            AgentError::BundleHashMismatch { computed, on_chain } => {
                assert_eq!(computed, r.evidence_bundle_hash);
                assert_eq!(on_chain, "ab".repeat(32));
            }
            other => panic!("se esperaba BundleHashMismatch, se obtuvo {other}"),
        }
    }

    #[test]
    fn refuses_when_supplier_never_submitted_evidence() {
        let r = report();
        let snap = snapshot(EscrowState::EvidenceSubmitted, None, 900);
        assert!(plan_attestation(&snap, &r, &signer()).is_err());
    }

    #[test]
    fn refuses_when_key_is_not_the_contract_engine() {
        let r = report();
        let mut snap = snapshot(
            EscrowState::EvidenceSubmitted,
            Some(hash_bytes(&r.evidence_bundle_hash)),
            900,
        );
        snap.config.engine = "GDUMMYAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA".into();
        let err = plan_attestation(&snap, &r, &signer()).unwrap_err();
        assert!(err.to_string().contains("require_auth"), "{err}");
    }

    #[test]
    fn fail_bundle_plans_attest_fail() {
        let mut bad = bundle();
        bad["invoice"]["amount"] = json!(999);
        let r = build_report(&bad, 1000, "CPUSD").unwrap();
        let snap = snapshot(
            EscrowState::EvidenceSubmitted,
            Some(hash_bytes(&r.evidence_bundle_hash)),
            900,
        );
        let plan = plan_attestation(&snap, &r, &signer()).unwrap();
        assert_eq!(plan.outcome, AttestationOutcome::Fail);
        assert_eq!(plan.expected_state_after, EscrowState::AttestedFail);
    }

    /// El outcome viaja como `Vec([Symbol(nombre)])`, la forma que acepta el host.
    fn assert_outcome_arg(arg: &ScVal, expected: AttestationOutcome) {
        let ScVal::Vec(Some(items)) = arg else {
            panic!("el outcome deberia ser un Vec, es {arg:?}")
        };
        let Some(ScVal::Symbol(name)) = items.first() else {
            panic!("el outcome deberia empezar por Symbol")
        };
        assert_eq!(name.0.as_slice(), expected.name().as_bytes());
    }

    #[test]
    fn attest_args_match_the_contract_signature() {
        let args = attest_args(AttestationOutcome::Fail, &[7u8; 32]).unwrap();
        // Dos ScVal sueltos, no una lista: el host los recibe por posicion.
        assert_eq!(args.len(), 2);
        // El enum se manda por nombre, no por indice numerico. Con un U32 el host no
        // lo convierte y el contrato hace trap al leer `outcome`.
        assert_outcome_arg(&args[0], AttestationOutcome::Fail);
        assert_eq!(bytes_n_from_scval(&args[1]).unwrap(), [7u8; 32]);
    }

    #[test]
    fn outcome_arg_is_never_a_bare_u32() {
        // Fijado porque la forma numerica parece funcionar en pruebas y solo falla en
        // la red, que es donde se quedo sin descubrir durante horas.
        for outcome in [AttestationOutcome::Pass, AttestationOutcome::Fail] {
            let args = attest_args(outcome, &[1u8; 32]).unwrap();
            assert!(
                !matches!(args[0], ScVal::U32(_)),
                "un U32 como outcome hace trap en el contrato"
            );
            assert_outcome_arg(&args[0], outcome);
        }
    }

    #[test]
    fn hash_bytes_is_hex_to_32() {
        assert_eq!(hash_bytes(&"ab".repeat(32)), [0xabu8; 32]);
    }

    #[test]
    fn signature_verifies_against_the_payload_it_signed() {
        use ed25519_dalek::Verifier;
        let s = signer();
        let payload = b"cangu-attest-payload";
        let sig = ed25519_dalek::Signature::from_bytes(&s.sign(payload));
        let vk = ed25519_dalek::VerifyingKey::from_bytes(&s.public_key_bytes()).unwrap();
        assert!(vk.verify(payload, &sig).is_ok());
    }

    #[test]
    fn signature_scval_carries_64_bytes() {
        let val = signature_scval(&signer(), b"x").unwrap();
        let ScVal::Vec(Some(items)) = val else {
            panic!("se esperaba Vec")
        };
        let ScVal::Bytes(b) = items.first().expect("firma vacia") else {
            panic!("se esperaba Bytes")
        };
        let bytes: &[u8] = b.0.as_ref();
        assert_eq!(bytes.len(), 64);
    }

    #[test]
    fn explorer_link_is_stable() {
        let hash = TxHash("abc123".into());
        assert_eq!(
            explorer_tx_link("https://stellar.expert/testnet", &hash),
            "https://stellar.expert/testnet/tx/abc123"
        );
    }

    // --- construccion y firma de la transaccion ---

    const TESTNET_PASSPHRASE: &str = "Test SDF Network ; September 2015";

    fn contract() -> xdr::ScAddress {
        "CA3D5KRYM6CB7OWQ6TWYRR3Z4T7GNZLKERYNZGGA5SOAOPIFY6YQGAXE"
            .parse()
            .unwrap()
    }

    fn source() -> xdr::MuxedAccount {
        xdr::MuxedAccount::Ed25519(signer().public_key_bytes().into())
    }

    fn invoke_of(tx: &xdr::Transaction) -> &xdr::InvokeHostFunctionOp {
        let op = tx.operations.first().expect("debe haber una operacion");
        match &op.body {
            xdr::OperationBody::InvokeHostFunction(invoke) => invoke,
            other => panic!("se esperaba InvokeHostFunction, se obtuvo {other:?}"),
        }
    }

    /// Datos de simulacion minimos, tal y como los devuelve el RPC en protocolo 23.
    fn simulation_for(
        engine: &xdr::ScAddress,
    ) -> (
        xdr::SorobanTransactionData,
        Vec<xdr::SorobanAuthorizationEntry>,
    ) {
        let data = xdr::SorobanTransactionData {
            ext: xdr::SorobanTransactionDataExt::V0,
            resources: xdr::SorobanResources {
                footprint: xdr::LedgerFootprint::default(),
                instructions: 1_000,
                disk_read_bytes: 100,
                write_bytes: 100,
            },
            resource_fee: 250_000,
        };
        let auth = vec![xdr::SorobanAuthorizationEntry {
            credentials: xdr::SorobanCredentials::Address(xdr::SorobanAddressCredentials {
                address: engine.clone(),
                nonce: 42,
                signature_expiration_ledger: 110,
                signature: ScVal::Vec(None),
            }),
            root_invocation: xdr::SorobanAuthorizedInvocation {
                function: xdr::SorobanAuthorizedFunction::ContractFn(xdr::InvokeContractArgs {
                    contract_address: contract(),
                    function_name: "attest".try_into().unwrap(),
                    args: xdr::VecM::default(),
                }),
                sub_invocations: xdr::VecM::default(),
            },
        }];
        (data, auth)
    }

    #[test]
    fn network_id_is_the_sha256_of_the_passphrase() {
        // Constante de testnet, calculada de forma independiente; si el algoritmo
        // cambiara, toda transaccion firmada seria rechazada con BAD_AUTH.
        assert_eq!(
            hex::encode(network_id_hash(TESTNET_PASSPHRASE)),
            "cee0302d59844d32bdca915c8203dd44b33fbb7edc19051ea37abedf28ecd472"
        );
    }

    #[test]
    fn builds_an_invoke_of_attest_with_the_two_expected_args() {
        let tx = build_invoke_tx(
            &contract(),
            &source(),
            7,
            AttestationOutcome::Fail,
            &[9u8; 32],
        )
        .unwrap();
        assert_eq!(tx.seq_num.0, 7);
        assert_eq!(tx.operations.len(), 1);
        assert!(
            matches!(tx.ext, xdr::TransactionExt::V0),
            "aun sin recursos de simulacion"
        );

        let invoke = invoke_of(&tx);
        let xdr::HostFunction::InvokeContract(args) = &invoke.host_function else {
            panic!("se esperaba InvokeContract")
        };
        assert_eq!(args.function_name.to_string(), "attest");
        assert_eq!(args.contract_address, contract());
        assert_eq!(args.args.len(), 2);
        assert_outcome_arg(args.args.first().unwrap(), AttestationOutcome::Fail);
        assert_eq!(
            bytes_n_from_scval(args.args.last().unwrap()).unwrap(),
            [9u8; 32]
        );
    }

    #[test]
    fn unsigned_envelope_has_no_signatures() {
        let tx = build_invoke_tx(
            &contract(),
            &source(),
            1,
            AttestationOutcome::Pass,
            &[0u8; 32],
        )
        .unwrap();
        let envelope = unsigned_envelope(&tx);
        let xdr::TransactionEnvelope::Tx(v1) = &envelope else {
            panic!("esperaba V1")
        };
        assert!(v1.signatures.is_empty());
    }

    #[test]
    fn apply_simulation_installs_resources_auth_and_the_resource_fee() {
        let mut tx = build_invoke_tx(
            &contract(),
            &source(),
            1,
            AttestationOutcome::Pass,
            &[0u8; 32],
        )
        .unwrap();
        let engine: xdr::ScAddress = signer().address().parse().unwrap();
        let (data, auth) = simulation_for(&engine);
        let resource_fee = data.resource_fee;
        apply_simulation(&mut tx, data, auth).unwrap();
        let xdr::TransactionExt::V1(installed) = &tx.ext else {
            panic!("esperaba sorobanData")
        };
        assert_eq!(installed.resources.instructions, 1_000);
        // Las credenciales van en la operacion, no en sorobanData (protocolo 23).
        assert_eq!(invoke_of(&tx).auth.len(), 1);
        assert_eq!(tx.fee, BASE_FEE_STROOPS + resource_fee as u32);
    }

    #[test]
    fn signature_hint_is_the_tail_of_the_public_key() {
        // Fijado porque la red no avisa de que el hint este mal: solo devuelve
        // TxBadAuth, que no dice si el problema es la firma o como se busco la clave.
        let s = signer();
        let mut tx = build_invoke_tx(
            &xdr::ScAddress::Contract(xdr::ContractId(xdr::Hash([3u8; 32]))),
            &xdr::MuxedAccount::Ed25519(s.public_key_bytes().into()),
            1,
            AttestationOutcome::Pass,
            &[9u8; 32],
        )
        .unwrap();
        let mut op = tx.operations.first().unwrap().clone();
        let xdr::OperationBody::InvokeHostFunction(invoke) = &mut op.body else {
            panic!()
        };
        invoke.auth =
            xdr::VecM::try_from(vec![auth_entry(creds_v2(s.address().parse().unwrap()))]).unwrap();
        tx.operations = xdr::VecM::try_from(vec![op]).unwrap();

        let signed =
            sign_envelope(&s, &tx, &network_id_hash(TESTNET_PASSPHRASE), ATTEST_FN).unwrap();
        let xdr::TransactionEnvelope::Tx(v1) = &signed else {
            panic!("esperaba V1")
        };
        let pk = s.public_key_bytes();
        let hint = v1.signatures.first().unwrap().hint.0;
        assert_eq!(hint, pk[pk.len() - 4..]);
        assert_ne!(
            hint,
            pk[..4],
            "el hint no debe ser el prefijo: eso fue lo que rompia el envio"
        );
    }

    #[test]
    fn sign_envelope_signs_the_payload_the_network_will_hash() {
        use ed25519_dalek::Verifier;
        use sha2::{Digest, Sha256};

        let s = signer();
        let engine: xdr::ScAddress = s.address().parse().unwrap();
        let mut tx = build_invoke_tx(
            &contract(),
            &source(),
            1,
            AttestationOutcome::Pass,
            &[0u8; 32],
        )
        .unwrap();
        let (data, auth) = simulation_for(&engine);
        apply_simulation(&mut tx, data, auth).unwrap();
        let signed =
            sign_envelope(&s, &tx, &network_id_hash(TESTNET_PASSPHRASE), ATTEST_FN).unwrap();

        let xdr::TransactionEnvelope::Tx(v1) = &signed else {
            panic!("esperaba V1")
        };
        assert_eq!(v1.signatures.len(), 1);
        let decorated = v1.signatures.first().unwrap();
        // El hint son los ultimos 4 bytes de la pubkey. Es lo que usa el nodo de
        // protocolo 28 para indexar la firma: con el prefijo la red busca una clave
        // que no existe y responde TxBadAuth aunque la firma sea valida.
        let pk = s.public_key_bytes();
        assert_eq!(decorated.hint.0, pk[pk.len() - 4..]);

        // La firma cubre el payload de la transaccion **con la firma todavia vacia**:
        // ese es el estado en el que la red la verifica.
        let mut pre_signature = v1.tx.clone();
        for op in pre_signature.operations.iter_mut() {
            let xdr::OperationBody::InvokeHostFunction(invoke) = &mut op.body else {
                continue;
            };
            for entry in invoke.auth.iter_mut() {
                if let xdr::SorobanCredentials::Address(c) | xdr::SorobanCredentials::AddressV2(c) =
                    &mut entry.credentials
                {
                    c.signature = ScVal::Vec(None);
                }
            }
        }
        let payload = xdr::TransactionSignaturePayload {
            network_id: network_id_hash(TESTNET_PASSPHRASE),
            tagged_transaction: xdr::TransactionSignaturePayloadTaggedTransaction::Tx(
                pre_signature,
            ),
        };
        let expected = Sha256::digest(payload.to_xdr(Limits::none()).unwrap());
        let sig = ed25519_dalek::Signature::from_slice(&decorated.signature.0).unwrap();
        let vk = ed25519_dalek::VerifyingKey::from_bytes(&s.public_key_bytes()).unwrap();
        assert!(
            vk.verify(&expected, &sig).is_ok(),
            "la firma no valida sobre el payload reconstruido"
        );

        // Y la credencial de auth lleva exactamente los mismos 64 bytes.
        let credentials = match invoke_of(&v1.tx).auth.first().unwrap().credentials {
            xdr::SorobanCredentials::Address(ref c) => c.signature.clone(),
            ref other => panic!("credenciales inesperadas: {other:?}"),
        };
        let ScVal::Vec(Some(items)) = credentials else {
            panic!("la firma de auth deberia ser un Vec")
        };
        let ScVal::Bytes(auth_bytes) = items.first().expect("firma vacia") else {
            panic!("la firma de auth deberia ser Bytes")
        };
        let auth_bytes: &[u8] = auth_bytes.0.as_ref();
        let decorated_bytes: &[u8] = decorated.signature.0.as_ref();
        assert_eq!(auth_bytes, decorated_bytes);
    }

    #[test]
    fn sign_envelope_fails_when_simulation_omits_the_engine_credentials() {
        let s = signer();
        // Otra cuenta valida, para que el fallo sea "no soy yo" y no "direccion invalida".
        let other_signer =
            EngineSigner::from_secret(&"07".repeat(32), crate::signer::KeySource::EnvVar).unwrap();
        let other: xdr::ScAddress = other_signer.address().parse().unwrap();
        assert_ne!(other, s.address().parse::<xdr::ScAddress>().unwrap());

        let mut tx = build_invoke_tx(
            &contract(),
            &source(),
            1,
            AttestationOutcome::Pass,
            &[0u8; 32],
        )
        .unwrap();
        let (data, auth) = simulation_for(&other);
        apply_simulation(&mut tx, data, auth).unwrap();
        let err =
            sign_envelope(&s, &tx, &network_id_hash(TESTNET_PASSPHRASE), ATTEST_FN).unwrap_err();
        assert!(
            err.to_string().contains("no devolvio credenciales"),
            "{err}"
        );
    }

    #[test]
    fn expected_states_match_the_contract_enum() {
        assert_eq!(
            expected_state_after(AttestationOutcome::Pass),
            EscrowState::AttestedPass
        );
        assert_eq!(
            expected_state_after(AttestationOutcome::Fail),
            EscrowState::AttestedFail
        );
    }

    // --- Confirmacion del evento Attested -----------------------------------
    //
    // Los eventos se construyen con la misma forma que emite el SDK 27: topic
    // `attested` y un `Map` con `outcome` (enum como Vec[Symbol]) y `report_hash`.

    const TEST_CONTRACT: [u8; 32] = [5u8; 32];

    fn event_data(outcome: &str, report_hash: [u8; 32]) -> ScVal {
        let entries: Vec<(ScVal, ScVal)> = vec![
            (
                ScVal::Symbol("outcome".try_into().unwrap()),
                ScVal::Vec(Some(xdr::ScVec(
                    vec_one(ScVal::Symbol(outcome.try_into().unwrap())).unwrap(),
                ))),
            ),
            (
                ScVal::Symbol("report_hash".try_into().unwrap()),
                ScVal::Bytes(xdr::ScBytes(
                    xdr::BytesM::try_from(report_hash.to_vec()).unwrap(),
                )),
            ),
        ];
        ScVal::Map(Some(
            xdr::ScMap::sorted_from_pairs(entries.into_iter()).unwrap(),
        ))
    }

    fn attested_event(
        contract: [u8; 32],
        outcome: &str,
        report_hash: [u8; 32],
    ) -> xdr::ContractEvent {
        xdr::ContractEvent {
            ext: xdr::ExtensionPoint::V0,
            type_: xdr::ContractEventType::Contract,
            contract_id: Some(xdr::ContractId(xdr::Hash(contract))),
            body: xdr::ContractEventBody::V0(xdr::ContractEventV0 {
                topics: xdr::VecM::try_from(vec![ScVal::Symbol(
                    ATTESTED_TOPIC.try_into().unwrap(),
                )])
                .unwrap(),
                data: event_data(outcome, report_hash),
            }),
        }
    }

    #[test]
    fn accepts_matching_attested_event() {
        let ev = attested_event(TEST_CONTRACT, "Pass", [9u8; 32]);
        find_attested_event(&[ev], &TEST_CONTRACT, AttestationOutcome::Pass, &[9u8; 32]).unwrap();
    }

    #[test]
    fn accepts_fail_event() {
        let ev = attested_event(TEST_CONTRACT, "Fail", [3u8; 32]);
        find_attested_event(&[ev], &TEST_CONTRACT, AttestationOutcome::Fail, &[3u8; 32]).unwrap();
    }

    #[test]
    fn rejects_event_from_another_contract() {
        // Mismo simbolo y mismos datos, pero de otro contrato: no es nuestra
        // atestacion aunque parezca identica.
        let ev = attested_event([6u8; 32], "Pass", [9u8; 32]);
        let err = find_attested_event(&[ev], &TEST_CONTRACT, AttestationOutcome::Pass, &[9u8; 32])
            .unwrap_err();
        assert!(format!("{err}").contains("no emitio"));
    }

    #[test]
    fn rejects_wrong_report_hash() {
        // Es el lazo con el reporte determinista: si el hash del evento no es el
        // firmado, la atestacion no es la que se evaluo.
        let ev = attested_event(TEST_CONTRACT, "Pass", [9u8; 32]);
        let err = find_attested_event(&[ev], &TEST_CONTRACT, AttestationOutcome::Pass, &[1u8; 32])
            .unwrap_err();
        assert!(format!("{err}").contains("report_hash distinto"));
    }

    #[test]
    fn rejects_wrong_outcome() {
        let ev = attested_event(TEST_CONTRACT, "Fail", [9u8; 32]);
        let err = find_attested_event(&[ev], &TEST_CONTRACT, AttestationOutcome::Pass, &[9u8; 32])
            .unwrap_err();
        assert!(format!("{err}").contains("outcome distinto"));
    }

    #[test]
    fn rejects_missing_event() {
        let err = find_attested_event(&[], &TEST_CONTRACT, AttestationOutcome::Pass, &[9u8; 32])
            .unwrap_err();
        assert!(format!("{err}").contains("no se puede"));
    }

    #[test]
    fn ignores_unrelated_topics() {
        // Un `Funded` o `EvidenceSubmitted` no debe confundirse con `Attested`.
        let mut ev = attested_event(TEST_CONTRACT, "Pass", [9u8; 32]);
        let xdr::ContractEventBody::V0(body) = &mut ev.body;
        body.topics = xdr::VecM::try_from(vec![ScVal::Symbol(
            "evidence_submitted".try_into().unwrap(),
        )])
        .unwrap();
        assert!(
            find_attested_event(&[ev], &TEST_CONTRACT, AttestationOutcome::Pass, &[9u8; 32])
                .is_err()
        );
    }

    #[test]
    fn rejects_event_with_extra_topics() {
        // Si el evento gana un campo `#[topic]`, la forma cambio y hay que revisarlo.
        let mut ev = attested_event(TEST_CONTRACT, "Pass", [9u8; 32]);
        let xdr::ContractEventBody::V0(body) = &mut ev.body;
        let mut topics = body.topics.to_vec();
        topics.push(ScVal::U32(1));
        body.topics = xdr::VecM::try_from(topics).unwrap();
        let err = find_attested_event(&[ev], &TEST_CONTRACT, AttestationOutcome::Pass, &[9u8; 32])
            .unwrap_err();
        assert!(format!("{err}").contains("1 topic"));
    }

    #[test]
    fn rejects_event_with_unknown_outcome_name() {
        let ev = attested_event(TEST_CONTRACT, "Maybe", [9u8; 32]);
        let err = find_attested_event(&[ev], &TEST_CONTRACT, AttestationOutcome::Pass, &[9u8; 32])
            .unwrap_err();
        assert!(format!("{err}").contains("desconocido"));
    }

    // --- CAP-71-02: credenciales de direccion v2 -----------------------------
    //
    // Las credenciales v1 y v2 se parecen en el tipo pero NO en el payload de
    // firma. Compartirlos es un error silencioso que la red manifiesta como
    // txBAD_AUTH mucho despues, cuando ya se creia que la transaccion era valida.

    /// Entrada de autorizacion con credenciales de la variante pedida.
    fn auth_entry(creds: xdr::SorobanCredentials) -> xdr::SorobanAuthorizationEntry {
        xdr::SorobanAuthorizationEntry {
            credentials: creds,
            root_invocation: xdr::SorobanAuthorizedInvocation {
                function: xdr::SorobanAuthorizedFunction::ContractFn(xdr::InvokeContractArgs {
                    contract_address: xdr::ScAddress::Contract(xdr::ContractId(xdr::Hash(
                        [3u8; 32],
                    ))),
                    function_name: "attest".try_into().unwrap(),
                    args: xdr::VecM::default(),
                }),
                sub_invocations: xdr::VecM::default(),
            },
        }
    }

    fn creds_v2(address: xdr::ScAddress) -> xdr::SorobanCredentials {
        xdr::SorobanCredentials::AddressV2(xdr::SorobanAddressCredentials {
            address,
            nonce: 7,
            signature_expiration_ledger: 100,
            signature: ScVal::Void,
        })
    }

    /// Extrae la firma que quedo en las credenciales.
    fn stored_signature(creds: &xdr::SorobanCredentials) -> ScVal {
        match creds {
            xdr::SorobanCredentials::Address(c) | xdr::SorobanCredentials::AddressV2(c) => {
                c.signature.clone()
            }
            _ => panic!("no son credenciales de direccion"),
        }
    }

    /// Bytes de la firma guardada, para compararla con una firma calculada.
    fn signature_bytes(sig: &ScVal) -> [u8; 64] {
        let ScVal::Vec(Some(items)) = sig else {
            panic!("la firma no es Vec")
        };
        let ScVal::Bytes(b) = &items.0[0] else {
            panic!("la firma no es Bytes")
        };
        let mut out = [0u8; 64];
        out.copy_from_slice(b.0.as_ref());
        out
    }

    #[test]
    fn v1_and_v2_credentials_produce_different_signatures() {
        // Si ambos dieran la misma firma, el agente estaria firmando v2 con el
        // payload de la transaccion: exactamente el fallo que describe CAP-71-02.
        let s = signer();
        let address: xdr::ScAddress = s.address().parse().unwrap();
        let network_id = xdr::Hash([1u8; 32]);

        let entry_v2 = auth_entry(creds_v2(address.clone()));
        let xdr::SorobanCredentials::AddressV2(creds_v2c) = &entry_v2.credentials else {
            panic!("se esperaban credenciales v2")
        };
        let v1 = legacy_auth_signature(&s, &[9u8; 32]).unwrap();
        let v2 = v2_auth_signature(&s, &network_id, creds_v2c, &entry_v2.root_invocation).unwrap();

        assert_ne!(
            signature_bytes(&v1),
            signature_bytes(&v2),
            "v1 y v2 no pueden firmarse con el mismo payload"
        );
    }

    #[test]
    fn v2_signature_is_over_the_address_bound_preimage() {
        // La firma v2 debe verificar contra sha256(HashIDPreimage) y NO contra el
        // payload de la transaccion.
        use ed25519_dalek::Verifier;
        let s = signer();
        let address: xdr::ScAddress = s.address().parse().unwrap();
        let network_id = xdr::Hash([4u8; 32]);
        let entry = auth_entry(creds_v2(address.clone()));
        let xdr::SorobanCredentials::AddressV2(creds) = &entry.credentials else {
            panic!()
        };
        let sig = v2_auth_signature(&s, &network_id, creds, &entry.root_invocation).unwrap();
        let bytes = signature_bytes(&sig);

        let preimage = xdr::HashIdPreimage::SorobanAuthorizationWithAddress(
            xdr::HashIdPreimageSorobanAuthorizationWithAddress {
                network_id: network_id.clone(),
                nonce: creds.nonce,
                signature_expiration_ledger: creds.signature_expiration_ledger,
                address: address.clone(),
                invocation: entry.root_invocation.clone(),
            },
        );
        use sha2::{Digest, Sha256};
        let digest = Sha256::digest(
            preimage
                .to_xdr(Limits::none())
                .expect("preimage serializable"),
        );
        let vk = ed25519_dalek::VerifyingKey::from_bytes(&s.public_key_bytes()).unwrap();
        assert!(
            vk.verify(&digest, &ed25519_dalek::Signature::from_bytes(&bytes))
                .is_ok(),
            "la firma v2 debe verificar contra el preimage con direccion"
        );
    }

    #[test]
    fn signing_an_envelope_with_v2_credentials_installs_the_v2_signature() {
        let s = signer();
        let address: xdr::ScAddress = s.address().parse().unwrap();
        let mut tx = build_invoke_tx(
            &xdr::ScAddress::Contract(xdr::ContractId(xdr::Hash([3u8; 32]))),
            &xdr::MuxedAccount::Ed25519(s.public_key_bytes().into()),
            1,
            AttestationOutcome::Pass,
            &[9u8; 32],
        )
        .unwrap();
        // Se anteponen credenciales v2 devueltas por una simulacion.
        let mut op = tx.operations.first().unwrap().clone();
        let xdr::OperationBody::InvokeHostFunction(invoke) = &mut op.body else {
            panic!()
        };
        invoke.auth = xdr::VecM::try_from(vec![auth_entry(creds_v2(address))]).unwrap();
        tx.operations = xdr::VecM::try_from(vec![op]).unwrap();

        let network_id = xdr::Hash([1u8; 32]);
        let envelope = sign_envelope(&s, &tx, &network_id, ATTEST_FN).unwrap();
        let xdr::TransactionEnvelope::Tx(v1) = envelope else {
            panic!()
        };
        let op = v1.tx.operations.first().unwrap();
        let xdr::OperationBody::InvokeHostFunction(invoke) = &op.body else {
            panic!()
        };
        let stored = stored_signature(&invoke.auth.first().unwrap().credentials);
        assert_ne!(stored, ScVal::Void, "la credencial debe quedar firmada");
    }
}
