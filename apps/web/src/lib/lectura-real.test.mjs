// Lee un contrato REAL de testnet y le pasa el resultado a los mismos
// decodificadores que usa la web. Comprueba el camino de lectura de verdad,
// no un doble.
import assert from "node:assert/strict";
import { test } from "node:test";
import * as StellarSdk from "@stellar/stellar-sdk";
import { mapEscrowState, mapEscrowConfig } from "./soroban.ts";

const REAL = "CCGZQCVPZPJLTTD4NSCL7MZWFH6PKFAAZCSGXCPZH7ZEB7T56L2WJ2OF";
const RPC = "https://soroban-testnet.stellar.org";

test("el decodificador de estado entiende lo que devuelve la cadena real", async () => {
  const srv = new StellarSdk.rpc.Server(RPC);
  const res = await srv.queryContract(REAL, "state");
  assert.equal(res.isReadCall, true, "la lectura debe venir marcada como read-only");
  const estado = mapEscrowState(res.result);
  assert.equal(estado, "ATTESTED_PASS", `decodifico: ${estado}`);
});

test("el decodificador de config entiende lo que devuelve la cadena real", async () => {
  const srv = new StellarSdk.rpc.Server(RPC);
  const res = await srv.queryContract(REAL, "config");
  assert.equal(res.isReadCall, true);
  const config = mapEscrowConfig(res.result);
  assert.ok(config, "la config debe decodificar");
  // Los 14 campos del EscrowConfig del contrato, por nombre.
  assert.equal(Object.keys(config).length, 14, "config con los 14 campos");
  // El importe llega como BigInt desde Soroban: la cadena lo declara i128.
  // La UI lo normaliza con String() antes de renderizar (ver types/escrow.ts).
  assert.equal(typeof config.amount, "bigint", "el SDK entrega i128 como BigInt");
  assert.equal(String(config.amount), "10000000000", "importe leido del contrato");
  assert.match(config.token, /^C[A-Z2-7]{55}$/, "token leido debe ser direccion C...");
  assert.match(config.engine, /^G[A-Z2-7]{55}$/, "engine leido debe ser direccion G...");
  assert.equal(config.fallbackOutcome, "REFUND", "fallback leido del contrato");
});

test("snapshot() tambien se lee y trae los plazos", async () => {
  const srv = new StellarSdk.rpc.Server(RPC);
  const res = await srv.queryContract(REAL, "snapshot");
  assert.equal(res.isReadCall, true);
  const s = res.result;
  assert.ok(s.state, "snapshot debe traer estado");
  assert.ok(s.ledger_timestamp > 0, "snapshot debe traer el reloj del ledger");
  assert.ok(s.config, "snapshot debe traer la config");
});

test("un contract id invalido falla cerrado", async () => {
  const srv = new StellarSdk.rpc.Server(RPC);
  await assert.rejects(
    () => srv.queryContract("no-es-un-contract-id", "state"),
    (e) => /Invalid contract ID|invalid/i.test(String(e.message ?? e)),
  );
});

test("un StrKey con CRC corrupto falla cerrado", async () => {
  const srv = new StellarSdk.rpc.Server(RPC);
  const roto = REAL.slice(0, -1) + (REAL.endsWith("O") ? "P" : "O");
  await assert.rejects(
    () => srv.queryContract(roto, "state"),
    (e) => /Invalid contract ID|invalid|checksum/i.test(String(e.message ?? e)),
  );
});
