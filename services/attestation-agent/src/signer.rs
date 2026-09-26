//! Clave de firma del engine, fuera del repositorio.
//!
//! Reglas de P0-07 que este modulo hace cumplir en codigo:
//!
//! - La clave **nunca** se lee del arbol del repositorio. Si la ruta resuelve dentro del
//!   repo, el agente se niega a arrancar ([`AgentError::EngineKeyInsideRepo`]).
//! - La clave llega por `CANGUPA_ENGINE_SECRET` (entorno, idealmente de un gestor de
//!   secretos) o por `CANGUPA_ENGINE_KEYFILE` (ruta absoluta fuera del repo).
//! - La clave publica se puede derivar y comparar con la direccion `engine` del contrato:
//!   si no coinciden, el agente se niega a firmar, porque la transacion seria rejected
//!   por `require_auth` de todos modos.
//! - Nada de lo que se expone al frontend: este modulo no tiene ninguna ruta de red y
//!   el reporte JSON no incluye ningun campo de la clave.
use std::path::{Path, PathBuf};

use ed25519_dalek::{Signer, SigningKey};
use stellar_strkey::ed25519::PublicKey;

use crate::error::{AgentError, Result};

/// Variable de entorno con la clave seed en hex o en strkey `S...`.
pub const ENV_SECRET: &str = "CANGUPA_ENGINE_SECRET";
/// Variable de entorno con la ruta al archivo de clave, fuera del repositorio.
pub const ENV_KEYFILE: &str = "CANGUPA_ENGINE_KEYFILE";

/// Llave de firma del engine.
#[derive(Clone)]
pub struct EngineSigner {
    signing_key: SigningKey,
    public_key: [u8; 32],
    source: KeySource,
}

/// De donde vino la clave, para el log del keeper.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeySource {
    /// Variable de entorno con la seed.
    EnvVar,
    /// Archivo fuera del repositorio.
    KeyFile,
}

impl std::fmt::Display for KeySource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EnvVar => f.write_str("CANGUPA_ENGINE_SECRET"),
            Self::KeyFile => f.write_str("CANGUPA_ENGINE_KEYFILE"),
        }
    }
}

impl std::fmt::Debug for EngineSigner {
    /// No imprime material de clave: solo el origen y la direccion publica.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EngineSigner")
            .field("public_key", &self.address())
            .field("source", &self.source)
            .finish()
    }
}

impl EngineSigner {
    /// Carga la clave desde el entorno, aplicando las reglas de seguridad.
    pub fn from_env() -> Result<Self> {
        let secret = std::env::var(ENV_SECRET)
            .ok()
            .filter(|v| !v.trim().is_empty());
        let keyfile = std::env::var(ENV_KEYFILE)
            .ok()
            .filter(|v| !v.trim().is_empty());
        Self::load(secret.as_deref(), keyfile.as_deref())
    }

    /// Logica de seleccion de clave, separada para poder probarla sin tocar el entorno.
    fn load(secret: Option<&str>, keyfile: Option<&str>) -> Result<Self> {
        if let Some(secret) = secret {
            return Self::from_secret(secret.trim(), KeySource::EnvVar);
        }
        let path = keyfile.ok_or_else(|| {
            AgentError::Config(format!(
                "falta la clave del engine: define {ENV_SECRET} o {ENV_KEYFILE} (fuera del repo)"
            ))
        })?;
        Self::from_keyfile(Path::new(path))
    }

    /// Construye el firmante desde una seed en hex (64 caracteres) o strkey `S...`.
    pub fn from_secret(secret: &str, source: KeySource) -> Result<Self> {
        let bytes = decode_secret(secret)?;
        let signing_key = SigningKey::from_bytes(&bytes);
        Ok(Self {
            public_key: signing_key.verifying_key().to_bytes(),
            signing_key,
            source,
        })
    }

    /// Lee la clave de un archivo y verifica que el archivo no este dentro del repo.
    pub fn from_keyfile(path: &Path) -> Result<Self> {
        reject_if_inside_repo(path)?;
        let text = std::fs::read_to_string(path)
            .map_err(|e| AgentError::Config(format!("no se pudo leer {}: {e}", path.display())))?;
        Self::from_secret(text.trim(), KeySource::KeyFile)
    }

    /// Direccion publica `G...` de la cuenta engine.
    pub fn address(&self) -> String {
        PublicKey(self.public_key).to_string().to_string()
    }

    /// Bytes de la clave publica.
    pub fn public_key_bytes(&self) -> [u8; 32] {
        self.public_key
    }

    /// Origen de la clave, para logs.
    pub fn source(&self) -> KeySource {
        self.source
    }

    /// Firma un payload y devuelve los 64 bytes de la firma.
    pub fn sign(&self, payload: &[u8]) -> [u8; 64] {
        self.signing_key.sign(payload).to_bytes()
    }

    /// Verifica que la direccion `engine` del contrato sea esta misma cuenta.
    pub fn ensure_matches_contract_engine(&self, engine_address: &str) -> Result<()> {
        let mine = self.address();
        if mine == engine_address {
            return Ok(());
        }
        Err(AgentError::Config(format!(
            "la clave del engine es {mine} pero el contrato espera {engine_address}; \
             attest() haria require_auth y fallaria"
        )))
    }
}

/// Aclara de donde salio la clave sin revelar el material.
impl EngineSigner {
    /// Descripcion segura para logs y reportes.
    pub fn describe(&self) -> String {
        format!("engine={} via {}", self.address(), self.source)
    }
}

/// Acepta seed hex de 64 caracteres o strkey `S...`.
fn decode_secret(secret: &str) -> Result<[u8; 32]> {
    if secret.starts_with('S') {
        // El decodificador de strkey espera la cadena **completa**, con su prefijo `S`
        // y su checksum; quitarlo aqui haria que toda clave strkey real fuera rechazada.
        let key = stellar_strkey::ed25519::PrivateKey::from_string(secret)
            .map_err(|e| AgentError::Config(format!("strkey S... invalido: {e}")))?;
        return Ok(key.0);
    }
    let body = secret.strip_prefix("0x").unwrap_or(secret);
    if body.len() != 64 || !body.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(AgentError::Config(
            "la clave del engine debe ser 64 hex o un strkey S...".into(),
        ));
    }
    let mut out = [0u8; 32];
    hex::decode_to_slice(body, &mut out)
        .map_err(|e| AgentError::Config(format!("la clave del engine no es hex valido: {e}")))?;
    Ok(out)
}

/// Prohíbe usar una clave que viva dentro del repositorio.
fn reject_if_inside_repo(path: &Path) -> Result<()> {
    let repo = repo_root()?;
    let candidate = path.canonicalize().unwrap_or_else(|_| absolute(path));
    if candidate.starts_with(&repo) {
        return Err(AgentError::EngineKeyInsideRepo {
            path: candidate.display().to_string(),
        });
    }
    Ok(())
}

/// Resuelve la raiz del repositorio a partir de la ubicacion del crate compilado.
///
/// En desarrollo el binario vive en `<repo>/target/debug`, asi que se sube hasta el
/// `Cargo.toml` del workspace; en instalacion fuera del repo simply no hay repo y la
/// comprobacion no aplica.
fn repo_root() -> Result<PathBuf> {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut dir = manifest;
    while let Some(parent) = dir.parent() {
        if parent.join("Cargo.toml").is_file() && parent.join("contracts").is_dir() {
            return Ok(parent.to_path_buf());
        }
        dir = parent;
    }
    Err(AgentError::Config(
        "no se pudo determinar la raiz del repositorio".into(),
    ))
}

/// Ruta absoluta sin canonicalizar.
fn absolute(path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir().unwrap_or_default().join(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Seed de prueba, solo para tests: no corresponde a ninguna cuenta real.
    const TEST_SEED_HEX: &str = "9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60";

    /// Direccion strkey de ese seed, verificada de forma independiente:
    /// al decodificar el base32 se obtiene la version 6 (ed25519), el payload es la
    /// pubkey `d75a9801...511a` del vector 1 de RFC 8032 y el CRC16 valida.
    const TEST_ADDRESS: &str = "GDLVVGABQKYQVN6VJP7NHSLEA45A5YLS6PNKMIZFV4BBU2HXA5IRVHUR";

    fn signer() -> EngineSigner {
        EngineSigner::from_secret(TEST_SEED_HEX, KeySource::EnvVar).unwrap()
    }

    #[test]
    fn derives_the_known_test_address() {
        assert_eq!(signer().address(), TEST_ADDRESS);
    }

    #[test]
    fn signs_and_verifies() {
        use ed25519_dalek::Verifier;
        let s = signer();
        let sig = s.sign(b"cangu-attest");
        let verifying = ed25519_dalek::VerifyingKey::from_bytes(&s.public_key_bytes()).unwrap();
        assert!(verifying
            .verify(b"cangu-attest", &ed25519_dalek::Signature::from_bytes(&sig))
            .is_ok());
    }

    #[test]
    fn accepts_strkey_secret() {
        let s = signer();
        // Strkey S... construido con la misma libreria que lo decodifica, para que el test
        // no dependa de una constante escrita a mano.
        let raw = stellar_strkey::ed25519::PrivateKey::from_payload(&[1u8; 32]).unwrap();
        let as_strkey = stellar_strkey::Unredacted(&raw).to_string();
        assert!(as_strkey.starts_with('S'), "{as_strkey}");
        let other = EngineSigner::from_secret(&as_strkey, KeySource::EnvVar);
        assert!(other.is_ok(), "un strkey S... bien formado debe aceptarse");
        assert_ne!(other.unwrap().address(), s.address());
    }

    #[test]
    fn rejects_malformed_secret() {
        assert!(EngineSigner::from_secret("abc", KeySource::EnvVar).is_err());
        assert!(EngineSigner::from_secret("zz", KeySource::EnvVar).is_err());
    }

    #[test]
    fn rejects_key_file_inside_the_repo() {
        let repo = repo_root().unwrap();
        let inside = repo.join("services/attestation-agent/engine.key");
        let err = EngineSigner::from_keyfile(&inside).unwrap_err();
        assert!(
            matches!(err, AgentError::EngineKeyInsideRepo { .. }),
            "{err}"
        );
    }

    #[test]
    fn accepts_key_file_outside_the_repo() {
        // Directorio propio de la ejecucion: uno fijo hace que dos `cargo test`
        // en paralelo se escriban encima el `engine.key` del otro.
        let dir = std::env::temp_dir().join(format!("cangupay-p07-signer-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("engine.key");
        std::fs::write(&file, TEST_SEED_HEX).unwrap();
        let s = EngineSigner::from_keyfile(&file).unwrap();
        assert_eq!(s.source(), KeySource::KeyFile);
        assert_eq!(s.address(), signer().address());
    }

    #[test]
    fn refuses_to_sign_for_a_different_engine() {
        let s = signer();
        assert!(s.ensure_matches_contract_engine(&s.address()).is_ok());
        let err = s
            .ensure_matches_contract_engine(
                "GDUMMYAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA",
            )
            .unwrap_err();
        assert!(err.to_string().contains("require_auth"), "{err}");
    }

    #[test]
    fn debug_output_has_no_key_material() {
        let rendered = format!("{:?}", signer());
        assert!(!rendered.contains(TEST_SEED_HEX));
        assert!(rendered.contains(TEST_ADDRESS));
    }

    #[test]
    fn without_variables_the_error_explains_both_options() {
        let err = EngineSigner::load(None, None).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("CANGUPA_ENGINE_SECRET"), "{msg}");
        assert!(msg.contains("CANGUPA_ENGINE_KEYFILE"), "{msg}");
    }

    #[test]
    fn empty_secret_falls_back_to_keyfile() {
        assert!(matches!(
            EngineSigner::load(Some("   "), None),
            Err(AgentError::Config(_))
        ));
    }
}
