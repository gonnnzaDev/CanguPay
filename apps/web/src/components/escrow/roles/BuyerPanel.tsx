"use client";

import React from "react";
import { EscrowDetailsData } from "@/types/escrow";
import { UserIcon } from "@/components/icons";
import { truncateHash, formatCountdown } from "../helpers";

export interface BuyerPanelProps {
  data: EscrowDetailsData;
  currentTime: number;
  deadlineDiffSeconds: number;
  fallbackBpsLabel: string;
  onCreateEscrow?: () => void;
}

export const BuyerPanel: React.FC<BuyerPanelProps> = ({
  data,
  currentTime,
  deadlineDiffSeconds,
  fallbackBpsLabel,
  onCreateEscrow,
}) => {
  return (
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
  );
};
