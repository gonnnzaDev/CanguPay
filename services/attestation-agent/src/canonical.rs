//! Canonicalizacion estricta y hashes · CanguPay ruleset.
//!
//! Paridad byte a byte con el motor Python de referencia (`app/canonical.py`):
//! `json.dumps(obj, sort_keys=True, separators=(",", ":"), ensure_ascii=False)`
//! codificado en UTF-8, sin BOM. Cualquier divergencia aqui rompe el criterio
//! "el hash del bundle presentado coincide con el procesado por el engine", asi que
//! `tests/golden.rs` fija los vectores de `fixtures/manifest.json`.
use std::collections::BTreeSet;
use std::fmt;

use serde::de::{self, Deserializer, MapAccess, SeqAccess, Visitor};
use serde_json::{Map, Number, Value};
use sha2::{Digest, Sha256};

use crate::error::{AgentError, Result};

/// Bytes considered whitespace por `normalize_id` (equivalente a
/// `ASCII_EDGE_WHITESPACE` del motor Python).
pub const ASCII_EDGE_WHITESPACE: [char; 4] = [' ', '\t', '\r', '\n'];

/// Valor JSON parseado en modo estricto: rechaza claves duplicadas y NaN/Infinity.
#[derive(Debug, Clone, PartialEq)]
pub struct Strict(pub Value);

impl Strict {
    /// Consume el envoltorio y devuelve el `Value`.
    pub fn into_value(self) -> Value {
        self.0
    }
}

impl<'de> serde::Deserialize<'de> for Strict {
    fn deserialize<D: Deserializer<'de>>(d: D) -> std::result::Result<Self, D::Error> {
        d.deserialize_any(StrictValueVisitor)
            .map(|v| Strict(v.into_value()))
    }
}

/// Intermediario recursivo: conserva el orden de claves para poder detectar duplicados.
enum StrictValue {
    Null,
    Bool(bool),
    Int(i64),
    Uint(u64),
    Float(f64),
    Str(String),
    Arr(Vec<StrictValue>),
    Obj(Vec<(String, StrictValue)>),
}

impl<'de> serde::Deserialize<'de> for StrictValue {
    fn deserialize<D: Deserializer<'de>>(d: D) -> std::result::Result<Self, D::Error> {
        d.deserialize_any(StrictValueVisitor)
    }
}

impl StrictValue {
    fn into_value(self) -> Value {
        match self {
            Self::Null => Value::Null,
            Self::Bool(b) => Value::Bool(b),
            Self::Int(i) => Value::Number(Number::from(i)),
            Self::Uint(u) => Value::Number(Number::from(u)),
            Self::Float(f) => Value::Number(Number::from_f64(f).unwrap_or_else(|| Number::from(0))),
            Self::Str(s) => Value::String(s),
            Self::Arr(items) => Value::Array(items.into_iter().map(Self::into_value).collect()),
            Self::Obj(pairs) => {
                let mut map = Map::new();
                for (k, v) in pairs {
                    map.insert(k, v.into_value());
                }
                Value::Object(map)
            }
        }
    }
}

struct StrictValueVisitor;

impl<'de> Visitor<'de> for StrictValueVisitor {
    type Value = StrictValue;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("un valor JSON sin claves duplicadas ni NaN/Infinity")
    }

    fn visit_unit<E: de::Error>(self) -> std::result::Result<Self::Value, E> {
        Ok(StrictValue::Null)
    }

    fn visit_none<E: de::Error>(self) -> std::result::Result<Self::Value, E> {
        Ok(StrictValue::Null)
    }

    fn visit_some<D: Deserializer<'de>>(self, d: D) -> std::result::Result<Self::Value, D::Error> {
        d.deserialize_any(self)
    }

    fn visit_bool<E: de::Error>(self, v: bool) -> std::result::Result<Self::Value, E> {
        Ok(StrictValue::Bool(v))
    }

    fn visit_i64<E: de::Error>(self, v: i64) -> std::result::Result<Self::Value, E> {
        Ok(StrictValue::Int(v))
    }

    fn visit_u64<E: de::Error>(self, v: u64) -> std::result::Result<Self::Value, E> {
        Ok(StrictValue::Uint(v))
    }

    fn visit_f64<E: de::Error>(self, v: f64) -> std::result::Result<Self::Value, E> {
        Ok(StrictValue::Float(v))
    }

    fn visit_str<E: de::Error>(self, v: &str) -> std::result::Result<Self::Value, E> {
        Ok(StrictValue::Str(v.to_owned()))
    }

    fn visit_seq<A: SeqAccess<'de>>(
        self,
        mut seq: A,
    ) -> std::result::Result<Self::Value, A::Error> {
        let mut items = Vec::new();
        while let Some(v) = seq.next_element::<StrictValue>()? {
            items.push(v);
        }
        Ok(StrictValue::Arr(items))
    }

    fn visit_map<A: MapAccess<'de>>(
        self,
        mut map: A,
    ) -> std::result::Result<Self::Value, A::Error> {
        let mut pairs: Vec<(String, StrictValue)> = Vec::new();
        let mut seen = BTreeSet::new();
        while let Some((k, v)) = map.next_entry::<String, StrictValue>()? {
            if !seen.insert(k.clone()) {
                return Err(de::Error::custom(format!("clave duplicada en JSON: {k}")));
            }
            pairs.push((k, v));
        }
        Ok(StrictValue::Obj(pairs))
    }
}

/// Parseo estricto equivalente a `app/canonical.py::strict_loads`.
///
/// Rechaza claves duplicadas en cualquier nivel y las constantes `NaN`/`Infinity`,
/// que `serde_json` ya rechaza en la sintaxis JSON.
pub fn strict_loads(text: &str) -> Result<Strict> {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let strict: Strict = serde_json::from_str(text)
        .map_err(|e| AgentError::BundleStructure(format!("JSON invalido: {e}")))?;
    Ok(strict)
}

/// Serializa a JSON canonico: claves ordenadas, separadores minimos, UTF-8 sin escapes ASCII.
pub fn canonical_json(value: &Value) -> Vec<u8> {
    let mut out = String::new();
    write_canonical(value, &mut out);
    out.into_bytes()
}

fn write_canonical(value: &Value, out: &mut String) {
    match value {
        Value::Null => out.push_str("null"),
        Value::Bool(true) => out.push_str("true"),
        Value::Bool(false) => out.push_str("false"),
        Value::Number(n) => out.push_str(&write_number(n)),
        Value::String(s) => write_string(s, out),
        Value::Array(items) => {
            out.push('[');
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                write_canonical(item, out);
            }
            out.push(']');
        }
        Value::Object(map) => {
            // Se ordena explicitamente: no se depende del backend de `serde_json::Map`
            // (puede ser IndexMap si otra dependencia activa `preserve_order`).
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort();
            out.push('{');
            for (i, k) in keys.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                write_string(k, out);
                out.push(':');
                write_canonical(&map[*k], out);
            }
            out.push('}');
        }
    }
}

/// Enteros exactos; flotantes con `repr()` al estilo de Python (`1.0`, `1e-07`).
fn write_number(n: &Number) -> String {
    if let Some(i) = n.as_i64() {
        return i.to_string();
    }
    if let Some(u) = n.as_u64() {
        return u.to_string();
    }
    match n.as_f64() {
        Some(f) => python_float_repr(f),
        None => n.to_string(),
    }
}

/// `repr(float)` de Python: shortest round-trip con `e` y exponente de dos digitos.
fn python_float_repr(f: f64) -> String {
    if f.is_nan() {
        return "NaN".to_owned();
    }
    if f.is_infinite() {
        return if f > 0.0 {
            "Infinity".into()
        } else {
            "-Infinity".into()
        };
    }
    // `{:?}` de Rust usa la representacion mas corta que hace round-trip.
    let repr = format!("{f:?}");
    if !repr.contains('e') && !repr.contains('E') {
        // Python siempre emite al menos un decimal en un float.
        return if repr.contains('.') {
            repr
        } else {
            format!("{repr}.0")
        };
    }
    let (mantissa, exponent) = repr.split_once(['e', 'E']).unwrap_or((repr.as_str(), "0"));
    let exp: i32 = exponent.parse().unwrap_or(0);
    let sign = if exp < 0 { '-' } else { '+' };
    format!("{mantissa}e{sign}{:02}", exp.unsigned_abs())
}

/// Escape de cadena identico a `json.dumps(..., ensure_ascii=False)`.
fn write_string(s: &str, out: &mut String) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\u{08}' => out.push_str("\\b"),
            '\t' => out.push_str("\\t"),
            '\n' => out.push_str("\\n"),
            '\u{0c}' => out.push_str("\\f"),
            '\r' => out.push_str("\\r"),
            c if (c as u32) < 0x20 => {
                out.push_str(&format!("\\u{:04x}", c as u32));
            }
            c => out.push(c),
        }
    }
    out.push('"');
}

/// SHA-256 en hex minusculas.
pub fn sha256_hex(data: &[u8]) -> String {
    let digest = Sha256::digest(data);
    hex::encode(digest)
}

/// SHA-256 de un valor canonico.
pub fn hash_value(value: &Value) -> String {
    sha256_hex(&canonical_json(value))
}

/// Hash del bundle de evidencia: las tres secciones juntas, como fija `docs/ruleset.md`.
pub fn compute_evidence_bundle_hash(bundle: &Value) -> String {
    hash_value(bundle)
}

/// Hash del reporte, calculado sobre el reporte sin `report_hash`.
pub fn compute_report_hash(report_without_hash: &Value) -> String {
    hash_value(report_without_hash)
}

/// Exige ASCII puro; el ruleset rechaza cualquier otro texto.
pub fn is_ascii_string(v: &Value) -> bool {
    matches!(v, Value::String(s) if s.is_ascii())
}

/// Recorta whitespace ASCII de los extremos y pasa a mayusculas, sin tocar espacios internos.
pub fn normalize_id(s: &str) -> String {
    s.trim_matches(|c: char| ASCII_EDGE_WHITESPACE.contains(&c))
        .to_ascii_uppercase()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn canonical_sorts_keys_and_uses_minimal_separators() {
        let v = json!({"b": 1, "a": {"d": 2, "c": 3}});
        assert_eq!(canonical_json(&v), br#"{"a":{"c":3,"d":2},"b":1}"#.to_vec());
    }

    #[test]
    fn canonical_keeps_non_ascii_unescaped() {
        let v = json!({"k": "ñandú-✓"});
        assert_eq!(
            canonical_json(&v),
            "{\"k\":\"ñandú-✓\"}".as_bytes().to_vec()
        );
    }

    #[test]
    fn canonical_escapes_control_chars_like_python() {
        let v = json!({"k": "a\u{1}b\nc\td"});
        assert_eq!(canonical_json(&v), br#"{"k":"a\u0001b\nc\td"}"#.to_vec());
    }

    #[test]
    fn strict_strips_leading_bom() {
        let parsed = strict_loads("\u{feff}{\"k\":1}").unwrap();
        assert_eq!(parsed.0, json!({"k": 1}));
    }

    #[test]
    fn strict_rejects_duplicate_keys() {
        let err = strict_loads("{\"a\":1,\"a\":2}").unwrap_err();
        assert!(err.to_string().contains("duplicada"), "{err}");
    }

    #[test]
    fn strict_rejects_duplicate_keys_nested() {
        let err = strict_loads("{\"o\":{\"a\":1,\"a\":2}}").unwrap_err();
        assert!(err.to_string().contains("duplicada"), "{err}");
    }

    #[test]
    fn strict_rejects_nan() {
        assert!(strict_loads("{\"a\":NaN}").is_err());
    }

    #[test]
    fn python_float_repr_matches_cpython() {
        assert_eq!(python_float_repr(1000.0), "1000.0");
        assert_eq!(python_float_repr(1000.5), "1000.5");
        assert_eq!(python_float_repr(1e-7), "1e-07");
        assert_eq!(python_float_repr(1e20), "1e+20");
    }

    #[test]
    fn normalize_id_trims_and_uppercases() {
        assert_eq!(normalize_id("  cp-01\t"), "CP-01");
        assert_eq!(normalize_id("cp 01"), "CP 01");
    }
}
