"use client";

import React from "react";
import { useWallet } from "@/providers/WalletProvider";

export function NetworkSecurityBanner() {
  const { isMainnetBlocked, network } = useWallet();

  if (!isMainnetBlocked) return null;

  return (
    <div
      role="alert"
      className="bg-red-600 text-white px-4 py-2.5 text-xs font-mono font-medium flex items-center justify-between border-b border-red-700 shadow-md relative z-40"
    >
      <div className="max-w-5xl mx-auto w-full flex items-center justify-between gap-3">
        <div className="flex items-center gap-2">
          <span className="flex h-2 w-2 relative">
            <span className="animate-ping absolute inline-flex h-full w-full rounded-full bg-red-200 opacity-75" />
            <span className="relative inline-flex rounded-full h-2 w-2 bg-white" />
          </span>
          <span className="font-bold tracking-wider">
            [BLOQUEO DE SEGURIDAD] RED DETECTADA: {network}
          </span>
          <span className="hidden sm:inline text-red-100">
            — La firma de contratos en Mainnet está prohibida por especificación P0.
          </span>
        </div>
        <div className="shrink-0 font-bold bg-white/20 px-2 py-0.5 rounded text-[11px]">
          CAMBIA A TESTNET EN FREIGHTER
        </div>
      </div>
    </div>
  );
}
