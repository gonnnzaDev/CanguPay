//! Vectores dorados: el motor Rust debe reproducir `fixtures/manifest.json` bit a bit.
//!
//! Vive en la libreria, y no en `tests/`, para que el subcomando `selfcheck` del binario
//! ejecute **exactamente la misma** comprobacion que el test. Si fueran dos
//! implementaciones, un `selfcheck` en verde no diria nada sobre lo que CI verifica.
use std::path::{Path, PathBuf};

use serde_json::Value;

use crate::canonical::{strict_loads, Strict};
use crate::error::{AgentError, Result};
use crate::report::build_report;

/// Casos del manifiesto.
pub const CASES: [&str; 3] = ["pass", "fail", "dispute"];

/// Raiz del repositorio, deducida de donde se compilo el crate.
pub fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

/// Directorio de fixtures del repositorio.
pub fn fixture_dir() -> PathBuf {
    repo_root().join("../../fixtures")
}

/// Carga el manifiesto de forma estricta.
pub fn manifest(dir: &Path) -> Result<Value> {
    let text = std::fs::read_to_string(dir.join("manifest.json")).map_err(|e| {
        AgentError::Io(format!(
            "no se pudo leer {}/manifest.json: {e}",
            dir.display()
        ))
    })?;
    strict_loads(&text)
        .map_err(|e| AgentError::BundleStructure(format!("manifest.json no es JSON estricto: {e}")))
        .map(|Strict(value)| value)
}

/// Reconstruye el bundle de un caso a partir de sus tres archivos.
pub fn load_case(dir: &Path, case: &str) -> Result<Value> {
    let case_dir = dir.join(case);
    let mut bundle = serde_json::Map::new();
    for (key, file) in [
        ("purchase_order", "purchase-order.json"),
        ("invoice", "invoice.json"),
        ("delivery", "delivery.json"),
    ] {
        let path = case_dir.join(file);
        let text = std::fs::read_to_string(&path)
            .map_err(|e| AgentError::Io(format!("no se pudo leer {}: {e}", path.display())))?;
        let Strict(value) = strict_loads(&text)
            .map_err(|e| AgentError::BundleStructure(format!("{}: {e}", path.display())))?;
        bundle.insert(key.to_owned(), value);
    }
    Ok(Value::Object(bundle))
}

/// Comprueba los tres vectores del manifiesto.
///
/// Devuelve cuantos casos reprodujeron el hash y el reporte esperados; el error lleva el
/// primer caso que no cuadre, con los dos hashes enfrentados.
pub fn verify_fixtures(dir: &Path) -> Result<usize> {
    let manifest = manifest(dir)?;
    for case in CASES {
        let expected = &manifest[case];
        let amount = expected["escrow_amount"]
            .as_i64()
            .ok_or_else(|| AgentError::BundleStructure(format!("{case}: falta escrow_amount")))?;
        let currency = expected["escrow_token_code"].as_str().ok_or_else(|| {
            AgentError::BundleStructure(format!("{case}: falta escrow_token_code"))
        })?;
        let report = build_report(&load_case(dir, case)?, amount as i128, currency)
            .map_err(|e| AgentError::BundleStructure(format!("{case}: {e}")))?;

        for (field, got, want) in [
            (
                "evidence_bundle_hash",
                report.evidence_bundle_hash.as_str(),
                expected["evidence_bundle_hash"].as_str(),
            ),
            (
                "report_hash",
                report.report_hash.as_str(),
                expected["report_hash"].as_str(),
            ),
            ("result", report.result, expected["result"].as_str()),
        ] {
            let want = want.unwrap_or_default();
            if got != want {
                return Err(AgentError::BundleStructure(format!(
                    "{case}.{field}: el motor dio {got} y el manifiesto espera {want}"
                )));
            }
        }

        let expected_checks = expected["checks"]
            .as_object()
            .ok_or_else(|| AgentError::BundleStructure(format!("{case}: faltan checks")))?;
        if expected_checks.len() != report.checks.len() {
            return Err(AgentError::BundleStructure(format!(
                "{case}: el motor evaluo {} reglas y el manifiesto espera {}",
                report.checks.len(),
                expected_checks.len()
            )));
        }
        for (name, status) in &report.checks {
            if expected_checks[name.as_str()] != status.as_str() {
                return Err(AgentError::BundleStructure(format!(
                    "{case}: la regla {name} dio {} y el manifiesto espera {}",
                    status.as_str(),
                    expected_checks[name.as_str()]
                )));
            }
        }
    }
    Ok(CASES.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixtures_in_the_repository_reproduce_the_manifest() {
        let n = verify_fixtures(&fixture_dir()).expect("los fixtures del repo deben cuadrar");
        assert_eq!(n, CASES.len());
    }

    #[test]
    fn a_manifest_with_a_wrong_hash_is_reported_not_ignored() {
        let source = fixture_dir();
        let dir = std::env::temp_dir().join("cangupay-p07-badmanifest");
        // Copia los fixtures y altera un hash del manifiesto: el chequeo debe fallar y
        // decir cual, en vez de dar el visto bueno.
        for case in CASES {
            let from = source.join(case);
            let to = dir.join(case);
            std::fs::create_dir_all(&to).unwrap();
            for file in ["purchase-order.json", "invoice.json", "delivery.json"] {
                std::fs::copy(from.join(file), to.join(file)).unwrap();
            }
        }
        let text = std::fs::read_to_string(source.join("manifest.json")).unwrap();
        let broken = text.replace(
            &text
                .split("\"report_hash\": \"")
                .nth(1)
                .unwrap()
                .split('"')
                .next()
                .unwrap()
                .to_string(),
            "0000000000000000000000000000000000000000000000000000000000000000",
        );
        std::fs::write(dir.join("manifest.json"), broken).unwrap();
        let err = verify_fixtures(&dir).unwrap_err();
        assert!(err.to_string().contains("report_hash"), "{err}");
    }
}
