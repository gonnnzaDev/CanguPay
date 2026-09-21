"use client";

import React, { useState } from "react";
import { EscrowDetails } from "@/components/escrow/EscrowDetails";
import { EscrowDetailsData, EscrowStatus } from "@/types/escrow";

// Mock fixtures representing key CanguPay P0 state machine stages
const mockEscrows: Record<string, EscrowDetailsData> = {
  FUNDED: {
    operationId: "CANGU-OP-2026-001",
    contractId: "CB4Z2T3A6Z7Y8X9W0V1U2T3S4R5Q6P7O8N9M0L1K2J3I4H5G6F7E8D9C",
    status: "FUNDED",
    amount: "15,000.0000000",
    asset: "CPUSD",
    parties: {
      buyer: "GBUYER...4X9Z",
      supplier: "GSUPPLIER...8K2L",
      engine: "GENGINE...1V3M",
      resolver: "GRESOLVER...9P0R",
    },
    activeDeadline: {
      type: "submission",
      label: "Envío de Evidencia Documental",
      timestamp: Math.floor(Date.now() / 1000) + 14400, // +4 hours
    },
    hashes: {
      evidenceBundleHash: "0x7a8b9c0d1e2f3a4b5c6d7e8f9a0b1c2d3e4f5a6b7c8d9e0f1a2b3c4d5e6f7a8b",
    },
    transactionHash: "a1b2c3d4e5f67890abcdef1234567890abcdef1234567890abcdef1234567890",
  },
  ATTESTED_PASS: {
    operationId: "CANGU-OP-2026-002",
    contractId: "CB4Z2T3A6Z7Y8X9W0V1U2T3S4R5Q6P7O8N9M0L1K2J3I4H5G6F7E8D9C",
    status: "ATTESTED_PASS",
    amount: "25,000.0000000",
    asset: "CPUSD",
    parties: {
      buyer: "GBUYER...4X9Z",
      supplier: "GSUPPLIER...8K2L",
      engine: "GENGINE...1V3M",
      resolver: "GRESOLVER...9P0R",
    },
    activeDeadline: {
      type: "action",
      label: "Ventana de Aprobación u Objeción del Comprador",
      timestamp: Math.floor(Date.now() / 1000) + 7200, // +2 hours
    },
    hashes: {
      evidenceBundleHash: "0x3f2a1b0c9d8e7f6a5b4c3d2e1f0a9b8c7d6e5f4a3b2c1d0e9f8a7b6c5d4e3f2a",
      reportHash: "0x9876543210abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
    },
    transactionHash: "b2c3d4e5f67890abcdef1234567890abcdef1234567890abcdef1234567890a1",
  },
  DISPUTED: {
    operationId: "CANGU-OP-2026-003",
    contractId: "CB4Z2T3A6Z7Y8X9W0V1U2T3S4R5Q6P7O8N9M0L1K2J3I4H5G6F7E8D9C",
    status: "DISPUTED",
    amount: "8,500.0000000",
    asset: "CPUSD",
    parties: {
      buyer: "GBUYER...4X9Z",
      supplier: "GSUPPLIER...8K2L",
      engine: "GENGINE...1V3M",
      resolver: "GRESOLVER...9P0R",
    },
    activeDeadline: {
      type: "resolution",
      label: "Resolución del Árbitro (Release / Refund / Split)",
      timestamp: Math.floor(Date.now() / 1000) + 28800, // +8 hours
    },
    hashes: {
      evidenceBundleHash: "0x11223344556677889900aabbccddeeff0011223344556677889900aabbccddee",
      reportHash: "0xaabbccddeeff0011223344556677889900aabbccddeeff001122334455667788",
      reasonHash: "0xccddeeff0011223344556677889900aabbccddeeff0011223344556677889900",
      disputeEvidenceHash: "0xeeff0011223344556677889900aabbccddeeff0011223344556677889900aabb",
    },
    transactionHash: "c3d4e5f67890abcdef1234567890abcdef1234567890abcdef1234567890a1b2",
  },
  RELEASED: {
    operationId: "CANGU-OP-2026-004",
    contractId: "CB4Z2T3A6Z7Y8X9W0V1U2T3S4R5Q6P7O8N9M0L1K2J3I4H5G6F7E8D9C",
    status: "RELEASED",
    amount: "12,000.0000000",
    asset: "CPUSD",
    parties: {
      buyer: "GBUYER...4X9Z",
      supplier: "GSUPPLIER...8K2L",
      engine: "GENGINE...1V3M",
      resolver: "GRESOLVER...9P0R",
    },
    hashes: {
      evidenceBundleHash: "0x44556677889900aabbccddeeff0011223344556677889900aabbccddeeff0011",
      reportHash: "0x223344556677889900aabbccddeeff0011223344556677889900aabbccddeeff",
    },
    transactionHash: "d4e5f67890abcdef1234567890abcdef1234567890abcdef1234567890a1b2c3",
  },
};

export default function Home() {
  const [activeScenario, setActiveScenario] = useState<string>("FUNDED");
  const [isLoading, setIsLoading] = useState<boolean>(false);
  const [hasError, setHasError] = useState<boolean>(false);
  const [isEmpty, setIsEmpty] = useState<boolean>(false);

  const currentData = isEmpty ? null : mockEscrows[activeScenario] || null;

  return (
    <div className="min-h-screen bg-neutral-50 dark:bg-neutral-950 font-sans">
      {/* Top Banner / Navigation */}
      <header className="border-b border-neutral-200 dark:border-neutral-800 bg-white dark:bg-neutral-900/60 sticky top-0 z-10 backdrop-blur-sm">
        <div className="max-w-5xl mx-auto px-4 py-3.5 flex items-center justify-between">
          <div className="flex items-center gap-3">
            <span className="h-6 w-6 rounded-md bg-neutral-900 text-white dark:bg-white dark:text-neutral-950 flex items-center justify-center font-bold text-xs">
              CP
            </span>
            <span className="font-semibold text-sm tracking-tight text-neutral-900 dark:text-neutral-100">
              CanguPay
            </span>
            <span className="text-[10px] uppercase font-mono px-1.5 py-0.5 rounded bg-neutral-100 dark:bg-neutral-800 text-neutral-500">
              P0-08 Preview
            </span>
          </div>

          <div className="text-xs text-neutral-500 font-mono">
            Red: <span className="text-blue-600 dark:text-blue-400">Stellar Testnet</span>
          </div>
        </div>
      </header>

      {/* Main Container */}
      <main className="max-w-5xl mx-auto px-4 py-8 space-y-6">
        {/* Mock Controls Banner */}
        <div className="p-4 rounded-xl border border-neutral-200 dark:border-neutral-800 bg-white dark:bg-neutral-900/40 text-xs">
          <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-3">
            <div>
              <span className="font-semibold uppercase tracking-wider text-neutral-500 block mb-1">
                Selector de Escenario UI (Mock Fixtures)
              </span>
              <p className="text-neutral-600 dark:text-neutral-400">
                Visualización presentacional desacoplada de la blockchain (Refs #10).
              </p>
            </div>

            <div className="flex flex-wrap items-center gap-1.5 font-mono">
              {(["FUNDED", "ATTESTED_PASS", "DISPUTED", "RELEASED"] as EscrowStatus[]).map(
                (status) => (
                  <button
                    key={status}
                    onClick={() => {
                      setIsEmpty(false);
                      setHasError(false);
                      setIsLoading(false);
                      setActiveScenario(status);
                    }}
                    className={`px-2.5 py-1 rounded text-xs transition-colors ${
                      !isEmpty && !hasError && !isLoading && activeScenario === status
                        ? "bg-neutral-900 text-white dark:bg-white dark:text-neutral-950 font-semibold"
                        : "bg-neutral-100 dark:bg-neutral-800 text-neutral-600 dark:text-neutral-300 hover:bg-neutral-200 dark:hover:bg-neutral-700"
                    }`}
                  >
                    {status}
                  </button>
                )
              )}

              <button
                onClick={() => {
                  setIsEmpty(false);
                  setHasError(false);
                  setIsLoading(true);
                }}
                className={`px-2.5 py-1 rounded text-xs transition-colors ${
                  isLoading
                    ? "bg-neutral-900 text-white dark:bg-white dark:text-neutral-950 font-semibold"
                    : "bg-neutral-100 dark:bg-neutral-800 text-neutral-600 dark:text-neutral-300 hover:bg-neutral-200 dark:hover:bg-neutral-700"
                }`}
              >
                Loading
              </button>

              <button
                onClick={() => {
                  setIsEmpty(false);
                  setIsLoading(false);
                  setHasError(true);
                }}
                className={`px-2.5 py-1 rounded text-xs transition-colors ${
                  hasError
                    ? "bg-neutral-900 text-white dark:bg-white dark:text-neutral-950 font-semibold"
                    : "bg-neutral-100 dark:bg-neutral-800 text-neutral-600 dark:text-neutral-300 hover:bg-neutral-200 dark:hover:bg-neutral-700"
                }`}
              >
                Error
              </button>

              <button
                onClick={() => {
                  setHasError(false);
                  setIsLoading(false);
                  setIsEmpty(true);
                }}
                className={`px-2.5 py-1 rounded text-xs transition-colors ${
                  isEmpty
                    ? "bg-neutral-900 text-white dark:bg-white dark:text-neutral-950 font-semibold"
                    : "bg-neutral-100 dark:bg-neutral-800 text-neutral-600 dark:text-neutral-300 hover:bg-neutral-200 dark:hover:bg-neutral-700"
                }`}
              >
                Empty
              </button>
            </div>
          </div>
        </div>

        {/* EscrowDetails Component View */}
        <div className="bg-white dark:bg-neutral-900 border border-neutral-200 dark:border-neutral-800 rounded-2xl p-2 sm:p-6 shadow-xs">
          <EscrowDetails
            data={currentData}
            isLoading={isLoading}
            error={hasError ? "No se pudo recuperar la entrada de ledger: RPC_TIMEOUT" : null}
            onRefresh={() => {
              setIsLoading(false);
              setHasError(false);
              setIsEmpty(false);
            }}
          />
        </div>
      </main>
    </div>
  );
}
