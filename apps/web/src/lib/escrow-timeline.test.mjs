import assert from "node:assert/strict";
import test from "node:test";
import * as StellarSdk from "@stellar/stellar-sdk";

import { fetchPublicEscrowTimeline, mapPublicEscrowEvent } from "./escrow-timeline.ts";

const closedAt = "2026-09-25T01:00:00Z";
const ledgerTime = Date.parse(closedAt) / 1000;
const txHash = "a".repeat(64);

function event(name, data, overrides = {}) {
  return {
    id: "123-0", type: "contract", ledger: 123, ledgerClosedAt: closedAt,
    inSuccessfulContractCall: true, txHash,
    topic: [StellarSdk.xdr.ScVal.scvSymbol(name)],
    value: StellarSdk.nativeToScVal(data),
    ...overrides,
  };
}

test("a Funded event carries its ledger-issued submission deadline and public tx hash", () => {
  const record = mapPublicEscrowEvent(event("funded", { submission_deadline: ledgerTime + 3600 }), {});
  assert.deepEqual(record, {
    id: "123-0", kind: "funded", ledger: 123, occurredAt: closedAt,
    txHash, deadline: ledgerTime + 3600, deadlineSource: "event",
  });
});

test("dispute resolution deadline is derived only from verified ledger close time and config period", () => {
  const input = event("dispute_raised", { reason_hash: "public" });
  const withPeriod = mapPublicEscrowEvent(input, { resolutionPeriod: 86400n });
  assert.equal(withPeriod?.deadline, ledgerTime + 86400);
  assert.equal(withPeriod?.deadlineSource, "derived");
  const withoutPeriod = mapPublicEscrowEvent(input, {});
  assert.equal(withoutPeriod?.deadline, undefined);
  assert.equal(withoutPeriod?.deadlineSource, undefined);
});

test("evidence and attestation deadlines use the corresponding periods, not a browser clock", () => {
  assert.equal(mapPublicEscrowEvent(event("evidence_submitted", {}), { attestationPeriod: 600 })?.deadline, ledgerTime + 600);
  assert.equal(mapPublicEscrowEvent(event("attested", { outcome: 0 }), { objectionPeriod: 100 })?.deadline, ledgerTime + 100);
  assert.equal(mapPublicEscrowEvent(event("attested", { outcome: 1 }), { correctionPeriod: 200 })?.deadline, ledgerTime + 200);
  assert.equal(mapPublicEscrowEvent(event("attested", { outcome: 42 }), { objectionPeriod: 100 }), null);
});

test("the timeline displays only exact public 32-byte hashes from successful events", () => {
  const reasonHash = Uint8Array.from({ length: 32 }, () => 0xaa);
  const disputeEvidenceHash = Uint8Array.from({ length: 32 }, () => 0xbb);
  assert.deepEqual(mapPublicEscrowEvent(event("dispute_raised", {
    reason_hash: reasonHash, dispute_evidence_hash: disputeEvidenceHash,
  }), {})?.hashes, { reasonHash: "aa".repeat(32), disputeEvidenceHash: "bb".repeat(32) });
  assert.deepEqual(mapPublicEscrowEvent(event("attested", {
    outcome: 0, report_hash: reasonHash,
  }), {})?.hashes, { reportHash: "aa".repeat(32) });
  assert.equal(mapPublicEscrowEvent(event("dispute_raised", {
    reason_hash: new Uint8Array(3), dispute_evidence_hash: disputeEvidenceHash,
  }), {})?.hashes?.reasonHash, undefined);
});

test("unknown, failed, malformed or diagnostic events never become audit entries", () => {
  assert.equal(mapPublicEscrowEvent(event("unknown", {}), {}), null);
  assert.equal(mapPublicEscrowEvent(event("funded", {}, { inSuccessfulContractCall: false }), {}), null);
  assert.equal(mapPublicEscrowEvent(event("funded", {}, { type: "system" }), {}), null);
  assert.equal(mapPublicEscrowEvent(event("funded", {}, { ledgerClosedAt: "bad" }), {}), null);
  assert.equal(mapPublicEscrowEvent(event("funded", {}, { txHash: "bad" }), {}), null);
  assert.equal(mapPublicEscrowEvent(event("funded", {}, { topic: [{}] }), {}), null);
  assert.equal(mapPublicEscrowEvent(event("funded", {}, { value: {} }), {})?.deadline, undefined);
});

test("RPC timeline is contract-filtered and explicitly marks retained history as partial", async () => {
  const contractId = StellarSdk.StrKey.encodeContract(Buffer.alloc(32, 1));
  const originalWindow = Object.getOwnPropertyDescriptor(globalThis, "window");
  const originalHealth = StellarSdk.rpc.Server.prototype.getHealth;
  const originalEvents = StellarSdk.rpc.Server.prototype.getEvents;
  const calls = [];
  try {
    Object.defineProperty(globalThis, "window", { configurable: true, value: {} });
    StellarSdk.rpc.Server.prototype.getHealth = async () => ({ oldestLedger: 120, latestLedger: 130, status: "healthy" });
    StellarSdk.rpc.Server.prototype.getEvents = async (options) => {
      calls.push(options);
      return { events: [event("funded", { submission_deadline: ledgerTime + 3600 })], oldestLedger: 120,
        latestLedger: 130, latestLedgerCloseTime: closedAt, cursor: "next" };
    };
    const result = await fetchPublicEscrowTimeline(contractId);
    assert.deepEqual(calls, [{ startLedger: 120, filters: [{ type: "contract", contractIds: [contractId] }], limit: 100 }]);
    assert.equal(result.events.length, 1);
    assert.equal(result.limitedHistory, true);
    assert.equal(result.ledgerTime, ledgerTime);
  } finally {
    StellarSdk.rpc.Server.prototype.getHealth = originalHealth;
    StellarSdk.rpc.Server.prototype.getEvents = originalEvents;
    if (originalWindow) Object.defineProperty(globalThis, "window", originalWindow);
    else Reflect.deleteProperty(globalThis, "window");
  }
});
