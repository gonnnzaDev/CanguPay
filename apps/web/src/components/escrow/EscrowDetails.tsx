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
  PackageIcon,
  EyeIcon,
  CpuIcon,
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

/**
 * Technical lifecycle stepper definition.
 * Lifecycle: CREADO -> FONDEADO -> EVIDENCIA -> ATESTACIÓN -> DISPUTA (si aplica) -> LIQUIDACIÓN
 */
interface LifecycleStep {
  id: string;
  label: string;
  sublabel?: string;
  isCurrent: boolean;
  isCompleted: boolean;
  isAlert?: boolean;
  isBypassed?: boolean;
}

function computeLifecycleSteps(status: EscrowStatus): LifecycleStep[] {
  const isCancelled = status === "CANCELLED";
  const isDisputed = status === "DISPUTED";
  const isTerminal = isTerminalStatus(status);

  // Status mapping indices
  // 0: CREATED
  // 1: FUNDED
  // 2: EVIDENCE_SUBMITTED
  // 3: ATTESTED_PASS / ATTESTED_FAIL
  // 4: DISPUTED
  // 5: LIQUIDACION (RELEASED, REFUNDED, SPLIT, CANCELLED)

  let activeIndex = 0;
  if (status === "CREATED") activeIndex = 0;
  else if (status === "FUNDED") activeIndex = 1;
  else if (status === "EVIDENCE_SUBMITTED") activeIndex = 2;
  else if (status === "ATTESTED_PASS" || status === "ATTESTED_FAIL") activeIndex = 3;
  else if (status === "DISPUTED") activeIndex = 4;
  else if (isTerminal) activeIndex = 5;

  return [
    {
      id: "step-1",
      label: "CREADO",
      sublabel: "Inicializado",
      isCurrent: activeIndex === 0,
      isCompleted: activeIndex > 0,
    },
    {
      id: "step-2",
      label: "FONDEADO",
      sublabel: "En Custodia",
      isCurrent: activeIndex === 1,
      isCompleted: activeIndex > 1,
      isBypassed: isCancelled,
    },
    {
      id: "step-3",
      label: "EVIDENCIA",
      sublabel: "Remisión Docs",
      isCurrent: activeIndex === 2,
      isCompleted: activeIndex > 2,
      isBypassed: isCancelled,
    },
    {
      id: "step-4",
      label: "ATESTACIÓN",
      sublabel: status === "ATTESTED_FAIL" ? "Observada" : "Dictamen Motor",
      isCurrent: activeIndex === 3,
      isCompleted: activeIndex > 3,
      isAlert: status === "ATTESTED_FAIL",
      isBypassed: isCancelled,
    },
    {
      id: "step-5",
      label: "DISPUTA",
      sublabel: isDisputed ? "Arbitraje Activo" : "Si aplica",
      isCurrent: isDisputed,
      isCompleted: activeIndex > 4 && isDisputed,
      isBypassed: !isDisputed && (activeIndex > 4 || isCancelled),
    },
    {
      id: "step-6",
      label: "LIQUIDACIÓN",
      sublabel: isCancelled
        ? "Cancelado"
        : status === "RELEASED"
          ? "Liberado"
          : status === "REFUNDED"
            ? "Reembolsado"
            : status === "SPLIT"
              ? "Dividido"
              : "Definitiva",
      isCurrent: isTerminal,
      isCompleted: isTerminal,
      isAlert: isCancelled,
    },
  ];
}

export const EscrowDetails: React.FC<EscrowDetailsProps> = ({
  data,
  isLoading = false,
  error = null,
  onRefresh,
  actionSlot,
  bannerSlot,
  viewerRole,
  canFinalize = false,
  onCreateEscrow,
}) => {
  const [copiedKey, setCopiedKey] = useState<string | null>(null);
  const currentTime = useCurrentTimestamp();

  const handleCopy = (text: string, key: string) => {
    if (!text || text === "—") return;
    navigator.clipboard?.writeText(text);
    setCopiedKey(key);
    setTimeout(() => setCopiedKey(null), 1800);
  };

  // 1. Loading State (High-Fidelity Skeleton)
  if (isLoading) {
    return (
      <div className="w-full max-w-5xl mx-auto p-4 sm:p-6 space-y-6 animate-pulse font-mono">
        {/* Header Skeleton */}
        <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-4 pb-5 border-b border-neutral-200/80 dark:border-neutral-800/80">
          <div className="space-y-2">
            <div className="h-3 w-32 bg-neutral-200 dark:bg-neutral-800 rounded-sm" />
            <div className="flex items-center gap-2">
              <div className="h-7 w-56 bg-neutral-200 dark:bg-neutral-800 rounded-md" />
              <div className="h-6 w-6 bg-neutral-200 dark:bg-neutral-800 rounded" />
            </div>
          </div>
          <div className="flex items-center gap-3">
            <div className="h-8 w-44 bg-neutral-200 dark:bg-neutral-800 rounded-lg" />
            <div className="h-8 w-8 bg-neutral-200 dark:bg-neutral-800 rounded-lg" />
          </div>
        </div>

        {/* Stepper Skeleton */}
        <div className="p-4 rounded-xl border border-neutral-200/80 dark:border-neutral-800/80 bg-white dark:bg-neutral-900/60 shadow-xs">
          <div className="h-3 w-40 bg-neutral-200 dark:bg-neutral-800 rounded-sm mb-3" />
          <div className="grid grid-cols-6 gap-2">
            {[1, 2, 3, 4, 5, 6].map((i) => (
              <div key={i} className="h-10 bg-neutral-100 dark:bg-neutral-800 rounded" />
            ))}
          </div>
        </div>

        {/* Role Panel Skeleton */}
        <div className="p-5 rounded-xl border border-neutral-200/80 dark:border-neutral-800/80 bg-white dark:bg-neutral-900/60 shadow-xs space-y-3">
          <div className="h-4 w-48 bg-neutral-200 dark:bg-neutral-800 rounded" />
          <div className="grid grid-cols-2 gap-3">
            <div className="h-14 bg-neutral-100 dark:bg-neutral-800 rounded" />
            <div className="h-14 bg-neutral-100 dark:bg-neutral-800 rounded" />
          </div>
        </div>

        {/* Primary Metrics Skeleton */}
        <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
          <div className="p-5 rounded-xl border border-neutral-200/80 dark:border-neutral-800/80 bg-white dark:bg-neutral-900/60 shadow-xs h-32" />
          <div className="p-5 rounded-xl border border-neutral-200/80 dark:border-neutral-800/80 bg-white dark:bg-neutral-900/60 shadow-xs md:col-span-2 h-32" />
        </div>
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
                className="inline-flex items-center gap-2 text-xs font-semibold px-3 py-1.5 rounded bg-neutral-900 text-white dark:bg-neutral-100 dark:text-neutral-900 hover:opacity-90 transition-opacity cursor-pointer font-mono"
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
        <p className="mt-1 text-sm text-neutral-500 dark:text-neutral-400 max-w-sm mx-auto font-mono text-xs">
          Seleccione una operación o cree un nuevo escrow comercial en Stellar Testnet.
        </p>
        <div className="mt-4 flex items-center justify-center gap-3">
          {onCreateEscrow && (
            <button
              onClick={onCreateEscrow}
              className="inline-flex items-center gap-2 text-xs font-semibold px-3.5 py-1.5 rounded-lg bg-teal-600 hover:bg-teal-700 text-white transition-colors shadow-2xs cursor-pointer font-mono"
            >
              <ShieldLockIcon size={14} />
              Crear Nuevo Escrow
            </button>
          )}
          {onRefresh && (
            <button
              onClick={onRefresh}
              className="inline-flex items-center gap-2 text-xs font-semibold px-3 py-1.5 rounded-lg border border-neutral-200 dark:border-neutral-700 bg-white dark:bg-neutral-900 text-neutral-800 dark:text-neutral-200 hover:bg-neutral-50 dark:hover:bg-neutral-800 transition-colors shadow-2xs cursor-pointer font-mono"
            >
              <RefreshIcon size={13} />
              Cargar Operación Escrow
            </button>
          )}
        </div>
      </div>
    );
  }

  const statusConfig = getStatusBadgeConfig(data.status);
  const activeActor = getActiveActor(data.status);
  const actorMeta = getActorMeta(activeActor);
  const isTerminal = isTerminalStatus(data.status);
  const lifecycleSteps = computeLifecycleSteps(data.status);

  // Client-safe deadline calculation
  let deadlineDiffSeconds = 0;
  if (data.activeDeadline?.timestamp && currentTime > 0) {
    deadlineDiffSeconds = data.activeDeadline.timestamp - currentTime;
  }

  const isExpired = deadlineDiffSeconds <= 0 && data.activeDeadline !== undefined && !isTerminal;
  const isFinalizeTriggerable = canFinalize || isExpired;

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

  const fallbackBpsLabel = data.fallbackSplitBps
    ? `${(data.fallbackSplitBps / 100).toFixed(0)}% Proveedor / ${(100 - data.fallbackSplitBps / 100).toFixed(0)}% Comprador`
    : "50% Proveedor / 50% Comprador";

  return (
    <div className="w-full max-w-5xl mx-auto p-4 sm:p-6 space-y-6 text-neutral-900 dark:text-neutral-100">
      {/* Header & Status Indicator */}
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
              className="p-1 text-neutral-400 hover:text-neutral-700 dark:hover:text-neutral-200 rounded transition-colors cursor-pointer"
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

        {/* Technical Split Status Block */}
        <div className="flex items-center gap-3">
          <div className={`flex items-center border ${statusConfig.borderColor} rounded-lg overflow-hidden bg-neutral-50/80 dark:bg-neutral-900/80 font-mono shadow-2xs`}>
            <span className="px-2.5 py-1.5 bg-neutral-100 dark:bg-neutral-800/80 text-[10px] font-bold tracking-wider text-neutral-500 dark:text-neutral-400 border-r border-neutral-200 dark:border-neutral-800 uppercase">
              STATUS
            </span>
            <span className={`px-3 py-1.5 font-bold text-xs tracking-wider uppercase ${statusConfig.textColor}`}>
              {statusConfig.label}
            </span>
          </div>

          {actionSlot}

          {onRefresh && (
            <button
              onClick={onRefresh}
              className="p-2 text-neutral-400 hover:text-neutral-700 dark:hover:text-neutral-200 transition-colors border border-neutral-200 dark:border-neutral-800 rounded-lg hover:bg-neutral-50 dark:hover:bg-neutral-900 cursor-pointer"
              title="Refrescar datos del ledger"
            >
              <RefreshIcon size={14} />
            </button>
          )}
        </div>
      </div>

      {bannerSlot}

      {/* Permissionless Finalize Alert Banner (when deadline expired or simulated) */}
      {isFinalizeTriggerable && !isTerminal && (
        <div className="p-4 rounded-xl border border-amber-500/40 bg-amber-500/10 text-amber-900 dark:text-amber-200 flex flex-col sm:flex-row sm:items-center justify-between gap-3 font-mono text-xs shadow-xs">
          <div className="flex items-start gap-2.5">
            <div className="p-1.5 rounded-md bg-amber-500/20 text-amber-600 dark:text-amber-400 shrink-0 mt-0.5">
              <ClockIcon size={16} />
            </div>
            <div>
              <span className="font-bold tracking-wider uppercase text-[11px] block">
                Vencimiento Contractual Alcanzado
              </span>
              <p className="text-[11px] text-amber-800 dark:text-amber-300 mt-0.5 leading-relaxed">
                El plazo límite ha expirado. Cualquier cuenta conectada (incluyendo observadores) está facultada para invocar <code className="font-bold">finalize()</code> y ejecutar la liquidación según la regla de fallback pactada ({data.fallbackOutcome || "SPLIT"}).
              </p>
            </div>
          </div>
          <span className="shrink-0 px-2.5 py-1 rounded bg-amber-500/20 text-amber-800 dark:text-amber-200 font-bold text-[10px] tracking-wider uppercase border border-amber-500/30">
            PERMISIÓN UNIVERSAL
          </span>
        </div>
      )}

      {/* 1. Lifecycle Timeline / Progression Stepper */}
      <div className="p-4 sm:p-5 rounded-xl border border-neutral-200/80 dark:border-neutral-800/80 bg-white dark:bg-neutral-900/60 shadow-xs font-mono">
        <div className="flex items-center justify-between mb-3 pb-2 border-b border-neutral-100 dark:border-neutral-800/60">
          <span className="text-[10px] font-bold text-neutral-500 dark:text-neutral-400 uppercase tracking-widest flex items-center gap-1.5">
            <ShieldLockIcon size={13} className="text-teal-600 dark:text-teal-400" />
            LIFECYCLE TIMELINE // PROGRESIÓN CONTRACTUAL
          </span>
          <span className="text-[10px] text-neutral-400 uppercase tracking-wider font-semibold">
            PASO {lifecycleSteps.findIndex((s) => s.isCurrent) + 1} DE 6
          </span>
        </div>

        <div className="grid grid-cols-2 sm:grid-cols-3 lg:grid-cols-6 gap-2">
          {lifecycleSteps.map((step, idx) => {
            const isCurrent = step.isCurrent;
            const isCompleted = step.isCompleted;
            const isAlert = step.isAlert;

            return (
              <div
                key={step.id}
                className={`relative p-2.5 rounded-lg border transition-all flex flex-col justify-between ${
                  isCurrent
                    ? isAlert
                      ? "border-rose-500/60 bg-rose-500/10 text-rose-900 dark:text-rose-200 ring-1 ring-rose-500/30 shadow-xs"
                      : "border-teal-500/60 bg-teal-500/10 text-teal-950 dark:text-teal-100 ring-1 ring-teal-500/30 shadow-xs"
                    : isCompleted
                      ? "border-emerald-500/30 bg-emerald-500/[0.04] dark:bg-emerald-500/[0.06] text-neutral-700 dark:text-neutral-300"
                      : "border-neutral-200/60 dark:border-neutral-800 bg-neutral-50/50 dark:bg-neutral-900/40 text-neutral-400 dark:text-neutral-500"
                }`}
              >
                <div className="flex items-center justify-between mb-1.5">
                  <span className="text-[9px] font-bold tracking-widest uppercase opacity-75">
                    0{idx + 1}
                  </span>
                  {isCompleted ? (
                    <span className="h-4 w-4 rounded-full bg-emerald-500/20 text-emerald-600 dark:text-emerald-400 flex items-center justify-center text-[10px]">
                      <CheckIcon size={10} />
                    </span>
                  ) : isCurrent ? (
                    <span
                      className={`h-2 w-2 rounded-full ${
                        isAlert ? "bg-rose-500 animate-pulse" : "bg-teal-500 animate-pulse"
                      }`}
                    />
                  ) : (
                    <span className="h-1.5 w-1.5 rounded-full bg-neutral-300 dark:bg-neutral-700" />
                  )}
                </div>

                <div>
                  <span
                    className={`block font-bold text-[11px] tracking-wider uppercase truncate ${
                      isCurrent
                        ? isAlert
                          ? "text-rose-700 dark:text-rose-400"
                          : "text-teal-700 dark:text-teal-300"
                        : isCompleted
                          ? "text-neutral-800 dark:text-neutral-200"
                          : "text-neutral-400 dark:text-neutral-500"
                    }`}
                  >
                    {step.label}
                  </span>
                  <span className="block text-[9px] text-neutral-500 dark:text-neutral-400 truncate mt-0.5">
                    {step.sublabel}
                  </span>
                </div>
              </div>
            );
          })}
        </div>
      </div>

      {/* 2. Role-Specific Focus Panel (Linder Lopez Frontend Specifications) */}
      {viewerRole === "buyer" && (
        <div className="p-5 rounded-xl border border-teal-500/30 dark:border-teal-500/30 bg-teal-500/[0.04] dark:bg-teal-950/25 shadow-xs font-mono space-y-4">
          <div className="flex items-center justify-between pb-3 border-b border-teal-500/20 dark:border-teal-500/25">
            <div className="flex items-center gap-2">
              <div className="p-1.5 rounded-lg bg-teal-500/10 dark:bg-teal-500/20 text-teal-600 dark:text-teal-400">
                <UserIcon size={16} />
              </div>
              <div>
                <h3 className="text-xs font-bold uppercase tracking-wider text-teal-950 dark:text-teal-200">
                  Panel de Control del Comprador (Buyer Perspective)
                </h3>
                <p className="text-[11px] text-neutral-500 dark:text-neutral-400">
                  Resumen ejecutivo y lista de control contractual de la orden
                </p>
              </div>
            </div>
            <span className="text-[10px] font-bold px-2 py-0.5 rounded bg-teal-500/20 dark:bg-teal-500/30 text-teal-800 dark:text-teal-300 uppercase">
              ROL: COMPRADOR
            </span>
          </div>

          <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-3 text-xs">
            {/* Metric A: Monto CPUSD */}
            <div className="p-3 rounded-lg border border-teal-500/20 dark:border-teal-500/30 bg-white/80 dark:bg-neutral-950/70">
              <span className="text-[10px] uppercase text-neutral-500 dark:text-neutral-400 font-semibold block mb-1">
                Monto de la Operación
              </span>
              <span className="text-sm font-bold text-neutral-950 dark:text-neutral-50">
                {data.amount} <span className="text-teal-600 dark:text-teal-400">{data.asset}</span>
              </span>
            </div>

            {/* Metric B: Proveedor Asignado */}
            <div className="p-3 rounded-lg border border-teal-500/20 dark:border-teal-500/30 bg-white/80 dark:bg-neutral-950/70">
              <span className="text-[10px] uppercase text-neutral-500 dark:text-neutral-400 font-semibold block mb-1">
                Proveedor Designado
              </span>
              <span className="text-xs font-medium text-neutral-800 dark:text-neutral-200 block truncate">
                {truncateHash(data.parties.supplier, 6, 6)}
              </span>
            </div>

            {/* Metric C: Plazo Activo */}
            <div className="p-3 rounded-lg border border-teal-500/20 dark:border-teal-500/30 bg-white/80 dark:bg-neutral-950/70">
              <span className="text-[10px] uppercase text-neutral-500 dark:text-neutral-400 font-semibold block mb-1">
                Plazo Contractual
              </span>
              <span className="text-xs font-medium text-amber-700 dark:text-amber-400 block truncate" suppressHydrationWarning>
                {data.activeDeadline && currentTime > 0
                  ? formatCountdown(deadlineDiffSeconds)
                  : "Sin plazo pendiente"}
              </span>
            </div>

            {/* Metric D: Fallback Acordado */}
            <div className="p-3 rounded-lg border border-teal-500/20 dark:border-teal-500/30 bg-white/80 dark:bg-neutral-950/70">
              <span className="text-[10px] uppercase text-neutral-500 dark:text-neutral-400 font-semibold block mb-1">
                Regla Fallback
              </span>
              <span className="text-xs font-bold text-neutral-900 dark:text-neutral-100 block truncate">
                {data.fallbackOutcome || "SPLIT"} ({data.fallbackOutcome === "SPLIT" ? fallbackBpsLabel : "100%"})
              </span>
            </div>
          </div>

          {/* Buyer Guidance Prompt */}
          <div className="p-3 rounded-lg bg-teal-500/5 dark:bg-teal-950/30 border border-teal-500/20 dark:border-teal-500/30 text-xs text-neutral-700 dark:text-neutral-300 flex flex-col sm:flex-row sm:items-center justify-between gap-3">
            <div>
              <span className="font-bold text-teal-800 dark:text-teal-300 uppercase text-[10px] block mb-0.5">
                Acción Recomendada:
              </span>
              <p className="text-[11px] text-neutral-600 dark:text-neutral-300 leading-relaxed">
                {data.status === "CREATED"
                  ? "La orden está creada pero los fondos aún no han sido transferidos. Puedes fondear el depósito en custodia o crear una orden adicional."
                  : data.status === "FUNDED"
                    ? "Los fondos están asegurados en Soroban. Esperando que el proveedor remita el lote documental de entrega física."
                    : data.status === "EVIDENCE_SUBMITTED"
                      ? "El lote documental ha sido recibido. El motor de atestación determinista está evaluando la evidencia."
                      : data.status === "ATTESTED_PASS"
                        ? "Atestación favorable emitida por el motor. Tienes ventana de objeción activa para aprobar la liberación o disputar."
                        : data.status === "ATTESTED_FAIL"
                          ? "El motor observó discrepancias en los documentos. El proveedor cuenta con un intento de corrección técnica."
                          : data.status === "DISPUTED"
                            ? "La operación se encuentra bajo revisión del árbitro neutral (resolver). Se espera dictamen vinculante."
                            : "La operación se encuentra liquidada en un estado inmutable."}
              </p>
            </div>
            {data.status === "CREATED" && onCreateEscrow && (
              <button
                onClick={onCreateEscrow}
                className="shrink-0 px-3 py-1.5 rounded-lg bg-teal-600 hover:bg-teal-700 text-white font-semibold text-[11px] transition-colors shadow-2xs cursor-pointer"
              >
                + Crear Nuevo Escrow
              </button>
            )}
          </div>
        </div>
      )}

      {viewerRole === "supplier" && (
        <div className="p-5 rounded-xl border border-indigo-500/30 dark:border-indigo-500/30 bg-indigo-500/[0.04] dark:bg-indigo-950/25 shadow-xs font-mono space-y-4">
          <div className="flex items-center justify-between pb-3 border-b border-indigo-500/20 dark:border-indigo-500/25">
            <div className="flex items-center gap-2">
              <div className="p-1.5 rounded-lg bg-indigo-500/10 dark:bg-indigo-500/20 text-indigo-600 dark:text-indigo-400">
                <PackageIcon size={16} />
              </div>
              <div>
                <h3 className="text-xs font-bold uppercase tracking-wider text-indigo-950 dark:text-indigo-200">
                  Panel de Control del Proveedor (Supplier Perspective)
                </h3>
                <p className="text-[11px] text-neutral-500 dark:text-neutral-400">
                  Fondos reservados en custodia y requisitos de entrega física
                </p>
              </div>
            </div>
            <span className="text-[10px] font-bold px-2 py-0.5 rounded bg-indigo-500/20 dark:bg-indigo-500/30 text-indigo-800 dark:text-indigo-300 uppercase">
              ROL: PROVEEDOR
            </span>
          </div>

          <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-3 text-xs">
            {/* Metric A: Fondos Reservados */}
            <div className="p-3 rounded-lg border border-indigo-500/20 dark:border-indigo-500/30 bg-white/80 dark:bg-neutral-950/70">
              <span className="text-[10px] uppercase text-neutral-500 dark:text-neutral-400 font-semibold block mb-1">
                Fondos Reservados en Custodia
              </span>
              <span className="text-sm font-bold text-neutral-950 dark:text-neutral-50">
                {data.amount} <span className="text-indigo-600 dark:text-indigo-400">{data.asset}</span>
              </span>
              <span className="text-[10px] text-emerald-600 dark:text-emerald-400 block mt-0.5">
                {data.status === "CREATED" ? "Pendiente de fondeo por comprador" : "Garantizados en Contrato Soroban"}
              </span>
            </div>

            {/* Metric B: Plazo de Entrega / Corrección */}
            <div className="p-3 rounded-lg border border-indigo-500/20 dark:border-indigo-500/30 bg-white/80 dark:bg-neutral-950/70">
              <span className="text-[10px] uppercase text-neutral-500 dark:text-neutral-400 font-semibold block mb-1">
                Plazo Límite de Entrega / Corrección
              </span>
              <span className="text-xs font-medium text-amber-700 dark:text-amber-400 block truncate" suppressHydrationWarning>
                {data.activeDeadline && currentTime > 0
                  ? formatCountdown(deadlineDiffSeconds)
                  : "Sin plazo activo"}
              </span>
              <span className="text-[10px] text-neutral-400 dark:text-neutral-500 block mt-0.5">
                {data.activeDeadline?.label || "Sin ventana pendiente"}
              </span>
            </div>

            {/* Metric C: Hashes de Evidencia */}
            <div className="p-3 rounded-lg border border-indigo-500/20 dark:border-indigo-500/30 bg-white/80 dark:bg-neutral-950/70">
              <span className="text-[10px] uppercase text-neutral-500 dark:text-neutral-400 font-semibold block mb-1">
                Lote Documental (Evidence Hash)
              </span>
              <span className="text-xs font-mono font-medium text-neutral-800 dark:text-neutral-200 block truncate">
                {data.hashes.evidenceBundleHash
                  ? truncateHash(data.hashes.evidenceBundleHash, 8, 6)
                  : "Pendiente de presentación"}
              </span>
              <span className="text-[10px] text-neutral-400 dark:text-neutral-500 block mt-0.5">
                {data.hashes.evidenceBundleHash ? "Hash computado off-chain" : "Requiere subir remisión"}
              </span>
            </div>
          </div>

          {/* Supplier Guidance */}
          <div className="p-3 rounded-lg bg-indigo-500/5 dark:bg-indigo-950/30 border border-indigo-500/20 dark:border-indigo-500/30 text-xs text-neutral-700 dark:text-neutral-300">
            <span className="font-bold text-indigo-800 dark:text-indigo-300 uppercase text-[10px] block mb-0.5">
              Estado de Ejecución:
            </span>
            <p className="text-[11px] text-neutral-600 dark:text-neutral-300 leading-relaxed">
              {data.status === "CREATED"
                ? "El comprador aún no ha bloqueado los fondos. No realice el despacho hasta que el estado avance a FONDEADO."
                : data.status === "FUNDED"
                  ? "Fondos asegurados en el contrato. Presente el lote documental (remisión / conocimiento de embarque / factura) antes del vencimiento."
                  : data.status === "ATTESTED_FAIL"
                    ? "El motor de atestación observó discrepancias. Tienes derecho a 1 intento de corrección técnica o a elevar disputa al árbitro."
                    : data.status === "ATTESTED_PASS"
                      ? "Atestación técnica aprobada por el motor. El comprador se encuentra en ventana de revisión para liberación final."
                      : data.status === "DISPUTED"
                        ? "Operación en arbitraje neutral. El resolver determinará la distribución de fondos definitiva."
                        : "Operación liquidada."}
            </p>
          </div>
        </div>
      )}

      {viewerRole === "resolver" && (
        <div className="p-5 rounded-xl border border-purple-500/30 dark:border-purple-500/30 bg-purple-500/[0.04] dark:bg-purple-950/25 shadow-xs font-mono space-y-4">
          <div className="flex items-center justify-between pb-3 border-b border-purple-500/20 dark:border-purple-500/25">
            <div className="flex items-center gap-2">
              <div className="p-1.5 rounded-lg bg-purple-500/10 dark:bg-purple-500/20 text-purple-600 dark:text-purple-400">
                <ScaleIcon size={16} />
              </div>
              <div>
                <h3 className="text-xs font-bold uppercase tracking-wider text-purple-950 dark:text-purple-200">
                  Panel del Árbitro Neutral (Resolver Perspective)
                </h3>
                <p className="text-[11px] text-neutral-500 dark:text-neutral-400">
                  Tribunal de arbitraje de la operación y expediente probatorio
                </p>
              </div>
            </div>
            <span className="text-[10px] font-bold px-2 py-0.5 rounded bg-purple-500/20 dark:bg-purple-500/30 text-purple-800 dark:text-purple-300 uppercase">
              ROL: ÁRBITRO (RESOLVER)
            </span>
          </div>

          <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-3 text-xs">
            {/* Metric A: Operación & Monto bajo Arbitraje */}
            <div className="p-3 rounded-lg border border-purple-500/20 dark:border-purple-500/30 bg-white/80 dark:bg-neutral-950/70">
              <span className="text-[10px] uppercase text-neutral-500 dark:text-neutral-400 font-semibold block mb-1">
                Fondo en Disputa
              </span>
              <span className="text-sm font-bold text-neutral-950 dark:text-neutral-50">
                {data.amount} <span className="text-purple-600 dark:text-purple-400">{data.asset}</span>
              </span>
              <span className="text-[10px] text-neutral-400 dark:text-neutral-500 block mt-0.5">
                Op ID: {data.operationId}
              </span>
            </div>

            {/* Metric B: Dictamen Previo del Motor */}
            <div className="p-3 rounded-lg border border-purple-500/20 dark:border-purple-500/30 bg-white/80 dark:bg-neutral-950/70">
              <span className="text-[10px] uppercase text-neutral-500 dark:text-neutral-400 font-semibold block mb-1">
                Dictamen Motor (Report Hash)
              </span>
              <span className="text-xs font-mono font-medium text-neutral-800 dark:text-neutral-200 block truncate">
                {data.hashes.reportHash ? truncateHash(data.hashes.reportHash, 8, 6) : "Sin dictamen registrado"}
              </span>
              <span className="text-[10px] text-neutral-400 dark:text-neutral-500 block mt-0.5">
                Evaluación automatizada previa
              </span>
            </div>

            {/* Metric C: Motivo y Pruebas de Disputa */}
            <div className="p-3 rounded-lg border border-purple-500/20 dark:border-purple-500/30 bg-white/80 dark:bg-neutral-950/70">
              <span className="text-[10px] uppercase text-neutral-500 dark:text-neutral-400 font-semibold block mb-1">
                Pruebas de Disputa (Hashes)
              </span>
              <div className="space-y-0.5 text-[10px]">
                <div className="truncate">
                  <span className="text-neutral-400 dark:text-neutral-500">Motivo:</span>{" "}
                  {data.hashes.reasonHash ? truncateHash(data.hashes.reasonHash, 6, 4) : "—"}
                </div>
                <div className="truncate">
                  <span className="text-neutral-400 dark:text-neutral-500">Evidencia:</span>{" "}
                  {data.hashes.disputeEvidenceHash ? truncateHash(data.hashes.disputeEvidenceHash, 6, 4) : "—"}
                </div>
              </div>
            </div>
          </div>

          {/* Resolver Guidance */}
          <div className="p-3 rounded-lg bg-purple-500/5 dark:bg-purple-950/30 border border-purple-500/20 dark:border-purple-500/30 text-xs text-neutral-700 dark:text-neutral-300">
            <span className="font-bold text-purple-800 dark:text-purple-300 uppercase text-[10px] block mb-0.5">
              Facultad Jurisdiccional:
            </span>
            <p className="text-[11px] text-neutral-600 dark:text-neutral-300 leading-relaxed">
              {data.status === "DISPUTED"
                ? "Como árbitro neutral designado, estás facultado para dirimir este conflicto mediante invocación vinculante en Soroban: Liberar a proveedor (Release), Reembolsar a comprador (Refund), o Dividir proporcionalmente (Split)."
                : "Esta operación no se encuentra actualmente en estado de disputa. La intervención del árbitro sólo se activa si alguna de las partes objeta el dictamen."}
            </p>
          </div>
        </div>
      )}

      {viewerRole === "observer" && (
        <div className="p-4 rounded-xl border border-neutral-200 dark:border-neutral-800 bg-neutral-100/60 dark:bg-neutral-900/80 shadow-xs font-mono text-xs flex items-start gap-3">
          <div className="p-2 rounded-lg bg-neutral-200/80 dark:bg-neutral-800 text-neutral-600 dark:text-neutral-400 shrink-0">
            <EyeIcon size={18} />
          </div>
          <div className="space-y-1">
            <div className="flex items-center gap-2">
              <span className="font-bold text-neutral-900 dark:text-neutral-100 text-[11px] uppercase tracking-wider">
                Modo Lectura / Observer
              </span>
              <span className="text-[9px] px-2 py-0.5 rounded bg-neutral-200/80 dark:bg-neutral-800 text-neutral-600 dark:text-neutral-400 font-bold uppercase">
                Solo Lectura
              </span>
            </div>
            <p className="text-[11px] text-neutral-600 dark:text-neutral-300 leading-relaxed">
              Cuenta conectada en modo observador. Visualización pública de solo lectura. Tu dirección Freighter no coincide con el Comprador, Proveedor ni Árbitro configurados en este contrato. Puedes auditar libremente todos los estados, hashes y plazos de la operación.
            </p>
          </div>
        </div>
      )}

      {viewerRole === null && (
        <div className="p-3 rounded-xl border border-neutral-200 dark:border-neutral-800 bg-neutral-50/80 dark:bg-neutral-900/70 text-neutral-600 dark:text-neutral-300 font-mono text-[11px] flex items-center justify-between">
          <span>
            Conecte su wallet Freighter para detectar automáticamente su rol en este contrato (Comprador, Proveedor o Árbitro).
          </span>
          <span className="text-[10px] text-teal-600 dark:text-teal-400 font-semibold uppercase">
            DETECCIÓN DINÁMICA
          </span>
        </div>
      )}

      {/* Primary Metrics: Amount & Current Turn */}
      <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
        {/* Metric 1: Amount */}
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

          <p className="text-xs text-neutral-500 dark:text-neutral-400 font-mono">
            Reserva atómica en contrato inteligente
          </p>
        </div>

        {/* Metric 2: Active Actor */}
        <div className="p-5 rounded-xl border border-neutral-200/80 dark:border-neutral-800/80 bg-white dark:bg-neutral-900/60 shadow-xs md:col-span-2 flex flex-col justify-between font-mono">
          <div className="flex items-center justify-between">
            <span className="text-[10px] font-bold text-neutral-500 dark:text-neutral-400 uppercase tracking-widest flex items-center gap-1.5">
              <UserIcon size={13} />
              TURNO DE ACCIÓN
            </span>
            {!isTerminal && (
              <span className="text-[10px] font-bold tracking-wider text-neutral-700 dark:text-neutral-300 border border-neutral-300 dark:border-neutral-700 bg-neutral-100 dark:bg-neutral-800 px-2 py-0.5 rounded">
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

      {/* Active Deadline Panel */}
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

      {/* Parties / Roles Panel & Fallback Configuration */}
      <div className="p-5 rounded-xl border border-neutral-200/80 dark:border-neutral-800/80 bg-white dark:bg-neutral-900/60 shadow-xs font-mono">
        <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-1.5 mb-3.5">
          <div className="flex items-center gap-2">
            <ScaleIcon size={15} className="text-neutral-400" />
            <h3 className="text-xs font-bold text-neutral-500 dark:text-neutral-400 uppercase tracking-widest">
              PARTICIPANTES DE LA OPERACIÓN
            </h3>
          </div>
          <span className="text-[10px] text-neutral-400 dark:text-neutral-500 uppercase tracking-wider flex items-center gap-1.5">
            <span>FREIGHTER ACCOUNTS</span>
            <span className="hidden sm:inline opacity-60">· Pasa el cursor para inspeccionar wallet</span>
          </span>
        </div>

        {/* Compact Minimized Participant Chips (Expands on Hover / Keyboard Focus) */}
        <div className="flex flex-wrap items-center justify-center gap-2.5 mb-4">
          {[
            {
              key: "buyer",
              label: "Buyer",
              roleDesc: "Comprador",
              address: data.parties.buyer,
              icon: UserIcon,
              iconBg: "bg-blue-500/10 dark:bg-blue-500/20",
              iconColor: "text-blue-600 dark:text-blue-400",
              hoverBorder: "hover:border-blue-500/40 focus-within:border-blue-500/40",
              isCurrentViewer: viewerRole === "buyer",
            },
            {
              key: "supplier",
              label: "Supplier",
              roleDesc: "Proveedor",
              address: data.parties.supplier,
              icon: PackageIcon,
              iconBg: "bg-amber-500/10 dark:bg-amber-500/20",
              iconColor: "text-amber-600 dark:text-amber-400",
              hoverBorder: "hover:border-amber-500/40 focus-within:border-amber-500/40",
              isCurrentViewer: viewerRole === "supplier",
            },
            {
              key: "engine",
              label: "Engine",
              roleDesc: "Motor Atestador",
              address: data.parties.engine,
              icon: CpuIcon,
              iconBg: "bg-purple-500/10 dark:bg-purple-500/20",
              iconColor: "text-purple-600 dark:text-purple-400",
              hoverBorder: "hover:border-purple-500/40 focus-within:border-purple-500/40",
              isCurrentViewer: false,
            },
            {
              key: "resolver",
              label: "Resolver",
              roleDesc: "Árbitro Neutral",
              address: data.parties.resolver,
              icon: ScaleIcon,
              iconBg: "bg-teal-500/10 dark:bg-teal-500/20",
              iconColor: "text-teal-600 dark:text-teal-400",
              hoverBorder: "hover:border-teal-500/40 focus-within:border-teal-500/40",
              isCurrentViewer: viewerRole === "resolver",
            },
          ].map((p) => {
            const isCopied = copiedKey === p.key;
            return (
              <div
                key={p.key}
                tabIndex={0}
                className={`group relative flex items-center gap-2 px-3 py-1.5 rounded-xl border border-neutral-200/80 dark:border-neutral-800/80 bg-neutral-50/70 dark:bg-neutral-900/70 hover:bg-white dark:hover:bg-neutral-850 hover:shadow-xs transition-all duration-300 ease-out cursor-pointer select-none ${p.hoverBorder}`}
                title={`${p.label} (${p.roleDesc}): ${p.address}`}
                onClick={() => handleCopy(p.address, p.key)}
                onKeyDown={(e) => {
                  if (e.key === "Enter" || e.key === " ") {
                    e.preventDefault();
                    handleCopy(p.address, p.key);
                  }
                }}
              >
                {/* Custom Role Icon with Palette */}
                <div
                  className={`p-1.5 rounded-lg ${p.iconBg} ${p.iconColor} shrink-0 transition-transform group-hover:scale-105 duration-200`}
                >
                  <p.icon size={14} />
                </div>

                {/* Minimized Role Label */}
                <div className="flex items-center gap-1.5 shrink-0">
                  <span className="text-xs font-bold text-neutral-800 dark:text-neutral-200 tracking-tight">
                    {p.label}
                  </span>
                  {p.isCurrentViewer && (
                    <span className="text-[9px] font-bold px-1.5 py-0.5 rounded bg-teal-500/15 text-teal-700 dark:text-teal-300 border border-teal-500/30">
                      TÚ
                    </span>
                  )}
                </div>

                {/* Animated Expandable Address & Copy Button (Expands on Hover / Keyboard Focus) */}
                <div className="max-w-0 opacity-0 overflow-hidden group-hover:max-w-[220px] group-hover:opacity-100 group-focus-within:max-w-[220px] group-focus-within:opacity-100 transition-all duration-300 ease-in-out flex items-center gap-1.5 pl-0 group-hover:pl-2 group-focus-within:pl-2 border-l-0 group-hover:border-l group-focus-within:border-l border-neutral-200 dark:border-neutral-750">
                  {isCopied ? (
                    <span className="text-[10px] font-bold text-emerald-600 dark:text-emerald-400 shrink-0 flex items-center gap-1">
                      <CheckIcon size={12} />
                      ¡Copiado!
                    </span>
                  ) : (
                    <>
                      <span className="font-mono text-[11px] text-neutral-500 dark:text-neutral-400 font-medium shrink-0">
                        {truncateHash(p.address, 4, 4)}
                      </span>
                      <button
                        type="button"
                        onClick={(e) => {
                          e.stopPropagation();
                          handleCopy(p.address, p.key);
                        }}
                        className="p-1 rounded text-neutral-400 hover:text-neutral-700 dark:hover:text-neutral-200 hover:bg-neutral-200/60 dark:hover:bg-neutral-800 transition-colors shrink-0 cursor-pointer"
                        title="Copiar dirección completa"
                        aria-label={`Copiar dirección de ${p.label}`}
                      >
                        <CopyIcon size={12} />
                      </button>
                    </>
                  )}
                </div>
              </div>
            );
          })}
        </div>

        {/* Fallback Contractual Configuration Card */}
        <div className="p-3.5 rounded-lg border border-teal-500/20 bg-teal-500/[0.02] dark:bg-teal-500/[0.04] flex flex-col sm:flex-row sm:items-center justify-between gap-2.5">
          <div className="space-y-0.5">
            <span className="text-[10px] uppercase font-bold text-teal-800 dark:text-teal-300 tracking-wider">
              Cláusula Fallback Preacordada
            </span>
            <p className="text-xs text-neutral-700 dark:text-neutral-300">
              Liquidación por defecto:{" "}
              <span className="font-bold text-teal-700 dark:text-teal-400">
                {data.fallbackOutcome || "SPLIT"}
              </span>{" "}
              ({data.fallbackOutcome === "SPLIT" ? fallbackBpsLabel : data.fallbackOutcome === "RELEASE" ? "100% Proveedor" : "100% Comprador"})
            </p>
          </div>
          <span className="text-[10px] text-neutral-400 sm:text-right max-w-xs">
            Ejecutable mediante <code className="text-teal-600 dark:text-teal-400">finalize()</code> tras vencer el plazo sin acción.
          </span>
        </div>
      </div>

      {/* Cryptographic Hashes Vault */}
      <div className="p-5 rounded-xl border border-neutral-200/80 dark:border-neutral-800/80 bg-white dark:bg-neutral-900/60 shadow-xs font-mono">
        <div className="flex items-center justify-between mb-4">
          <div className="flex items-center gap-2">
            <HashIcon size={15} className="text-neutral-400" />
            <h3 className="text-xs font-bold text-neutral-500 dark:text-neutral-400 uppercase tracking-widest">
              BÓVEDA DE HASHES CRIPTOGRÁFICOS
            </h3>
          </div>
          <span className="text-[10px] text-neutral-500 uppercase tracking-widest px-2 py-0.5 rounded border border-neutral-200/60 dark:border-neutral-800 bg-neutral-100/60 dark:bg-neutral-900/60">
            BytesN&lt;32&gt;
          </span>
        </div>

        <div className="space-y-2.5 text-xs">
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
                  className="p-1 text-neutral-400 hover:text-neutral-700 dark:hover:text-neutral-200 rounded transition-colors cursor-pointer"
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
                  className="p-1 text-neutral-400 hover:text-neutral-700 dark:hover:text-neutral-200 rounded transition-colors cursor-pointer"
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
                  className="p-1 text-neutral-400 hover:text-neutral-700 dark:hover:text-neutral-200 rounded transition-colors cursor-pointer"
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
                  className="p-1 text-neutral-400 hover:text-neutral-700 dark:hover:text-neutral-200 rounded transition-colors cursor-pointer"
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
      <div className="pt-2 flex flex-col sm:flex-row sm:items-center justify-between gap-3 text-xs text-neutral-500 font-mono">
        <div>
          {data.contractId && (
            <span className="text-neutral-600 dark:text-neutral-400">
              Contrato: {truncateHash(data.contractId, 12, 8)}
            </span>
          )}
        </div>

        {explorerUrl && (
          <a
            href={explorerUrl}
            target="_blank"
            rel="noopener noreferrer"
            className="inline-flex items-center gap-1.5 text-teal-600 dark:text-teal-400 hover:underline font-semibold"
          >
            Consultar en Stellar Expert
            <ExternalLinkIcon size={13} />
          </a>
        )}
      </div>

      {/* Audit Disclaimer */}
      <p className="text-[11px] text-neutral-400 dark:text-neutral-500 border-t border-neutral-200/60 dark:border-neutral-800/60 pt-3 font-mono">
        Nota de auditoría: Los plazos contractuales se computan en función del timestamp oficial del ledger (<code className="text-[10px]">env.ledger().timestamp()</code>), no de la hora del dispositivo.
      </p>
    </div>
  );
};
