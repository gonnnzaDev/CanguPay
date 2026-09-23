"use client";

import * as StellarSdk from "@stellar/stellar-sdk";
import type { EscrowStatus, FallbackOutcome } from "../types/escrow";

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
