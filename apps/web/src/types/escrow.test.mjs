import assert from "node:assert/strict";
import test from "node:test";

import { deriveWalletRole, getAvailableActions, mapOnChainEscrow } from "./escrow.ts";

const parties = {
  buyer: "GBUYER",
  supplier: "GSUPPLIER",
  resolver: "GRESOLVER",
  engine: "GENGINE",
};

test("wallet role is derived from the current escrow participants", () => {
  assert.equal(deriveWalletRole(null, parties), null);
  assert.equal(deriveWalletRole(" GBUYER ", parties), "buyer");
  assert.equal(deriveWalletRole("GSUPPLIER", parties), "supplier");
  assert.equal(deriveWalletRole("GRESOLVER", parties), "resolver");
  assert.equal(deriveWalletRole("GENGINE", parties), "observer");
  assert.equal(deriveWalletRole("GOTHER", parties), "observer");
  assert.equal(deriveWalletRole("GOTHER", null), null);
  assert.equal(deriveWalletRole("GOTHER", { ...parties, resolver: "" }), null);
});

test("finalize is available only in eligible nonterminal states after expiry", () => {
  for (const status of ["CREATED", "CANCELLED", "RELEASED", "REFUNDED", "SPLIT"]) {
    assert.equal(getAvailableActions("observer", status, true).some((action) => action.id === "finalize"), false, status);
  }
  assert.equal(getAvailableActions("observer", "FUNDED", false).length, 0);
  for (const [status, outcome] of [
    ["FUNDED", "REFUNDED"],
    ["EVIDENCE_SUBMITTED", "REFUNDED"],
    ["ATTESTED_PASS", "RELEASED"],
    ["ATTESTED_FAIL", "REFUNDED"],
  ]) {
    assert.equal(getAvailableActions(null, status, true).find((action) => action.id === "finalize")?.expectedOutcome, outcome);
  }
});

test("disputed finalize maps each fallback to its terminal outcome", () => {
  for (const [fallback, outcome] of [
    ["RELEASE", "RELEASED"],
    ["REFUND", "REFUNDED"],
    ["SPLIT", "SPLIT"],
  ]) {
    assert.equal(getAvailableActions("observer", "DISPUTED", true, undefined, fallback).find((action) => action.id === "finalize")?.expectedOutcome, outcome);
  }
  assert.equal(getAvailableActions("observer", "DISPUTED", true).find((action) => action.id === "finalize")?.expectedOutcome, undefined);
});

test("chain mapping never fills missing fields from mock data", () => {
  const partial = mapOnChainEscrow("FUNDED", { buyer: parties.buyer }, "CCONTRACT");
  assert.equal(partial?.source, "onchain");
  assert.equal(partial?.status, "FUNDED");
  assert.equal(partial?.operationId, "CCONTRACT");
  assert.equal(partial?.amount, "");
  assert.equal(partial?.asset, "");
  assert.deepEqual(partial?.hashes, {});
  assert.equal(partial?.activeDeadline, undefined);
  assert.equal(partial?.fallbackOutcome, undefined);
  assert.equal(partial?.parties.supplier, "");
  assert.equal(mapOnChainEscrow("UNKNOWN", {}, "CCONTRACT"), null);
  assert.equal(mapOnChainEscrow(null, {}, "CCONTRACT"), null);
  assert.equal(mapOnChainEscrow("FUNDED", { amount: 123n, fallbackOutcome: "RELEASE" }, "CCONTRACT")?.amount, "123");
});

test("chain participants, not preview participants, control the wallet role", () => {
  const chainData = mapOnChainEscrow("DISPUTED", { parties, fallbackOutcome: "REFUND" }, "CCONTRACT");
  assert.equal(deriveWalletRole("GRESOLVER", chainData?.parties), "resolver");
  assert.equal(deriveWalletRole("GBUYER", chainData?.parties), "buyer");
  assert.equal(deriveWalletRole("GOTHER", chainData?.parties), "observer");
  assert.equal(getAvailableActions("observer", chainData.status, true, undefined, chainData.fallbackOutcome)
    .find((action) => action.id === "finalize")?.expectedOutcome, "REFUNDED");
});
