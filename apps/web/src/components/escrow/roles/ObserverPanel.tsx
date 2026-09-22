"use client";

import React from "react";
import { EyeIcon } from "@/components/icons";

export const ObserverPanel: React.FC = () => {
  return (
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
  );
};
