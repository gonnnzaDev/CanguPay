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
    let outcome_val = ScVal::U32(outcome.as_u32());
    let hash_val = ScVal::Bytes(xdr::ScBytes(
        xdr::BytesM::try_from(report_hash.to_vec())
            .map_err(|e| AgentError::Xdr(format!("report_hash no cabe en BytesM: {e}")))?,
    ));
    Ok(vec![outcome_val, hash_val])
}

/// Nombre de la funcion del contrato.
pub const ATTEST_FN: &str = "attest";

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
/// Busca la entrada de auth que corresponde a la direccion del engine; si la simulacion
/// no la devolvio, es un error explicito y no un fallo silencioso.
pub fn sign_envelope(
    signer: &EngineSigner,
    tx: &xdr::Transaction,
    network_id: &xdr::Hash,
) -> Result<xdr::TransactionEnvelope> {
    let engine: xdr::ScAddress = signer
        .address()
        .parse::<xdr::ScAddress>()
        .map_err(|e| AgentError::Xdr(format!("direccion del engine invalida: {e}")))?;
    let payload_hash = signature_payload_hash(network_id, tx)?;
    let signature = signature_scval(signer, &payload_hash)?;

    let mut tx = tx.clone();
    let mut found = false;
    for op in tx.operations.iter_mut() {
        let xdr::OperationBody::InvokeHostFunction(invoke) = &mut op.body else {
            continue;
        };
        for entry in invoke.auth.iter_mut() {
            let credentials = match &mut entry.credentials {
                xdr::SorobanCredentials::Address(c) | xdr::SorobanCredentials::AddressV2(c) => c,
                _ => continue,
            };
            if credentials.address != engine {
                continue;
            }
            credentials.signature = signature.clone();
            found = true;
        }
    }
    if !found {
        return Err(AgentError::Network(format!(
            "la simulacion no devolvio credenciales para el engine {}; \
             no se puede firmar {ATTEST_FN}",
            signer.address()
        )));
    }

    let hint = xdr::SignatureHint(signer.public_key_bytes()[..4].try_into().unwrap());
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
            attestation_deadline: Some(1_000),
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

    #[test]
    fn attest_args_match_the_contract_signature() {
        let args = attest_args(AttestationOutcome::Fail, &[7u8; 32]).unwrap();
        // Dos ScVal sueltos, no una lista: el host los recibe por posicion.
        assert_eq!(args.len(), 2);
        assert_eq!(args[0], ScVal::U32(1));
        assert_eq!(bytes_n_from_scval(&args[1]).unwrap(), [7u8; 32]);
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
        assert_eq!(args.args.first(), Some(&ScVal::U32(1)));
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
        let signed = sign_envelope(&s, &tx, &network_id_hash(TESTNET_PASSPHRASE)).unwrap();

        let xdr::TransactionEnvelope::Tx(v1) = &signed else {
            panic!("esperaba V1")
        };
        assert_eq!(v1.signatures.len(), 1);
        let decorated = v1.signatures.first().unwrap();
        // El hint es el prefijo de 4 bytes de la pubkey, como espera el protocolo.
        assert_eq!(decorated.hint.0, s.public_key_bytes()[..4]);

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
        let err = sign_envelope(&s, &tx, &network_id_hash(TESTNET_PASSPHRASE)).unwrap_err();
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
}
