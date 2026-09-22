"use client";

import React, { useState } from "react";
import {
  CanguPayLogo,
  NetworkIcon,
  CopyIcon,
  CheckIcon,
  RoleIcon,
  AlertCircleIcon,
  CloseIcon,
} from "@/components/icons";
import { ThemeSwitcher } from "@/components/theme/ThemeSwitcher";
import { ProfileSelector } from "@/components/wallet/ProfileSelector";
import { useWallet } from "@/providers/WalletProvider";
import { UserRole } from "@/types/wallet";
import { EscrowDetails } from "@/components/escrow/EscrowDetails";
import {
  EscrowDetailsData,
  EscrowStatus,
  isTerminalStatus,
} from "@/types/escrow";

interface TransactionFeedback {
  type: "success" | "error";
  actionName: string;
  txHash?: string;
  errorDetails?: string;
}

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
  const [activeScenario, setActiveScenario] = useState<EscrowStatus>("FUNDED");
  const [isLoading, setIsLoading] = useState<boolean>(false);
  const [hasError, setHasError] = useState<boolean>(false);
  const [isEmpty, setIsEmpty] = useState<boolean>(false);

  const {
    activeProfile,
    network,
    networkPassphrase,
    isExactTestnet,
    isFreighterInstalled,
    signTransactionGuard,
  } = useWallet();

  const [copiedHash, setCopiedHash] = useState<boolean>(false);
  const [isSigning, setIsSigning] = useState<boolean>(false);
  const [feedback, setFeedback] = useState<TransactionFeedback | null>(null);

  const handleRoleAction = async (actionName: string, nextStatus?: EscrowStatus) => {
    setIsSigning(true);
    try {
      // Mock representative Soroban authorization XDR for testing P0-08 guard
      const mockTxXdr = "AAAAAgAAAABn80rGj5F2s...mock...xdr";
      const res = await signTransactionGuard(mockTxXdr);

      if (!res.success) {
        setFeedback({
          type: "error",
          actionName,
          errorDetails: res.error || "Firma rechazada o bloqueada por la guardia de seguridad.",
        });
      } else {
        setFeedback({
          type: "success",
          actionName,
          txHash: "0x7a8b9c0d1e2f3a4b5c6d7e8f9a0b1c2d3e4f5a6b7c8d9e0f1a2b3c4d5e6f7a8b",
        });

        // Trigger realistic state change in the contract view with high-fidelity skeleton
        if (nextStatus) {
          setIsLoading(true);
          setTimeout(() => {
            setActiveScenario(nextStatus);
            setIsLoading(false);
          }, 400);
        }
      }
    } catch (err: unknown) {
      setFeedback({
        type: "error",
        actionName,
        errorDetails: err instanceof Error ? err.message : "Error inesperado al firmar.",
      });
    } finally {
      setIsSigning(false);
    }
  };

  const handleRefresh = () => {
    setIsLoading(true);
    setHasError(false);
    setIsEmpty(false);
    setTimeout(() => {
      setIsLoading(false);
    }, 450);
  };

  const getActionForRoleAndStatus = (
    role: UserRole,
    status: EscrowStatus
  ): { label: string; fullName: string; nextStatus?: EscrowStatus } | null => {
    if (isTerminalStatus(status)) return null;

    if (role === "buyer") {
      if (status === "FUNDED" || status === "ATTESTED_PASS") {
        return {
          label: "Liberar Fondos",
          fullName: "Aprobar y Liberar Fondos",
          nextStatus: "RELEASED",
        };
      }
    } else if (role === "supplier") {
      if (status === "FUNDED") {
        return {
          label: "Presentar Evidencia",
          fullName: "Presentar Evidencia Documental",
          nextStatus: "ATTESTED_PASS",
        };
      }
    } else if (role === "resolver") {
      if (status === "DISPUTED") {
        return {
          label: "Resolver Disputa",
          fullName: "Emitir Dictamen Arbitral",
          nextStatus: "RELEASED",
        };
      }
    } else if (role === "engine") {
      if (status === "FUNDED") {
        return {
          label: "Atestar Reglas",
          fullName: "Atestar Reglas del Contrato",
          nextStatus: "ATTESTED_PASS",
        };
      }
    }
    return null;
  };

  const actionInfo = getActionForRoleAndStatus(activeProfile.role, activeScenario);

  const actionSlot = actionInfo ? (
    <button
      type="button"
      disabled={isSigning || isLoading}
      onClick={() => handleRoleAction(actionInfo.fullName, actionInfo.nextStatus)}
      className="px-3 py-1.5 rounded-lg border border-teal-600 dark:border-teal-500 bg-teal-600 hover:bg-teal-700 text-white font-mono text-xs font-semibold transition-all cursor-pointer disabled:opacity-50 shadow-2xs flex items-center gap-1.5"
    >
      {isSigning ? (
        <span>Firmando...</span>
      ) : (
        <>
          <span className="font-normal text-teal-100 hidden sm:inline">Firmar:</span>
          <span>{actionInfo.label}</span>
        </>
      )}
    </button>
  ) : null;

  const bannerSlot = feedback ? (
    <div
      role="alert"
      className={`p-3 rounded-xl border flex items-start sm:items-center justify-between gap-3 text-xs font-mono transition-all animate-in fade-in duration-200 ${
        feedback.type === "error"
          ? "bg-red-50/70 border-red-200/80 dark:bg-red-950/30 dark:border-red-900/50 text-red-900 dark:text-red-200"
          : "bg-emerald-50/60 border-emerald-200/80 dark:bg-emerald-950/30 dark:border-emerald-900/50 text-emerald-900 dark:text-emerald-200"
      }`}
    >
      <div className="flex items-center gap-2.5 min-w-0">
        <span
          className={`p-1 rounded-md shrink-0 flex items-center justify-center ${
            feedback.type === "error"
              ? "bg-red-200/60 dark:bg-red-900/60 text-red-700 dark:text-red-300"
              : "bg-emerald-200/60 dark:bg-emerald-900/60 text-emerald-700 dark:text-emerald-300"
          }`}
        >
          {feedback.type === "error" ? (
            <AlertCircleIcon size={14} />
          ) : (
            <CheckIcon size={14} className="stroke-[2.5]" />
          )}
        </span>

        <div className="flex flex-wrap items-center gap-x-2 gap-y-1 text-xs">
          <span className="font-bold tracking-tight">
            {feedback.type === "error" ? "Firma Bloqueada:" : "Transacción Autorizada:"}
          </span>
          <span className="text-neutral-700 dark:text-neutral-300 font-medium">
            {feedback.actionName}
          </span>

          {feedback.type === "success" && feedback.txHash && (
            <>
              <span className="text-neutral-300 dark:text-neutral-700 hidden sm:inline">•</span>
              <div className="flex items-center gap-1 text-neutral-500 dark:text-neutral-400">
                <span className="text-[10px]">HASH:</span>
                <span className="text-[11px] text-neutral-700 dark:text-neutral-300">
                  {feedback.txHash.slice(0, 8)}...{feedback.txHash.slice(-6)}
                </span>
                <button
                  type="button"
                  onClick={() => {
                    if (feedback.txHash && navigator.clipboard) {
                      navigator.clipboard.writeText(feedback.txHash);
                      setCopiedHash(true);
                      setTimeout(() => setCopiedHash(false), 1500);
                    }
                  }}
                  className="p-0.5 rounded text-neutral-400 hover:text-teal-600 dark:hover:text-teal-400 transition-colors cursor-pointer"
                  title="Copiar Hash de Transacción"
                  aria-label="Copiar Hash de Transacción"
                >
                  {copiedHash ? (
                    <CheckIcon size={11} className="text-emerald-500" />
                  ) : (
                    <CopyIcon size={11} />
                  )}
                </button>
              </div>
            </>
          )}

          {feedback.type === "error" && feedback.errorDetails && (
            <>
              <span className="text-neutral-300 dark:text-neutral-700 hidden sm:inline">•</span>
              <span className="text-red-700 dark:text-red-300 text-[11px]">
                {feedback.errorDetails}
              </span>
            </>
          )}
        </div>
      </div>

      <button
        type="button"
        onClick={() => setFeedback(null)}
        className="p-1 rounded-md text-neutral-400 hover:text-neutral-700 dark:hover:text-neutral-200 hover:bg-neutral-100/60 dark:hover:bg-neutral-800/60 transition-colors cursor-pointer shrink-0"
        title="Cerrar notificación"
        aria-label="Cerrar notificación"
      >
        <CloseIcon size={13} />
      </button>
    </div>
  ) : null;

  const currentData = isEmpty ? null : mockEscrows[activeScenario] || null;

  return (
    <div className="min-h-screen font-sans antialiased selection:bg-teal-500/20">
      {/* Top Navbar (Pinned with progressive fade mask layer - zero hard cutoffs) */}
      <header className="sticky top-0 z-30 pointer-events-none">
        {/* Progressive backdrop blur and cubic fade mask */}
        <div
          aria-hidden="true"
          className="absolute inset-x-0 top-0 h-24 pointer-events-none backdrop-blur-md bg-white/85 dark:bg-neutral-900/85 header-fade-mask"
        />

        {/* Header interactive content */}
        <div className="pointer-events-auto relative z-10 max-w-5xl mx-auto px-4 sm:px-6 h-16 flex items-center justify-between">
          <div className="flex items-center gap-3">
            {/* Official CanguPay Brand Logo (transparent vector) */}
            <CanguPayLogo className="h-7 sm:h-8 w-auto" />
            <span className="hidden sm:inline-block text-neutral-300 dark:text-neutral-700 font-light">
              |
            </span>
            <div
              className="hidden sm:flex items-center gap-1.5 px-2.5 py-1 rounded-lg border border-neutral-200/80 dark:border-neutral-800/80 bg-neutral-100/70 dark:bg-neutral-900/70 text-neutral-700 dark:text-neutral-300 font-mono text-[11px] font-bold tracking-wider uppercase shadow-2xs"
              title={`Rol detectado en el contrato: ${activeProfile.roleLabel}`}
            >
              <span>{activeProfile.role}</span>
              <RoleIcon role={activeProfile.role} size={13} className="text-teal-600 dark:text-teal-400" />
            </div>
          </div>

          {/* Header Controls: Profile/Wallet Selector, Theme Mode & Dynamic Network */}
          <div className="flex items-center gap-2 sm:gap-3">
            <ProfileSelector />
            <ThemeSwitcher />

            {/* Technical Network Indicator with NetworkIcon */}
            <div
              className={`flex items-center font-mono text-xs border rounded-lg overflow-hidden shadow-2xs ${
                !isFreighterInstalled
                  ? "border-neutral-200 dark:border-neutral-800 bg-neutral-100/60 dark:bg-neutral-900/60 text-neutral-400"
                  : isExactTestnet
                    ? "border-teal-500/30 bg-teal-500/5 text-teal-700 dark:text-teal-300"
                    : "border-red-500 bg-red-500/10 text-red-600 dark:text-red-400"
              }`}
              title={
                !isFreighterInstalled
                  ? "Freighter no detectado"
                  : isExactTestnet
                    ? `Red Stellar validada: ${network} (${networkPassphrase || ""})`
                    : `Red bloqueada: ${network} (requiere Testnet oficial)`
              }
            >
              <span
                className="px-2 py-1.5 bg-neutral-200/50 dark:bg-neutral-800 text-neutral-500 dark:text-neutral-400 border-r border-neutral-200 dark:border-neutral-800 flex items-center justify-center"
                aria-label="Red Stellar"
              >
                <NetworkIcon size={13} />
              </span>
              <span
                className={`px-2.5 py-1 text-[11px] font-bold tracking-wider ${
                  !isFreighterInstalled
                    ? "text-neutral-400"
                    : isExactTestnet
                      ? "text-teal-700 dark:text-teal-300"
                      : "text-red-600 dark:text-red-400 animate-pulse"
                }`}
              >
                {!isFreighterInstalled ? "NO DETECTADO" : isExactTestnet ? "TESTNET" : network}
              </span>
            </div>
          </div>
        </div>
      </header>

      {/* Main Content: Pure Production UI */}
      <main className="max-w-5xl mx-auto px-4 sm:px-6 py-8">
        <div className="bg-white/95 dark:bg-neutral-900/85 backdrop-blur-xs border border-neutral-200/80 dark:border-neutral-800/80 rounded-2xl shadow-xs">
          <EscrowDetails
            data={currentData}
            isLoading={isLoading}
            error={hasError ? "Fallo de conexión al nodo RPC de Stellar Testnet (TIMEOUT)" : null}
            onRefresh={handleRefresh}
            actionSlot={actionSlot}
            bannerSlot={bannerSlot}
          />
        </div>
      </main>
    </div>
  );
}
