"use client";

import React from "react";
import { EscrowDetailsData } from "@/types/escrow";
import { ScaleIcon } from "@/components/icons";
import { truncateHash } from "../helpers";
import { useLanguage } from "@/providers/LanguageProvider";

export interface ResolverPanelProps {
  data: EscrowDetailsData;
}

export const ResolverPanel: React.FC<ResolverPanelProps> = ({ data }) => {
  const { t } = useLanguage();

  return (
    <div className="p-5 rounded-xl border border-purple-500/30 dark:border-purple-500/30 bg-purple-500/[0.04] dark:bg-purple-950/25 shadow-xs font-mono space-y-4">
      <div className="flex items-center justify-between pb-3 border-b border-purple-500/20 dark:border-purple-500/25">
        <div className="flex items-center gap-2">
          <div className="p-1.5 rounded-lg bg-purple-500/10 dark:bg-purple-500/20 text-purple-600 dark:text-purple-400">
            <ScaleIcon size={16} />
          </div>
          <div>
            <h3 className="text-xs font-bold uppercase tracking-wider text-purple-950 dark:text-purple-200">
              {t("roles.resolver.title")}
            </h3>
          </div>
        </div>
        <span className="text-[10px] font-bold px-2 py-0.5 rounded bg-purple-500/20 dark:bg-purple-500/30 text-purple-800 dark:text-purple-300 uppercase">
          {t("roles.resolver.badge")}
        </span>
      </div>

      <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-3 text-xs">
        {/* Metric A: Operación & Monto bajo Arbitraje */}
        <div className="p-3 rounded-lg border border-purple-500/20 dark:border-purple-500/30 bg-white/80 dark:bg-neutral-950/70">
          <span className="text-[10px] uppercase text-neutral-500 dark:text-neutral-400 font-semibold block mb-1">
            {t("roles.resolver.disputed_funds")}
          </span>
          <span className="text-sm font-bold text-neutral-950 dark:text-neutral-50">
            {data.amount} <span className="text-purple-600 dark:text-purple-400">{data.asset}</span>
          </span>
          <span className="text-[10px] text-neutral-400 dark:text-neutral-500 block mt-0.5">
            {t("roles.resolver.op_id", { id: data.operationId })}
          </span>
        </div>

        {/* Metric B: Dictamen Previo del Motor */}
        <div className="p-3 rounded-lg border border-purple-500/20 dark:border-purple-500/30 bg-white/80 dark:bg-neutral-950/70">
          <span className="text-[10px] uppercase text-neutral-500 dark:text-neutral-400 font-semibold block mb-1">
            {t("roles.resolver.engine_ruling")}
          </span>
          <span className="text-xs font-mono font-medium text-neutral-800 dark:text-neutral-200 block truncate">
            {data.hashes.reportHash ? truncateHash(data.hashes.reportHash, 8, 6) : t("roles.resolver.no_ruling")}
          </span>
          <span className="text-[10px] text-neutral-400 dark:text-neutral-500 block mt-0.5">
            {t("hashes.report_desc")}
          </span>
        </div>

        {/* Metric C: Motivo y Pruebas de Disputa */}
        <div className="p-3 rounded-lg border border-purple-500/20 dark:border-purple-500/30 bg-white/80 dark:bg-neutral-950/70">
          <span className="text-[10px] uppercase text-neutral-500 dark:text-neutral-400 font-semibold block mb-1">
            {t("roles.resolver.dispute_hashes")}
          </span>
          <div className="space-y-0.5 text-[10px]">
            <div className="truncate">
              <span className="text-neutral-400 dark:text-neutral-500">{t("roles.resolver.reason_label")}</span>{" "}
              {data.hashes.reasonHash ? truncateHash(data.hashes.reasonHash, 6, 4) : "—"}
            </div>
            <div className="truncate">
              <span className="text-neutral-400 dark:text-neutral-500">{t("roles.resolver.evidence_label")}</span>{" "}
              {data.hashes.disputeEvidenceHash ? truncateHash(data.hashes.disputeEvidenceHash, 6, 4) : "—"}
            </div>
          </div>
        </div>
      </div>
    </div>
  );
};
