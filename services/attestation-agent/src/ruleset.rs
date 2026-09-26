//! Motor de reglas determinista · CanguPay ruleset.
//!
//! Puerto de `app/ruleset.py` (ruleset v1.0.0) con dos mejoras exigidas por P0-07:
//!
//! 1. Cada FAIL señala el **campo exacto** que fallo (`purchase_order.supplier_id`,
//!    `invoice.amount`, ...) y no solo el grupo de regla.
//! 2. El valor real se reporta solo si existe en el bundle; nunca se inventa un valor
//!    esperado para un campo ausente.
//!
//! Los estados por regla y el `result` son identicos a los del motor Python, de modo que
//! `fixtures/manifest.json` sigue siendo el vector dorado valido.
use serde_json::Value;

use crate::canonical::{is_ascii_string, normalize_id};
use crate::error::{AgentError, Result};

/// Version del ruleset implementada por este motor.
pub const RULESET_VERSION: &str = "1.0.0";

/// Secciones exactas del bundle, en el orden que usa el motor Python.
pub const SECTIONS: [&str; 3] = ["purchase_order", "invoice", "delivery"];

/// Estado de una regla o de un campo.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    /// El valor existe, es valido y coincide con su contraparte.
    Match,
    /// El valor existe y es valido, pero no coincide con su contraparte.
    Mismatch,
    /// El campo no esta presente en el JSON.
    Missing,
    /// El campo existe pero su tipo o formato no cumple el ruleset.
    Invalid,
}

impl Status {
    /// Nombre estable en el reporte, identico a los literales del motor Python.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Match => "MATCH",
            Self::Mismatch => "MISMATCH",
            Self::Missing => "MISSING",
            Self::Invalid => "INVALID",
        }
    }

    /// Un estado que no es `Match` hace fallar el bundle.
    pub fn is_ok(self) -> bool {
        matches!(self, Self::Match)
    }
}

/// Resultado de la evaluacion de un campo individual.
#[derive(Debug, Clone, PartialEq)]
pub struct FieldOutcome {
    /// Ruta del campo dentro del bundle, por ejemplo `purchase_order.supplier_id`.
    pub path: String,
    /// Estado del campo.
    pub status: Status,
    /// Valor observado, solo si el campo existe y es un string ASCII valido.
    pub observed: Option<String>,
    /// Valor con el que se compara, solo si la contraparte existe y es valida.
    pub expected: Option<String>,
}

impl FieldOutcome {
    fn with_values(
        path: &str,
        status: Status,
        observed: Option<String>,
        expected: Option<String>,
    ) -> Self {
        Self {
            path: path.to_owned(),
            status,
            observed,
            expected,
        }
    }
}

/// Resultado de la evaluacion completa de un bundle.
#[derive(Debug, Clone, PartialEq)]
pub struct Evaluation {
    /// `PASS` o `FAIL`.
    pub result: &'static str,
    /// Estado por regla, con las mismas cinco claves que el motor Python.
    pub checks: Vec<(&'static str, Status)>,
    /// Detalle por campo; incluye los campos que si pasan, para trazabilidad.
    pub fields: Vec<FieldOutcome>,
    /// Version del ruleset.
    pub ruleset_version: &'static str,
}

impl Evaluation {
    /// Campos que fallaron, en orden estable.
    pub fn failed_fields(&self) -> impl Iterator<Item = &FieldOutcome> {
        self.fields.iter().filter(|f| !f.status.is_ok())
    }
}

/// Estado de un campo string segun `str_field` del motor Python.
fn str_field(section: &Value, field: &str) -> (Status, Option<String>) {
    let Some(v) = section.get(field) else {
        return (Status::Missing, None);
    };
    if !is_ascii_string(v) {
        return (Status::Invalid, None);
    }
    let s = v.as_str().unwrap_or_default();
    let n = normalize_id(s);
    if n.is_empty() {
        return (Status::Invalid, None);
    }
    (Status::Match, Some(n))
}

/// Estado de un campo entero positivo segun `amount_field` del motor Python.
fn amount_field(section: &Value, field: &str) -> (Status, Option<i128>) {
    let Some(v) = section.get(field) else {
        return (Status::Missing, None);
    };
    // serde_json separa bool de number, a diferencia de `isinstance(True, int)` de Python.
    let Some(n) = v.as_number() else {
        return (Status::Invalid, None);
    };
    let as_i128 = |v: i64| Some(i128::from(v));
    let as_u128 = |v: u64| Some(i128::from(v));
    let Some(i) = n
        .as_i64()
        .and_then(as_i128)
        .or_else(|| n.as_u64().and_then(as_u128))
    else {
        return (Status::Invalid, None);
    };
    if i <= 0 {
        return (Status::Invalid, None);
    }
    (Status::Match, Some(i))
}

/// Estado de un token de moneda, con la misma normalizacion que los IDs.
fn token_field(value: &str) -> (Status, Option<String>) {
    if !value.is_ascii() {
        return (Status::Invalid, None);
    }
    let n = normalize_id(value);
    if n.is_empty() {
        return (Status::Invalid, None);
    }
    (Status::Match, Some(n))
}

/// Valida que el bundle sea un objeto con exactamente las tres secciones.
pub fn validate_bundle_structure(bundle: &Value) -> Result<()> {
    let Value::Object(map) = bundle else {
        return Err(AgentError::BundleStructure(format!(
            "El bundle debe ser un objeto JSON, no {}",
            type_name(bundle)
        )));
    };
    let mut keys: Vec<&str> = map.keys().map(String::as_str).collect();
    keys.sort();
    let expected = ["delivery", "invoice", "purchase_order"];
    if keys != expected {
        return Err(AgentError::BundleStructure(format!(
            "Claves exactas requeridas purchase_order/invoice/delivery; recibidas: {keys:?}"
        )));
    }
    for (i, section) in SECTIONS.iter().enumerate() {
        if !map[*section].is_object() {
            return Err(AgentError::BundleStructure(format!(
                "La seccion '{section}' debe ser un objeto JSON, no {}",
                type_name(&map[*section])
            )));
        }
        if i == 2 {
            break;
        }
    }
    Ok(())
}

fn type_name(v: &Value) -> &'static str {
    match v {
        Value::Null => "null",
        Value::Bool(_) => "bool",
        Value::Number(_) => "number",
        Value::String(_) => "str",
        Value::Array(_) => "list",
        Value::Object(_) => "object",
    }
}

/// Evalua el bundle completo contra el monto y la moneda acordados en el escrow.
pub fn evaluate_bundle(
    bundle: &Value,
    escrow_amount: i128,
    escrow_token_code: &str,
) -> Result<Evaluation> {
    validate_bundle_structure(bundle)?;
    let po = &bundle["purchase_order"];
    let inv = &bundle["invoice"];
    let dl = &bundle["delivery"];

    let mut fields: Vec<FieldOutcome> = Vec::new();

    // Regla supplier: supplier_id de la PO y de la factura.
    let (po_s, po_sv) = str_field(po, "supplier_id");
    let (inv_s, inv_sv) = str_field(inv, "supplier_id");
    let supplier = compare_pair(po_s, po_sv.as_deref(), inv_s, inv_sv.as_deref());
    fields.push(FieldOutcome::with_values(
        "purchase_order.supplier_id",
        po_s,
        po_sv.clone(),
        inv_sv.clone(),
    ));
    fields.push(FieldOutcome::with_values(
        "invoice.supplier_id",
        field_status(inv_s, supplier),
        inv_sv.clone(),
        po_sv.clone(),
    ));

    // Regla purchase_order: id de la PO referenciado por factura y entrega.
    let (po_id, po_idv) = str_field(po, "id");
    let (inv_po, inv_pov) = str_field(inv, "purchase_order_id");
    let (dl_po, dl_pov) = str_field(dl, "purchase_order_id");
    let inv_po_cmp = compare_to_anchor(inv_po, inv_pov.as_deref(), po_idv.as_deref());
    let dl_po_cmp = compare_to_anchor(dl_po, dl_pov.as_deref(), po_idv.as_deref());
    let purchase_order = worst([po_id, inv_po_cmp, dl_po_cmp]);
    fields.push(FieldOutcome::with_values(
        "purchase_order.id",
        po_id,
        po_idv.clone(),
        inv_pov.clone(),
    ));
    fields.push(FieldOutcome::with_values(
        "invoice.purchase_order_id",
        field_status(inv_po, inv_po_cmp),
        inv_pov.clone(),
        po_idv.clone(),
    ));
    fields.push(FieldOutcome::with_values(
        "delivery.purchase_order_id",
        field_status(dl_po, dl_po_cmp),
        dl_pov.clone(),
        po_idv.clone(),
    ));

    // Regla amount: el monto de la PO y de la factura contra el monto del escrow.
    let (po_a, po_av) = amount_field(po, "amount");
    let (inv_a, inv_av) = amount_field(inv, "amount");
    let (escrow_a, escrow_av) = if escrow_amount > 0 {
        (Status::Match, Some(escrow_amount))
    } else {
        (Status::Invalid, None)
    };
    let amount = if po_a.is_ok() && inv_a.is_ok() && escrow_a.is_ok() {
        let po_s = po_av.map(|v| v.to_string());
        let inv_s = inv_av.map(|v| v.to_string());
        let esc_s = escrow_av.map(|v| v.to_string());
        Status::if_equal(&[po_s.as_deref(), inv_s.as_deref(), esc_s.as_deref()])
    } else {
        worst([po_a, inv_a, escrow_a])
    };
    // Cada monto se compara por separado contra el monto del escrow, que es el ancla:
    // asi el campo que coincide no aparece como fallido cuando el otro diverge.
    let escrow_amount_str = escrow_av.map(|v| v.to_string());
    let po_amount_cmp = compare_to_anchor(
        po_a,
        po_av.map(|v| v.to_string()).as_deref(),
        escrow_amount_str.as_deref(),
    );
    let inv_amount_cmp = compare_to_anchor(
        inv_a,
        inv_av.map(|v| v.to_string()).as_deref(),
        escrow_amount_str.as_deref(),
    );
    fields.push(FieldOutcome::with_values(
        "purchase_order.amount",
        po_amount_cmp,
        po_av.map(|v| v.to_string()),
        Some(escrow_amount.to_string()),
    ));
    fields.push(FieldOutcome::with_values(
        "invoice.amount",
        inv_amount_cmp,
        inv_av.map(|v| v.to_string()),
        Some(escrow_amount.to_string()),
    ));

    // Regla currency: PO y factura contra el token del escrow.
    let (po_c, po_cv) = str_field(po, "currency");
    let (inv_c, inv_cv) = str_field(inv, "currency");
    let (escrow_c, escrow_cv) = token_field(escrow_token_code);
    let currency = if po_c.is_ok() && inv_c.is_ok() && escrow_c.is_ok() {
        Status::if_equal(&[po_cv.as_deref(), inv_cv.as_deref(), escrow_cv.as_deref()])
    } else {
        worst([po_c, inv_c, escrow_c])
    };
    let escrow_currency_str = escrow_cv.clone();
    let po_currency_cmp = compare_to_anchor(po_c, po_cv.as_deref(), escrow_currency_str.as_deref());
    let inv_currency_cmp =
        compare_to_anchor(inv_c, inv_cv.as_deref(), escrow_currency_str.as_deref());
    fields.push(FieldOutcome::with_values(
        "purchase_order.currency",
        po_currency_cmp,
        po_cv.clone(),
        Some(escrow_token_code.to_owned()),
    ));
    fields.push(FieldOutcome::with_values(
        "invoice.currency",
        inv_currency_cmp,
        inv_cv.clone(),
        Some(escrow_token_code.to_owned()),
    ));

    // Regla delivery: la entrega debe estar aceptada explicitamente.
    let (dl_status, dl_observed) = match dl.get("accepted") {
        None => (Status::Missing, None),
        Some(Value::Bool(true)) => (Status::Match, Some("true".to_owned())),
        Some(Value::Bool(false)) => (Status::Mismatch, Some("false".to_owned())),
        Some(_) => (Status::Invalid, None),
    };
    fields.push(FieldOutcome::with_values(
        "delivery.accepted",
        dl_status,
        dl_observed,
        Some("true".to_owned()),
    ));

    let checks = vec![
        ("supplier", supplier),
        ("purchase_order", purchase_order),
        ("amount", amount),
        ("currency", currency),
        ("delivery", dl_status),
    ];
    let result = if checks.iter().all(|(_, s)| s.is_ok()) {
        "PASS"
    } else {
        "FAIL"
    };
    Ok(Evaluation {
        result,
        checks,
        fields,
        ruleset_version: RULESET_VERSION,
    })
}

impl Status {
    /// Compara valores normalizados: `Match` solo si todos son iguales y existen.
    fn if_equal(values: &[Option<&str>]) -> Self {
        let mut iter = values.iter();
        match iter.next() {
            None | Some(None) => Self::Missing,
            Some(Some(first)) => {
                if iter.all(|v| *v == Some(*first)) {
                    Self::Match
                } else {
                    Self::Mismatch
                }
            }
        }
    }
}

/// Estado combinado de hasta tres campos: `Match` es el mejor y `Missing` el peor,
/// de modo que un unico campo invalido o ausente nunca queda enmascarado por un `MATCH`.
fn worst(states: [Status; 3]) -> Status {
    states
        .into_iter()
        .max_by_key(|s| match s {
            Status::Match => 1,
            Status::Mismatch => 2,
            Status::Invalid => 3,
            Status::Missing => 4,
        })
        .unwrap_or(Status::Missing)
}

/// Compara dos campos entre si: si ambos son validos compara, si no conserva el peor estado.
fn compare_pair(a: Status, av: Option<&str>, b: Status, bv: Option<&str>) -> Status {
    if a.is_ok() && b.is_ok() {
        Status::if_equal(&[av, bv])
    } else {
        worst([a, b, Status::Match])
    }
}

/// Estado de un campo que se compara: si el campo falta o es invalido se conserva ese
/// motivo; si es valido, hereda el resultado de la comparacion contra el ancla.
fn field_status(local: Status, comparison: Status) -> Status {
    if local.is_ok() {
        comparison
    } else {
        local
    }
}

/// Compara un campo contra un ancla ya normalizada.
fn compare_to_anchor(status: Status, value: Option<&str>, anchor: Option<&str>) -> Status {
    if !status.is_ok() || anchor.is_none() {
        return status;
    }
    Status::if_equal(&[value, anchor])
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
    fn pass_bundle_passes() {
        let ev = evaluate_bundle(&bundle(), 1000, "CPUSD").unwrap();
        assert_eq!(ev.result, "PASS");
        assert!(ev.failed_fields().next().is_none());
    }

    #[test]
    fn structure_requires_exact_sections() {
        let mut b = bundle();
        b.as_object_mut().unwrap().remove("invoice");
        let err = evaluate_bundle(&b, 1000, "CPUSD").unwrap_err();
        assert!(err.to_string().contains("Claves exactas"), "{err}");
    }

    #[test]
    fn structure_rejects_non_object_section() {
        let mut b = bundle();
        b["delivery"] = json!([]);
        let err = evaluate_bundle(&b, 1000, "CPUSD").unwrap_err();
        assert!(err.to_string().contains("delivery"), "{err}");
    }

    #[test]
    fn missing_field_is_reported_by_path_without_inventing_value() {
        let mut b = bundle();
        b["invoice"].as_object_mut().unwrap().remove("amount");
        let ev = evaluate_bundle(&b, 1000, "CPUSD").unwrap();
        assert_eq!(ev.result, "FAIL");
        let f = ev
            .fields
            .iter()
            .find(|f| f.path == "invoice.amount")
            .unwrap();
        assert_eq!(f.status, Status::Missing);
        assert_eq!(f.observed, None);
        assert_eq!(f.expected.as_deref(), Some("1000"));
    }

    #[test]
    fn amount_mismatch_points_at_both_sides() {
        let mut b = bundle();
        b["invoice"]["amount"] = json!(999);
        let ev = evaluate_bundle(&b, 1000, "CPUSD").unwrap();
        assert_eq!(ev.result, "FAIL");
        assert_eq!(
            ev.checks.iter().find(|(k, _)| *k == "amount").unwrap().1,
            Status::Mismatch
        );
        let f = ev
            .fields
            .iter()
            .find(|f| f.path == "invoice.amount")
            .unwrap();
        assert_eq!(f.observed.as_deref(), Some("999"));
        assert_eq!(f.expected.as_deref(), Some("1000"));
    }

    #[test]
    fn bool_amount_is_invalid_not_matched() {
        let mut b = bundle();
        b["invoice"]["amount"] = json!(true);
        let ev = evaluate_bundle(&b, 1000, "CPUSD").unwrap();
        assert_eq!(ev.result, "FAIL");
        assert_eq!(
            ev.fields
                .iter()
                .find(|f| f.path == "invoice.amount")
                .unwrap()
                .status,
            Status::Invalid
        );
    }

    #[test]
    fn negative_and_zero_amounts_are_invalid() {
        for bad in [json!(-5), json!(0)] {
            let mut b = bundle();
            b["purchase_order"]["amount"] = bad.clone();
            let ev = evaluate_bundle(&b, 1000, "CPUSD").unwrap();
            assert_eq!(
                ev.fields
                    .iter()
                    .find(|f| f.path == "purchase_order.amount")
                    .unwrap()
                    .status,
                Status::Invalid
            );
        }
    }

    #[test]
    fn supplier_mismatch_is_reported() {
        let mut b = bundle();
        b["invoice"]["supplier_id"] = json!("s-2");
        let ev = evaluate_bundle(&b, 1000, "CPUSD").unwrap();
        assert_eq!(ev.result, "FAIL");
        let f = ev
            .fields
            .iter()
            .find(|f| f.path == "invoice.supplier_id")
            .unwrap();
        assert_eq!(f.status, Status::Mismatch);
        assert_eq!(f.observed.as_deref(), Some("S-2"));
        assert_eq!(f.expected.as_deref(), Some("S-1"));
    }

    #[test]
    fn currency_comparison_normalizes() {
        let ev = evaluate_bundle(&bundle(), 1000, " cpusd ").unwrap();
        assert_eq!(ev.result, "PASS");
    }

    #[test]
    fn delivery_false_is_mismatch() {
        let mut b = bundle();
        b["delivery"]["accepted"] = json!(false);
        let ev = evaluate_bundle(&b, 1000, "CPUSD").unwrap();
        assert_eq!(ev.result, "FAIL");
        assert_eq!(
            ev.checks.iter().find(|(k, _)| *k == "delivery").unwrap().1,
            Status::Mismatch
        );
    }

    #[test]
    fn non_ascii_field_is_invalid() {
        let mut b = bundle();
        b["purchase_order"]["supplier_id"] = json!("proveedor-ñ");
        let ev = evaluate_bundle(&b, 1000, "CPUSD").unwrap();
        assert_eq!(ev.result, "FAIL");
        assert_eq!(
            ev.fields
                .iter()
                .find(|f| f.path == "purchase_order.supplier_id")
                .unwrap()
                .status,
            Status::Invalid
        );
    }

    #[test]
    fn escrow_amount_mismatch_fails() {
        let ev = evaluate_bundle(&bundle(), 1001, "CPUSD").unwrap();
        assert_eq!(ev.result, "FAIL");
        assert_eq!(
            ev.checks.iter().find(|(k, _)| *k == "amount").unwrap().1,
            Status::Mismatch
        );
    }
}
