"use client";

import React from "react";
import { EscrowDetailsData } from "@/types/escrow";
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

  return (
    <div className="p-5 rounded-xl border border-indigo-500/30 dark:border-indigo-500/30 bg-indigo-500/[0.04] dark:bg-indigo-950/25 shadow-xs font-mono space-y-4">
      <div className="flex items-center justify-between pb-3 border-b border-indigo-500/20 dark:border-indigo-500/25">
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
        <span className="text-[10px] font-bold px-2 py-0.5 rounded bg-indigo-500/20 dark:bg-indigo-500/30 text-indigo-800 dark:text-indigo-300 uppercase">
          {t("roles.supplier.badge")}
        </span>
      </div>

      <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-3 text-xs">
        {/* Metric A: Fondos Reservados */}
        <div className="p-3 rounded-lg border border-indigo-500/20 dark:border-indigo-500/30 bg-white/80 dark:bg-neutral-950/70">
          <span className="text-[10px] uppercase text-neutral-500 dark:text-neutral-400 font-semibold block mb-1">
            {t("roles.supplier.reserved_funds")}
          </span>
          <span className="text-sm font-bold text-neutral-950 dark:text-neutral-50">
            {data.amount} <span className="text-indigo-600 dark:text-indigo-400">{data.asset}</span>
          </span>
          <span className="text-[10px] text-emerald-600 dark:text-emerald-400 block mt-0.5">
            {data.status === "CREATED" ? t("roles.supplier.funds_pending") : t("roles.supplier.funds_guaranteed")}
          </span>
        </div>

        {/* Metric B: Plazo de Entrega / Corrección */}
        <div className="p-3 rounded-lg border border-indigo-500/20 dark:border-indigo-500/30 bg-white/80 dark:bg-neutral-950/70">
          <span className="text-[10px] uppercase text-neutral-500 dark:text-neutral-400 font-semibold block mb-1">
            {t("roles.supplier.delivery_deadline")}
          </span>
          <span className="text-xs font-medium text-amber-700 dark:text-amber-400 block truncate" suppressHydrationWarning>
            {data.activeDeadline && currentTime > 0
              ? formatCountdown(deadlineDiffSeconds)
              : t("roles.supplier.no_active_deadline")}
          </span>
          <span className="text-[10px] text-neutral-400 dark:text-neutral-500 block mt-0.5">
            {data.activeDeadline?.label || t("roles.supplier.no_active_deadline")}
          </span>
        </div>

        {/* Metric C: Hashes de Evidencia */}
        <div className="p-3 rounded-lg border border-indigo-500/20 dark:border-indigo-500/30 bg-white/80 dark:bg-neutral-950/70">
          <span className="text-[10px] uppercase text-neutral-500 dark:text-neutral-400 font-semibold block mb-1">
            {t("roles.supplier.evidence_bundle")}
          </span>
          <span className="text-xs font-mono font-medium text-neutral-800 dark:text-neutral-200 block truncate">
            {data.hashes.evidenceBundleHash
              ? truncateHash(data.hashes.evidenceBundleHash, 8, 6)
              : t("roles.supplier.evidence_pending")}
          </span>
          <span className="text-[10px] text-neutral-400 dark:text-neutral-500 block mt-0.5">
            {data.hashes.evidenceBundleHash ? t("roles.supplier.evidence_computed") : t("roles.supplier.evidence_requires")}
          </span>
        </div>
      </div>
    </div>
  );
};
