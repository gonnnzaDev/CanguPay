use thiserror::Error;

/// Errores del agente de atestación (P0-07).
///
/// Cada variante mapea a un exit code estable del CLI `cangu-attest`, para que el
/// keeper y los scripts de demo puedan distinguir rechazo de error de infraestructura.
#[derive(Debug, Error)]
pub enum AgentError {
    /// El bundle no cumple la estructura de tres secciones o tiene JSON no parseable.
    #[error("bundle invalido: {0}")]
    BundleStructure(String),

    /// El hash calculado localmente difiere del hash presentado en la cadena.
    #[error("hash del bundle calculado {computed} != hash en cadena {on_chain}")]
    BundleHashMismatch { computed: String, on_chain: String },

    /// El estado del contrato no permite la operacion solicitada.
    #[error("estado {actual:?} no permite {operacion}")]
    InvalidState { actual: String, operacion: String },

    /// El plazo relevante ya vencio.
    #[error("plazo {plazo} vencido: faltan {remaining}s")]
    DeadlinePassed { plazo: String, remaining: i64 },

    /// Falta configuracion obligatoria del agente (red, contrato, RPC, clave).
    #[error("configuracion invalida: {0}")]
    Config(String),

    /// La configuracion local del agente no coincide con la del contrato.
    ///
    /// Se contrasta antes de firmar: el importe y la divisa son los del contrato, no
    /// los que vinieron por linea de comandos. Firmar contra un importe distinto al
    /// que el contrato tiene en cadena produce una atestacion que no describe el escrow.
    #[error("la configuracion local no coincide con el contrato: {0}")]
    ContractConfigMismatch(String),

    /// La clave del engine no debe vivir dentro del repositorio.
    #[error(
        "la clave del engine esta dentro del repositorio ({path}); moverla fuera (regla P0-07)"
    )]
    EngineKeyInsideRepo { path: String },

    /// Error de transporte contra la red (RPC, envio o confirmacion).
    #[error("error de red: {0}")]
    Network(String),

    /// Error de serializacion XDR al construir la transaccion.
    #[error("error XDR: {0}")]
    Xdr(String),

    /// Error de E/S local.
    #[error("error de io: {0}")]
    Io(String),
}

pub type Result<T> = std::result::Result<T, AgentError>;

/// Exit codes del CLI, alineados con los del motor Python de la etapa 1.
pub mod exit {
    /// PASS u operacion completada.
    pub const OK: i32 = 0;
    /// FAIL determinista del ruleset.
    pub const FAIL: i32 = 1;
    /// Rechazo (estructura, hash, estado, plazo) o error de uso.
    pub const REJECT: i32 = 2;
    /// Error de infraestructura (red, XDR, io).
    pub const INFRA: i32 = 3;
}

impl AgentError {
    /// Exit code estable para el CLI y el keeper.
    pub fn exit_code(&self) -> i32 {
        match self {
            Self::BundleStructure(_)
            | Self::BundleHashMismatch { .. }
            | Self::InvalidState { .. }
            | Self::DeadlinePassed { .. }
            | Self::Config(_)
            | Self::ContractConfigMismatch(_)
            | Self::EngineKeyInsideRepo { .. } => exit::REJECT,
            Self::Network(_) | Self::Xdr(_) | Self::Io(_) => exit::INFRA,
        }
    }
}

impl From<std::io::Error> for AgentError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e.to_string())
    }
}
