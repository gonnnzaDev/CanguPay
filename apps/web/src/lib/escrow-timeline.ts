"use client";

import * as StellarSdk from "@stellar/stellar-sdk";
import type { EscrowHashes } from "../types/escrow";

export type EscrowTimelineKind =
  | "created" | "funded" | "evidence_submitted" | "attested" | "approved"
  | "dispute_raised" | "resolved" | "finalized" | "cancelled";

export interface EscrowTimelineEntry {
  id: string;
  kind: EscrowTimelineKind;
  ledger: number;
  occurredAt: string;
  txHash: string;
  deadline?: number;
  deadlineSource?: "event" | "derived";
  hashes?: EscrowHashes;
}

export interface EscrowTimelineRead {
  events: EscrowTimelineEntry[];
  limitedHistory: boolean;
  ledgerTime: number | null;
  error?: string;
}

export interface EscrowTimelineConfig {
  attestationPeriod?: unknown;
  objectionPeriod?: unknown;
  correctionPeriod?: unknown;
  resolutionPeriod?: unknown;
}

const EVENT_KINDS: Record<string, EscrowTimelineKind> = {
  escrow_created: "created", funded: "funded", evidence_submitted: "evidence_submitted",
  attested: "attested", approved: "approved", dispute_raised: "dispute_raised",
  resolved: "resolved", finalized: "finalized", cancelled: "cancelled",
};

const MAX_DATE_SECONDS = 8_640_000_000_000;

function seconds(value: unknown): number | null {
  if (typeof value !== "number" && typeof value !== "bigint") return null;
  const number = Number(value);
  return Number.isSafeInteger(number) && number >= 0 && number <= MAX_DATE_SECONDS ? number : null;
}

function timestamp(value: string): number | null {
  const time = Date.parse(value);
  return Number.isFinite(time) ? seconds(Math.floor(time / 1000)) : null;
}

function decode(value: StellarSdk.xdr.ScVal): unknown {
  try {
    return StellarSdk.scValToNative(value);
  } catch {
    return null;
  }
}

function eventFields(event: Pick<StellarSdk.rpc.Api.EventResponse, "value">): Record<string, unknown> {
  const decoded = decode(event.value);
  return decoded && typeof decoded === "object" && !Array.isArray(decoded)
    ? decoded as Record<string, unknown> : {};
}

function publicHash(value: unknown): string | null {
  return value instanceof Uint8Array && value.length === 32
    ? Array.from(value, (byte) => byte.toString(16).padStart(2, "0")).join("") : null;
}

/** Events are public but not a complete contract history outside the RPC retention window. */
export function mapPublicEscrowEvent(
  event: Pick<StellarSdk.rpc.Api.EventResponse,
    "id" | "type" | "ledger" | "ledgerClosedAt" | "inSuccessfulContractCall" | "txHash" | "topic" | "value">,
  config: EscrowTimelineConfig,
): EscrowTimelineEntry | null {
  if (event.type !== "contract" || !event.inSuccessfulContractCall ||
      !Number.isSafeInteger(event.ledger) || event.ledger < 1 ||
      !/^[a-f0-9]{64}$/i.test(event.txHash) || !event.topic.length) return null;

  const closedAt = timestamp(event.ledgerClosedAt);
  if (closedAt === null) return null;

  const rawTopic = decode(event.topic[0]);
  if (typeof rawTopic !== "string") return null;
  const kind = EVENT_KINDS[rawTopic.replace(/([a-z0-9])([A-Z])/g, "$1_$2").toLowerCase()];
  if (!kind) return null;

  const entry: EscrowTimelineEntry = {
    id: event.id, kind, ledger: event.ledger,
    occurredAt: event.ledgerClosedAt, txHash: event.txHash,
  };

  const fields = eventFields(event);
  const exposed: EscrowHashes = {};
  const candidates: Array<[keyof EscrowHashes, unknown]> = kind === "dispute_raised"
    ? [["reasonHash", fields.reason_hash], ["disputeEvidenceHash", fields.dispute_evidence_hash]]
    : kind === "evidence_submitted" ? [["evidenceBundleHash", fields.evidence_bundle_hash]]
      : kind === "attested" ? [["reportHash", fields.report_hash]] : [];
  for (const [name, value] of candidates) {
    const hash = publicHash(value);
    if (hash) exposed[name] = hash;
  }
  if (Object.keys(exposed).length) entry.hashes = exposed;
  const attestationOutcome = seconds(fields.outcome);
  const period = kind === "evidence_submitted" ? config.attestationPeriod
    : kind === "dispute_raised" ? config.resolutionPeriod
      : kind === "attested" && attestationOutcome === 0 ? config.objectionPeriod
        : kind === "attested" && attestationOutcome === 1 ? config.correctionPeriod : null;

  if (kind === "attested" && attestationOutcome !== 0 && attestationOutcome !== 1) return null;

  if (kind === "funded") {
    const deadline = seconds(fields.submission_deadline);
    if (deadline !== null && deadline >= closedAt) {
      entry.deadline = deadline;
      entry.deadlineSource = "event";
    }
  } else {
    const duration = seconds(period);
    if (duration !== null && duration > 0 && closedAt + duration <= MAX_DATE_SECONDS) {
      entry.deadline = closedAt + duration;
      entry.deadlineSource = "derived";
    }
  }
  return entry;
}

const RPC_URL = process.env.NEXT_PUBLIC_SOROBAN_RPC_URL || "https://soroban-testnet.stellar.org";
let server: StellarSdk.rpc.Server | null = null;

export async function fetchPublicEscrowTimeline(
  contractId: string,
  config: EscrowTimelineConfig = {},
): Promise<EscrowTimelineRead> {
  const empty: EscrowTimelineRead = { events: [], limitedHistory: true, ledgerTime: null };
  if (!StellarSdk.StrKey.isValidContract(contractId)) return { ...empty, error: "Invalid contract ID" };
  if (typeof window === "undefined") return { ...empty, error: "RPC unavailable on the server" };

  try {
    server ??= new StellarSdk.rpc.Server(RPC_URL);
    const health = await server.getHealth();
    const response = await server.getEvents({
      startLedger: health.oldestLedger,
      filters: [{ type: "contract", contractIds: [contractId] }],
      limit: 100,
    });
    const events = response.events
      .map((event) => mapPublicEscrowEvent(event, config))
      .filter((entry): entry is EscrowTimelineEntry => entry !== null)
      .sort((a, b) => a.ledger - b.ledger || a.id.localeCompare(b.id));
    return {
      events,
      limitedHistory: response.oldestLedger > 1 || response.events.length === 100,
      ledgerTime: timestamp(response.latestLedgerCloseTime),
    };
  } catch (error) {
    return { ...empty, error: error instanceof Error ? error.message : "RPC unavailable" };
  }
}
