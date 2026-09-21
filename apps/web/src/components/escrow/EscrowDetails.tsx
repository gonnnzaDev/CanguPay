"use client";

import React, { useState } from "react";
import {
  EscrowDetailsProps,
  EscrowStatus,
  EscrowActor,
  getActiveActor,
  isTerminalStatus,
} from "@/types/escrow";
import {
  CopyIcon,
  CheckIcon,
  RefreshIcon,
  ExternalLinkIcon,
  ShieldLockIcon,
  ClockIcon,
  HashIcon,
  UserIcon,
  AlertCircleIcon,
  ScaleIcon,
  FileTextIcon,
} from "@/components/icons";

/**
 * Truncates an address or cryptographic hash for concise display.
 */
function truncateHash(hash: string, start = 8, end = 6): string {
  if (!hash) return "—";
  if (hash.length <= start + end) return hash;
  return `${hash.slice(0, start)}...${hash.slice(-end)}`;
}

/**
 * Maps EscrowStatus to technical status styling without generic pills or dots.
 */
function getStatusBadgeConfig(status: EscrowStatus): {
  label: string;
  textColor: string;
  borderColor: string;
} {
  switch (status) {
    case "CREATED":
      return {
        label: "CREADO",
        textColor: "text-amber-700 dark:text-amber-400",
        borderColor: "border-amber-500/30",
      };
    case "FUNDED":
      return {
        label: "FONDEADO",
        textColor: "text-teal-700 dark:text-teal-400",
        borderColor: "border-teal-500/30",
      };
    case "EVIDENCE_SUBMITTED":
      return {
        label: "EVIDENCIA PRESENTADA",
        textColor: "text-blue-700 dark:text-blue-400",
        borderColor: "border-blue-500/30",
      };
    case "ATTESTED_PASS":
      return {
        label: "ATESTACIÓN APROBADA",
        textColor: "text-emerald-700 dark:text-emerald-400",
        borderColor: "border-emerald-500/30",
      };
    case "ATTESTED_FAIL":
      return {
        label: "ATESTACIÓN OBSERVADA",
        textColor: "text-rose-700 dark:text-rose-400",
        borderColor: "border-rose-500/30",
      };
    case "DISPUTED":
      return {
        label: "EN DISPUTA",
        textColor: "text-purple-700 dark:text-purple-400",
        borderColor: "border-purple-500/30",
      };
    case "RELEASED":
      return {
        label: "FONDOS LIBERADOS",
        textColor: "text-emerald-700 dark:text-emerald-400",
        borderColor: "border-emerald-500/30",
      };
    case "REFUNDED":
      return {
        label: "REEMBOLSADO",
        textColor: "text-neutral-700 dark:text-neutral-300",
        borderColor: "border-neutral-500/30",
      };
    case "SPLIT":
      return {
        label: "DIVISIÓN LIQUIDADA",
        textColor: "text-cyan-700 dark:text-cyan-400",
        borderColor: "border-cyan-500/30",
      };
    case "CANCELLED":
      return {
        label: "CANCELADO",
        textColor: "text-neutral-600 dark:text-neutral-400",
        borderColor: "border-neutral-500/30",
      };
    default:
      return {
        label: status,
        textColor: "text-neutral-600 dark:text-neutral-400",
        borderColor: "border-neutral-500/30",
      };
  }
}

/**
 * Human-readable actor descriptions for B2B stakeholders.
 */
function getActorMeta(actor: EscrowActor): { label: string; description: string } {
  switch (actor) {
    case "buyer":
      return {
        label: "Comprador (Buyer)",
        description: "Revisión de orden para fondeo de depósito o validación de liberación definitiva.",
      };
    case "supplier":
      return {
        label: "Proveedor (Supplier)",
        description: "Remisión del lote documental de entrega física o presentación de corrección técnica.",
      };
    case "engine":
      return {
        label: "Motor de Atestación (Engine)",
        description: "Evaluación documental determinista de remisiones y facturas contra reglas preacordadas.",
      };
    case "resolver":
      return {
        label: "Árbitro Neutral (Resolver)",
        description: "Revisión de alegaciones contradictorias para dictamen vinculante (Release, Refund o Split).",
      };
    case "none":
    default:
      return {
        label: "Sin acción pendiente",
        description: "La operación se encuentra liquidada en un estado terminal inmutable.",
      };
  }
}

function subscribeToClock(callback: () => void): () => void {
  const interval = setInterval(callback, 1000);
  return () => clearInterval(interval);
}

function useCurrentTimestamp(): number {
  return React.useSyncExternalStore(
    subscribeToClock,
    () => Math.floor(Date.now() / 1000),
    () => 0
  );
}

export const EscrowDetails: React.FC<EscrowDetailsProps> = ({
  data,
  isLoading = false,
  error = null,
  onRefresh,
}) => {
  const [copiedKey, setCopiedKey] = useState<string | null>(null);
  const currentTime = useCurrentTimestamp();

  const handleCopy = (text: string, key: string) => {
    if (!text || text === "—") return;
    navigator.clipboard?.writeText(text);
    setCopiedKey(key);
    setTimeout(() => setCopiedKey(null), 1800);
  };

  // 1. Loading State (Sober Skeleton)
  if (isLoading) {
    return (
      <div className="w-full max-w-5xl mx-auto p-6 space-y-6 animate-pulse">
        <div className="flex items-center justify-between">
          <div className="h-8 bg-neutral-200 dark:bg-neutral-800 rounded w-64" />
          <div className="h-7 bg-neutral-200 dark:bg-neutral-800 rounded w-32" />
        </div>
        <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
          <div className="h-32 bg-neutral-100 dark:bg-neutral-900 border border-neutral-200/60 dark:border-neutral-800/60 rounded-xl" />
          <div className="h-32 bg-neutral-100 dark:bg-neutral-900 border border-neutral-200/60 dark:border-neutral-800/60 rounded-xl md:col-span-2" />
        </div>
        <div className="h-44 bg-neutral-100 dark:bg-neutral-900 border border-neutral-200/60 dark:border-neutral-800/60 rounded-xl" />
        <div className="h-56 bg-neutral-100 dark:bg-neutral-900 border border-neutral-200/60 dark:border-neutral-800/60 rounded-xl" />
      </div>
    );
  }

  // 2. Error State
  if (error) {
    return (
      <div className="w-full max-w-5xl mx-auto p-6">
        <div className="rounded-xl border border-rose-500/20 bg-rose-500/5 p-6 text-neutral-900 dark:text-neutral-100 shadow-xs">
          <div className="flex items-start gap-3">
            <div className="p-2 rounded bg-rose-500/10 text-rose-600 dark:text-rose-400">
              <AlertCircleIcon size={18} />
            </div>
            <div className="space-y-1">
              <h3 className="text-base font-semibold tracking-tight text-neutral-900 dark:text-neutral-100">
                Fallo de consulta en el contrato
              </h3>
              <p className="text-sm text-neutral-600 dark:text-neutral-400 font-mono">
                {error}
              </p>
            </div>
          </div>
          {onRefresh && (
            <div className="mt-4 pt-3 border-t border-rose-500/10 flex justify-end">
              <button
                onClick={onRefresh}
                className="inline-flex items-center gap-2 text-xs font-semibold px-3 py-1.5 rounded bg-neutral-900 text-white dark:bg-neutral-100 dark:text-neutral-900 hover:opacity-90 transition-opacity"
              >
                <RefreshIcon size={13} />
                Reintentar sincronización
              </button>
            </div>
          )}
        </div>
      </div>
    );
  }

  // 3. Empty State
  if (!data) {
    return (
      <div className="w-full max-w-5xl mx-auto p-12 text-center border border-dashed border-neutral-200 dark:border-neutral-800 rounded-xl bg-neutral-50/50 dark:bg-neutral-950/50">
        <div className="mx-auto w-12 h-12 rounded bg-neutral-100 dark:bg-neutral-900 flex items-center justify-center text-neutral-400 mb-3 border border-neutral-200/60 dark:border-neutral-800/60">
          <FileTextIcon size={20} />
        </div>
        <h3 className="text-base font-semibold text-neutral-900 dark:text-neutral-100">
          No hay contrato activo
        </h3>
        <p className="mt-1 text-sm text-neutral-500 dark:text-neutral-400 max-w-sm mx-auto">
          Seleccione una operación para consultar los datos del escrow en Stellar Testnet.
        </p>
      </div>
    );
  }

  const statusConfig = getStatusBadgeConfig(data.status);
  const activeActor = getActiveActor(data.status);
  const actorMeta = getActorMeta(activeActor);
  const isTerminal = isTerminalStatus(data.status);

  // Client-safe deadline calculation
  let deadlineDiffSeconds = 0;
  if (data.activeDeadline?.timestamp && currentTime > 0) {
    deadlineDiffSeconds = data.activeDeadline.timestamp - currentTime;
  }

  const formatCountdown = (diff: number) => {
    if (diff <= 0) return "Plazo vencido según tiempo local (confirmar ledger)";
    const hours = Math.floor(diff / 3600);
    const minutes = Math.floor((diff % 3600) / 60);
    const seconds = diff % 60;
    return `~${hours}h ${minutes}m ${seconds}s restantes`;
  };

  const explorerUrl = data.transactionHash
    ? `${data.explorerBaseUrl || "https://stellar.expert/explorer/testnet"}/tx/${data.transactionHash}`
    : data.contractId
    ? `${data.explorerBaseUrl || "https://stellar.expert/explorer/testnet"}/contract/${data.contractId}`
    : null;

  return (
    <div className="w-full max-w-5xl mx-auto p-4 sm:p-6 space-y-6 text-neutral-900 dark:text-neutral-100">
      {/* Header & Status Indicator (No generic pill, no dot) */}
      <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-4 pb-5 border-b border-neutral-200/80 dark:border-neutral-800/80">
        <div>
          <div className="flex items-center gap-2 mb-1.5 font-mono text-[11px]">
            <span className="text-teal-600 dark:text-teal-400 font-semibold flex items-center gap-1.5">
              <ShieldLockIcon size={14} />
              SOROBAN // ESCROW
            </span>
          </div>

          <div className="flex items-center gap-2.5">
            <h1 className="text-xl sm:text-2xl font-bold tracking-tight font-mono text-neutral-950 dark:text-neutral-50">
              {data.operationId}
            </h1>
            <button
              onClick={() => handleCopy(data.operationId, "operationId")}
              className="p-1 text-neutral-400 hover:text-neutral-700 dark:hover:text-neutral-200 rounded transition-colors"
              title="Copiar ID"
            >
              {copiedKey === "operationId" ? (
                <CheckIcon size={14} className="text-emerald-500" />
              ) : (
                <CopyIcon size={14} />
              )}
            </button>
          </div>
        </div>

        {/* Technical Split Status Block (Replaces generic pill with dot) */}
        <div className="flex items-center gap-3">
          <div className={`flex items-center border ${statusConfig.borderColor} rounded-lg overflow-hidden bg-neutral-50/80 dark:bg-neutral-900/80 font-mono shadow-2xs`}>
            <span className="px-2.5 py-1.5 bg-neutral-100 dark:bg-neutral-800/80 text-[10px] font-bold tracking-wider text-neutral-500 dark:text-neutral-400 border-r border-neutral-200 dark:border-neutral-800 uppercase">
              STATUS
            </span>
            <span className={`px-3 py-1.5 font-bold text-xs tracking-wider uppercase ${statusConfig.textColor}`}>
              {statusConfig.label}
            </span>
          </div>

          {onRefresh && (
            <button
              onClick={onRefresh}
              className="p-2 text-neutral-400 hover:text-neutral-700 dark:hover:text-neutral-200 transition-colors border border-neutral-200 dark:border-neutral-800 rounded-lg hover:bg-neutral-50 dark:hover:bg-neutral-900"
              title="Refrescar datos del ledger"
            >
              <RefreshIcon size={14} />
            </button>
          )}
        </div>
      </div>

      {/* Primary Metrics: Amount & Current Turn */}
      <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
        {/* Metric 1: Amount (Overflow-proof with decimal hierarchy) */}
        <div className="p-5 rounded-xl border border-neutral-200/80 dark:border-neutral-800/80 bg-white dark:bg-neutral-900/60 shadow-xs flex flex-col justify-between min-w-0">
          <div className="flex items-center justify-between gap-2">
            <span className="text-[10px] font-bold text-neutral-500 dark:text-neutral-400 uppercase tracking-widest font-mono truncate">
              FONDOS BAJO CUSTODIA
            </span>
            <span className="text-[10px] font-mono text-teal-700 dark:text-teal-300 font-bold border border-teal-500/30 bg-teal-500/5 px-2 py-0.5 rounded shrink-0">
              {data.asset} {"//"} SAC
            </span>
          </div>

          {(() => {
            const [intPart, decPart] = data.amount.split(".");
            return (
              <div className="my-2 flex items-baseline gap-1 font-mono flex-wrap min-w-0">
                <span className="text-2xl sm:text-3xl font-extrabold tracking-tight text-neutral-950 dark:text-white truncate">
                  {intPart}
                </span>
                {decPart !== undefined && (
                  <span className="text-xs sm:text-sm font-semibold text-neutral-400 dark:text-neutral-500 shrink-0">
                    .{decPart}
                  </span>
                )}
                <span className="text-xs font-mono font-bold text-teal-600 dark:text-teal-400 ml-1 shrink-0">
                  {data.asset}
                </span>
              </div>
            );
          })()}

          <p className="text-xs text-neutral-500 dark:text-neutral-400">
            Reserva atómica en contrato inteligente
          </p>
        </div>

        {/* Metric 2: Active Actor */}
        <div className="p-5 rounded-xl border border-neutral-200/80 dark:border-neutral-800/80 bg-white dark:bg-neutral-900/60 shadow-xs md:col-span-2 flex flex-col justify-between">
          <div className="flex items-center justify-between">
            <span className="text-[10px] font-bold text-neutral-500 dark:text-neutral-400 uppercase tracking-widest flex items-center gap-1.5 font-mono">
              <UserIcon size={13} />
              TURNO DE ACCIÓN
            </span>
            {!isTerminal && (
              <span className="font-mono text-[10px] font-bold tracking-wider text-neutral-700 dark:text-neutral-300 border border-neutral-300 dark:border-neutral-700 bg-neutral-100 dark:bg-neutral-800 px-2 py-0.5 rounded">
                EN CURSO
              </span>
            )}
          </div>
          <div className="mt-2">
            <h4 className="text-base font-semibold tracking-tight text-neutral-900 dark:text-neutral-100">
              {actorMeta.label}
            </h4>
            <p className="text-xs text-neutral-500 dark:text-neutral-400 mt-1 leading-relaxed">
              {actorMeta.description}
            </p>
          </div>
        </div>
      </div>

      {/* Active Deadline Panel (Hydration Safe) */}
      {data.activeDeadline && !isTerminal && (
        <div className="p-4 sm:p-5 rounded-xl border border-amber-500/25 bg-amber-500/[0.03] dark:bg-amber-500/[0.02]">
          <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-3">
            <div className="flex items-start gap-3">
              <div className="p-2 rounded bg-amber-500/10 text-amber-600 dark:text-amber-400 shrink-0">
                <ClockIcon size={17} />
              </div>
              <div>
                <span className="text-[10px] font-bold text-amber-700 dark:text-amber-400 uppercase tracking-widest font-mono block">
                  PLAZO LÍMITE: {data.activeDeadline.label}
                </span>
                <p
                  className="text-sm font-semibold mt-0.5 text-neutral-900 dark:text-neutral-100 font-mono"
                  suppressHydrationWarning
                >
                  {currentTime > 0
                    ? formatCountdown(deadlineDiffSeconds)
                    : "Sincronizando reloj con ledger..."}
                </p>
              </div>
            </div>
            <div className="sm:text-right">
              <span className="text-[10px] text-neutral-500 font-mono block uppercase">
                Timestamp Límite:
              </span>
              <span className="text-xs font-mono font-medium text-neutral-700 dark:text-neutral-300">
                {data.activeDeadline.timestamp}
              </span>
            </div>
          </div>
        </div>
      )}

      {/* Parties / Roles Panel */}
      <div className="p-5 rounded-xl border border-neutral-200/80 dark:border-neutral-800/80 bg-white dark:bg-neutral-900/60 shadow-xs">
        <div className="flex items-center gap-2 mb-4">
          <ScaleIcon size={15} className="text-neutral-400" />
          <h3 className="text-xs font-bold text-neutral-500 dark:text-neutral-400 uppercase tracking-widest font-mono">
            PARTICIPANTES DE LA OPERACIÓN
          </h3>
        </div>

        <div className="grid grid-cols-1 sm:grid-cols-2 gap-3 text-xs font-mono">
          {/* Buyer */}
          <div className="p-3 rounded-lg bg-neutral-50 dark:bg-neutral-900/90 border border-neutral-200/50 dark:border-neutral-800/60 flex items-center justify-between gap-2">
            <div className="truncate">
              <span className="text-neutral-400 block mb-0.5 uppercase text-[10px] tracking-wider font-semibold">
                Comprador (Buyer)
              </span>
              <span className="text-neutral-800 dark:text-neutral-200 truncate block">
                {data.parties.buyer}
              </span>
            </div>
            <button
              onClick={() => handleCopy(data.parties.buyer, "buyer")}
              className="p-1 text-neutral-400 hover:text-neutral-700 dark:hover:text-neutral-200 rounded transition-colors"
              title="Copiar dirección"
            >
              {copiedKey === "buyer" ? (
                <CheckIcon size={14} className="text-emerald-500" />
              ) : (
                <CopyIcon size={14} />
              )}
            </button>
          </div>

          {/* Supplier */}
          <div className="p-3 rounded-lg bg-neutral-50 dark:bg-neutral-900/90 border border-neutral-200/50 dark:border-neutral-800/60 flex items-center justify-between gap-2">
            <div className="truncate">
              <span className="text-neutral-400 block mb-0.5 uppercase text-[10px] tracking-wider font-semibold">
                Proveedor (Supplier)
              </span>
              <span className="text-neutral-800 dark:text-neutral-200 truncate block">
                {data.parties.supplier}
              </span>
            </div>
            <button
              onClick={() => handleCopy(data.parties.supplier, "supplier")}
              className="p-1 text-neutral-400 hover:text-neutral-700 dark:hover:text-neutral-200 rounded transition-colors"
              title="Copiar dirección"
            >
              {copiedKey === "supplier" ? (
                <CheckIcon size={14} className="text-emerald-500" />
              ) : (
                <CopyIcon size={14} />
              )}
            </button>
          </div>

          {/* Engine */}
          <div className="p-3 rounded-lg bg-neutral-50 dark:bg-neutral-900/90 border border-neutral-200/50 dark:border-neutral-800/60 flex items-center justify-between gap-2">
            <div className="truncate">
              <span className="text-neutral-400 block mb-0.5 uppercase text-[10px] tracking-wider font-semibold">
                Motor de Atestación (Engine)
              </span>
              <span className="text-neutral-800 dark:text-neutral-200 truncate block">
                {data.parties.engine}
              </span>
            </div>
            <button
              onClick={() => handleCopy(data.parties.engine, "engine")}
              className="p-1 text-neutral-400 hover:text-neutral-700 dark:hover:text-neutral-200 rounded transition-colors"
              title="Copiar dirección"
            >
              {copiedKey === "engine" ? (
                <CheckIcon size={14} className="text-emerald-500" />
              ) : (
                <CopyIcon size={14} />
              )}
            </button>
          </div>

          {/* Resolver */}
          <div className="p-3 rounded-lg bg-neutral-50 dark:bg-neutral-900/90 border border-neutral-200/50 dark:border-neutral-800/60 flex items-center justify-between gap-2">
            <div className="truncate">
              <span className="text-neutral-400 block mb-0.5 uppercase text-[10px] tracking-wider font-semibold">
                Árbitro Neutral (Resolver)
              </span>
              <span className="text-neutral-800 dark:text-neutral-200 truncate block">
                {data.parties.resolver}
              </span>
            </div>
            <button
              onClick={() => handleCopy(data.parties.resolver, "resolver")}
              className="p-1 text-neutral-400 hover:text-neutral-700 dark:hover:text-neutral-200 rounded transition-colors"
              title="Copiar dirección"
            >
              {copiedKey === "resolver" ? (
                <CheckIcon size={14} className="text-emerald-500" />
              ) : (
                <CopyIcon size={14} />
              )}
            </button>
          </div>
        </div>
      </div>

      {/* Cryptographic Hashes Vault */}
      <div className="p-5 rounded-xl border border-neutral-200/80 dark:border-neutral-800/80 bg-white dark:bg-neutral-900/60 shadow-xs">
        <div className="flex items-center justify-between mb-4">
          <div className="flex items-center gap-2">
            <HashIcon size={15} className="text-neutral-400" />
            <h3 className="text-xs font-bold text-neutral-500 dark:text-neutral-400 uppercase tracking-widest font-mono">
              BÓVEDA DE HASHES CRIPTOGRÁFICOS
            </h3>
          </div>
          <span className="text-[10px] font-mono text-neutral-500 uppercase tracking-widest px-2 py-0.5 rounded border border-neutral-200/60 dark:border-neutral-800 bg-neutral-100/60 dark:bg-neutral-900/60">
            BytesN&lt;32&gt;
          </span>
        </div>

        <div className="space-y-2.5 text-xs font-mono">
          {/* Evidence Bundle Hash */}
          <div className="flex flex-col sm:flex-row sm:items-center justify-between p-2.5 rounded-lg bg-neutral-50 dark:bg-neutral-900/80 border border-neutral-200/50 dark:border-neutral-800/60 gap-1.5">
            <span className="text-neutral-500 font-sans text-xs">
              Lote Documental (Evidence Bundle):
            </span>
            <div className="flex items-center gap-2">
              <span className="text-neutral-800 dark:text-neutral-200">
                {truncateHash(data.hashes.evidenceBundleHash || "")}
              </span>
              {data.hashes.evidenceBundleHash && (
                <button
                  onClick={() =>
                    handleCopy(data.hashes.evidenceBundleHash!, "evidenceHash")
                  }
                  className="p-1 text-neutral-400 hover:text-neutral-700 dark:hover:text-neutral-200 rounded transition-colors"
                  title="Copiar hash"
                >
                  {copiedKey === "evidenceHash" ? (
                    <CheckIcon size={14} className="text-emerald-500" />
                  ) : (
                    <CopyIcon size={14} />
                  )}
                </button>
              )}
            </div>
          </div>

          {/* Engine Report Hash */}
          <div className="flex flex-col sm:flex-row sm:items-center justify-between p-2.5 rounded-lg bg-neutral-50 dark:bg-neutral-900/80 border border-neutral-200/50 dark:border-neutral-800/60 gap-1.5">
            <span className="text-neutral-500 font-sans text-xs">
              Dictamen del Motor (Report Hash):
            </span>
            <div className="flex items-center gap-2">
              <span className="text-neutral-800 dark:text-neutral-200">
                {truncateHash(data.hashes.reportHash || "")}
              </span>
              {data.hashes.reportHash && (
                <button
                  onClick={() =>
                    handleCopy(data.hashes.reportHash!, "reportHash")
                  }
                  className="p-1 text-neutral-400 hover:text-neutral-700 dark:hover:text-neutral-200 rounded transition-colors"
                  title="Copiar hash"
                >
                  {copiedKey === "reportHash" ? (
                    <CheckIcon size={14} className="text-emerald-500" />
                  ) : (
                    <CopyIcon size={14} />
                  )}
                </button>
              )}
            </div>
          </div>

          {/* Dispute Reason Hash */}
          <div className="flex flex-col sm:flex-row sm:items-center justify-between p-2.5 rounded-lg bg-neutral-50 dark:bg-neutral-900/80 border border-neutral-200/50 dark:border-neutral-800/60 gap-1.5">
            <span className="text-neutral-500 font-sans text-xs">
              Motivo de Objeción (Reason Hash):
            </span>
            <div className="flex items-center gap-2">
              <span className="text-neutral-800 dark:text-neutral-200">
                {truncateHash(data.hashes.reasonHash || "")}
              </span>
              {data.hashes.reasonHash && (
                <button
                  onClick={() =>
                    handleCopy(data.hashes.reasonHash!, "reasonHash")
                  }
                  className="p-1 text-neutral-400 hover:text-neutral-700 dark:hover:text-neutral-200 rounded transition-colors"
                  title="Copiar hash"
                >
                  {copiedKey === "reasonHash" ? (
                    <CheckIcon size={14} className="text-emerald-500" />
                  ) : (
                    <CopyIcon size={14} />
                  )}
                </button>
              )}
            </div>
          </div>

          {/* Dispute Evidence Hash */}
          <div className="flex flex-col sm:flex-row sm:items-center justify-between p-2.5 rounded-lg bg-neutral-50 dark:bg-neutral-900/80 border border-neutral-200/50 dark:border-neutral-800/60 gap-1.5">
            <span className="text-neutral-500 font-sans text-xs">
              Pruebas de Disputa (Dispute Evidence):
            </span>
            <div className="flex items-center gap-2">
              <span className="text-neutral-800 dark:text-neutral-200">
                {truncateHash(data.hashes.disputeEvidenceHash || "")}
              </span>
              {data.hashes.disputeEvidenceHash && (
                <button
                  onClick={() =>
                    handleCopy(
                      data.hashes.disputeEvidenceHash!,
                      "disputeEvidenceHash"
                    )
                  }
                  className="p-1 text-neutral-400 hover:text-neutral-700 dark:hover:text-neutral-200 rounded transition-colors"
                  title="Copiar hash"
                >
                  {copiedKey === "disputeEvidenceHash" ? (
                    <CheckIcon size={14} className="text-emerald-500" />
                  ) : (
                    <CopyIcon size={14} />
                  )}
                </button>
              )}
            </div>
          </div>
        </div>
      </div>

      {/* Explorer Link & Audit Note */}
      <div className="pt-2 flex flex-col sm:flex-row sm:items-center justify-between gap-3 text-xs text-neutral-500">
        <div>
          {data.contractId && (
            <span className="font-mono text-neutral-600 dark:text-neutral-400">
              Contrato: {truncateHash(data.contractId, 12, 8)}
            </span>
          )}
        </div>

        {explorerUrl && (
          <a
            href={explorerUrl}
            target="_blank"
            rel="noopener noreferrer"
            className="inline-flex items-center gap-1.5 text-teal-600 dark:text-teal-400 hover:underline font-semibold font-mono"
          >
            Consultar en Stellar Expert
            <ExternalLinkIcon size={13} />
          </a>
        )}
      </div>

      {/* Audit Disclaimer */}
      <p className="text-[11px] text-neutral-400 dark:text-neutral-500 border-t border-neutral-200/60 dark:border-neutral-800/60 pt-3">
        Nota de auditoría: Los plazos contractuales se computan en función del timestamp oficial del ledger (<code className="font-mono text-[10px]">env.ledger().timestamp()</code>), no de la hora del dispositivo.
      </p>
    </div>
  );
};
