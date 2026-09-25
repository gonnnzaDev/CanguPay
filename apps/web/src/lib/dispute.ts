import type { EscrowStatus, FallbackOutcome } from "../types/escrow";
import type { UserRole } from "../types/wallet";

type Issue = "action" | "reasonHash" | "disputeEvidenceHash" | "outcome" | "splitBps";

type Validation<T> =
  | { ok: true; value: T }
  | { ok: false; issues: Issue[] };

export function canRaiseDispute(role: UserRole | null, status: EscrowStatus): boolean {
  return (role === "buyer" && status === "ATTESTED_PASS") ||
    (role === "supplier" && status === "ATTESTED_FAIL");
}

export function canResolveDispute(role: UserRole | null, status: EscrowStatus): boolean {
  return role === "resolver" && status === "DISPUTED";
}

function normalizeHash(value: string): string | null {
  const match = /^(?:0x)?([a-f0-9]{64})$/i.exec(value.trim());
  return match ? match[1].toLowerCase() : null;
}

/** Validate public hashes only; private documents are never passed to the contract. */
export function validateDisputeRequest(input: {
  role: UserRole | null;
  status: EscrowStatus;
  reasonHash: string;
  disputeEvidenceHash: string;
}): Validation<{ reasonHash: string; disputeEvidenceHash: string }> {
  if (!canRaiseDispute(input.role, input.status)) return { ok: false, issues: ["action"] };

  const reasonHash = normalizeHash(input.reasonHash);
  const disputeEvidenceHash = normalizeHash(input.disputeEvidenceHash);
  const issues: Issue[] = [];
  if (!reasonHash) issues.push("reasonHash");
  if (!disputeEvidenceHash) issues.push("disputeEvidenceHash");
  if (issues.length) return { ok: false, issues };

  return { ok: true, value: { reasonHash: reasonHash!, disputeEvidenceHash: disputeEvidenceHash! } };
}

/** A preview of contract argument validation, not proof of a valid ledger deadline. */
export function validateResolutionRequest(input: {
  role: UserRole | null;
  status: EscrowStatus;
  outcome: string;
  splitBps: string;
}): Validation<{ outcome: FallbackOutcome; splitBps: number }> {
  if (!canResolveDispute(input.role, input.status)) return { ok: false, issues: ["action"] };

  const outcome = input.outcome;
  if (outcome !== "RELEASE" && outcome !== "REFUND" && outcome !== "SPLIT") {
    return { ok: false, issues: ["outcome"] };
  }

  const bps = input.splitBps.trim();
  if (outcome === "SPLIT") {
    if (!/^[1-9][0-9]{0,3}$/.test(bps) || Number(bps) > 9999) {
      return { ok: false, issues: ["splitBps"] };
    }
    return { ok: true, value: { outcome, splitBps: Number(bps) } };
  }

  if (bps !== "" && bps !== "0") return { ok: false, issues: ["splitBps"] };
  return { ok: true, value: { outcome, splitBps: 0 } };
}
