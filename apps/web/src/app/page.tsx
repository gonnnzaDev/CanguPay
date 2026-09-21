"use client";

import React, { useState } from "react";
import { CanguPayLogo } from "@/components/icons";
import { ThemeSwitcher } from "@/components/theme/ThemeSwitcher";
import { EscrowDetails } from "@/components/escrow/EscrowDetails";
import { EscrowDetailsData, EscrowStatus } from "@/types/escrow";

// Representative fixtures for CanguPay P0 state machine stages with deterministic timestamps
const BASE_LEDGER_TIME = 1758412800; // Deterministic reference timestamp (eliminates hydration drift)

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
      timestamp: BASE_LEDGER_TIME + 14400,
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
      label: "Ventana de Objeción del Comprador",
      timestamp: BASE_LEDGER_TIME + 7200,
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
      label: "Resolución del Árbitro",
      timestamp: BASE_LEDGER_TIME + 28800,
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
    <div className="min-h-screen font-sans antialiased selection:bg-teal-500/20">
      {/* Top Navbar (Pinned / Fixed to top) */}
      <header className="sticky top-0 z-30 border-b border-neutral-200/80 dark:border-neutral-800/80 bg-white/85 dark:bg-neutral-900/85 backdrop-blur-md">
        <div className="max-w-5xl mx-auto px-4 sm:px-6 h-16 flex items-center justify-between">
          <div className="flex items-center gap-3">
            {/* Official CanguPay Brand Logo (transparent vector) */}
            <CanguPayLogo className="h-7 sm:h-8 w-auto" />
            <span className="hidden sm:inline-block text-neutral-300 dark:text-neutral-700 font-light">
              |
            </span>
            <span className="hidden sm:inline-block text-xs text-neutral-500 dark:text-neutral-400 font-mono tracking-tight">
              B2B CONDITIONAL ESCROW
            </span>
          </div>

          {/* Header Controls: Theme Mode & Network */}
          <div className="flex items-center gap-2 sm:gap-3">
            <ThemeSwitcher />

            {/* Technical Network Indicator */}
            <div className="flex items-center font-mono text-xs border border-neutral-200 dark:border-neutral-800 rounded-lg overflow-hidden bg-neutral-100/60 dark:bg-neutral-900/60 shadow-2xs">
              <span className="px-2.5 py-1 bg-neutral-200/50 dark:bg-neutral-800 text-[10px] text-neutral-500 dark:text-neutral-400 font-bold border-r border-neutral-200 dark:border-neutral-800">
                NETWORK
              </span>
              <span className="px-2.5 py-1 text-[11px] font-bold text-teal-700 dark:text-teal-300 tracking-wider">
                TESTNET
              </span>
            </div>
          </div>
        </div>
      </header>

      {/* Main Content */}
      <main className="max-w-5xl mx-auto px-4 sm:px-6 py-8 space-y-6">
        {/* Scenario Switcher Console */}
        <div className="p-4 rounded-xl border border-neutral-200/80 dark:border-neutral-800/80 bg-white/90 dark:bg-neutral-900/70 backdrop-blur-xs shadow-xs">
          <div className="flex flex-col md:flex-row md:items-center justify-between gap-3">
            <div>
              <span className="text-[10px] font-mono font-bold uppercase tracking-widest text-neutral-400 block">
                CONSOLA DE CONTROL
              </span>
              <h2 className="text-sm font-semibold tracking-tight text-neutral-900 dark:text-neutral-100">
                Simulador de Ciclo de Vida del Contrato
              </h2>
            </div>

            {/* Segmented control tabs */}
            <div className="flex flex-wrap items-center gap-1 font-mono text-xs p-1 bg-neutral-100 dark:bg-neutral-800/60 rounded-lg border border-neutral-200/60 dark:border-neutral-700/60">
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
                    className={`px-2.5 py-1 rounded-md text-[11px] font-semibold transition-all ${
                      !isEmpty && !hasError && !isLoading && activeScenario === status
                        ? "bg-white dark:bg-neutral-900 text-neutral-950 dark:text-neutral-50 shadow-xs border border-neutral-200/60 dark:border-neutral-700/60"
                        : "text-neutral-500 hover:text-neutral-900 dark:hover:text-neutral-200"
                    }`}
                  >
                    {status}
                  </button>
                )
              )}

              <span className="text-neutral-300 dark:text-neutral-700 mx-0.5">|</span>

              <button
                onClick={() => {
                  setIsEmpty(false);
                  setHasError(false);
                  setIsLoading(true);
                }}
                className={`px-2 py-1 rounded-md text-[11px] font-semibold transition-all ${
                  isLoading
                    ? "bg-white dark:bg-neutral-900 text-neutral-950 dark:text-neutral-50 shadow-xs border border-neutral-200/60 dark:border-neutral-700/60"
                    : "text-neutral-500 hover:text-neutral-900 dark:hover:text-neutral-200"
                }`}
              >
                Carga
              </button>

              <button
                onClick={() => {
                  setIsEmpty(false);
                  setIsLoading(false);
                  setHasError(true);
                }}
                className={`px-2 py-1 rounded-md text-[11px] font-semibold transition-all ${
                  hasError
                    ? "bg-white dark:bg-neutral-900 text-neutral-950 dark:text-neutral-50 shadow-xs border border-neutral-200/60 dark:border-neutral-700/60"
                    : "text-neutral-500 hover:text-neutral-900 dark:hover:text-neutral-200"
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
                className={`px-2 py-1 rounded-md text-[11px] font-semibold transition-all ${
                  isEmpty
                    ? "bg-white dark:bg-neutral-900 text-neutral-950 dark:text-neutral-50 shadow-xs border border-neutral-200/60 dark:border-neutral-700/60"
                    : "text-neutral-500 hover:text-neutral-900 dark:hover:text-neutral-200"
                }`}
              >
                Vacío
              </button>
            </div>
          </div>
        </div>

        {/* EscrowDetails Component */}
        <div className="bg-white/95 dark:bg-neutral-900/85 backdrop-blur-xs border border-neutral-200/80 dark:border-neutral-800/80 rounded-2xl shadow-xs">
          <EscrowDetails
            data={currentData}
            isLoading={isLoading}
            error={hasError ? "Fallo de conexión al nodo RPC de Stellar Testnet (TIMEOUT)" : null}
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
