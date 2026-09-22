"use client";

import React, { useState } from "react";
import { useWallet } from "@/providers/WalletProvider";
import { CopyIcon, CheckIcon, LogoutDoorIcon, WalletIcon } from "@/components/icons";

export function ProfileSelector() {
  const {
    activeProfile,
    isConnected,
    isConnecting,
    isFreighterInstalled,
    connectFreighter,
    disconnectFreighter,
    isMainnetBlocked,
  } = useWallet();

  const [copied, setCopied] = useState(false);

  const handleCopy = (addr: string) => {
    if (navigator.clipboard) {
      navigator.clipboard.writeText(addr);
      setCopied(true);
      setTimeout(() => setCopied(false), 1500);
    }
  };

  const formatAddress = (addr: string) => {
    if (!addr || addr.length < 12) return addr;
    return `${addr.slice(0, 4)}...${addr.slice(-4)}`;
  };

  return (
    <div className="flex items-center gap-2 font-mono">
      {/* Asset Balance Pill */}
      <div className="hidden lg:flex items-center gap-1.5 px-2.5 py-1 rounded-lg border border-neutral-200/80 dark:border-neutral-800/80 bg-neutral-100/70 dark:bg-neutral-900/70 text-xs shadow-2xs">
        <span className="text-neutral-400 dark:text-neutral-500 text-[10px]">
          SALDO:
        </span>
        <span className="font-semibold text-neutral-800 dark:text-neutral-200">
          {activeProfile.balance.split(".")[0]}
        </span>
        <span className="text-[10px] font-bold text-teal-600 dark:text-teal-400">
          {activeProfile.assetCode}
        </span>
      </div>

      {/* Active Identity Pill (No redundant dropdown) */}
      <div className="flex items-center gap-1.5 px-2.5 py-1 rounded-lg border border-neutral-200 dark:border-neutral-800 bg-white/90 dark:bg-neutral-900/90 text-xs shadow-2xs">
        {/* Status Dot */}
        <span
          className={`h-2 w-2 rounded-full ${
            isMainnetBlocked
              ? "bg-red-500 animate-pulse"
              : isConnected
                ? "bg-teal-500"
                : "bg-amber-400"
          }`}
          title={
            isMainnetBlocked
              ? "Mainnet detectado (bloqueado)"
              : isConnected
                ? "Wallet conectada"
                : "Perfil institucional activo"
          }
        />

        {/* Wallet Icon */}
        <WalletIcon size={13} className="text-neutral-500 dark:text-neutral-400" />

        {/* Address */}
        <span className="text-neutral-600 dark:text-neutral-300 font-medium text-[11px]">
          {formatAddress(activeProfile.address)}
        </span>

        {/* Copy Icon Button (1 icon, no text) */}
        <button
          type="button"
          onClick={() => handleCopy(activeProfile.address)}
          className="p-1 rounded text-neutral-400 hover:text-teal-600 dark:hover:text-teal-400 hover:bg-neutral-100 dark:hover:bg-neutral-800 transition-colors cursor-pointer"
          title="Copiar dirección pública"
          aria-label="Copiar dirección pública"
        >
          {copied ? (
            <CheckIcon size={12} className="text-emerald-500" />
          ) : (
            <CopyIcon size={12} />
          )}
        </button>
      </div>

      {/* Direct Freighter Connect / Disconnect Action */}
      {isConnected ? (
        <button
          type="button"
          onClick={(e) => {
            e.preventDefault();
            e.stopPropagation();
            disconnectFreighter();
          }}
          className="group inline-flex items-center justify-center p-1.5 rounded-lg border border-red-200/80 dark:border-red-900/50 bg-red-50/50 dark:bg-red-950/30 text-red-600 dark:text-red-400 hover:bg-red-100 dark:hover:bg-red-900/40 hover:text-red-700 dark:hover:text-red-300 transition-all cursor-pointer shadow-2xs"
          title="Desconectar wallet Freighter"
          aria-label="Desconectar wallet Freighter"
        >
          <LogoutDoorIcon size={15} />
        </button>
      ) : (
        <button
          type="button"
          disabled={isConnecting}
          onClick={(e) => {
            e.preventDefault();
            e.stopPropagation();
            connectFreighter();
          }}
          className="inline-flex items-center px-2.5 py-1 rounded-lg border border-teal-600/80 dark:border-teal-500/80 bg-teal-600 hover:bg-teal-700 text-white text-[11px] font-semibold transition-colors cursor-pointer disabled:opacity-50 shadow-2xs"
          title={isFreighterInstalled ? "Conectar wallet Freighter" : "Instalar Freighter"}
        >
          {isConnecting
            ? "Conectando..."
            : isFreighterInstalled
              ? "Conectar Wallet"
              : "Instalar Freighter"}
        </button>
      )}
    </div>
  );
}
