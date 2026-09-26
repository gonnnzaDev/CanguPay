//! Utilidad de desarrollo: despliegue y montaje de un escrow en testnet.
//!
//! **Herramienta de pruebas, no parte del agente.** Ejecuta el camino completo:
//! sube el WASM del token de prueba y el del escrow, crea ambos contratos, y llama a
//! `initialize`, `mint`, `fund` y `submit_evidence` para dejar el contrato listo para
//! que el agente ateste.
//!
//! Reutiliza la firma de `attestation_agent::attest`, de modo que la clave se maneja
//! con el mismo codigo que el motor en produccion.
//!
//! Uso:
//!   cargo run --release --example testnet_setup -p attestation-agent -- <claves> <bundle>
use std::collections::HashMap;
use std::str::FromStr;

use attestation_agent::attest::{network_id_hash, signature_payload_hash, unsigned_envelope};
use attestation_agent::signer::{EngineSigner, KeySource};
use sha2::{Digest, Sha256};
use stellar_rpc_client::Client;
use stellar_xdr as xdr;

const RPC: &str = "https://soroban-testnet.stellar.org";
const NETWORK: &str = "Test SDF Network ; September 2015";
const ESCROW_WASM: &str = "target/wasm32v1-none/release/conditional_payment.wasm";
const TOKEN_WASM: &str = "target/wasm32v1-none/release/test_token.wasm";
const TX_SUCCESS: &str = "SUCCESS";

fn main() {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("runtime");
    if let Err(e) = rt.block_on(run()) {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}

async fn run() -> Result<(), String> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.len() < 2 {
        return Err("uso: testnet_setup <archivo de claves> <bundle.json>".into());
    }
    let keys = parse_keys(&read(&args[0])?);
    // El bundle se arma con el mismo lector que usan los fixtures y `selfcheck`, de
    // modo que la evidencia que sube el proveedor sea exactamente la que evalua el motor.
    let bundle: serde_json::Value =
        attestation_agent::golden::load_case(&attestation_agent::golden::fixture_dir(), &args[1])
            .map_err(|e| e.to_string())?;

    // El hash del bundle lo calcula el motor de produccion, no esta herramienta.
    let amount = bundle["purchase_order"]["amount"].as_i64().unwrap_or(0) as i128;
    let report = attestation_agent::report::build_report(&bundle, amount, "CPUSD")
        .map_err(|e| e.to_string())?;
    println!("BUNDLE {} verdict={}", args[1], report.result);
    println!("BUNDLE_REPORT_HASH {}", report.report_hash);
    println!("BUNDLE_HASH {}", report.evidence_bundle_hash);

    let client = Client::new(RPC).map_err(|e| e.to_string())?;
    let deployer = keys["deployer"].clone();
    let deployer_address = address_of(&deployer);

    // 1. Subir los dos WASM.
    let token_wasm = read_wasm(TOKEN_WASM)?;
    let escrow_wasm = read_wasm(ESCROW_WASM)?;
    let token_wasm_hash = upload(&client, &deployer, &token_wasm, "test-token.wasm").await?;
    println!("WASM_HASH test-token {}", hex::encode(token_wasm_hash));
    let escrow_wasm_hash =
        upload(&client, &deployer, &escrow_wasm, "conditional_payment.wasm").await?;
    println!(
        "WASM_HASH conditional_payment {}",
        hex::encode(escrow_wasm_hash)
    );

    // 2. Crear el token de prueba. El constructor no recibe argumentos: la
    //    inicializacion va aparte para que el despliegue sea reproducible.
    let token = create(
        &client,
        &deployer,
        &deployer_address,
        "tstusd",
        token_wasm_hash,
        &[],
    )
    .await?;
    println!("CONTRACT_ID token {}", token);

    let hash = call(
        &client,
        &deployer,
        &token,
        "initialize",
        &[address(&keys["buyer"])],
        "token.initialize",
    )
    .await?;
    println!("TXGEN token_initialize {hash}");

    let hash = call(
        &client,
        &deployer,
        &token,
        "mint",
        &[address(&keys["buyer"]), i128(amount)],
        "token.mint",
    )
    .await?;
    println!("TXGEN token_mint {hash}");

    // 3. Crear el escrow.
    let escrow = create(
        &client,
        &deployer,
        &deployer_address,
        "canguusd",
        escrow_wasm_hash,
        &[],
    )
    .await?;
    println!("CONTRACT_ID escrow {escrow}");

    // 4. initialize(config) lo firma el buyer.
    let config = build_config(&keys, &token, amount);
    let hash = call(
        &client,
        &keys["buyer"],
        &escrow,
        "initialize",
        &[config],
        "escrow.initialize",
    )
    .await?;
    println!("TXGEN escrow_initialize {hash}");

    // 5. fund() lo firma el buyer y mueve los tokens al contrato.
    let hash = call(&client, &keys["buyer"], &escrow, "fund", &[], "escrow.fund").await?;
    println!("TXGEN escrow_fund {hash}");

    // 6. submit_evidence() lo firma el supplier con el hash del bundle.
    let evidence = hex::decode(&report.evidence_bundle_hash).map_err(|e| e.to_string())?;
    let hash = call(
        &client,
        &keys["supplier"],
        &escrow,
        "submit_evidence",
        &[bytes(&evidence)],
        "escrow.submit_evidence",
    )
    .await?;
    println!("TXGEN escrow_submit_evidence {hash}");

    println!("CANGUPA_CONTRACT_ID={escrow}");
    Ok(())
}

// ---------------------------------------------------------------------------
// Operaciones de cadena
// ---------------------------------------------------------------------------

/// Sube un WASM y devuelve su hash.
async fn upload(
    client: &Client,
    secret: &str,
    wasm: &[u8],
    label: &str,
) -> Result<[u8; 32], String> {
    let op = xdr::Operation {
        source_account: None,
        body: xdr::OperationBody::InvokeHostFunction(xdr::InvokeHostFunctionOp {
            host_function: xdr::HostFunction::UploadContractWasm(
                xdr::BytesM::try_from(wasm.to_vec()).map_err(|e| e.to_string())?,
            ),
            auth: xdr::VecM::default(),
        }),
    };
    let hash = submit(client, secret, vec![op], &format!("upload {label}")).await?;
    println!("TXGEN upload_{} {hash}", label.replace('.', "_"));
    Ok(sha256(wasm))
}

/// Crea un contrato a partir de un WASM ya subido y devuelve su direccion `C...`.
async fn create(
    client: &Client,
    secret: &str,
    deployer: &xdr::ScAddress,
    salt_seed: &str,
    wasm_hash: [u8; 32],
    ctor: &[xdr::ScVal],
) -> Result<String, String> {
    // El preimage lleva un salt derivado de una cadena legible, lo que hace el
    // despliegue reproducible: las mismas entradas dan la misma direccion.
    let preimage = xdr::ContractIdPreimage::Address(xdr::ContractIdPreimageFromAddress {
        address: deployer.clone(),
        salt: xdr::Uint256(sha256(salt_seed.as_bytes())),
    });
    let op = xdr::Operation {
        source_account: None,
        body: xdr::OperationBody::InvokeHostFunction(xdr::InvokeHostFunctionOp {
            host_function: xdr::HostFunction::CreateContractV2(xdr::CreateContractArgsV2 {
                contract_id_preimage: preimage.clone(),
                executable: xdr::ContractExecutable::Wasm(xdr::Hash(wasm_hash)),
                constructor_args: xdr::VecM::try_from(ctor.to_vec()).map_err(|e| e.to_string())?,
            }),
            auth: xdr::VecM::default(),
        }),
    };
    // La direccion se calcula con la misma formula del protocolo, y despues se
    // contrasta con la que devolvio la transaccion: si no coinciden, no se publica.
    let derived = xdr::Hash(contract_id_of(&preimage, wasm_hash, ctor)?);
    let expected = format!("{}", stellar_strkey::Contract(derived.0));
    let (hash, result) =
        submit_full(client, secret, vec![op], &format!("create {salt_seed}")).await?;
    println!("TXGEN create_{salt_seed} {hash}");
    let actual = result
        .as_ref()
        .and_then(|v| match v {
            xdr::ScVal::Address(a) => Some(format!("{a}")),
            _ => None,
        })
        .ok_or_else(|| format!("create {salt_seed}: la transaccion no devolvio una direccion"))?;
    if actual != expected {
        return Err(format!(
            "create {salt_seed}: la cadena dio {actual} y el calculo local {expected}"
        ));
    }
    println!("CONTRACT_DERIVED {salt_seed} {expected}");
    Ok(actual)
}

/// Invoca una funcion y devuelve el hash de la transaccion.
async fn call(
    client: &Client,
    secret: &str,
    contract: &str,
    fn_name: &str,
    args: &[xdr::ScVal],
    label: &str,
) -> Result<String, String> {
    let op = xdr::Operation {
        source_account: None,
        body: xdr::OperationBody::InvokeHostFunction(xdr::InvokeHostFunctionOp {
            host_function: xdr::HostFunction::InvokeContract(xdr::InvokeContractArgs {
                contract_address: parse_contract(contract)?,
                function_name: fn_name.try_into().map_err(|_| "simbolo invalido")?,
                args: xdr::VecM::try_from(args.to_vec()).map_err(|e| e.to_string())?,
            }),
            auth: xdr::VecM::default(),
        }),
    };
    let hash = submit(client, secret, vec![op], label).await?;
    println!("TXGEN {label} {hash}");
    Ok(hash)
}

/// Simula, firma, envia y espera confirmacion. Devuelve solo el hash.
async fn submit(
    client: &Client,
    secret: &str,
    ops: Vec<xdr::Operation>,
    label: &str,
) -> Result<String, String> {
    submit_full(client, secret, ops, label)
        .await
        .map(|(h, _)| h)
}

/// Igual que [`submit`], pero tambien devuelve el valor de retorno del host.
async fn submit_full(
    client: &Client,
    secret: &str,
    ops: Vec<xdr::Operation>,
    label: &str,
) -> Result<(String, Option<xdr::ScVal>), String> {
    let address = address_of(secret);
    let account = client
        .get_account(&address.to_string())
        .await
        .map_err(|e| format!("{label}: getAccount: {e}"))?;
    let signer = EngineSigner::from_secret(secret_of(secret), KeySource::KeyFile)
        .map_err(|e| e.to_string())?;
    let mut tx = xdr::Transaction {
        source_account: xdr::MuxedAccount::Ed25519(signer.public_key_bytes().into()),
        fee: 1000,
        seq_num: xdr::SequenceNumber(
            std::env::var("CANGUPA_SEQ_OVERRIDE")
                .ok()
                .and_then(|v| v.parse::<i64>().ok())
                .unwrap_or(account.seq_num.0),
        ),
        cond: xdr::Preconditions::default(),
        memo: xdr::Memo::None,
        operations: xdr::VecM::try_from(ops).map_err(|e| e.to_string())?,
        ext: xdr::TransactionExt::V0,
    };

    eprintln!(
        "DEBUG {label}: account seq={} tx seq={}",
        account.seq_num.0, tx.seq_num.0
    );
    let simulation = client
        .simulate_transaction_envelope(
            &unsigned_envelope(&tx),
            Some(stellar_rpc_client::AuthMode::Record),
        )
        .await
        .map_err(|e| format!("{label}: simulate: {e}"))?;
    let data = simulation
        .transaction_data()
        .map_err(|e| format!("{label}: {e}"))?;
    let auth: Vec<xdr::SorobanAuthorizationEntry> = simulation
        .results()
        .map_err(|e| format!("{label}: {e}"))?
        .into_iter()
        .flat_map(|r| r.auth)
        .collect();
    let return_value = simulation
        .results()
        .ok()
        .and_then(|mut r| r.pop())
        .map(|r| r.xdr);

    tx.fee = tx
        .fee
        .saturating_add(u32::try_from(data.resource_fee).unwrap_or(u32::MAX));
    tx.ext = xdr::TransactionExt::V1(data);
    // Las credenciales que devolvio la simulacion van en la operacion: sin ellas,
    // cualquier funcion con `require_auth` se rechaza.
    install_auth(&mut tx, &address, secret, &auth)?;
    let envelope = sign_envelope(secret, &tx)?;

    let hash = match client.send_transaction(&envelope).await {
        Ok(h) => h,
        Err(e) if format!("{e}").contains("TxBadSeq") => {
            // El RPC puede devolver una entrada de cuenta de un ledger anterior al que
            // la creo, asi que la secuencia valida es la siguiente. Se relee y se
            // reintenta una vez en vez de asumir que la red esta rota.
            let fresh = client
                .get_account(&address.to_string())
                .await
                .map_err(|e| format!("{label}: relectura de cuenta: {e}"))?;
            let mut retry = tx.clone();
            retry.seq_num = xdr::SequenceNumber(fresh.seq_num.0 + 1);
            let envelope = sign_full(secret, &retry, &address, &auth)?;
            client.send_transaction(&envelope).await.map_err(|e2| {
                format!(
                    "{label}: sendTransaction (reintento con seq {}): {e2}",
                    fresh.seq_num.0 + 1
                )
            })?
        }
        Err(e) => return Err(format!("{label}: sendTransaction: {e}")),
    };
    let response = client
        .get_transaction_polling(&hash, None)
        .await
        .map_err(|e| format!("{label}: getTransaction: {e}"))?;
    if response.status != TX_SUCCESS {
        return Err(format!(
            "{label}: estado {}{}",
            response.status,
            failure_detail(&response)
        ));
    }
    Ok((hash.to_string(), return_value))
}

/// Instala en la operacion las credenciales de Soroban firmadas por la cuenta.
///
/// Si la simulacion no devolvio ninguna credencial para esta cuenta, la funcion no
/// usaba `auth` y basta con firmar el sobre: asi se despliegan contratos y se llaman
/// funciones publicas.
fn install_auth(
    tx: &mut xdr::Transaction,
    address: &xdr::ScAddress,
    secret: &str,
    auth: &[xdr::SorobanAuthorizationEntry],
) -> Result<(), String> {
    if auth.is_empty() {
        return Ok(());
    }
    let signer = EngineSigner::from_secret(secret_of(secret), KeySource::KeyFile)
        .map_err(|e| e.to_string())?;
    let network_id = network_id_hash(NETWORK);
    let payload_hash = signature_payload_hash(&network_id, tx).map_err(|e| e.to_string())?;
    let sig = attestation_agent::attest::signature_scval(&signer, &payload_hash)
        .map_err(|e| e.to_string())?;
    for op in tx.operations.iter_mut() {
        let xdr::OperationBody::InvokeHostFunction(invoke) = &mut op.body else {
            continue;
        };
        // Solo las invocaciones de contrato llevan credenciales. Ponerlas en una
        // subida de WASM o en una creacion de contrato produce un error de auth.
        if !matches!(
            invoke.host_function,
            xdr::HostFunction::InvokeContract(_) | xdr::HostFunction::CreateContractV2(_)
        ) {
            continue;
        }
        invoke.auth = xdr::VecM::try_from(auth.to_vec()).map_err(|e| e.to_string())?;
        for entry in invoke.auth.iter_mut() {
            if let xdr::SorobanCredentials::Address(c) | xdr::SorobanCredentials::AddressV2(c) =
                &mut entry.credentials
            {
                if c.address == *address {
                    c.signature = sig.clone();
                }
            }
        }
    }
    Ok(())
}

/// Firma el sobre y las credenciales de Soroban de una transaccion ya montada.
fn sign_full(
    secret: &str,
    tx: &xdr::Transaction,
    address: &xdr::ScAddress,
    auth: &[xdr::SorobanAuthorizationEntry],
) -> Result<xdr::TransactionEnvelope, String> {
    let mut copy = tx.clone();
    install_auth(&mut copy, address, secret, auth)?;
    sign_envelope(secret, &copy)
}

/// Firma el sobre de la transaccion con la cuenta fuente.
fn sign_envelope(secret: &str, tx: &xdr::Transaction) -> Result<xdr::TransactionEnvelope, String> {
    let signer = EngineSigner::from_secret(secret_of(secret), KeySource::KeyFile)
        .map_err(|e| e.to_string())?;
    let network_id = network_id_hash(NETWORK);
    let payload_hash = signature_payload_hash(&network_id, tx).map_err(|e| e.to_string())?;
    let hint = xdr::SignatureHint(signer.public_key_bytes()[..4].try_into().unwrap());
    let decorated = xdr::DecoratedSignature {
        hint,
        signature: xdr::Signature(
            xdr::BytesM::try_from(signer.sign(&payload_hash).to_vec())
                .map_err(|e| e.to_string())?,
        ),
    };
    Ok(xdr::TransactionEnvelope::Tx(xdr::TransactionV1Envelope {
        tx: tx.clone(),
        signatures: xdr::VecM::try_from(vec![decorated]).map_err(|e| e.to_string())?,
    }))
}

/// Diagnostico de una transaccion fallida: sin esto el error real se pierde.
fn failure_detail(response: &stellar_rpc_client::GetTransactionResponse) -> String {
    let mut out = Vec::new();
    for ev in response.events.diagnostic_events.iter() {
        out.push(format!("{:?}", ev.event));
    }
    if out.is_empty() {
        String::new()
    } else {
        format!(":: {}", out.join(" | "))
    }
}

// ---------------------------------------------------------------------------
// Construccion de argumentos
// ---------------------------------------------------------------------------

/// `EscrowConfig` como la serializa `#[contracttype]`: un `Vec` de 14 campos.
///
/// Plazos de una hora:Dan espacio para observar el contrato en el explorer sin que
/// venza nada a mitad de la demo.
fn build_config(keys: &HashMap<String, String>, token: &str, amount: i128) -> xdr::ScVal {
    let u = |v: u64| xdr::ScVal::U64(v);
    xdr::ScVal::Vec(Some(xdr::ScVec(
        xdr::VecM::try_from(vec![
            address(&keys["buyer"]),
            address(&keys["supplier"]),
            address(&keys["engine"]),
            address(&keys["resolver"]),
            xdr::ScVal::Address(parse_contract(token).expect("direccion de token valida")),
            i128(amount),
            u(3600),            // submission_period
            u(3600),            // attestation_period
            u(3600),            // objection_period
            u(3600),            // correction_period
            u(3600),            // resolution_period
            xdr::ScVal::U32(2), // fallback_outcome = Refund
            xdr::ScVal::U32(0), // fallback_split_bps
            xdr::ScVal::U32(1), // max_correction_attempts
        ])
        .expect("config"),
    )))
}

fn address(str_addr: &str) -> xdr::ScVal {
    xdr::ScVal::Address(xdr::ScAddress::from_str(str_addr).expect("direccion de cuenta valida"))
}

/// El archivo de claves guarda `direccion:secreto`; estas funciones separan las dos.
fn secret_of(entry: &str) -> &str {
    entry.split(':').nth(1).unwrap_or(entry)
}

fn address_of(secret: &str) -> xdr::ScAddress {
    xdr::ScAddress::from_str(secret.split(':').next().expect("direccion"))
        .expect("direccion valida")
}

fn parse_contract(str_addr: &str) -> Result<xdr::ScAddress, String> {
    xdr::ScAddress::from_str(str_addr).map_err(|e| format!("C... invalido '{str_addr}': {e}"))
}

fn i128(value: i128) -> xdr::ScVal {
    xdr::ScVal::I128(xdr::Int128Parts {
        hi: (value >> 64) as i64,
        lo: (value & u64::MAX as i128) as u64,
    })
}

fn bytes(raw: &[u8]) -> xdr::ScVal {
    xdr::ScVal::Bytes(xdr::ScBytes(
        xdr::BytesM::try_from(raw.to_vec()).expect("bytes validos"),
    ))
}

/// Direccion del contrato segun el protocolo: `sha256(network || preimage || executable || ctor)`.
fn contract_id_of(
    preimage: &xdr::ContractIdPreimage,
    wasm_hash: [u8; 32],
    ctor: &[xdr::ScVal],
) -> Result<[u8; 32], String> {
    use stellar_xdr::{Limits, WriteXdr};
    let network = xdr::Hash(sha256(NETWORK.as_bytes()));
    let executable = xdr::ContractExecutable::Wasm(xdr::Hash(wasm_hash));
    let args = xdr::ScVec(xdr::VecM::try_from(ctor.to_vec()).map_err(|e| e.to_string())?);
    let mut buf = Vec::new();
    network
        .write_xdr(&mut stellar_xdr::Limited::new(&mut buf, Limits::none()))
        .map_err(|e| e.to_string())?;
    preimage
        .write_xdr(&mut stellar_xdr::Limited::new(&mut buf, Limits::none()))
        .map_err(|e| e.to_string())?;
    executable
        .write_xdr(&mut stellar_xdr::Limited::new(&mut buf, Limits::none()))
        .map_err(|e| e.to_string())?;
    args.write_xdr(&mut stellar_xdr::Limited::new(&mut buf, Limits::none()))
        .map_err(|e| e.to_string())?;
    Ok(sha256(&buf))
}

fn sha256(data: &[u8]) -> [u8; 32] {
    let mut out = [0u8; 32];
    out.copy_from_slice(&Sha256::digest(data));
    out
}

fn read(path: &str) -> Result<String, String> {
    std::fs::read_to_string(path).map_err(|e| format!("no se pudo leer {path}: {e}"))
}

fn read_wasm(path: &str) -> Result<Vec<u8>, String> {
    std::fs::read(path).map_err(|e| format!("no se pudo leer {path}: {e}"))
}

fn parse_keys(text: &str) -> HashMap<String, String> {
    let mut out = HashMap::new();
    for line in text
        .lines()
        .filter(|l| l.contains('=') && !l.starts_with('#'))
    {
        let (name, rest) = line.split_once('=').expect("nombre=direccion:secreto");
        out.insert(name.to_string(), rest.to_string());
    }
    out
}
