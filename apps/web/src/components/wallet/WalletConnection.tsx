"use client";

import React, { useState, useEffect } from "react";
import { useWallet } from "@/providers/WalletProvider";
import { useLanguage } from "@/providers/LanguageProvider";
import {
  CopyIcon,
  CheckIcon,
  LogoutDoorIcon,
  WalletIcon,
} from "@/components/icons";

/**
 * Clean wallet connection component.
 * Displays real Freighter address when connected, or connection triggers when disconnected.
 * When connected and on Testnet, fetches native XLM balance via Horizon Testnet client-side.
 * No fake profiles, no fake balance, no manual role switching.
 */
export function WalletConnection() {
  const { t } = useLanguage();
  const {
    address,
    isConnected,
    isConnecting,
    isFreighterInstalled,
    connectFreighter,
    disconnectFreighter,
    isSigningBlocked,
    isExactTestnet,
  } = useWallet();

  const [copied, setCopied] = useState(false);
  const [balance, setBalance] = useState<string | null>(null);
  const [isBalanceLoading, setIsBalanceLoading] = useState<boolean>(false);
  const [balanceError, setBalanceError] = useState<boolean>(false);

  useEffect(() => {
    if (!isConnected || !address || !isExactTestnet) {
      return;
    }

    let isMounted = true;
    const controller = new AbortController();

    async function fetchHorizonBalance() {
      setIsBalanceLoading(true);
      setBalanceError(false);
      try {
        const res = await fetch(`https://horizon-testnet.stellar.org/accounts/${address}`, {
          signal: controller.signal,
        });

        if (!isMounted) return;

        if (res.status === 404) {
          // Account exists in wallet but not funded on Testnet ledger yet
          setBalance("0.00 XLM");
          setIsBalanceLoading(false);
          return;
        }

        if (!res.ok) {
          setBalanceError(true);
          setIsBalanceLoading(false);
          return;
        }

        const data = await res.json();
        const nativeBalance = data.balances?.find(
          (b: { asset_type: string; balance: string }) => b.asset_type === "native"
        );

        if (nativeBalance && nativeBalance.balance) {
          const num = parseFloat(nativeBalance.balance);
          const formatted = num.toLocaleString("en-US", {
            minimumFractionDigits: 2,
            maximumFractionDigits: 2,
          });
          setBalance(`${formatted} XLM`);
        } else {
          setBalance("0.00 XLM");
        }
      } catch (err: unknown) {
        if ((err as Error)?.name !== "AbortError") {
          setBalanceError(true);
        }
      } finally {
        if (isMounted) {
          setIsBalanceLoading(false);
        }
      }
    }

    fetchHorizonBalance();

    return () => {
      isMounted = false;
      controller.abort();
    };
  }, [isConnected, address, isExactTestnet]);

  const displayBalance = !isConnected || !address || !isExactTestnet ? null : balance;
  const displayLoading = !isConnected || !address || !isExactTestnet ? false : isBalanceLoading;
  const displayError = !isConnected || !address || !isExactTestnet ? false : balanceError;

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

  if (!isConnected || !address) {
    return (
      <div className="flex items-center gap-2 font-mono">
        <div className="hidden sm:flex items-center gap-1.5 px-2.5 py-1 rounded-lg border border-neutral-200 dark:border-neutral-800 bg-neutral-100/70 dark:bg-neutral-900/70 text-neutral-500 dark:text-neutral-400 text-xs">
          <span className="h-1.5 w-1.5 rounded-full bg-neutral-400" />
          <span className="text-[11px]">{t("wallet.not_connected")}</span>
        </div>

        <button
          type="button"
          disabled={isConnecting}
          onClick={(e) => {
            e.preventDefault();
            connectFreighter();
          }}
          className="inline-flex items-center px-2.5 py-1 rounded-lg border border-teal-600/80 dark:border-teal-500/80 bg-teal-600 hover:bg-teal-700 text-white text-[11px] font-semibold font-mono hover:-translate-y-0.5 active:translate-y-0 active:scale-[0.98] transition-all duration-150 cursor-pointer disabled:opacity-50 shadow-2xs focus-visible:ring-2 focus-visible:ring-teal-500/50 focus-visible:outline-none"
          title={isFreighterInstalled ? t("wallet.connect_wallet") : t("wallet.install_freighter")}
        >
          {isConnecting
            ? t("wallet.connecting")
            : isFreighterInstalled
              ? t("wallet.connect_wallet")
              : t("wallet.install_freighter")}
        </button>
      </div>
    );
  }

  return (
    <div className="flex items-center gap-2 font-mono">
      {/* Native XLM Balance Pill */}
      <div
        className="hidden md:flex items-center gap-1.5 px-2.5 py-1 rounded-lg border border-teal-500/30 bg-teal-500/[0.05] dark:bg-teal-500/[0.08] text-xs shadow-2xs"
        title="Balance de XLM nativo en Stellar Testnet"
      >
        <span className="text-[10px] uppercase font-bold text-teal-700/80 dark:text-teal-400/80">
          {t("wallet.native_xlm")}:
        </span>
        <span className="text-teal-800 dark:text-teal-200 font-semibold text-[11px]">
          {displayLoading ? (
            <span className="animate-pulse">...</span>
          ) : displayError ? (
            <span className="text-neutral-400">n/d</span>
          ) : (
            displayBalance || "0.00 XLM"
          )}
        </span>
      </div>

      {/* Connected Address Pill */}
      <div className="flex items-center gap-1.5 px-2.5 py-1 rounded-lg border border-neutral-200 dark:border-neutral-800 bg-white/90 dark:bg-neutral-900/90 text-xs shadow-2xs">
        {/* Status Dot */}
        <span
          className={`h-2 w-2 rounded-full ${
            isSigningBlocked
              ? "bg-red-500 animate-pulse"
              : "bg-teal-500"
          }`}
          title={
            isSigningBlocked
              ? t("wallet.signing_blocked_title")
              : t("wallet.freighter_connected")
          }
        />

        <WalletIcon size={13} className="text-neutral-500 dark:text-neutral-400" />

        <span className="text-neutral-600 dark:text-neutral-300 font-medium text-[11px]">
          {formatAddress(address)}
        </span>

        <button
          type="button"
          onClick={() => handleCopy(address)}
          className="p-1 rounded text-neutral-400 hover:text-teal-600 dark:hover:text-teal-400 hover:bg-neutral-100 dark:hover:bg-neutral-800 transition-all duration-150 hover:scale-110 active:scale-95 cursor-pointer focus-visible:ring-2 focus-visible:ring-teal-500/50 focus-visible:outline-none"
          title={t("common.copy_full_address")}
          aria-label={t("common.copy_full_address")}
        >
          {copied ? (
            <CheckIcon size={12} className="text-emerald-500" />
          ) : (
            <CopyIcon size={12} />
          )}
        </button>
      </div>

      {/* Disconnect Action */}
      <button
        type="button"
        onClick={(e) => {
          e.preventDefault();
          disconnectFreighter();
        }}
        className="group inline-flex items-center justify-center p-1.5 rounded-lg border border-red-200/80 dark:border-red-900/50 bg-red-50/50 dark:bg-red-950/30 text-red-600 dark:text-red-400 hover:bg-red-100 dark:hover:bg-red-900/40 hover:text-red-700 dark:hover:text-red-300 hover:-translate-y-0.5 active:translate-y-0 active:scale-[0.98] transition-all duration-150 cursor-pointer shadow-2xs focus-visible:ring-2 focus-visible:ring-red-500/50 focus-visible:outline-none"
        title={t("wallet.disconnect")}
        aria-label={t("wallet.disconnect")}
      >
        <LogoutDoorIcon size={15} />
      </button>
    </div>
  );
}
