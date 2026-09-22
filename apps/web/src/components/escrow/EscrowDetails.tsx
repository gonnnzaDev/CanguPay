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
  CpuIcon,
  ChevronDownIcon,
  EyeIcon,
  WalletIcon,
} from "@/components/icons";
import { truncateHash, formatCountdown } from "./helpers";
import { useLanguage } from "@/providers/LanguageProvider";
import { useWallet } from "@/providers/WalletProvider";

export { truncateHash, formatCountdown };

/**
 * Maps EscrowStatus to technical status styling.
 */
function getStatusBadgeConfig(
  status: EscrowStatus,
  t: (path: string, params?: Record<string, string | number>) => string
): {
  label: string;
  textColor: string;
  borderColor: string;
  bgColor: string;
} {
  const label = t(`status.${status}`);
  switch (status) {
    case "CREATED":
      return {
        label,
        textColor: "text-amber-700 dark:text-amber-400",
        borderColor: "border-amber-500/30",
        bgColor: "bg-amber-500/10",
      };
    case "FUNDED":
      return {
        label,
        textColor: "text-teal-700 dark:text-teal-400",
        borderColor: "border-teal-500/30",
        bgColor: "bg-teal-500/10",
      };
    case "EVIDENCE_SUBMITTED":
      return {
        label,
        textColor: "text-blue-700 dark:text-blue-400",
        borderColor: "border-blue-500/30",
        bgColor: "bg-blue-500/10",
      };
    case "ATTESTED_PASS":
      return {
        label,
        textColor: "text-emerald-700 dark:text-emerald-400",
        borderColor: "border-emerald-500/30",
        bgColor: "bg-emerald-500/10",
      };
    case "ATTESTED_FAIL":
      return {
        label,
        textColor: "text-rose-700 dark:text-rose-400",
        borderColor: "border-rose-500/30",
        bgColor: "bg-rose-500/10",
      };
    case "DISPUTED":
      return {
        label,
        textColor: "text-purple-700 dark:text-purple-400",
        borderColor: "border-purple-500/30",
        bgColor: "bg-purple-500/10",
      };
    case "RELEASED":
      return {
        label,
        textColor: "text-emerald-700 dark:text-emerald-400",
        borderColor: "border-emerald-500/30",
        bgColor: "bg-emerald-500/10",
      };
    case "REFUNDED":
      return {
        label,
        textColor: "text-neutral-700 dark:text-neutral-300",
        borderColor: "border-neutral-500/30",
        bgColor: "bg-neutral-500/10",
      };
    case "SPLIT":
      return {
        label,
        textColor: "text-cyan-700 dark:text-cyan-400",
        borderColor: "border-cyan-500/30",
        bgColor: "bg-cyan-500/10",
      };
    case "CANCELLED":
      return {
        label,
        textColor: "text-neutral-600 dark:text-neutral-400",
        borderColor: "border-neutral-500/30",
        bgColor: "bg-neutral-500/10",
      };
    default:
      return {
        label,
        textColor: "text-neutral-600 dark:text-neutral-400",
        borderColor: "border-neutral-500/30",
        bgColor: "bg-neutral-500/10",
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

function getActiveTurnLabel(
  activeActor: EscrowActor,
  status: EscrowStatus,
  t: (path: string) => string
): string {
  if (isTerminalStatus(status)) {
    return t("actors.active_turn_action.none");
  }
  switch (activeActor) {
    case "supplier":
      return t("actors.active_turn_action.supplier");
    case "buyer":
      return t("actors.active_turn_action.buyer");
    case "engine":
      return t("actors.active_turn_action.engine");
    case "resolver":
      return t("actors.active_turn_action.resolver");
    default:
      return t("actors.active_turn_action.none");
  }
}

export const EscrowDetails: React.FC<EscrowDetailsProps> = ({
  data,
  isLoading = false,
  error = null,
  onRefresh,
  actionSlot,
  viewerRole,
  canFinalize = false,
  onCreateEscrow,
}) => {
  const { t } = useLanguage();
  const { isConnected, connectFreighter, isConnecting } = useWallet();
  const [copiedKey, setCopiedKey] = useState<string | null>(null);
  const [isTechDetailsOpen, setIsTechDetailsOpen] = useState<boolean>(false);
  const currentTime = useCurrentTimestamp();

  const handleCopy = (text: string, key: string) => {
    if (!text || text === "—") return;
    navigator.clipboard?.writeText(text);
    setCopiedKey(key);
    setTimeout(() => setCopiedKey(null), 1800);
  };

  // 1. Loading State (High-Fidelity Skeleton Matching the 6-Level Hierarchy)
  if (isLoading) {
    return (
      <div className="w-full max-w-5xl mx-auto p-4 sm:p-6 divide-y divide-neutral-200/60 dark:divide-neutral-800/60 animate-pulse font-mono space-y-6">
        {/* Header Skeleton */}
        <div className="flex items-center justify-between pb-4">
          <div className="space-y-2">
            <div className="h-3 w-28 bg-neutral-200 dark:bg-neutral-800 rounded-sm" />
            <div className="h-6 w-48 bg-neutral-200 dark:bg-neutral-800 rounded-md" />
          </div>
          <div className="h-8 w-8 bg-neutral-200 dark:bg-neutral-800 rounded-lg" />
        </div>

        {/* Hero Amount Skeleton */}
        <div className="py-6 space-y-2">
          <div className="h-3 w-36 bg-neutral-200 dark:bg-neutral-800 rounded-sm" />
          <div className="h-12 w-64 bg-neutral-200 dark:bg-neutral-800 rounded-lg" />
          <div className="h-3 w-24 bg-neutral-200 dark:bg-neutral-800 rounded-sm" />
        </div>

        {/* Unified Action Card Skeleton */}
        <div className="py-6">
          <div className="p-5 rounded-xl border border-neutral-200/70 dark:border-neutral-800/70 bg-neutral-50/50 dark:bg-neutral-900/40 space-y-4">
            <div className="flex justify-between items-center">
              <div className="h-6 w-28 bg-neutral-200 dark:bg-neutral-800 rounded-md" />
              <div className="h-6 w-40 bg-neutral-200 dark:bg-neutral-800 rounded-md" />
            </div>
            <div className="h-5 w-72 bg-neutral-200 dark:bg-neutral-800 rounded-md" />
            <div className="h-10 w-full bg-neutral-100 dark:bg-neutral-800/60 rounded-lg" />
          </div>
        </div>

        {/* Rail Stepper Skeleton */}
        <div className="py-6">
          <div className="h-8 w-full bg-neutral-100 dark:bg-neutral-800/60 rounded-lg" />
        </div>

        {/* Metadata Skeleton */}
        <div className="py-6 space-y-3">
          <div className="h-3 w-40 bg-neutral-200 dark:bg-neutral-800 rounded-sm" />
          <div className="flex gap-2">
            {[1, 2, 3, 4].map((i) => (
              <div key={i} className="h-8 w-28 bg-neutral-200 dark:bg-neutral-800 rounded-xl" />
            ))}
          </div>
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
                className="inline-flex items-center gap-2 text-xs font-semibold px-3 py-1.5 rounded bg-neutral-900 text-white dark:bg-neutral-100 dark:text-neutral-900 hover:opacity-90 hover:-translate-y-0.5 active:translate-y-0 active:scale-[0.98] transition-all duration-150 cursor-pointer font-mono focus-visible:ring-2 focus-visible:ring-teal-500/50 focus-visible:outline-none"
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
              className="inline-flex items-center gap-2 text-xs font-semibold px-3.5 py-1.5 rounded-lg bg-teal-600 hover:bg-teal-700 text-white hover:-translate-y-0.5 active:translate-y-0 active:scale-[0.98] transition-all duration-150 shadow-2xs cursor-pointer font-mono focus-visible:ring-2 focus-visible:ring-teal-500/50 focus-visible:outline-none"
            >
              <ShieldLockIcon size={14} />
              {t("alerts.create_new_escrow")}
            </button>
          )}
          {onRefresh && (
            <button
              onClick={onRefresh}
              className="inline-flex items-center gap-2 text-xs font-semibold px-3 py-1.5 rounded-lg border border-neutral-200 dark:border-neutral-700 bg-white dark:bg-neutral-900 text-neutral-800 dark:text-neutral-200 hover:bg-neutral-50 dark:hover:bg-neutral-800 hover:-translate-y-0.5 active:translate-y-0 active:scale-[0.98] transition-all duration-150 shadow-2xs cursor-pointer font-mono focus-visible:ring-2 focus-visible:ring-teal-500/50 focus-visible:outline-none"
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

  // Stepper lifecycle mapping
  const stepLabels = [
    t("lifecycle.step_1.label"),
    t("lifecycle.step_2.label"),
    t("lifecycle.step_3.label"),
    t("lifecycle.step_4.label"),
    t("lifecycle.step_5.label"),
    t("lifecycle.step_6.label"),
  ];

  let activeIndex = 0;
  if (data.status === "CREATED") activeIndex = 0;
  else if (data.status === "FUNDED") activeIndex = 1;
  else if (data.status === "EVIDENCE_SUBMITTED") activeIndex = 2;
  else if (data.status === "ATTESTED_PASS" || data.status === "ATTESTED_FAIL") activeIndex = 3;
  else if (data.status === "DISPUTED") activeIndex = 4;
  else if (isTerminal) activeIndex = 5;

  // Client-safe deadline calculation
  let deadlineDiffSeconds = 0;
  if (data.activeDeadline?.timestamp && currentTime > 0) {
    deadlineDiffSeconds = data.activeDeadline.timestamp - currentTime;
  }
  const isExpired =
    ((deadlineDiffSeconds <= 0 && data.activeDeadline !== undefined) || canFinalize) &&
    !isTerminal;

  const explorerUrl = data.transactionHash
    ? `${data.explorerBaseUrl || "https://stellar.expert/explorer/testnet"}/tx/${data.transactionHash}`
    : data.contractId
      ? `${data.explorerBaseUrl || "https://stellar.expert/explorer/testnet"}/contract/${data.contractId}`
      : null;

  const supplierPct = data.fallbackSplitBps ? data.fallbackSplitBps / 100 : 50;
  const buyerPct = data.fallbackSplitBps ? 100 - data.fallbackSplitBps / 100 : 50;
  const fallbackBpsLabel = `${t("fallback.supplier_share", { pct: supplierPct })} / ${t("fallback.buyer_share", { pct: buyerPct })}`;

  const [intPart, decPart] = data.amount.split(".");

  const participants = [
    {
      key: "buyer",
      label: "Buyer",
      roleDesc: t("participants.buyer_desc"),
      address: data.parties.buyer,
      icon: UserIcon,
      iconBg: "bg-blue-500/10 dark:bg-blue-500/20",
      iconColor: "text-blue-600 dark:text-blue-400",
      hoverBg: "hover:bg-blue-50/60 dark:hover:bg-blue-950/40",
      hoverBorder:
        "hover:border-blue-500/40 dark:hover:border-blue-400/40 focus-within:border-blue-500/40 dark:focus-within:border-blue-400/40",
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
      hoverBorder:
        "hover:border-amber-500/40 dark:hover:border-amber-400/40 focus-within:border-amber-500/40 dark:focus-within:border-amber-400/40",
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
      hoverBorder:
        "hover:border-purple-500/40 dark:hover:border-purple-400/40 focus-within:border-purple-500/40 dark:focus-within:border-purple-400/40",
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
      hoverBorder:
        "hover:border-teal-500/40 dark:hover:border-teal-400/40 focus-within:border-teal-500/40 dark:focus-within:border-teal-400/40",
      isCurrentViewer: viewerRole === "resolver",
    },
  ];

  const hashItems = [
    {
      key: "evidenceHash",
      label: t("hashes.evidence"),
      roleDesc: t("hashes.evidence_desc"),
      hash: data.hashes.evidenceBundleHash,
      icon: FileTextIcon,
      iconBg: "bg-cyan-500/10 dark:bg-cyan-500/20",
      iconColor: "text-cyan-600 dark:text-cyan-400",
      hoverBg: "hover:bg-cyan-50/60 dark:hover:bg-cyan-950/40",
      hoverBorder:
        "hover:border-cyan-500/40 dark:hover:border-cyan-400/40 focus-within:border-cyan-500/40 dark:focus-within:border-cyan-400/40",
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
      hoverBorder:
        "hover:border-purple-500/40 dark:hover:border-purple-400/40 focus-within:border-purple-500/40 dark:focus-within:border-purple-400/40",
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
      hoverBorder:
        "hover:border-amber-500/40 dark:hover:border-amber-400/40 focus-within:border-amber-500/40 dark:focus-within:border-amber-400/40",
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
      hoverBorder:
        "hover:border-rose-500/40 dark:hover:border-rose-400/40 focus-within:border-rose-500/40 dark:focus-within:border-rose-400/40",
    },
  ];

  return (
    <div className="w-full max-w-5xl mx-auto p-4 sm:p-6 divide-y divide-neutral-200/60 dark:divide-neutral-800/60 text-neutral-900 dark:text-neutral-100">
      {/* 1. Header & ID (Level 1) */}
      <div className="flex items-center justify-between gap-4 pb-4">
        <div>
          <div className="flex items-center gap-2 mb-1 font-mono text-[11px]">
            <span className="text-teal-600 dark:text-teal-400 font-semibold flex items-center gap-1.5">
              <ShieldLockIcon size={14} />
              SOROBAN // ESCROW
            </span>
          </div>

          <div className="flex items-center gap-2">
            <h1 className="text-xl sm:text-2xl font-bold tracking-tight font-mono text-neutral-950 dark:text-neutral-50">
              {data.operationId}
            </h1>
            <button
              onClick={() => handleCopy(data.operationId, "operationId")}
              className="p-1 text-neutral-400 hover:text-neutral-700 dark:hover:text-neutral-200 rounded transition-all duration-150 hover:scale-110 active:scale-95 cursor-pointer focus-visible:ring-2 focus-visible:ring-teal-500/50 focus-visible:outline-none"
              title={t("common.copy")}
              aria-label={t("common.copy")}
            >
              {copiedKey === "operationId" ? (
                <CheckIcon size={14} className="text-emerald-500" />
              ) : (
                <CopyIcon size={14} />
              )}
            </button>
          </div>
        </div>

        {onRefresh && (
          <button
            onClick={onRefresh}
            className="p-2 text-neutral-400 hover:text-neutral-700 dark:hover:text-neutral-200 transition-all duration-150 border border-neutral-200 dark:border-neutral-800 rounded-lg hover:bg-neutral-50 dark:hover:bg-neutral-900 cursor-pointer hover:-translate-y-0.5 active:translate-y-0 active:scale-[0.98] focus-visible:ring-2 focus-visible:ring-teal-500/50 focus-visible:outline-none"
            title={t("common.refresh")}
            aria-label={t("common.refresh")}
          >
            <RefreshIcon size={14} />
          </button>
        )}
      </div>

      {/* 2. Hero Amount (Level 2 Focus) */}
      <div className="py-6">
        <span className="text-[11px] font-mono font-semibold tracking-wider text-neutral-400 dark:text-neutral-500 uppercase block mb-1">
          {t("metrics.funds_under_custody")}
        </span>
        <div className="flex items-baseline flex-wrap">
          <span className="text-4xl sm:text-5xl font-extrabold tracking-tight text-neutral-950 dark:text-white font-mono">
            {intPart}
          </span>
          {decPart !== undefined && (
            <span className="text-xl sm:text-2xl font-medium text-neutral-400 dark:text-neutral-500 font-mono">
              .{decPart}
            </span>
          )}
          <span className="text-base sm:text-lg font-bold text-teal-600 dark:text-teal-400 font-mono ml-2">
            {data.asset}
          </span>
        </div>
        <p className="text-xs font-mono text-neutral-400 dark:text-neutral-500 mt-1">
          SAC · Stellar Testnet
        </p>
      </div>

      {/* 3. Unified Status, Active Turn & Action (Level 3 Focus) */}
      <div className="py-6">
        <div className="p-4 sm:p-5 rounded-xl border border-neutral-200/80 dark:border-neutral-800/80 bg-neutral-50/60 dark:bg-neutral-900/50 shadow-2xs font-mono space-y-4">
          {/* Top row: Status Badge & Deadline Countdown */}
          <div className="flex flex-wrap items-center justify-between gap-3">
            <div
              className={`inline-flex items-center gap-1.5 px-3 py-1 rounded-lg border ${statusConfig.borderColor} ${statusConfig.bgColor} ${statusConfig.textColor} font-bold text-xs tracking-wider uppercase shadow-2xs`}
            >
              <span className="h-1.5 w-1.5 rounded-full bg-current" />
              <span>{statusConfig.label}</span>
            </div>

            {/* Single Active Deadline countdown or Expired badge */}
            {data.activeDeadline && !isTerminal && (
              <div>
                {isExpired ? (
                  <div className="inline-flex items-center gap-1.5 px-2.5 py-1 rounded-lg border border-amber-500/40 bg-amber-500/10 text-amber-800 dark:text-amber-300 text-xs font-bold shadow-2xs">
                    <ClockIcon size={14} className="text-amber-500" />
                    <span>{t("alerts.deadline_expired_badge")}</span>
                  </div>
                ) : (
                  <div
                    className="inline-flex items-center gap-1.5 px-2.5 py-1 rounded-lg border border-neutral-200 dark:border-neutral-800 bg-white/80 dark:bg-neutral-800/80 text-neutral-700 dark:text-neutral-300 text-xs font-medium shadow-2xs"
                    suppressHydrationWarning
                  >
                    <ClockIcon size={13} className="text-teal-600 dark:text-teal-400" />
                    <span>
                      {currentTime > 0
                        ? `${formatCountdown(deadlineDiffSeconds)} ${t("metrics.remaining")}`
                        : t("metrics.syncing_clock")}
                    </span>
                  </div>
                )}
              </div>
            )}
          </div>

          {/* Middle row: Active turn & required action */}
          <div className="pt-1">
            <span className="text-[10px] font-bold text-neutral-400 dark:text-neutral-500 uppercase tracking-widest block mb-1">
              {t("metrics.action_turn")}
            </span>
            <p className="text-sm sm:text-base font-semibold text-neutral-900 dark:text-neutral-100 tracking-tight">
              {getActiveTurnLabel(activeActor, data.status, t)}
            </p>
          </div>

          {/* Bottom row: Primary CTA area */}
          <div className="pt-2 border-t border-neutral-200/60 dark:border-neutral-800/60">
            {!isConnected || viewerRole === null ? (
              <div className="flex flex-wrap items-center justify-between gap-3 p-3 rounded-lg border border-dashed border-neutral-300 dark:border-neutral-700 bg-white/60 dark:bg-neutral-900/60 text-xs">
                <span className="text-neutral-600 dark:text-neutral-400 font-medium">
                  {t("roles.disconnected.prompt")}
                </span>
                <button
                  type="button"
                  onClick={connectFreighter}
                  disabled={isConnecting}
                  className="px-3.5 py-1.5 rounded-lg bg-teal-600 hover:bg-teal-700 text-white font-semibold text-xs hover:-translate-y-0.5 active:translate-y-0 active:scale-[0.98] transition-all duration-150 cursor-pointer flex items-center gap-1.5 shadow-2xs focus-visible:ring-2 focus-visible:ring-teal-500/50 focus-visible:outline-none"
                >
                  <WalletIcon size={14} />
                  <span>
                    {isConnecting ? t("wallet.connecting") : t("wallet.connect_wallet")}
                  </span>
                </button>
              </div>
            ) : viewerRole === "observer" ? (
              <div className="flex flex-wrap items-center justify-between gap-3 p-2.5 rounded-lg border border-neutral-200/80 dark:border-neutral-800 bg-neutral-100/70 dark:bg-neutral-800/50 text-xs text-neutral-600 dark:text-neutral-400">
                <div className="flex items-center gap-2">
                  <EyeIcon size={14} className="text-neutral-400" />
                  <span>{t("roles.observer.banner_short")}</span>
                </div>
                {actionSlot}
              </div>
            ) : (
              <div className="flex flex-wrap items-center gap-2">
                {actionSlot}
              </div>
            )}
          </div>
        </div>
      </div>

      {/* 4. Progress Rail Stepper (Level 4 Focus) */}
      <div className="py-6 font-mono">
        <div className="flex items-center justify-between mb-4">
          <span className="text-[10px] font-bold text-neutral-400 dark:text-neutral-500 uppercase tracking-widest flex items-center gap-1.5">
            <ShieldLockIcon size={13} className="text-teal-600 dark:text-teal-400" />
            LIFECYCLE TIMELINE // PROGRESIÓN CONTRACTUAL
          </span>
          <span className="text-[10px] text-neutral-400 uppercase tracking-wider font-semibold">
            {t("lifecycle.step_counter", {
              current: Math.min(6, activeIndex + 1),
              total: 6,
            })}
          </span>
        </div>

        <div className="py-2 px-1 overflow-x-auto">
          <div className="min-w-[500px] sm:min-w-0 relative flex items-start justify-between">
            {/* Background connecting rail line */}
            <div className="absolute top-3.5 left-6 right-6 h-0.5 bg-neutral-200 dark:bg-neutral-800 -z-0" />
            {/* Active connecting rail line */}
            <div
              className="absolute top-3.5 left-6 h-0.5 bg-teal-500 dark:bg-teal-400 -z-0 transition-all duration-500 ease-out"
              style={{
                width: `${Math.min(100, Math.max(0, (activeIndex / 5) * 100))}%`,
                maxWidth: "calc(100% - 3rem)",
              }}
            />

            {/* Nodes */}
            {stepLabels.map((label, idx) => {
              const isCompleted = idx < activeIndex || (idx === 5 && isTerminal);
              const isCurrent =
                (idx === activeIndex && !isTerminal) || (idx === 5 && isTerminal);
              const isAlert =
                (data.status === "ATTESTED_FAIL" && idx === 3) ||
                (data.status === "CANCELLED" && idx === 5);

              return (
                <div
                  key={idx}
                  className="flex flex-col items-center relative z-10 select-none group"
                  style={{ width: "16.666%" }}
                >
                  {/* Circle Node */}
                  <div
                    className={`w-7 h-7 rounded-full flex items-center justify-center transition-all duration-300 ease-out ${
                      isCompleted
                        ? "bg-teal-500 dark:bg-teal-500 text-white shadow-xs"
                        : isCurrent
                          ? isAlert
                            ? "bg-rose-50 dark:bg-rose-950/60 border-2 border-rose-500 text-rose-600 dark:text-rose-400 ring-4 ring-rose-500/20 animate-breath-rose"
                            : "bg-teal-50 dark:bg-teal-950/60 border-2 border-teal-500 text-teal-600 dark:text-teal-400 ring-4 ring-teal-500/20 dark:ring-teal-400/20 animate-breath"
                          : "bg-white dark:bg-neutral-900 border border-neutral-300 dark:border-neutral-700 text-neutral-400 dark:text-neutral-500"
                    }`}
                  >
                    {isCompleted ? (
                      <CheckIcon size={12} className="stroke-[3] transition-all duration-300 ease-out opacity-100 scale-100" />
                    ) : isCurrent ? (
                      <span
                        className={`w-2 h-2 rounded-full ${
                          isAlert ? "bg-rose-500" : "bg-teal-500"
                        } transition-transform duration-300`}
                      />
                    ) : (
                      <span className="w-1.5 h-1.5 rounded-full bg-neutral-300 dark:bg-neutral-600 transition-colors duration-300" />
                    )}
                  </div>

                  {/* Label */}
                  <span
                    className={`mt-2 text-[11px] font-mono tracking-tight text-center truncate max-w-[85px] transition-colors duration-200 ${
                      isCurrent
                        ? isAlert
                          ? "font-bold text-rose-700 dark:text-rose-400"
                          : "font-bold text-teal-700 dark:text-teal-300"
                        : isCompleted
                          ? "font-medium text-neutral-700 dark:text-neutral-300"
                          : "font-normal text-neutral-400 dark:text-neutral-500"
                    }`}
                  >
                    {label}
                  </span>
                </div>
              );
            })}
          </div>
        </div>
      </div>

      {/* 5. Participants & Fallback (Level 5 Focus) */}
      <div className="py-6 font-mono space-y-3.5">
        <div className="flex items-center justify-between">
          <span className="text-[10px] font-bold text-neutral-400 dark:text-neutral-500 uppercase tracking-widest flex items-center gap-1.5">
            <ScaleIcon size={13} className="text-neutral-400" />
            {t("participants.title")}
          </span>
          <span className="text-[10px] text-neutral-400 dark:text-neutral-500 uppercase tracking-wider">
            {t("participants.freighter_accounts")}
          </span>
        </div>

        {/* 4 Expandable Participant Chips */}
        <div className="flex flex-wrap items-center gap-2.5">
          {participants.map((p) => {
            const isCopied = copiedKey === p.key;
            return (
              <div
                key={p.key}
                tabIndex={0}
                className={`group relative flex items-center gap-2 px-3 py-1.5 rounded-xl border border-neutral-200/80 dark:border-neutral-800 bg-neutral-50/80 dark:bg-neutral-900/80 ${p.hoverBg} hover:shadow-xs dark:hover:shadow-neutral-950/50 hover:-translate-y-0.5 active:translate-y-0 active:scale-[0.99] transition-all duration-300 ease-[cubic-bezier(0.16,1,0.3,1)] cursor-pointer select-none ${p.hoverBorder} focus-visible:ring-2 focus-visible:ring-teal-500/50 focus-visible:outline-none`}
                title={`${p.label} (${p.roleDesc}): ${p.address}`}
                onClick={() => handleCopy(p.address, p.key)}
                onKeyDown={(e) => {
                  if (e.key === "Enter" || e.key === " ") {
                    e.preventDefault();
                    handleCopy(p.address, p.key);
                  }
                }}
              >
                <div
                  className={`p-1.5 rounded-lg ${p.iconBg} ${p.iconColor} shrink-0 transition-transform duration-200 group-hover:scale-105`}
                >
                  <p.icon size={14} />
                </div>

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

                {/* Animated Expandable Address & Copy Button */}
                <div className="max-w-0 opacity-0 overflow-hidden group-hover:max-w-[220px] group-hover:opacity-100 group-focus-within:max-w-[220px] group-focus-within:opacity-100 transition-[max-width,padding,opacity] duration-300 ease-[cubic-bezier(0.16,1,0.3,1)] flex items-center gap-1.5 pl-0 group-hover:pl-2 group-focus-within:pl-2 border-l-0 group-hover:border-l group-focus-within:border-l border-neutral-200 dark:border-neutral-700">
                  {isCopied ? (
                    <span className="text-[10px] font-bold text-emerald-600 dark:text-emerald-400 shrink-0 flex items-center gap-1 transition-all duration-200 ease-out animate-in fade-in zoom-in-90">
                      <CheckIcon size={12} className="stroke-[3]" />
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
                        className="p-1 rounded text-neutral-400 hover:text-neutral-800 dark:text-neutral-400 dark:hover:text-neutral-100 hover:bg-neutral-200/60 dark:hover:bg-neutral-800 transition-all duration-150 hover:scale-110 active:scale-95 shrink-0 cursor-pointer focus-visible:ring-2 focus-visible:ring-teal-500/50 focus-visible:outline-none"
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

        {/* Fallback Condition Inline Summary Row */}
        <div className="flex items-center gap-2 text-xs font-mono text-neutral-600 dark:text-neutral-400 pt-1">
          <span className="font-semibold text-neutral-400 dark:text-neutral-500 uppercase tracking-wider text-[10px]">
            Fallback:
          </span>
          <span className="font-bold text-teal-700 dark:text-teal-400">
            {data.fallbackOutcome || "SPLIT"}
          </span>
          <span className="text-neutral-300 dark:text-neutral-700">·</span>
          <span className="text-[11px] text-neutral-600 dark:text-neutral-300">
            {data.fallbackOutcome === "SPLIT"
              ? fallbackBpsLabel
              : data.fallbackOutcome === "RELEASE"
                ? t("fallback.supplier_share", { pct: 100 })
                : t("fallback.buyer_share", { pct: 100 })}
          </span>
        </div>
      </div>

      {/* 6. Technical Details Accordion / Progressive Disclosure (Level 6 Focus) */}
      <div className="pt-4 font-mono">
        <button
          type="button"
          onClick={() => setIsTechDetailsOpen(!isTechDetailsOpen)}
          className="w-full flex items-center justify-between py-2 text-xs text-neutral-500 dark:text-neutral-400 hover:text-neutral-800 dark:hover:text-neutral-200 transition-colors cursor-pointer group rounded-lg focus-visible:ring-2 focus-visible:ring-teal-500/50 focus-visible:outline-none"
          aria-expanded={isTechDetailsOpen}
        >
          <div className="flex items-center gap-2">
            <HashIcon
              size={14}
              className="text-neutral-400 group-hover:text-teal-600 dark:group-hover:text-teal-400 transition-colors"
            />
            <span className="font-bold uppercase tracking-wider text-[11px] text-neutral-700 dark:text-neutral-300">
              {t("technical_details.title")}
            </span>
            <span className="text-[10px] text-neutral-400 dark:text-neutral-500">
              ({t("technical_details.subtitle")})
            </span>
          </div>
          <div className="flex items-center gap-1.5 text-[11px]">
            <span className="text-neutral-400 group-hover:text-neutral-600 dark:group-hover:text-neutral-300 transition-colors">
              {isTechDetailsOpen ? t("common.collapse") : t("common.expand")}
            </span>
            <ChevronDownIcon
              size={14}
              className={`transition-transform duration-300 ease-[cubic-bezier(0.16,1,0.3,1)] text-neutral-400 group-hover:text-neutral-600 dark:group-hover:text-neutral-300 ${
                isTechDetailsOpen ? "rotate-180" : ""
              }`}
            />
          </div>
        </button>

        <div
          className={`grid transition-[grid-template-rows,opacity] duration-300 ease-out ${
            isTechDetailsOpen
              ? "grid-rows-[1fr] opacity-100 mt-2"
              : "grid-rows-[0fr] opacity-0 pointer-events-none"
          }`}
          aria-hidden={!isTechDetailsOpen}
        >
          <div className="overflow-hidden space-y-4 pt-1">
            {/* The 4 Cryptographic Hash Chips */}
            <div>
              <span className="text-[10px] font-bold text-neutral-400 dark:text-neutral-500 uppercase tracking-widest block mb-2">
                {t("hashes.title")} · {t("hashes.bytes_label")}
              </span>
              <div className="flex flex-wrap items-center gap-2.5">
                {hashItems.map((item) => {
                  const hasHash = Boolean(item.hash);
                  const isCopied = copiedKey === item.key;

                  return (
                    <div
                      key={item.key}
                      tabIndex={hasHash ? 0 : -1}
                      className={`group relative flex items-center gap-2 px-3 py-1.5 rounded-xl border transition-all duration-300 ease-[cubic-bezier(0.16,1,0.3,1)] select-none ${
                        hasHash
                          ? `border-neutral-200/80 dark:border-neutral-800 bg-neutral-50/80 dark:bg-neutral-900/80 ${item.hoverBg} ${item.hoverBorder} hover:shadow-xs dark:hover:shadow-neutral-950/50 hover:-translate-y-0.5 active:translate-y-0 active:scale-[0.99] cursor-pointer focus-visible:ring-2 focus-visible:ring-teal-500/50 focus-visible:outline-none`
                          : "border-neutral-200/50 dark:border-neutral-800/50 bg-neutral-50/40 dark:bg-neutral-900/40 opacity-55 cursor-default"
                      }`}
                      title={
                        hasHash
                          ? `${item.roleDesc}: ${item.hash}`
                          : `${item.roleDesc}: ${t("hashes.pending")}`
                      }
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
                      <div
                        className={`p-1.5 rounded-lg ${item.iconBg} ${item.iconColor} shrink-0 transition-transform duration-200 ${hasHash ? "group-hover:scale-105" : ""}`}
                      >
                        <item.icon size={14} />
                      </div>

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

                      {/* Expandable Hash & Copy Button */}
                      {hasHash && item.hash && (
                        <div className="max-w-0 opacity-0 overflow-hidden group-hover:max-w-[220px] group-hover:opacity-100 group-focus-within:max-w-[220px] group-focus-within:opacity-100 transition-[max-width,padding,opacity] duration-300 ease-[cubic-bezier(0.16,1,0.3,1)] flex items-center gap-1.5 pl-0 group-hover:pl-2 group-focus-within:pl-2 border-l-0 group-hover:border-l group-focus-within:border-l border-neutral-200 dark:border-neutral-700">
                          {isCopied ? (
                            <span className="text-[10px] font-bold text-emerald-600 dark:text-emerald-400 shrink-0 flex items-center gap-1 transition-all duration-200 ease-out animate-in fade-in zoom-in-90">
                              <CheckIcon size={12} className="stroke-[3]" />
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
                                className="p-1 rounded text-neutral-400 hover:text-neutral-800 dark:text-neutral-400 dark:hover:text-neutral-100 hover:bg-neutral-200/60 dark:hover:bg-neutral-800 transition-all duration-150 hover:scale-110 active:scale-95 shrink-0 cursor-pointer focus-visible:ring-2 focus-visible:ring-teal-500/50 focus-visible:outline-none"
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

            {/* Contract ID, Explorer link & Ledger Timestamp */}
            <div className="flex flex-wrap items-center justify-between gap-3 pt-3 border-t border-neutral-100 dark:border-neutral-800/80 text-xs text-neutral-500">
              <div className="flex items-center gap-3 flex-wrap">
                {data.contractId && (
                  <div className="flex items-center gap-1.5">
                    <span className="text-neutral-400 dark:text-neutral-500 text-[11px]">
                      {t("technical_details.contract_id")}:
                    </span>
                    <span className="font-mono text-neutral-700 dark:text-neutral-300 text-[11px] font-medium">
                      {truncateHash(data.contractId, 10, 8)}
                    </span>
                    <button
                      type="button"
                      onClick={() => handleCopy(data.contractId!, "contractId")}
                      className="p-1 text-neutral-400 hover:text-neutral-700 dark:hover:text-neutral-200 rounded transition-all duration-150 hover:scale-110 active:scale-95 cursor-pointer focus-visible:ring-2 focus-visible:ring-teal-500/50 focus-visible:outline-none"
                      title={t("common.copy")}
                    >
                      {copiedKey === "contractId" ? (
                        <CheckIcon size={12} className="text-emerald-500" />
                      ) : (
                        <CopyIcon size={12} />
                      )}
                    </button>
                  </div>
                )}

                {data.updatedAtLedger && (
                  <div className="flex items-center gap-1.5 border-l border-neutral-200 dark:border-neutral-800 pl-3">
                    <span className="text-neutral-400 dark:text-neutral-500 text-[11px]">
                      {t("technical_details.ledger_timestamp")}:
                    </span>
                    <span className="font-mono text-neutral-700 dark:text-neutral-300 text-[11px]">
                      {data.updatedAtLedger}
                    </span>
                  </div>
                )}
              </div>

              {explorerUrl && (
                <a
                  href={explorerUrl}
                  target="_blank"
                  rel="noopener noreferrer"
                  className="inline-flex items-center gap-1.5 text-teal-600 dark:text-teal-400 hover:underline hover:-translate-y-0.5 transition-all duration-150 font-semibold text-xs focus-visible:ring-2 focus-visible:ring-teal-500/50 focus-visible:outline-none rounded"
                >
                  <span>{t("alerts.explorer_link")}</span>
                  <ExternalLinkIcon size={13} />
                </a>
              )}
            </div>
          </div>
        </div>
      </div>
    </div>
  );
};
