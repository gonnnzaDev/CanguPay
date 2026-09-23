"use client";

import React from "react";
import { EscrowDetailsData } from "@/types/escrow";
import { UserIcon } from "@/components/icons";
import { truncateHash, formatCountdown } from "../helpers";
import { useLanguage } from "@/providers/LanguageProvider";

export interface BuyerPanelProps {
  data: EscrowDetailsData;
  currentTime: number;
  deadlineDiffSeconds: number;
  fallbackBpsLabel: string;
}

export const BuyerPanel: React.FC<BuyerPanelProps> = ({
  data,
  currentTime,
  deadlineDiffSeconds,
  fallbackBpsLabel,
}) => {
  const { t } = useLanguage();

  return (
    <div className="p-5 rounded-xl border border-teal-500/30 dark:border-teal-500/30 bg-teal-500/[0.04] dark:bg-teal-950/25 shadow-xs font-mono space-y-4">
      <div className="flex items-center pb-3 border-b border-teal-500/20 dark:border-teal-500/25">
        <div className="flex items-center gap-2">
          <div className="p-1.5 rounded-lg bg-teal-500/10 dark:bg-teal-500/20 text-teal-600 dark:text-teal-400">
            <UserIcon size={16} />
          </div>
          <div>
            <h3 className="text-xs font-bold uppercase tracking-wider text-teal-950 dark:text-teal-200">
              {t("roles.buyer.title")}
            </h3>
          </div>
        </div>
      </div>

      <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-3 text-xs">
        {/* Designated supplier */}
        <div className="p-3 rounded-lg border border-teal-500/20 dark:border-teal-500/30 bg-white/80 dark:bg-neutral-950/70">
          <span className="text-[10px] uppercase text-neutral-500 dark:text-neutral-400 font-semibold block mb-1">
            {t("roles.buyer.designated_supplier")}
          </span>
          <span className="text-xs font-medium text-neutral-800 dark:text-neutral-200 block truncate">
            {data.parties.supplier ? truncateHash(data.parties.supplier, 6, 6) : t("alerts.unavailable")}
          </span>
        </div>

        {/* Active deadline */}
        <div className="p-3 rounded-lg border border-teal-500/20 dark:border-teal-500/30 bg-white/80 dark:bg-neutral-950/70">
          <span className="text-[10px] uppercase text-neutral-500 dark:text-neutral-400 font-semibold block mb-1">
            {t("roles.buyer.contractual_deadline")}
          </span>
          <span className="text-xs font-medium text-amber-700 dark:text-amber-400 block truncate" suppressHydrationWarning>
            {data.activeDeadline && currentTime > 0
              ? deadlineDiffSeconds <= 0
                ? t(data.source === "onchain" ? "metrics.deadline_passed_local" : "metrics.deadline_passed_preview")
                : t("metrics.time_remaining", { time: formatCountdown(deadlineDiffSeconds) })
              : data.source === "onchain" ? t("alerts.unavailable") : t("roles.buyer.no_pending_deadline")}
          </span>
        </div>

        {/* Agreed fallback */}
        <div className="p-3 rounded-lg border border-teal-500/20 dark:border-teal-500/30 bg-white/80 dark:bg-neutral-950/70">
          <span className="text-[10px] uppercase text-neutral-500 dark:text-neutral-400 font-semibold block mb-1">
            {t("roles.buyer.fallback_rule")}
          </span>
          <span className="text-xs font-bold text-neutral-900 dark:text-neutral-100 block truncate">
            {data.fallbackOutcome ? t(`fallback.outcomes.${data.fallbackOutcome}`) : t("alerts.unavailable")}
            {data.fallbackOutcome === "SPLIT" && ` (${fallbackBpsLabel})`}
          </span>
        </div>
      </div>
    </div>
  );
};
