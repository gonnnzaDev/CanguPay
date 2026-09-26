"use client";

import React from "react";
import { EscrowDetailsData, isTerminalStatus } from "@/types/escrow";
import { PackageIcon } from "@/components/icons";
import { truncateHash, formatCountdown } from "../helpers";
import { useLanguage } from "@/providers/LanguageProvider";

export interface SupplierPanelProps {
  data: EscrowDetailsData;
  currentTime: number;
  deadlineDiffSeconds: number;
}

export const SupplierPanel: React.FC<SupplierPanelProps> = ({
  data,
  currentTime,
  deadlineDiffSeconds,
}) => {
  const { t } = useLanguage();
  const isTerminal = isTerminalStatus(data.status);

  return (
    <div className="p-5 rounded-xl border border-indigo-500/30 dark:border-indigo-500/30 bg-indigo-500/[0.04] dark:bg-indigo-950/25 shadow-xs font-mono space-y-4">
      <div className="flex items-center pb-3 border-b border-indigo-500/20 dark:border-indigo-500/25">
        <div className="flex items-center gap-2">
          <div className="p-1.5 rounded-lg bg-indigo-500/10 dark:bg-indigo-500/20 text-indigo-600 dark:text-indigo-400">
            <PackageIcon size={16} />
          </div>
          <div>
            <h3 className="text-xs font-bold uppercase tracking-wider text-indigo-950 dark:text-indigo-200">
              {t("roles.supplier.title")}
            </h3>
          </div>
        </div>
      </div>

      <div className="grid grid-cols-1 sm:grid-cols-2 gap-3 text-xs">
        {/* Contract amount is not evidence of token custody. */}
        <div className="p-3 rounded-lg border border-indigo-500/20 dark:border-indigo-500/30 bg-white/80 dark:bg-neutral-950/70">
          <span className="text-[10px] uppercase text-neutral-500 dark:text-neutral-400 font-semibold block mb-1">
            {t(isTerminal ? "roles.supplier.historical_amount" : data.status === "CREATED" ? "roles.supplier.deposit_status" : "roles.supplier.reserved_funds")}
          </span>
          <span className="text-xs font-medium text-neutral-800 dark:text-neutral-200 block">
            {isTerminal
              ? t(`roles.supplier.terminal.${data.status}`)
              : data.status === "CREATED"
                ? t("roles.supplier.funds_pending")
                : t("roles.supplier.contract_amount_above")}
          </span>
          <span className="text-[10px] text-neutral-600 dark:text-neutral-300 block mt-1">
            {t(data.source === "onchain" ? "roles.supplier.balance_unverified" : "roles.supplier.preview_unverified")}
          </span>
        </div>

        <div className="p-3 rounded-lg border border-indigo-500/20 dark:border-indigo-500/30 bg-white/80 dark:bg-neutral-950/70">
          <span className="text-[10px] uppercase text-neutral-500 dark:text-neutral-400 font-semibold block mb-1">
            {t("roles.supplier.conditions")}
          </span>
          <span className="text-xs font-medium text-neutral-800 dark:text-neutral-200 block">
            {data.fallbackOutcome ? t(`fallback.outcomes.${data.fallbackOutcome}`) : t("alerts.unavailable")}
          </span>
          {data.status === "ATTESTED_FAIL" && (
            <span className="text-[10px] text-neutral-600 dark:text-neutral-300 block mt-1">{t("roles.supplier.correction_limit")}</span>
          )}
        </div>

        {/* Delivery or correction deadline */}
        <div className="p-3 rounded-lg border border-indigo-500/20 dark:border-indigo-500/30 bg-white/80 dark:bg-neutral-950/70">
          <span className="text-[10px] uppercase text-neutral-500 dark:text-neutral-400 font-semibold block mb-1">
            {t("roles.supplier.delivery_deadline")}
          </span>
          <span className="text-xs font-medium text-amber-700 dark:text-amber-400 block truncate" suppressHydrationWarning>
            {data.activeDeadline && currentTime > 0
              ? deadlineDiffSeconds <= 0
                ? t(data.source === "onchain" ? "metrics.deadline_passed_local" : "metrics.deadline_passed_preview")
                : t("metrics.time_remaining", { time: formatCountdown(deadlineDiffSeconds) })
              : data.source === "onchain" ? t("alerts.unavailable") : t("roles.supplier.no_active_deadline")}
          </span>
          <span className="text-[10px] text-neutral-400 dark:text-neutral-500 block mt-0.5">
            {data.activeDeadline?.label || t(data.source === "onchain" ? "alerts.unavailable" : "roles.supplier.no_active_deadline")}
          </span>
        </div>

        {/* Submitted evidence */}
        <div className="p-3 rounded-lg border border-indigo-500/20 dark:border-indigo-500/30 bg-white/80 dark:bg-neutral-950/70">
          <span className="text-[10px] uppercase text-neutral-500 dark:text-neutral-400 font-semibold block mb-1">
            {t("roles.supplier.evidence_bundle")}
          </span>
          <span className="text-xs font-mono font-medium text-neutral-800 dark:text-neutral-200 block truncate">
            {data.hashes.evidenceBundleHash
              ? truncateHash(data.hashes.evidenceBundleHash, 8, 6)
              : t(data.source === "onchain" ? "alerts.unavailable" : "roles.supplier.evidence_pending")}
          </span>
          <span className="text-[10px] text-neutral-400 dark:text-neutral-500 block mt-0.5">
            {data.hashes.evidenceBundleHash ? t("roles.supplier.evidence_computed") : data.source === "onchain" ? t("alerts.unavailable") : t("roles.supplier.evidence_requires")}
          </span>
        </div>
      </div>
    </div>
  );
};
