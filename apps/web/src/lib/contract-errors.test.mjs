// Regresion de los errores encontrados probando el flujo en navegador.
//
// 1. El codigo del frontend no compilaba por isFallbackOutcome sin exportar, y en
//    runtime eso hacia que fetchOnChainSnapshot fallara siempre. Este test no lo
//    detecta por si solo, pero comprueba que el predicado ahora hace su trabajo.
// 2. Con el plazo vencido la web ofrecia Aprobar y Disputar, que el contrato
//    rechaza con ObjectionDeadlinePassed.
// 3. El error del contrato llegaba al usuario con la traza del evento
//    diagnostico entera.
import assert from "node:assert/strict";
import { createRequire } from "node:module";
import test from "node:test";

import { getAvailableActions, isFallbackOutcome } from "../types/escrow.ts";
import {
  codigoDeErrorContrato,
  mensajeDeErrorContrato,
  esPlazoVencido,
  errorLegible,
} from "./contract-errors.ts";

// soroban.ts importa sin extension, asi que necesita createRequire: el tsconfig
// del app no habilita especificadores .ts.
// Sin cast: este fichero es .mjs y el "as typeof" solo compila en .ts.
const { isCompleteEscrowConfig } = createRequire(import.meta.url)("./soroban.ts");

// ---------- 1. el predicado de config completa ----------

const CONFIG_COMPLETA = {
  amount: 10000000000n,
  attestationPeriod: 3600n,
  buyer: "GBUYER",
  correctionPeriod: 3600n,
  engine: "GENGINE",
  fallbackOutcome: "REFUND",
  fallbackSplitBps: 0,
  maxCorrectionAttempts: 1,
  objectionPeriod: 3600n,
  resolutionPeriod: 3600n,
  resolver: "GRESOLVER",
  submissionPeriod: 3600n,
  supplier: "GSUPPLIER",
  token: "CTOKEN",
};

test("una config con los 14 campos se acepta como completa", () => {
  assert.equal(isCompleteEscrowConfig(CONFIG_COMPLETA), true);
});

test("una config a la que le falta un campo se rechaza", () => {
  const { resolver, ...sinResolver } = CONFIG_COMPLETA;
  assert.equal(resolver, "GRESOLVER", "el fixture debe tener resolver");
  assert.equal(isCompleteEscrowConfig(sinResolver), false);
});

test("un fallbackOutcome desconocido se rechaza", () => {
  assert.equal(isCompleteEscrowConfig({ ...CONFIG_COMPLETA, fallbackOutcome: "INVENTADO" }), false);
  assert.equal(isCompleteEscrowConfig({ ...CONFIG_COMPLETA, fallbackOutcome: null }), false);
});

test("los tres resultados de fallback son los unicos validos", () => {
  assert.equal(isFallbackOutcome("RELEASE"), true);
  assert.equal(isFallbackOutcome("REFUND"), true);
  assert.equal(isFallbackOutcome("SPLIT"), true);
  assert.equal(isFallbackOutcome("RELEASED"), false);
  assert.equal(isFallbackOutcome(undefined), false);
});

// ---------- 2. el plazo vencido deja solo finalizar ----------

test("con el plazo vencido el comprador no ve Aprobar ni Disputar", () => {
  const acciones = getAvailableActions("buyer", "ATTESTED_PASS", true, undefined, "REFUND", true);
  const ids = acciones.map((a) => a.id);
  assert.ok(!ids.includes("approve"), `no debe ofrecer Aprobar: ${ids.join(",")}`);
  assert.ok(!ids.includes("dispute_pass"), `no debe ofrecer Disputar: ${ids.join(",")}`);
  assert.ok(ids.includes("finalize"), `debe ofrecer finalizar: ${ids.join(",")}`);
});

test("con el plazo vencido el proveedor tampoco ve las suyas", () => {
  const acciones = getAvailableActions("supplier", "ATTESTED_PASS", true, undefined, "REFUND", true);
  assert.deepEqual(acciones.map((a) => a.id), ["finalize"]);
});

test("con el plazo vencido y sin poder finalizar no hay ninguna accion", () => {
  const acciones = getAvailableActions("buyer", "ATTESTED_PASS", false, undefined, "REFUND", true);
  assert.deepEqual(acciones, [], "sin finalize posible no se ofrece nada, no algo que fallara");
});

test("con el plazo vencido en disputa el resultado sigue la matriz", () => {
  const conSplit = getAvailableActions("resolver", "DISPUTED", true, undefined, "SPLIT", true);
  assert.equal(conSplit[0]?.expectedOutcome, "SPLIT");
  const conRelease = getAvailableActions("resolver", "DISPUTED", true, undefined, "RELEASE", true);
  assert.equal(conRelease[0]?.expectedOutcome, "RELEASED");
});

test("con el plazo vivo el comprador si ve Aprobar y Disputar", () => {
  const acciones = getAvailableActions("buyer", "ATTESTED_PASS", false, undefined, "REFUND", false);
  const ids = acciones.map((a) => a.id);
  assert.ok(ids.includes("approve"), `debe ofrecer Aprobar: ${ids.join(",")}`);
  assert.ok(ids.includes("dispute_pass"), `debe ofrecer Disputar: ${ids.join(",")}`);
});

test("sin snapshot no se ofrece ninguna accion: no se sabe si el plazo vencio", () => {
  // Esta es la ventana en la que la web ya tiene estado y config pero todavia no
  // el snapshot. Si se ofrece algo, seria ofrecer Aprobar a ciegas.
  for (const rol of ["buyer", "supplier", "resolver", "observer"]) {
    for (const estado of ["ATTESTED_PASS", "ATTESTED_FAIL", "FUNDED", "DISPUTED"]) {
      const ids = getAvailableActions(rol, estado, true, undefined, "REFUND", false, false).map(
        (a) => a.id,
      );
      assert.deepEqual(ids, [], `rol ${rol} en ${estado} ofrece ${ids.join(",")} sin saber el plazo`);
    }
  }
});

test("con el snapshot resuelto el plazo vivo si ofrece las acciones del estado", () => {
  const ids = getAvailableActions("buyer", "ATTESTED_PASS", false, undefined, "REFUND", false, true).map(
    (a) => a.id,
  );
  assert.ok(ids.includes("approve"), `debe ofrecer Aprobar: ${ids.join(",")}`);
});

test("finalizar sigue disponible para cualquiera: el contrato no exige rol", () => {
  for (const rol of ["buyer", "supplier", "resolver", "observer"]) {
    const ids = getAvailableActions(rol, "ATTESTED_PASS", true, undefined, "REFUND", true).map(
      (a) => a.id,
    );
    assert.ok(ids.includes("finalize"), `el rol ${rol} debe poder finalizar por vencimiento`);
  }
});

// ---------- 3. el error del contrato se traduce ----------

const ERROR_REAL =
  'HostError: Error(Contract, #17) Event log (newest first): 0: [Diagnostic Event] ' +
  "contract:CCGZQCVPZPJLTTD4NSCL7MZWFH6PKFAAZCSGXCPZH7ZEB7T56L2WJ2OF, topics:[error, " +
  "Error(Contract, #17)], data:\"escalating error to VM trap from failed host function call: " +
  'fail_with_error" 1: [Diagnostic Event] contract:CCGZQCVPZPJLTTD4NSCL7MZWF';

test("el codigo del contrato se saca de la traza del RPC", () => {
  assert.equal(codigoDeErrorContrato(new Error(ERROR_REAL)), 17);
  assert.equal(codigoDeErrorContrato(new Error("Error(Contract, #5)")), 5);
  assert.equal(codigoDeErrorContrato(new Error("algo sin codigo")), null);
  assert.equal(codigoDeErrorContrato(null), null);
});

test("el codigo 17 se traduce a un mensaje de plazo de objecion", () => {
  assert.match(mensajeDeErrorContrato(new Error(ERROR_REAL)), /plazo de objecion/i);
});

test("un plazo vencido se reconoce como tal", () => {
  assert.equal(esPlazoVencido(new Error(ERROR_REAL)), true);
  assert.equal(esPlazoVencido(new Error("Error(Contract, #5)")), false, "estado invalido no es un plazo");
  assert.equal(esPlazoVencido(new Error("Error(Contract, #16)")), true, "atestacion tambien es plazo");
  assert.equal(esPlazoVencido(new Error("Error(Contract, #7)")), true, "entrega tambien es plazo");
});

test("el mensaje que ve el usuario no lleva la traza del evento diagnostico", () => {
  const mensaje = errorLegible(new Error(ERROR_REAL));
  assert.ok(!/Event log/.test(mensaje), "no debe filtrar la traza");
  assert.ok(!/Diagnostic Event/.test(mensaje), "no debe filtrar el evento diagnostico");
  assert.ok(!/fail_with_error/.test(mensaje), "no debe filtrar el trap de la VM");
  assert.ok(!/CCGZQCVPZPJLTTD4NSCL7MZWF/.test(mensaje), "no debe filtrar el id del contrato");
});

test("el mensaje de un plazo vencido dice que toca finalizar", () => {
  const mensaje = errorLegible(new Error(ERROR_REAL));
  assert.match(mensaje, /venci[oó]/i);
  assert.match(mensaje, /finalizar/i);
});

test("un error que no es del contrato se devuelve tal cual", () => {
  const original = "TypeError: no se puede leer la propiedad";
  assert.equal(errorLegible(new Error(original)), original);
  assert.equal(errorLegible("cadena suelta"), "cadena suelta");
});
