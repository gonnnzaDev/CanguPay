"use client";

import React, { useState } from "react";
import {
  EscrowDetailsProps,
  EscrowStatus,
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
  CpuIcon,
} from "@/components/icons";
import { truncateHash, formatCountdown } from "./helpers";
import {
  BuyerPanel,
  SupplierPanel,
  ResolverPanel,
  ObserverPanel,
} from "./roles";
import { useLanguage } from "@/providers/LanguageProvider";

export { truncateHash, formatCountdown };

/**
 * Maps EscrowStatus to technical status styling without generic pills or dots.
 */
function getStatusBadgeConfig(
  status: EscrowStatus,
  t: (path: string, params?: Record<string, string | number>) => string
): {
  label: string;
  textColor: string;
  borderColor: string;
} {
  const label = t(`status.${status}`);
  switch (status) {
    case "CREATED":
      return {
        label,
        textColor: "text-amber-700 dark:text-amber-400",
        borderColor: "border-amber-500/30",
      };
    case "FUNDED":
      return {
        label,
        textColor: "text-teal-700 dark:text-teal-400",
        borderColor: "border-teal-500/30",
      };
    case "EVIDENCE_SUBMITTED":
      return {
        label,
        textColor: "text-blue-700 dark:text-blue-400",
        borderColor: "border-blue-500/30",
      };
    case "ATTESTED_PASS":
      return {
        label,
        textColor: "text-emerald-700 dark:text-emerald-400",
        borderColor: "border-emerald-500/30",
      };
    case "ATTESTED_FAIL":
      return {
        label,
        textColor: "text-rose-700 dark:text-rose-400",
        borderColor: "border-rose-500/30",
      };
    case "DISPUTED":
      return {
        label,
        textColor: "text-purple-700 dark:text-purple-400",
        borderColor: "border-purple-500/30",
      };
    case "RELEASED":
      return {
        label,
        textColor: "text-emerald-700 dark:text-emerald-400",
        borderColor: "border-emerald-500/30",
      };
    case "REFUNDED":
      return {
        label,
        textColor: "text-neutral-700 dark:text-neutral-300",
        borderColor: "border-neutral-500/30",
      };
    case "SPLIT":
      return {
        label,
        textColor: "text-cyan-700 dark:text-cyan-400",
        borderColor: "border-cyan-500/30",
      };
    case "CANCELLED":
      return {
        label,
        textColor: "text-neutral-600 dark:text-neutral-400",
        borderColor: "border-neutral-500/30",
      };
    default:
      return {
        label,
        textColor: "text-neutral-600 dark:text-neutral-400",
        borderColor: "border-neutral-500/30",
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

function computeLifecycleSteps(
  status: EscrowStatus,
  t: (path: string, params?: Record<string, string | number>) => string
): LifecycleStep[] {
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
      label: t("lifecycle.step_1.label"),
      sublabel: t("lifecycle.step_1.sublabel"),
      isCurrent: activeIndex === 0,
      isCompleted: activeIndex > 0,
    },
    {
      id: "step-2",
      label: t("lifecycle.step_2.label"),
      sublabel: t("lifecycle.step_2.sublabel"),
      isCurrent: activeIndex === 1,
      isCompleted: activeIndex > 1,
      isBypassed: isCancelled,
    },
    {
      id: "step-3",
      label: t("lifecycle.step_3.label"),
      sublabel: t("lifecycle.step_3.sublabel"),
      isCurrent: activeIndex === 2,
      isCompleted: activeIndex > 2,
      isBypassed: isCancelled,
    },
    {
      id: "step-4",
      label: t("lifecycle.step_4.label"),
      sublabel: t("lifecycle.step_4.sublabel"),
      isCurrent: activeIndex === 3,
      isCompleted: activeIndex > 3,
      isAlert: status === "ATTESTED_FAIL",
      isBypassed: isCancelled,
    },
    {
      id: "step-5",
      label: t("lifecycle.step_5.label"),
      sublabel: t("lifecycle.step_5.sublabel"),
      isCurrent: isDisputed,
      isCompleted: activeIndex > 4 && isDisputed,
      isBypassed: !isDisputed && (activeIndex > 4 || isCancelled),
    },
    {
      id: "step-6",
      label: t("lifecycle.step_6.label"),
      sublabel: t("lifecycle.step_6.sublabel"),
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
  const { t } = useLanguage();
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
                {t("alerts.contract_error")}
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
                {t("alerts.retry_sync")}
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
          {t("alerts.no_contract")}
        </h3>
        <p className="mt-1 text-sm text-neutral-500 dark:text-neutral-400 max-w-sm mx-auto font-mono text-xs">
          {t("alerts.no_contract_desc")}
        </p>
        <div className="mt-4 flex items-center justify-center gap-3">
          {onCreateEscrow && (
            <button
              onClick={onCreateEscrow}
              className="inline-flex items-center gap-2 text-xs font-semibold px-3.5 py-1.5 rounded-lg bg-teal-600 hover:bg-teal-700 text-white transition-colors shadow-2xs cursor-pointer font-mono"
            >
              <ShieldLockIcon size={14} />
              {t("alerts.create_new_escrow")}
            </button>
          )}
          {onRefresh && (
            <button
              onClick={onRefresh}
              className="inline-flex items-center gap-2 text-xs font-semibold px-3 py-1.5 rounded-lg border border-neutral-200 dark:border-neutral-700 bg-white dark:bg-neutral-900 text-neutral-800 dark:text-neutral-200 hover:bg-neutral-50 dark:hover:bg-neutral-800 transition-colors shadow-2xs cursor-pointer font-mono"
            >
              <RefreshIcon size={13} />
              {t("alerts.load_escrow")}
            </button>
          )}
        </div>
      </div>
    );
  }

  const statusConfig = getStatusBadgeConfig(data.status, t);
  const activeActor = getActiveActor(data.status);
  const isTerminal = isTerminalStatus(data.status);
  const lifecycleSteps = computeLifecycleSteps(data.status, t);
  const currentStepIndex = Math.max(0, lifecycleSteps.findIndex((s) => s.isCurrent));

  // Client-safe deadline calculation
  let deadlineDiffSeconds = 0;
  if (data.activeDeadline?.timestamp && currentTime > 0) {
    deadlineDiffSeconds = data.activeDeadline.timestamp - currentTime;
  }

  const isExpired = deadlineDiffSeconds <= 0 && data.activeDeadline !== undefined && !isTerminal;
  const isFinalizeTriggerable = canFinalize || isExpired;

  const explorerUrl = data.transactionHash
    ? `${data.explorerBaseUrl || "https://stellar.expert/explorer/testnet"}/tx/${data.transactionHash}`
    : data.contractId
      ? `${data.explorerBaseUrl || "https://stellar.expert/explorer/testnet"}/contract/${data.contractId}`
      : null;

  const supplierPct = data.fallbackSplitBps ? data.fallbackSplitBps / 100 : 50;
  const buyerPct = data.fallbackSplitBps ? 100 - data.fallbackSplitBps / 100 : 50;
  const fallbackBpsLabel = `${t("fallback.supplier_share", { pct: supplierPct })} / ${t("fallback.buyer_share", { pct: buyerPct })}`;

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
              {t(`status.${data.status}`)}
            </span>
          </div>

          {actionSlot}

          {onRefresh && (
            <button
              onClick={onRefresh}
              className="p-2 text-neutral-400 hover:text-neutral-700 dark:hover:text-neutral-200 transition-colors border border-neutral-200 dark:border-neutral-800 rounded-lg hover:bg-neutral-50 dark:hover:bg-neutral-900 cursor-pointer"
              title={t("common.refresh")}
            >
              <RefreshIcon size={14} />
            </button>
          )}
        </div>
      </div>

      {bannerSlot}

      {/* Permissionless Finalize Alert Banner (when deadline expired or simulated) */}
      {isFinalizeTriggerable && !isTerminal && (
        <div className="p-3 sm:p-4 rounded-xl border border-amber-500/40 bg-amber-500/10 text-amber-900 dark:text-amber-200 flex flex-col sm:flex-row sm:items-center justify-between gap-3 font-mono text-xs shadow-xs">
          <div className="flex items-center gap-2.5">
            <div className="p-1.5 rounded-md bg-amber-500/20 text-amber-600 dark:text-amber-400 shrink-0">
              <ClockIcon size={16} />
            </div>
            <p className="text-xs text-amber-800 dark:text-amber-300 font-medium">
              {t("alerts.finalize_expired", { outcome: data.fallbackOutcome || "SPLIT" })}
            </p>
          </div>
          <span className="shrink-0 px-2.5 py-1 rounded bg-amber-500/20 text-amber-800 dark:text-amber-200 font-bold text-[10px] tracking-wider uppercase border border-amber-500/30">
            {t("alerts.universal_permission")}
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
            {t("lifecycle.step_counter", { current: currentStepIndex + 1, total: 6 })}
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
                    {t(`lifecycle.step_${idx + 1}.label`)}
                  </span>
                  <span className="block text-[9px] text-neutral-500 dark:text-neutral-400 truncate mt-0.5">
                    {t(`lifecycle.step_${idx + 1}.sublabel`)}
                  </span>
                </div>
              </div>
            );
          })}
        </div>
      </div>

      {/* 2. Role-Specific Focus Panel (Linder Lopez Frontend Specifications) */}
      {viewerRole === "buyer" && (
        <BuyerPanel
          data={data}
          currentTime={currentTime}
          deadlineDiffSeconds={deadlineDiffSeconds}
          fallbackBpsLabel={fallbackBpsLabel}
          onCreateEscrow={onCreateEscrow}
        />
      )}

      {viewerRole === "supplier" && (
        <SupplierPanel
          data={data}
          currentTime={currentTime}
          deadlineDiffSeconds={deadlineDiffSeconds}
        />
      )}

      {viewerRole === "resolver" && (
        <ResolverPanel data={data} />
      )}

      {viewerRole === "observer" && (
        <ObserverPanel />
      )}

      {viewerRole === null && (
        <div className="p-3 rounded-xl border border-neutral-200 dark:border-neutral-800 bg-neutral-50/80 dark:bg-neutral-900/70 text-neutral-600 dark:text-neutral-400 font-mono text-xs">
          {t("roles.disconnected.banner")}
        </div>
      )}

      {/* Primary Metrics: Amount & Current Turn */}
      <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
        {/* Metric 1: Amount */}
        <div className="p-5 rounded-xl border border-neutral-200/80 dark:border-neutral-800/80 bg-white dark:bg-neutral-900/60 shadow-xs flex flex-col justify-between min-w-0">
          <div className="flex items-center justify-between gap-2">
            <span className="text-[10px] font-bold text-neutral-500 dark:text-neutral-400 uppercase tracking-widest font-mono truncate">
              {t("metrics.funds_under_custody")}
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
        </div>

        {/* Metric 2: Active Actor */}
        <div className="p-5 rounded-xl border border-neutral-200/80 dark:border-neutral-800/80 bg-white dark:bg-neutral-900/60 shadow-xs md:col-span-2 flex flex-col justify-between font-mono">
          <div className="flex items-center justify-between">
            <span className="text-[10px] font-bold text-neutral-500 dark:text-neutral-400 uppercase tracking-widest flex items-center gap-1.5">
              <UserIcon size={13} />
              {t("metrics.action_turn")}
            </span>
            {!isTerminal && (
              <span className="text-[10px] font-bold tracking-wider text-neutral-700 dark:text-neutral-300 border border-neutral-300 dark:border-neutral-700 bg-neutral-100 dark:bg-neutral-800 px-2 py-0.5 rounded">
                {t("metrics.in_progress")}
              </span>
            )}
          </div>
          <div className="mt-2">
            <h4 className="text-base font-semibold tracking-tight text-neutral-900 dark:text-neutral-100">
              {t(`actors.${activeActor}`)}
            </h4>
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
                  {t("metrics.deadline_label", { label: data.activeDeadline.label })}
                </span>
                <p
                  className="text-sm font-semibold mt-0.5 text-neutral-900 dark:text-neutral-100 font-mono"
                  suppressHydrationWarning
                >
                  {currentTime > 0
                    ? formatCountdown(deadlineDiffSeconds)
                    : t("metrics.syncing_clock")}
                </p>
              </div>
            </div>
            <div className="sm:text-right">
              <span className="text-[10px] text-neutral-500 font-mono block uppercase">
                {t("metrics.timestamp_limit")}
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
        <div className="flex items-center justify-between gap-1.5 mb-3.5">
          <div className="flex items-center gap-2">
            <ScaleIcon size={15} className="text-neutral-400" />
            <h3 className="text-xs font-bold text-neutral-500 dark:text-neutral-400 uppercase tracking-widest">
              {t("participants.title")}
            </h3>
          </div>
          <span className="text-[10px] text-neutral-400 dark:text-neutral-500 uppercase tracking-wider">
            {t("participants.freighter_accounts")}
          </span>
        </div>

        {/* Compact Minimized Participant Chips (Expands on Hover / Keyboard Focus) */}
        <div className="flex flex-wrap items-center justify-center gap-2.5 mb-4">
          {[
            {
              key: "buyer",
              label: "Buyer",
              roleDesc: t("participants.buyer_desc"),
              address: data.parties.buyer,
              icon: UserIcon,
              iconBg: "bg-blue-500/10 dark:bg-blue-500/20",
              iconColor: "text-blue-600 dark:text-blue-400",
              hoverBg: "hover:bg-blue-50/60 dark:hover:bg-blue-950/40",
              hoverBorder: "hover:border-blue-500/40 dark:hover:border-blue-400/40 focus-within:border-blue-500/40 dark:focus-within:border-blue-400/40",
              isCurrentViewer: viewerRole === "buyer",
            },
            {
              key: "supplier",
              label: "Supplier",
              roleDesc: t("participants.supplier_desc"),
              address: data.parties.supplier,
              icon: PackageIcon,
              iconBg: "bg-amber-500/10 dark:bg-amber-500/20",
              iconColor: "text-amber-600 dark:text-amber-400",
              hoverBg: "hover:bg-amber-50/60 dark:hover:bg-amber-950/40",
              hoverBorder: "hover:border-amber-500/40 dark:hover:border-amber-400/40 focus-within:border-amber-500/40 dark:focus-within:border-amber-400/40",
              isCurrentViewer: viewerRole === "supplier",
            },
            {
              key: "engine",
              label: "Engine",
              roleDesc: t("participants.engine_desc"),
              address: data.parties.engine,
              icon: CpuIcon,
              iconBg: "bg-purple-500/10 dark:bg-purple-500/20",
              iconColor: "text-purple-600 dark:text-purple-400",
              hoverBg: "hover:bg-purple-50/60 dark:hover:bg-purple-950/40",
              hoverBorder: "hover:border-purple-500/40 dark:hover:border-purple-400/40 focus-within:border-purple-500/40 dark:focus-within:border-purple-400/40",
              isCurrentViewer: false,
            },
            {
              key: "resolver",
              label: "Resolver",
              roleDesc: t("participants.resolver_desc"),
              address: data.parties.resolver,
              icon: ScaleIcon,
              iconBg: "bg-teal-500/10 dark:bg-teal-500/20",
              iconColor: "text-teal-600 dark:text-teal-400",
              hoverBg: "hover:bg-teal-50/60 dark:hover:bg-teal-950/40",
              hoverBorder: "hover:border-teal-500/40 dark:hover:border-teal-400/40 focus-within:border-teal-500/40 dark:focus-within:border-teal-400/40",
              isCurrentViewer: viewerRole === "resolver",
            },
          ].map((p) => {
            const isCopied = copiedKey === p.key;
            return (
              <div
                key={p.key}
                tabIndex={0}
                className={`group relative flex items-center gap-2 px-3 py-1.5 rounded-xl border border-neutral-200/80 dark:border-neutral-800 bg-neutral-50/80 dark:bg-neutral-900/80 ${p.hoverBg} hover:shadow-xs dark:hover:shadow-neutral-950/50 transition-all duration-300 ease-out cursor-pointer select-none ${p.hoverBorder}`}
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
                  <span className="text-xs font-bold text-neutral-800 dark:text-neutral-100 tracking-tight">
                    {p.label}
                  </span>
                  {p.isCurrentViewer && (
                    <span className="text-[9px] font-bold px-1.5 py-0.5 rounded bg-teal-500/15 text-teal-700 dark:text-teal-300 border border-teal-500/30">
                      {t("common.you")}
                    </span>
                  )}
                </div>

                {/* Animated Expandable Address & Copy Button (Expands on Hover / Keyboard Focus) */}
                <div className="max-w-0 opacity-0 overflow-hidden group-hover:max-w-[220px] group-hover:opacity-100 group-focus-within:max-w-[220px] group-focus-within:opacity-100 transition-all duration-300 ease-in-out flex items-center gap-1.5 pl-0 group-hover:pl-2 group-focus-within:pl-2 border-l-0 group-hover:border-l group-focus-within:border-l border-neutral-200 dark:border-neutral-700">
                  {isCopied ? (
                    <span className="text-[10px] font-bold text-emerald-600 dark:text-emerald-400 shrink-0 flex items-center gap-1">
                      <CheckIcon size={12} />
                      {t("common.copied")}
                    </span>
                  ) : (
                    <>
                      <span className="font-mono text-[11px] text-neutral-600 dark:text-neutral-300 font-medium shrink-0">
                        {truncateHash(p.address, 4, 4)}
                      </span>
                      <button
                        type="button"
                        onClick={(e) => {
                          e.stopPropagation();
                          handleCopy(p.address, p.key);
                        }}
                        className="p-1 rounded text-neutral-400 hover:text-neutral-800 dark:text-neutral-400 dark:hover:text-neutral-100 hover:bg-neutral-200/60 dark:hover:bg-neutral-800 transition-colors shrink-0 cursor-pointer"
                        title={t("common.copy_full_address")}
                        aria-label={t("common.copy_full_address")}
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
        <div className="p-3.5 rounded-lg border border-teal-500/20 bg-teal-500/[0.02] dark:bg-teal-500/[0.04]">
          <span className="text-[10px] uppercase font-bold text-teal-800 dark:text-teal-300 tracking-wider block mb-0.5">
            {t("fallback.clause_title")}
          </span>
          <p className="text-xs text-neutral-700 dark:text-neutral-300">
            {t("fallback.default_settlement")}{" "}
            <span className="font-bold text-teal-700 dark:text-teal-400">
              {data.fallbackOutcome || "SPLIT"}
            </span>{" "}
            ({data.fallbackOutcome === "SPLIT"
              ? fallbackBpsLabel
              : data.fallbackOutcome === "RELEASE"
                ? t("fallback.supplier_share", { pct: 100 })
                : t("fallback.buyer_share", { pct: 100 })})
          </p>
        </div>
      </div>

      {/* Cryptographic Hashes Chips */}
      <div className="p-5 rounded-xl border border-neutral-200/80 dark:border-neutral-800/80 bg-white dark:bg-neutral-900/60 shadow-xs font-mono">
        <div className="flex items-center justify-between gap-1.5 mb-3.5">
          <div className="flex items-center gap-2">
            <HashIcon size={15} className="text-neutral-400" />
            <h3 className="text-xs font-bold text-neutral-500 dark:text-neutral-400 uppercase tracking-widest">
              {t("hashes.title")}
            </h3>
          </div>
          <span className="text-[10px] text-neutral-400 dark:text-neutral-500 uppercase tracking-wider">
            {t("hashes.bytes_label")}
          </span>
        </div>

        {/* Compact Minimized Hash Chips (Expands on Hover / Keyboard Focus) */}
        <div className="flex flex-wrap items-center justify-center gap-2.5">
          {[
            {
              key: "evidenceHash",
              label: t("hashes.evidence"),
              roleDesc: t("hashes.evidence_desc"),
              hash: data.hashes.evidenceBundleHash,
              icon: FileTextIcon,
              iconBg: "bg-cyan-500/10 dark:bg-cyan-500/20",
              iconColor: "text-cyan-600 dark:text-cyan-400",
              hoverBg: "hover:bg-cyan-50/60 dark:hover:bg-cyan-950/40",
              hoverBorder: "hover:border-cyan-500/40 dark:hover:border-cyan-400/40 focus-within:border-cyan-500/40 dark:focus-within:border-cyan-400/40",
            },
            {
              key: "reportHash",
              label: t("hashes.report"),
              roleDesc: t("hashes.report_desc"),
              hash: data.hashes.reportHash,
              icon: CpuIcon,
              iconBg: "bg-purple-500/10 dark:bg-purple-500/20",
              iconColor: "text-purple-600 dark:text-purple-400",
              hoverBg: "hover:bg-purple-50/60 dark:hover:bg-purple-950/40",
              hoverBorder: "hover:border-purple-500/40 dark:hover:border-purple-400/40 focus-within:border-purple-500/40 dark:focus-within:border-purple-400/40",
            },
            {
              key: "reasonHash",
              label: t("hashes.reason"),
              roleDesc: t("hashes.reason_desc"),
              hash: data.hashes.reasonHash,
              icon: AlertCircleIcon,
              iconBg: "bg-amber-500/10 dark:bg-amber-500/20",
              iconColor: "text-amber-600 dark:text-amber-400",
              hoverBg: "hover:bg-amber-50/60 dark:hover:bg-amber-950/40",
              hoverBorder: "hover:border-amber-500/40 dark:hover:border-amber-400/40 focus-within:border-amber-500/40 dark:focus-within:border-amber-400/40",
            },
            {
              key: "disputeEvidenceHash",
              label: t("hashes.dispute"),
              roleDesc: t("hashes.dispute_desc"),
              hash: data.hashes.disputeEvidenceHash,
              icon: ShieldLockIcon,
              iconBg: "bg-rose-500/10 dark:bg-rose-500/20",
              iconColor: "text-rose-600 dark:text-rose-400",
              hoverBg: "hover:bg-rose-50/60 dark:hover:bg-rose-950/40",
              hoverBorder: "hover:border-rose-500/40 dark:hover:border-rose-400/40 focus-within:border-rose-500/40 dark:focus-within:border-rose-400/40",
            },
          ].map((item) => {
            const hasHash = Boolean(item.hash);
            const isCopied = copiedKey === item.key;

            return (
              <div
                key={item.key}
                tabIndex={hasHash ? 0 : -1}
                className={`group relative flex items-center gap-2 px-3 py-1.5 rounded-xl border transition-all duration-300 ease-out select-none ${
                  hasHash
                    ? `border-neutral-200/80 dark:border-neutral-800 bg-neutral-50/80 dark:bg-neutral-900/80 ${item.hoverBg} ${item.hoverBorder} hover:shadow-xs dark:hover:shadow-neutral-950/50 cursor-pointer`
                    : "border-neutral-200/50 dark:border-neutral-800/50 bg-neutral-50/40 dark:bg-neutral-900/40 opacity-55 cursor-default"
                }`}
                title={hasHash ? `${item.roleDesc}: ${item.hash}` : `${item.roleDesc}: ${t("hashes.pending")}`}
                onClick={() => {
                  if (hasHash && item.hash) {
                    handleCopy(item.hash, item.key);
                  }
                }}
                onKeyDown={(e) => {
                  if (hasHash && item.hash && (e.key === "Enter" || e.key === " ")) {
                    e.preventDefault();
                    handleCopy(item.hash, item.key);
                  }
                }}
              >
                {/* Hash Category Icon */}
                <div
                  className={`p-1.5 rounded-lg ${item.iconBg} ${item.iconColor} shrink-0 transition-transform ${hasHash ? "group-hover:scale-105 duration-200" : ""}`}
                >
                  <item.icon size={14} />
                </div>

                {/* Minimized Hash Label */}
                <div className="flex items-center gap-1.5 shrink-0">
                  <span className="text-xs font-bold text-neutral-800 dark:text-neutral-100 tracking-tight">
                    {item.label}
                  </span>
                  {hasHash ? (
                    <span className="h-1.5 w-1.5 rounded-full bg-emerald-500 shrink-0" />
                  ) : (
                    <span className="text-[9px] text-neutral-400 dark:text-neutral-500 font-normal">
                      ({t("hashes.pending")})
                    </span>
                  )}
                </div>

                {/* Animated Expandable Hash & Copy Button (Only if hash present) */}
                {hasHash && item.hash && (
                  <div className="max-w-0 opacity-0 overflow-hidden group-hover:max-w-[220px] group-hover:opacity-100 group-focus-within:max-w-[220px] group-focus-within:opacity-100 transition-all duration-300 ease-in-out flex items-center gap-1.5 pl-0 group-hover:pl-2 group-focus-within:pl-2 border-l-0 group-hover:border-l group-focus-within:border-l border-neutral-200 dark:border-neutral-700">
                    {isCopied ? (
                      <span className="text-[10px] font-bold text-emerald-600 dark:text-emerald-400 shrink-0 flex items-center gap-1">
                        <CheckIcon size={12} />
                        {t("common.copied")}
                      </span>
                    ) : (
                      <>
                        <span className="font-mono text-[11px] text-neutral-600 dark:text-neutral-300 font-medium shrink-0">
                          {truncateHash(item.hash, 4, 4)}
                        </span>
                        <button
                          type="button"
                          onClick={(e) => {
                            e.stopPropagation();
                            handleCopy(item.hash!, item.key);
                          }}
                          className="p-1 rounded text-neutral-400 hover:text-neutral-800 dark:text-neutral-400 dark:hover:text-neutral-100 hover:bg-neutral-200/60 dark:hover:bg-neutral-800 transition-colors shrink-0 cursor-pointer"
                          title={t("common.copy_full_hash")}
                          aria-label={t("common.copy_full_hash")}
                        >
                          <CopyIcon size={12} />
                        </button>
                      </>
                    )}
                  </div>
                )}
              </div>
            );
          })}
        </div>
      </div>

      {/* Explorer Link & Audit Note */}
      <div className="pt-2 flex flex-col sm:flex-row sm:items-center justify-between gap-3 text-xs text-neutral-500 font-mono">
        <div>
          {data.contractId && (
            <span className="text-neutral-600 dark:text-neutral-400">
              {t("alerts.contract_prefix", { id: truncateHash(data.contractId, 12, 8) })}
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
            {t("alerts.explorer_link")}
            <ExternalLinkIcon size={13} />
          </a>
        )}
      </div>
    </div>
  );
};
