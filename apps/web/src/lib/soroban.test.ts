import assert from "node:assert/strict";
import { createRequire } from "node:module";
import { after, before, test } from "node:test";
import * as StellarSdk from "@stellar/stellar-sdk";

// Node 24 can load this TypeScript module directly; the app's TS config does not enable .ts import specifiers.
const { fetchOnChainEscrow, fetchOnChainSnapshot, isCompleteEscrowConfig, mapEscrowConfig, mapEscrowState } = createRequire(import.meta.url)(
  "./soroban.ts"
) as typeof import("./soroban");

test("maps every documented Soroban state variant to the UI status", () => {
  const variants = {
    Created: "CREATED",
    Funded: "FUNDED",
    EvidenceSubmitted: "EVIDENCE_SUBMITTED",
    AttestedPass: "ATTESTED_PASS",
    AttestedFail: "ATTESTED_FAIL",
    Disputed: "DISPUTED",
    Cancelled: "CANCELLED",
    Released: "RELEASED",
    Refunded: "REFUNDED",
    Split: "SPLIT",
  } as const;

  for (const [variant, status] of Object.entries(variants)) {
    assert.equal(mapEscrowState(variant), status);
    assert.equal(mapEscrowState({ tag: variant, values: undefined }), status);
  }

  // ContractSpec.enumToNative returns the u32 discriminant, not the variant name.
  const decodedStates = [
    "CREATED", "FUNDED", "EVIDENCE_SUBMITTED", "ATTESTED_PASS", "ATTESTED_FAIL",
    "RELEASED", "CANCELLED", "REFUNDED", "DISPUTED", "SPLIT",
  ];
  decodedStates.forEach((status, discriminant) => {
    assert.equal(mapEscrowState(discriminant), status);
  });
});

test("rejects unknown or malformed state instead of treating it as an escrow status", () => {
  for (const state of ["Surprise", "Evidence_Submitted", {}, { tag: "Surprise" }, null,
    -1, 10, 2.5, NaN, "2"]) {
    assert.equal(mapEscrowState(state), null);
  }
});

test("normalizes contract config keys and the fallback enum without changing original data", () => {
  const raw = {
    buyer: "GBUYER",
    fallback_outcome: { tag: "Split", values: undefined },
    fallback_split_bps: 3500,
    submission_period: 3600,
    amount: BigInt(10),
  };

  assert.deepEqual(mapEscrowConfig(raw), {
    buyer: "GBUYER",
    fallbackOutcome: "SPLIT",
    fallbackSplitBps: 3500,
    submissionPeriod: 3600,
    amount: BigInt(10),
  });
  assert.equal("fallback_outcome" in raw, true);
  assert.equal(mapEscrowConfig(null), null);
  assert.equal(mapEscrowConfig([]), null);
  assert.equal(mapEscrowConfig({ fallback_outcome: { tag: "Unknown" } }), null);
});

test("maps the explicit numeric fallback discriminants and rejects unknown values", () => {
  for (const [discriminant, outcome] of [[1, "RELEASE"], [2, "REFUND"], [3, "SPLIT"]] as const) {
    assert.deepEqual(mapEscrowConfig({ fallback_outcome: discriminant, fallback_split_bps: 5000 }), {
      fallbackOutcome: outcome,
      fallbackSplitBps: 5000,
    });
  }

  for (const invalid of [0, 4, -1, 1.5, NaN, "1"]) {
    assert.equal(mapEscrowConfig({ fallback_outcome: invalid }), null);
  }
});

test("only marks snapshots complete when all EscrowConfig fields are present", () => {
  const complete = {
    amount: BigInt(10), attestationPeriod: 1, buyer: "GBUYER", correctionPeriod: 1,
    engine: "GENGINE", fallbackOutcome: "RELEASE", fallbackSplitBps: 0,
    maxCorrectionAttempts: 1, objectionPeriod: 1, resolutionPeriod: 1,
    resolver: "GRESOLVER", submissionPeriod: 1, supplier: "GSUPPLIER", token: "GTOKEN",
  };
  assert.equal(isCompleteEscrowConfig(complete), true);
  assert.equal(isCompleteEscrowConfig({ buyer: "GBUYER" }), false);
});

const originalWindow = Object.getOwnPropertyDescriptor(globalThis, "window");
const originalQuery = StellarSdk.rpc.Server.prototype.queryContract;

function setQueryResponse(
  response: (id: string, method: string) => { result: unknown; isReadCall: boolean }
) {
  StellarSdk.rpc.Server.prototype.queryContract = async <T>(id: string, method: string) =>
    response(id, method) as { result: T; isReadCall: boolean };
}

before(() => {
  Object.defineProperty(globalThis, "window", { configurable: true, value: {} });
});

after(() => {
  StellarSdk.rpc.Server.prototype.queryContract = originalQuery;
  if (originalWindow) Object.defineProperty(globalThis, "window", originalWindow);
  else Reflect.deleteProperty(globalThis, "window");
});

test("reads both view methods with queryContract and returns normalized values", async () => {
  const calls: Array<[string, string]> = [];
  setQueryResponse((id, method) => {
    calls.push([id, method]);
    return method === "state"
      ? { result: 2, isReadCall: true }
      : { result: { buyer: "GBUYER", fallback_outcome: 1 }, isReadCall: true };
  });

  const result = await fetchOnChainEscrow("CONTRACT");
  assert.deepEqual(calls, [["CONTRACT", "state"], ["CONTRACT", "config"]]);
  assert.equal(result.state, "EVIDENCE_SUBMITTED");
  assert.deepEqual(result.config, { buyer: "GBUYER", fallbackOutcome: "RELEASE" });
  assert.deepEqual(result.rawConfig, { buyer: "GBUYER", fallback_outcome: 1 });
  assert.equal(result.error, undefined);
});

test("does not accept simulation-only results from either query", async () => {
  for (const unsafeMethod of ["state", "config"]) {
    setQueryResponse((_id, method) => ({
      result: method === "state" ? "Funded" : { buyer: "GBUYER" },
      isReadCall: method !== unsafeMethod,
    }));
    const result = await fetchOnChainEscrow("CONTRACT");
    assert.equal(result.state, null);
    assert.equal(result.config, null);
    assert.match(result.error ?? "", /read-only/i);
  }
});

test("unknown state and malformed config fail closed", async () => {
  for (const [state, config] of [[10, { buyer: "GBUYER" }], [1, []],
    [1, { fallback_outcome: 4 }]] as const) {
    setQueryResponse((_id, method) => ({
      result: method === "state" ? state : config,
      isReadCall: true,
    }));
    const result = await fetchOnChainEscrow("CONTRACT");
    assert.equal(result.state, null);
    assert.equal(result.config, null);
    assert.ok(result.error);
  }
});

test("does not treat an incomplete snapshot as a complete on-chain read", async () => {
  setQueryResponse((_id, method) => {
    assert.equal(method, "snapshot");
    return { result: { state: "Funded", config: { buyer: "GBUYER" } }, isReadCall: true };
  });
  const result = await fetchOnChainSnapshot("CONTRACT");
  assert.equal(result.state, null);
  assert.match(result.error ?? "", /incomplete snapshot/i);
});
