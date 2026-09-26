//! Paridad con los fixtures publicados por el motor de referencia.
//!
//! La comprobacion real esta en `attestation_agent::golden`, dentro de la libreria, para
//! que este test y el subcomando `selfcheck` del binario ejecuten el mismo codigo.
use serde_json::Value;

use attestation_agent::canonical::{strict_loads, Strict};
use attestation_agent::golden::{fixture_dir, load_case, manifest, verify_fixtures, CASES};
use attestation_agent::report::build_report;

#[test]
fn every_case_reproduces_the_manifest() {
    assert_eq!(verify_fixtures(&fixture_dir()).unwrap(), CASES.len());
}

#[test]
fn bundle_hash_is_insensitive_to_key_order_and_whitespace() {
    let manifest = manifest(&fixture_dir()).unwrap();
    let expected = manifest["pass"]["evidence_bundle_hash"].as_str().unwrap();

    let compact = r#"{"purchase_order":{"amount":10000000000,"currency":"CPUSD","id":"PO-001","supplier_id":"SUP-001"},"invoice":{"amount":10000000000,"currency":"CPUSD","id":"INV-001","purchase_order_id":"PO-001","supplier_id":"SUP-001"},"delivery":{"accepted":true,"purchase_order_id":"PO-001"}}"#;
    let spaced = r#"{
        "delivery": { "accepted": true, "purchase_order_id": "PO-001" },
        "invoice": { "id": "INV-001", "purchase_order_id": "PO-001", "supplier_id": "SUP-001", "amount": 10000000000, "currency": "CPUSD" },
        "purchase_order": { "id": "PO-001", "supplier_id": "SUP-001", "amount": 10000000000, "currency": "CPUSD" }
    }"#;

    for text in [compact, spaced] {
        let Strict(value) = strict_loads(text).unwrap();
        let report = build_report(&value, 10_000_000_000, "CPUSD").unwrap();
        assert_eq!(&report.evidence_bundle_hash, expected);
    }
}

#[test]
fn fail_case_names_the_failing_field() {
    let manifest = manifest(&fixture_dir()).unwrap();
    let bundle: Value = load_case(&fixture_dir(), "fail").unwrap();
    let report = build_report(
        &bundle,
        manifest["fail"]["escrow_amount"].as_i64().unwrap() as i128,
        "CPUSD",
    )
    .unwrap();
    assert_eq!(report.result, "FAIL");
    let paths: Vec<&str> = report
        .failed_fields
        .iter()
        .map(|f| f.path.as_str())
        .collect();
    assert!(
        paths.contains(&"invoice.amount"),
        "fallos reportados: {paths:?}"
    );
    let f = report
        .failed_fields
        .iter()
        .find(|f| f.path == "invoice.amount")
        .unwrap();
    assert_eq!(f.status.as_str(), "MISMATCH");
    assert!(
        f.observed.is_some(),
        "el valor observado existe en el fixture"
    );
    assert_eq!(f.expected.as_deref(), Some("10000000000"));
}

#[test]
fn same_bundle_same_hash_across_repeated_runs() {
    let bundle = load_case(&fixture_dir(), "pass").unwrap();
    let mut seen = std::collections::BTreeSet::new();
    for _ in 0..8 {
        let report = build_report(&bundle, 10_000_000_000, "CPUSD").unwrap();
        seen.insert((report.report_hash.clone(), report.detail_hash.clone()));
    }
    assert_eq!(seen.len(), 1, "el motor debe ser determinista");
}

/// Paridad con el motor Python de referencia en bundles que **no** estan en el manifiesto.
///
/// Los valores esperados se obtuvieron ejecutando `app/engine.py` sobre estos mismos
/// bundles. Asi se cubre el caso mas peligroso: que el motor Rust y el de referencia
/// diverjan en algo que los fixtures no exercised. `detail_hash` no se compara porque es
/// una adicion de Rust que el motor Python no produce, a proposito.
#[test]
fn matches_the_python_reference_on_bundles_outside_the_manifest() {
    // (bundle, escrow_amount, evidence_bundle_hash, report_hash, result)
    let cases: [(&str, i128, &str, &str, &str); 2] = [
        (
            r#"{"purchase_order":{"id":"PO-001","supplier_id":"SUP-001","amount":1000,"currency":"CPUSD"},"invoice":{"id":"INV-001","purchase_order_id":"PO-001","supplier_id":"SUP-001","amount":1000,"currency":"CPUSD"},"delivery":{"purchase_order_id":"PO-001","accepted":true}}"#,
            1000,
            "96a3ba11a55dd590134c47c3ad93f0031730b155c97798a0cbadba7a1d224cce",
            "48a64ae142009495742c89d484cbed2d809d420dfcaaffdefe19f3c4d230b447",
            "PASS",
        ),
        (
            r#"{"purchase_order":{"id":"PO-001","supplier_id":"SUP-001","amount":1000,"currency":"CPUSD"},"invoice":{"id":"INV-001","purchase_order_id":"PO-001","supplier_id":"SUP-001","amount":900,"currency":"CPUSD"},"delivery":{"purchase_order_id":"PO-001","accepted":true}}"#,
            1000,
            "8b4cc611e8a86a0b3558155bc7e7f90fd428dcbc31365b8727675a108c2c6b29",
            "2769763537ee9f80e46a2495c4688679ef22bba2468d3063ec966721348756d6",
            "FAIL",
        ),
    ];

    for (json, amount, bundle_hash, report_hash, result) in cases {
        let Strict(value) = strict_loads(json).unwrap();
        let report = build_report(&value, amount, "CPUSD").unwrap();
        assert_eq!(report.evidence_bundle_hash, bundle_hash, "bundle hash");
        assert_eq!(report.report_hash, report_hash, "report hash");
        assert_eq!(report.result, result);
    }
}
