"use client";

import * as StellarSdk from "@stellar/stellar-sdk";
import type {
  EscrowDeadline,
  EscrowHashes,
  EscrowStatus,
  FallbackOutcome,
} from "../types/escrow.ts";
// Con extension a proposito: sin ella el test de este modulo no carga, porque
// el resolutor ESM de node no prueba extensiones y el tsconfig del app esta en
// moduleResolution "bundler".
import { isFallbackOutcome } from "../types/escrow.ts";

const RPC_URL = process.env.NEXT_PUBLIC_SOROBAN_RPC_URL || "https://soroban-testnet.stellar.org";
const CONTRACT_ID = process.env.NEXT_PUBLIC_ESCROW_CONTRACT_ID || "";

// Lazily create server to avoid SSR issues
let server: InstanceType<typeof StellarSdk.rpc.Server> | null = null;
function getServer() {
  if (!server && typeof window !== "undefined") {
    server = new StellarSdk.rpc.Server(RPC_URL);
  }
  return server;
}

const STATE_VARIANTS: Record<string, EscrowStatus> = {
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
};

// Discriminants follow EscrowState's declaration order in the contract ABI.
const STATE_DISCRIMINANTS: readonly EscrowStatus[] = [
  "CREATED", "FUNDED", "EVIDENCE_SUBMITTED", "ATTESTED_PASS", "ATTESTED_FAIL",
  "RELEASED", "CANCELLED", "REFUNDED", "DISPUTED", "SPLIT",
];

function enumTag(value: unknown): string | null {
  if (typeof value === "string") return value;
  if (value && typeof value === "object" && !Array.isArray(value) &&
      typeof (value as { tag?: unknown }).tag === "string") {
    return (value as { tag: string }).tag;
  }
  return null;
}

export function mapEscrowState(value: unknown): EscrowStatus | null {
  if (typeof value === "number") {
    return Number.isInteger(value) ? STATE_DISCRIMINANTS[value] ?? null : null;
  }
  const tag = enumTag(value);
  return tag && Object.hasOwn(STATE_VARIANTS, tag) ? STATE_VARIANTS[tag] : null;
}

const FALLBACK_VARIANTS: Record<string, FallbackOutcome> = {
  Release: "RELEASE",
  Refund: "REFUND",
  Split: "SPLIT",
};

const FALLBACK_DISCRIMINANTS: Record<number, FallbackOutcome> = {
  1: "RELEASE",
  2: "REFUND",
  3: "SPLIT",
};

function mapFallbackOutcome(value: unknown): FallbackOutcome | null {
  if (typeof value === "number") {
    return Object.hasOwn(FALLBACK_DISCRIMINANTS, value) ? FALLBACK_DISCRIMINANTS[value] : null;
  }
  const tag = enumTag(value);
  return tag && Object.hasOwn(FALLBACK_VARIANTS, tag) ? FALLBACK_VARIANTS[tag] : null;
}

export function mapEscrowConfig(value: unknown): Record<string, unknown> | null {
  if (!value || typeof value !== "object" || Array.isArray(value)) return null;

  const config: Record<string, unknown> = {};
  for (const [key, field] of Object.entries(value)) {
    const camelKey = key.replace(/_([a-z0-9])/g, (_, letter: string) => letter.toUpperCase());
    if (camelKey === "fallbackOutcome") {
      const outcome = mapFallbackOutcome(field);
      if (!outcome) return null;
      config[camelKey] = outcome;
    } else {
      config[camelKey] = field;
    }
  }
  return config;
}

const REQUIRED_CONFIG_FIELDS = [
  "amount", "attestationPeriod", "buyer", "correctionPeriod", "engine",
  "fallbackOutcome", "fallbackSplitBps", "maxCorrectionAttempts", "objectionPeriod",
  "resolutionPeriod", "resolver", "submissionPeriod", "supplier", "token",
] as const;

/** A complete snapshot must carry every EscrowConfig field, not merely a valid subset. */
export function isCompleteEscrowConfig(config: Record<string, unknown> | null): boolean {
  if (!config || !REQUIRED_CONFIG_FIELDS.every((field) => Object.hasOwn(config, field))) return false;
  const isInteger = (value: unknown) => typeof value === "number" || typeof value === "bigint";
  const isAddress = (value: unknown) => typeof value === "string" && value.length > 0;
  return isInteger(config.amount) && isInteger(config.attestationPeriod) &&
    isAddress(config.buyer) && isInteger(config.correctionPeriod) && isAddress(config.engine) &&
    isFallbackOutcome(config.fallbackOutcome) && isInteger(config.fallbackSplitBps) &&
    isInteger(config.maxCorrectionAttempts) && isInteger(config.objectionPeriod) &&
    isInteger(config.resolutionPeriod) && isAddress(config.resolver) &&
    isInteger(config.submissionPeriod) && isAddress(config.supplier) && isAddress(config.token);
}

/**
 * Reads EscrowConfig and EscrowState directly from chain via soroban RPC.
 * Uses `state()` and `config()` view functions of the ConditionalPayment contract.
 * Returns null fields and an error when the contract ID, read, or mapping fails.
 */
export async function fetchOnChainEscrow(contractId: string = CONTRACT_ID): Promise<{
  config: Record<string, unknown> | null;
  state: EscrowStatus | null;
  rawConfig: unknown | null;
  error?: string;
}> {
  if (!contractId) {
    return { config: null, state: null, rawConfig: null, error: "NEXT_PUBLIC_ESCROW_CONTRACT_ID vacío" };
  }
  const srv = getServer();
  if (!srv) {
    return { config: null, state: null, rawConfig: null, error: "RPC no disponible en SSR" };
  }

  try {
    const stateResponse = await srv.queryContract<unknown>(contractId, "state");
    if (!stateResponse.isReadCall) throw new Error("State query is not read-only");
    const state = mapEscrowState(stateResponse.result);
    if (!state) throw new Error("Unknown escrow state returned by contract");

    const configResponse = await srv.queryContract<unknown>(contractId, "config");
    if (!configResponse.isReadCall) throw new Error("Config query is not read-only");
    const config = mapEscrowConfig(configResponse.result);
    if (!config) throw new Error("Invalid escrow config returned by contract");

    return { config, state, rawConfig: configResponse.result };
  } catch (e) {
    return {
      config: null,
      state: null,
      rawConfig: null,
      error: e instanceof Error ? e.message : String(e),
    };
  }
}

export function parseBytes32(val: unknown): string | undefined {
  if (!val) return undefined;
  if (typeof val === "string") {
    const clean = val.replace(/^0x/i, "").toLowerCase();
    return clean.length === 64 ? clean : undefined;
  }
  if (val instanceof Uint8Array || Buffer.isBuffer(val)) {
    if (val.length === 32) return Buffer.from(val).toString("hex").toLowerCase();
  }
  if (typeof val === "object" && val !== null) {
    const bytes = Object.values(val as Record<string, number>);
    if (bytes.length === 32 && bytes.every((b) => typeof b === "number")) {
      return Buffer.from(bytes).toString("hex").toLowerCase();
    }
  }
  return undefined;
}

export interface EscrowSnapshotResult {
  state: EscrowStatus | null;
  config: Record<string, unknown> | null;
  hashes: EscrowHashes;
  activeDeadline?: EscrowDeadline;
  ledgerTimestamp?: number;
  correctionAttempts?: number;
  error?: string;
}

export async function fetchOnChainSnapshot(
  contractId: string = CONTRACT_ID
): Promise<EscrowSnapshotResult> {
  if (!contractId) {
    return { state: null, config: null, hashes: {}, error: "NEXT_PUBLIC_ESCROW_CONTRACT_ID vacío" };
  }
  const srv = getServer();
  if (!srv) {
    return { state: null, config: null, hashes: {}, error: "RPC no disponible en SSR" };
  }

  try {
    const snapResponse = await srv.queryContract<Record<string, unknown>>(contractId, "snapshot");
    if (!snapResponse.isReadCall) throw new Error("Snapshot query is not read-only");
    const snap = snapResponse.result;
    if (!snap || typeof snap !== "object") throw new Error("Invalid snapshot returned by contract");

    const state = mapEscrowState(snap.state);
    const config = mapEscrowConfig(snap.config);
    if (!state || !isCompleteEscrowConfig(config)) {
      throw new Error("Incomplete snapshot returned by contract");
    }

    const hashes: EscrowHashes = {};
    const reasonHash = parseBytes32(snap.dispute_reason_hash);
    if (reasonHash) hashes.reasonHash = reasonHash;
    const disputeEvidenceHash = parseBytes32(snap.dispute_evidence_hash);
    if (disputeEvidenceHash) hashes.disputeEvidenceHash = disputeEvidenceHash;
    const reportHash = parseBytes32(snap.report_hash);
    if (reportHash) hashes.reportHash = reportHash;
    const evidenceBundleHash = parseBytes32(snap.evidence_bundle_hash);
    if (evidenceBundleHash) hashes.evidenceBundleHash = evidenceBundleHash;

    const toSecs = (val: unknown): number | undefined => {
      if (typeof val === "number") return val;
      if (typeof val === "bigint" || typeof val === "string") {
        const n = Number(val);
        return Number.isFinite(n) ? n : undefined;
      }
      return undefined;
    };

    let activeDeadline: EscrowDeadline | undefined;
    if (state === "DISPUTED" && snap.resolution_deadline) {
      const ts = toSecs(snap.resolution_deadline);
      if (ts) activeDeadline = { type: "resolution", label: "Resolución de Disputa (Árbitro)", timestamp: ts };
    } else if (state === "ATTESTED_PASS" && snap.objection_deadline) {
      const ts = toSecs(snap.objection_deadline);
      if (ts) activeDeadline = { type: "action", label: "Objeción de Comprador", timestamp: ts };
    } else if (state === "ATTESTED_FAIL" && snap.correction_deadline) {
      const ts = toSecs(snap.correction_deadline);
      if (ts) activeDeadline = { type: "submission", label: "Corrección de Proveedor", timestamp: ts };
    } else if (state === "EVIDENCE_SUBMITTED" && snap.attestation_deadline) {
      const ts = toSecs(snap.attestation_deadline);
      if (ts) activeDeadline = { type: "attestation", label: "Atestación de Motor", timestamp: ts };
    } else if (state === "FUNDED" && snap.submission_deadline) {
      const ts = toSecs(snap.submission_deadline);
      if (ts) activeDeadline = { type: "submission", label: "Entrega de Proveedor", timestamp: ts };
    }

    const ledgerTimestamp = toSecs(snap.ledger_timestamp);
    const correctionAttempts = typeof snap.correction_attempts === "number" ? snap.correction_attempts : undefined;

    return {
      state,
      config,
      hashes,
      activeDeadline,
      ledgerTimestamp,
      correctionAttempts,
    };
  } catch (e) {
    return {
      state: null,
      config: null,
      hashes: {},
      error: e instanceof Error ? e.message : String(e),
    };
  }
}
