# Attestation agent · etapa 1 (P0-07 parcial)

Motor determinista + CLI + tests. Sin HTTP, sin firma, sin keeper: eso es etapa 2 y depende de P0-00 (stack/ABI) y P0-02 (contrato).
Los stubs Rust de `src/` permanecen intactos hasta que D-003 se cierre en P0-00.
Uso: `python -m app.engine` (oráculo contra fixtures/manifest.json) y
`python -m app.engine evaluate <pass|fail|dispute> <hash64> <amount> [token]`.
Exit codes: 0 PASS/ok · 1 FAIL · 2 rechazo o error de uso.
Este motor no lee el contrato ni envía transacciones; los valores de escrow son supuestos de trabajo etiquetados como tales.