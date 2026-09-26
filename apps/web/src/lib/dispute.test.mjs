import assert from "node:assert/strict";
import test from "node:test";

import {
  canRaiseDispute,
  canResolveDispute,
  validateDisputeRequest,
  validateResolutionRequest,
} from "./dispute.ts";

const reason = "a".repeat(64);
const evidence = "b".repeat(64);

test("only the buyer may dispute PASS and the supplier may dispute FAIL", () => {
  assert.equal(canRaiseDispute("buyer", "ATTESTED_PASS"), true);
  assert.equal(canRaiseDispute("supplier", "ATTESTED_FAIL"), true);
  for (const [role, status] of [
    ["buyer", "ATTESTED_FAIL"], ["supplier", "ATTESTED_PASS"],
    ["resolver", "DISPUTED"], ["observer", "ATTESTED_PASS"],
    [null, "ATTESTED_FAIL"], ["buyer", "RELEASED"],
  ]) {
    assert.equal(canRaiseDispute(role, status), false, `${role} at ${status}`);
    assert.deepEqual(validateDisputeRequest({ role, status, reasonHash: reason, disputeEvidenceHash: evidence }).issues, ["action"]);
  }
});

test("dispute requires two complete public 32-byte hashes, not files or arbitrary text", () => {
  const eligible = { role: "buyer", status: "ATTESTED_PASS" };
  assert.deepEqual(validateDisputeRequest({ ...eligible, reasonHash: `0x${reason.toUpperCase()}`, disputeEvidenceHash: evidence }), {
    ok: true,
    value: { reasonHash: reason, disputeEvidenceHash: evidence },
  });
  for (const invalid of ["", "0x", "a".repeat(63), "g".repeat(64), `${reason}.pdf`]) {
    const result = validateDisputeRequest({ ...eligible, reasonHash: invalid, disputeEvidenceHash: evidence });
    assert.equal(result.ok, false, invalid);
    assert.deepEqual(result.issues, ["reasonHash"]);
  }
  assert.deepEqual(validateDisputeRequest({ ...eligible, reasonHash: "bad", disputeEvidenceHash: "bad" }).issues,
    ["reasonHash", "disputeEvidenceHash"]);
});

test("only the resolver may prepare a ruling while DISPUTED", () => {
  assert.equal(canResolveDispute("resolver", "DISPUTED"), true);
  for (const [role, status] of [["buyer", "DISPUTED"], ["observer", "DISPUTED"], ["resolver", "ATTESTED_PASS"], [null, "DISPUTED"]]) {
    assert.equal(canResolveDispute(role, status), false);
    assert.deepEqual(validateResolutionRequest({ role, status, outcome: "SPLIT", splitBps: "5000" }).issues, ["action"]);
  }
});

test("split accepts only whole basis points 1..9999; other outcomes require zero", () => {
  const eligible = { role: "resolver", status: "DISPUTED" };
  for (const amount of ["1", "5000", "9999"]) {
    assert.deepEqual(validateResolutionRequest({ ...eligible, outcome: "SPLIT", splitBps: amount }), {
      ok: true,
      value: { outcome: "SPLIT", splitBps: Number(amount) },
    });
  }
  for (const amount of ["", "0", "10000", "10001", "1.5", "-1", "2e3", "Infinity"]) {
    assert.deepEqual(validateResolutionRequest({ ...eligible, outcome: "SPLIT", splitBps: amount }).issues, ["splitBps"]);
  }
  for (const outcome of ["RELEASE", "REFUND"]) {
    assert.deepEqual(validateResolutionRequest({ ...eligible, outcome, splitBps: "" }), {
      ok: true,
      value: { outcome, splitBps: 0 },
    });
    assert.deepEqual(validateResolutionRequest({ ...eligible, outcome, splitBps: "1" }).issues, ["splitBps"]);
  }
  assert.deepEqual(validateResolutionRequest({ ...eligible, outcome: "UNKNOWN", splitBps: "0" }).issues, ["outcome"]);
});
