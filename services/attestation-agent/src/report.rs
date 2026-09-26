//! Reporte reproducible de una evaluacion.
//!
//! El objeto que se hashea y se manda a `attest()` conserva **exactamente** la forma
//! de la ruleset v1.0.0 (`ruleset_version`, `result`, `checks`, `evidence_bundle_hash`),
//! de modo que `report_hash` sigue siendo comparable con `fixtures/manifest.json` y con
//! lo queverify el motor Python de referencia.
//!
//! El detalle por campo exigido por P0-07 viaja en un objeto aparte (`fields`) con su
//! propio `detail_hash`: es reproducible y tamper-evident sin cambiar el hash on-chain.
use serde_json::{json, Map, Value};

use crate::canonical::{compute_evidence_bundle_hash, compute_report_hash, hash_value};
use crate::error::Result;
use crate::ruleset::{Evaluation, Status, SECTIONS};

/// Reporte completo: parte compatible con v1.0.0 mas el detalle por campo.
#[derive(Debug, Clone, PartialEq)]
pub struct Report {
    /// Version del ruleset.
    pub ruleset_version: &'static str,
    /// `PASS` o `FAIL`.
    pub result: &'static str,
    /// Hash del bundle de evidencia calculado localmente.
    pub evidence_bundle_hash: String,
    /// Hash del reporte en su forma v1.0.0; es el valor que va a `attest()`.
    pub report_hash: String,
    /// Hash del detalle por campo.
    pub detail_hash: String,
    /// Estado por regla, en el orden fijo de la ruleset.
    pub checks: Vec<(String, Status)>,
    /// Campos que fallaron, con su valor observado y el esperado solo si existe en los datos.
    pub failed_fields: Vec<FieldReport>,
}

impl Report {
    /// `true` si el bundle cumple el ruleset.
    pub fn is_pass(&self) -> bool {
        self.result == "PASS"
    }

    /// Representacion JSON completa, con las claves en orden canonico.
    pub fn to_json(&self) -> Value {
        let mut out = Map::new();
        out.insert("ruleset_version".into(), json!(self.ruleset_version));
        out.insert("result".into(), json!(self.result));
        out.insert("checks".into(), checks_to_json(&self.checks));
        out.insert(
            "evidence_bundle_hash".into(),
            json!(self.evidence_bundle_hash),
        );
        out.insert("report_hash".into(), json!(self.report_hash));
        out.insert("detail_hash".into(), json!(self.detail_hash));
        out.insert(
            "failed_fields".into(),
            Value::Array(
                self.failed_fields
                    .iter()
                    .map(FieldReport::to_json)
                    .collect(),
            ),
        );
        Value::Object(out)
    }

    /// Linea resumen para logs y para el keeper.
    pub fn summary(&self) -> String {
        format!(
            "result={} report_hash={} bundle_hash={} campos_fallidos={}",
            self.result,
            self.report_hash,
            self.evidence_bundle_hash,
            self.failed_fields.len()
        )
    }
}

/// Detalle de un campo que fallo.
#[derive(Debug, Clone, PartialEq)]
pub struct FieldReport {
    /// Ruta del campo en el bundle.
    pub path: String,
    /// Estado: `MISMATCH`, `MISSING` o `INVALID`.
    pub status: Status,
    /// Valor observado, si el campo existe y pudo normalizarse.
    pub observed: Option<String>,
    /// Valor esperado, derivado de los datos presentes; nunca inventado.
    pub expected: Option<String>,
}

impl FieldReport {
    fn to_json(&self) -> Value {
        let mut out = Map::new();
        out.insert("path".into(), json!(self.path));
        out.insert("status".into(), json!(self.status.as_str()));
        out.insert("observed".into(), json!(self.observed));
        out.insert("expected".into(), json!(self.expected));
        Value::Object(out)
    }
}

/// Checks en forma de objeto, para el hash y para el reporte.
fn checks_to_json(checks: &[(String, Status)]) -> Value {
    let mut map = Map::new();
    for (k, v) in checks {
        map.insert(k.clone(), json!(v.as_str()));
    }
    Value::Object(map)
}

/// Construye el reporte a partir del bundle y del contexto del escrow.
pub fn build_report(
    bundle: &Value,
    escrow_amount: i128,
    escrow_token_code: &str,
) -> Result<Report> {
    let evidence_bundle_hash = compute_evidence_bundle_hash(bundle);
    let evaluation = crate::ruleset::evaluate_bundle(bundle, escrow_amount, escrow_token_code)?;
    Ok(finalize_report(evaluation, evidence_bundle_hash))
}

/// Convierte una evaluacion en reporte, calculando `report_hash` y `detail_hash`.
pub fn finalize_report(evaluation: Evaluation, evidence_bundle_hash: String) -> Report {
    let checks: Vec<(String, Status)> = evaluation
        .checks
        .iter()
        .map(|(k, s)| ((*k).to_owned(), *s))
        .collect();

    // Forma hashada v1.0.0: no incluye el detalle por campo.
    let hashed = json!({
        "ruleset_version": evaluation.ruleset_version,
        "result": evaluation.result,
        "checks": checks_to_json(&checks),
        "evidence_bundle_hash": evidence_bundle_hash,
    });
    let report_hash = compute_report_hash(&hashed);

    let fields: Vec<Value> = evaluation
        .fields
        .iter()
        .map(|f| {
            json!({
                "path": f.path,
                "status": f.status.as_str(),
                "observed": f.observed,
                "expected": f.expected,
            })
        })
        .collect();
    let detail_hash = hash_value(&json!({ "fields": fields }));

    let failed_fields = evaluation
        .fields
        .iter()
        .filter(|f| !f.status.is_ok())
        .map(|f| FieldReport {
            path: f.path.clone(),
            status: f.status,
            observed: f.observed.clone(),
            expected: f.expected.clone(),
        })
        .collect();

    Report {
        ruleset_version: evaluation.ruleset_version,
        result: evaluation.result,
        evidence_bundle_hash,
        report_hash,
        detail_hash,
        checks,
        failed_fields,
    }
}

/// Nombres de las secciones del bundle, expuestos para el CLI.
pub fn section_names() -> [&'static str; 3] {
    SECTIONS
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn bundle() -> Value {
        json!({
            "purchase_order": {"id": "po-01", "supplier_id": "s-1", "amount": 1000, "currency": "CPUSD"},
            "invoice": {"purchase_order_id": "po-01", "supplier_id": "s-1", "amount": 1000, "currency": "CPUSD"},
            "delivery": {"purchase_order_id": "po-01", "accepted": true}
        })
    }

    #[test]
    fn report_is_reproducible() {
        let a = build_report(&bundle(), 1000, "CPUSD").unwrap();
        let b = build_report(&bundle(), 1000, "CPUSD").unwrap();
        assert_eq!(a.report_hash, b.report_hash);
        assert_eq!(a.detail_hash, b.detail_hash);
        assert_eq!(a.evidence_bundle_hash, b.evidence_bundle_hash);
        assert!(a.is_pass());
    }

    #[test]
    fn report_hash_ignores_key_order_of_the_bundle() {
        let reordered = json!({
            "delivery": {"accepted": true, "purchase_order_id": "po-01"},
            "invoice": {"currency": "CPUSD", "amount": 1000, "supplier_id": "s-1", "purchase_order_id": "po-01"},
            "purchase_order": {"currency": "CPUSD", "amount": 1000, "supplier_id": "s-1", "id": "po-01"}
        });
        let a = build_report(&bundle(), 1000, "CPUSD").unwrap();
        let b = build_report(&reordered, 1000, "CPUSD").unwrap();
        assert_eq!(a.evidence_bundle_hash, b.evidence_bundle_hash);
        assert_eq!(a.report_hash, b.report_hash);
    }

    #[test]
    fn report_hash_changes_when_amount_changes() {
        let a = build_report(&bundle(), 1000, "CPUSD").unwrap();
        let b = build_report(&bundle(), 1001, "CPUSD").unwrap();
        assert_ne!(a.report_hash, b.report_hash);
    }

    #[test]
    fn fail_lists_the_failing_field() {
        let mut bad = bundle();
        bad["invoice"]["amount"] = json!(999);
        let r = build_report(&bad, 1000, "CPUSD").unwrap();
        assert!(!r.is_pass());
        assert_eq!(r.failed_fields.len(), 1);
        let f = &r.failed_fields[0];
        assert_eq!(f.path, "invoice.amount");
        assert_eq!(f.status, Status::Mismatch);
        assert_eq!(f.observed.as_deref(), Some("999"));
        assert_eq!(f.expected.as_deref(), Some("1000"));
    }

    #[test]
    fn missing_field_has_no_observed_value() {
        let mut bad = bundle();
        bad["invoice"]
            .as_object_mut()
            .unwrap()
            .remove("supplier_id");
        let r = build_report(&bad, 1000, "CPUSD").unwrap();
        let f = r
            .failed_fields
            .iter()
            .find(|f| f.path == "invoice.supplier_id")
            .unwrap();
        assert_eq!(f.status, Status::Missing);
        assert_eq!(f.observed, None);
    }

    #[test]
    fn report_json_exposes_hashes_and_checks() {
        let r = build_report(&bundle(), 1000, "CPUSD").unwrap();
        let v = r.to_json();
        assert_eq!(v["result"], json!("PASS"));
        assert_eq!(v["checks"]["supplier"], json!("MATCH"));
        assert_eq!(v["report_hash"], json!(r.report_hash));
        assert_eq!(v["failed_fields"], json!([]));
    }
}
