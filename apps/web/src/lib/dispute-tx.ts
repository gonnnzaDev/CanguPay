"use client";

import * as StellarSdk from "@stellar/stellar-sdk";
import type { FallbackOutcome } from "../types/escrow";

export const TESTNET_PASSPHRASE = "Test SDF Network ; September 2015";

const DEFAULT_RPC_URL =
  process.env.NEXT_PUBLIC_SOROBAN_RPC_URL || "https://soroban-testnet.stellar.org";

function hexToBytes(hex: string): Buffer {
  const clean = hex.replace(/^0x/i, "").trim();
  if (clean.length !== 64 || !/^[0-9a-fA-F]{64}$/.test(clean)) {
    throw new Error("El hash debe ser una cadena hexadecimal válida de 32 bytes (64 caracteres).");
  }
  return Buffer.from(clean, "hex");
}

function outcomeToScVal(outcome: FallbackOutcome): StellarSdk.xdr.ScVal {
  // Soroban enum variants are serialized as ScVal.scvVec([ScVal.scvSymbol(VariantName)])
  const name =
    outcome === "RELEASE" ? "Release" : outcome === "REFUND" ? "Refund" : "Split";
  return StellarSdk.xdr.ScVal.scvVec([StellarSdk.xdr.ScVal.scvSymbol(name)]);
}

export async function buildRaiseDisputeTx({
  contractId,
  userAddress,
  reasonHash,
  disputeEvidenceHash,
  rpcUrl = DEFAULT_RPC_URL,
  networkPassphrase = TESTNET_PASSPHRASE,
}: {
  contractId: string;
  userAddress: string;
  reasonHash: string;
  disputeEvidenceHash: string;
  rpcUrl?: string;
  networkPassphrase?: string;
}): Promise<string> {
  const srv = new StellarSdk.rpc.Server(rpcUrl);
  const account = await srv.getAccount(userAddress);
  const contract = new StellarSdk.Contract(contractId);

  const reasonBytes = hexToBytes(reasonHash);
  const evidenceBytes = hexToBytes(disputeEvidenceHash);

  const op = contract.call(
    "raise_dispute",
    StellarSdk.xdr.ScVal.scvBytes(reasonBytes),
    StellarSdk.xdr.ScVal.scvBytes(evidenceBytes)
  );

  const tx = new StellarSdk.TransactionBuilder(account, {
    fee: "1000",
    networkPassphrase,
  })
    .addOperation(op)
    .setTimeout(60)
    .build();

  const prepared = await srv.prepareTransaction(tx);
  return prepared.toXDR();
}

export async function buildResolveTx({
  contractId,
  userAddress,
  outcome,
  splitBps,
  rpcUrl = DEFAULT_RPC_URL,
  networkPassphrase = TESTNET_PASSPHRASE,
}: {
  contractId: string;
  userAddress: string;
  outcome: FallbackOutcome;
  splitBps: number;
  rpcUrl?: string;
  networkPassphrase?: string;
}): Promise<string> {
  const srv = new StellarSdk.rpc.Server(rpcUrl);
  const account = await srv.getAccount(userAddress);
  const contract = new StellarSdk.Contract(contractId);

  const outcomeVal = outcomeToScVal(outcome);
  const splitBpsVal = StellarSdk.xdr.ScVal.scvU32(outcome === "SPLIT" ? splitBps : 0);

  const op = contract.call("resolve", outcomeVal, splitBpsVal);

  const tx = new StellarSdk.TransactionBuilder(account, {
    fee: "1000",
    networkPassphrase,
  })
    .addOperation(op)
    .setTimeout(60)
    .build();

  const prepared = await srv.prepareTransaction(tx);
  return prepared.toXDR();
}

export async function submitSorobanTransaction({
  signedXdr,
  rpcUrl = DEFAULT_RPC_URL,
  networkPassphrase = TESTNET_PASSPHRASE,
}: {
  signedXdr: string;
  rpcUrl?: string;
  networkPassphrase?: string;
}): Promise<{ success: boolean; txHash?: string; error?: string }> {
  try {
    const srv = new StellarSdk.rpc.Server(rpcUrl);
    const tx = StellarSdk.TransactionBuilder.fromXDR(signedXdr, networkPassphrase);
    const sendResponse = await srv.sendTransaction(tx);

    if (sendResponse.status === "ERROR") {
      return {
        success: false,
        error: "Transacción rechazada por el nodo Soroban RPC.",
      };
    }

    const txHash = sendResponse.hash;
    let attempts = 0;
    while (attempts < 15) {
      await new Promise((resolve) => setTimeout(resolve, 1500));
      const txStatus = await srv.getTransaction(txHash);
      if (txStatus.status === "SUCCESS") {
        return { success: true, txHash };
      }
      if (txStatus.status === "FAILED") {
        return {
          success: false,
          error: "La transacción fue ejecutada pero falló en el ledger.",
          txHash,
        };
      }
      attempts++;
    }

    return { success: true, txHash };
  } catch (err: unknown) {
    return {
      success: false,
      error: err instanceof Error ? err.message : String(err),
    };
  }
}
