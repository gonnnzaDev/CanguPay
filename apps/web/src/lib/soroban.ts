"use client";

import * as StellarSdk from "@stellar/stellar-sdk";

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

/**
 * Reads EscrowConfig and EscrowState directly from chain via soroban RPC.
 * Uses `state()` and `config()` view functions of the ConditionalPayment contract.
 * Returns null if contractId not set or RPC fails (caller must fallback to mock).
 */
export async function fetchOnChainEscrow(contractId: string = CONTRACT_ID): Promise<{
  config: unknown | null;
  state: string | null;
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
    // Use SorobanRpc to simulate view calls (no signing needed for state/config)
    const source = StellarSdk.Keypair.random(); // dummy source for simulation
    const contract = new StellarSdk.Contract(contractId);

    // state() -> EscrowState
    const stateOp = contract.call("state");
    // config() -> EscrowConfig
    const configOp = contract.call("config");

    // Build a dummy transaction for simulation
    const account = new StellarSdk.Account(source.publicKey(), "0");
    const txState = new StellarSdk.TransactionBuilder(account, {
      fee: "100",
      networkPassphrase: StellarSdk.Networks.TESTNET,
    })
      .addOperation(stateOp)
      .setTimeout(30)
      .build();

    const simState = await srv.simulateTransaction(txState);
    let stateVal: string | null = null;
    if (StellarSdk.rpc.Api.isSimulationSuccess(simState) && simState.result?.retval) {
      try {
        // retval is ScVal; convert via scValToNative
        const native = StellarSdk.scValToNative(simState.result.retval as unknown as StellarSdk.xdr.ScVal);
        // native is string like "Created" or enum variant
        if (typeof native === "string") stateVal = native.toUpperCase();
        else if (native && typeof native === "object" && "tag" in (native as Record<string, unknown>)) {
          stateVal = String((native as { tag: string }).tag).toUpperCase();
        } else {
          stateVal = String(native).toUpperCase();
        }
      } catch {
        stateVal = null;
      }
    }

    const account2 = new StellarSdk.Account(source.publicKey(), "0");
    const txConfig = new StellarSdk.TransactionBuilder(account2, {
      fee: "100",
      networkPassphrase: StellarSdk.Networks.TESTNET,
    })
      .addOperation(configOp)
      .setTimeout(30)
      .build();

    const simConfig = await srv.simulateTransaction(txConfig);
    let configVal: unknown | null = null;
    if (StellarSdk.rpc.Api.isSimulationSuccess(simConfig) && simConfig.result?.retval) {
      try {
        configVal = StellarSdk.scValToNative(simConfig.result.retval as unknown as StellarSdk.xdr.ScVal);
      } catch {
        configVal = null;
      }
    }

    return { config: configVal, state: stateVal, rawConfig: configVal };
  } catch (e) {
    return {
      config: null,
      state: null,
      rawConfig: null,
      error: e instanceof Error ? e.message : String(e),
    };
  }
}

/**
 * Derives the expected finalize outcome dynamically from on-chain state,
 * matching docs/state-machine.md:23.
 */
export function deriveFinalizeOutcome(
  status: string,
  fallbackOutcome?: string | null
): string {
  switch (status) {
    case "FUNDED":
      return "REFUNDED"; // SUBMISSION_TIMEOUT
    case "EVIDENCE_SUBMITTED":
      return "REFUNDED"; // ATTESTATION_TIMEOUT
    case "ATTESTED_PASS":
      return "RELEASED"; // NO_OBJECTION
    case "ATTESTED_FAIL":
      return "REFUNDED"; // CORRECTION_TIMEOUT
    case "DISPUTED":
      return fallbackOutcome || "SPLIT"; // RESOLUTION_TIMEOUT → fallback
    default:
      return "REFUNDED";
  }
}
