"use client";

import React from "react";
import { EscrowDetailsData } from "@/types/escrow";
import { PackageIcon } from "@/components/icons";
import { truncateHash, formatCountdown } from "../helpers";

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
  return (
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
  );
};
