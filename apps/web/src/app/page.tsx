"use client";

import React, { useState } from "react";
import {
  CanguPayLogo,
  NetworkIcon,
  RoleIcon,
  CloseIcon,
} from "@/components/icons";
import { ThemeSwitcher } from "@/components/theme/ThemeSwitcher";
import { LanguageSwitcher } from "@/components/theme/LanguageSwitcher";
import { WalletConnection } from "@/components/wallet/WalletConnection";
import { useWallet } from "@/providers/WalletProvider";
import { useLanguage } from "@/providers/LanguageProvider";
import { EscrowDetails } from "@/components/escrow/EscrowDetails";
import { CreateEscrowModal } from "@/components/escrow/CreateEscrowModal";
import {
  EscrowStatus,
  deriveWalletRole,
  getAvailableActions,
} from "@/types/escrow";

import { mockEscrows } from "@/dev/mockEscrow";

export default function Home() {
  const { t } = useLanguage();
  const [activeScenario, setActiveScenario] = useState<EscrowStatus>("FUNDED");
  const [isLoading, setIsLoading] = useState<boolean>(false);
  const [hasError, setHasError] = useState<boolean>(false);
  const [isEmpty, setIsEmpty] = useState<boolean>(false);
  const [isSimulatingExpired, setIsSimulatingExpired] = useState<boolean>(false);
  const [isCreateModalOpen, setIsCreateModalOpen] = useState<boolean>(false);
  const [isDevToolbarOpen, setIsDevToolbarOpen] = useState<boolean>(false);

  const {
    address,
    network,
    networkPassphrase,
    isExactTestnet,
    isFreighterInstalled,
  } = useWallet();

  const handleRefresh = () => {
    setIsLoading(true);
    setHasError(false);
    setIsEmpty(false);
    setTimeout(() => {
      setIsLoading(false);
    }, 450);
  };

  const currentData = isEmpty ? null : mockEscrows[activeScenario] || null;
  const derivedRole = deriveWalletRole(address, currentData?.parties);
  const availableActions = currentData
    ? getAvailableActions(derivedRole, currentData.status, isSimulatingExpired, t)
    : [];

  const handleActionClick = (actionId: string) => {
    if (actionId === "create") {
      setIsCreateModalOpen(true);
    }
  };

  const actionSlot = (
    <div className="flex flex-wrap items-center gap-2">
      {currentData?.status === "EVIDENCE_SUBMITTED" && (
        <div className="flex items-center gap-1.5 px-2.5 py-1.5 rounded-lg border border-blue-500/30 bg-blue-500/10 text-blue-700 dark:text-blue-300 font-mono text-[11px] font-medium shadow-2xs">
          <span className="h-1.5 w-1.5 rounded-full bg-blue-500 animate-pulse" />
          <span>{t("alerts.waiting_engine")}</span>
        </div>
      )}

      {availableActions.map((action) => {
        const isInteractive = action.id === "create";
        const actionLabel = action.labelKey ? t(action.labelKey) : action.label;
        const actionDesc = action.descKey ? t(action.descKey) : action.description;

        return (
          <button
            key={action.id}
            type="button"
            disabled={!isInteractive}
            onClick={() => handleActionClick(action.id)}
            title={
              isInteractive
                ? `${action.fullName} — ${actionDesc}`
                : `${action.fullName} — ${actionDesc} (${t("common.pending_onchain")})`
            }
            className={`px-3 py-1.5 rounded-lg font-mono text-xs font-semibold shadow-2xs flex items-center gap-1.5 border transition-all ${
              isInteractive
                ? "border-teal-600 bg-teal-600 text-white hover:bg-teal-700 cursor-pointer opacity-100"
                : action.variant === "danger"
                  ? "border-rose-500/40 bg-rose-500/10 text-rose-700 dark:text-rose-300 opacity-60 cursor-not-allowed"
                  : action.variant === "secondary"
                    ? "border-cyan-500/40 bg-cyan-500/10 text-cyan-700 dark:text-cyan-300 opacity-60 cursor-not-allowed"
                    : "border-teal-600/50 bg-teal-600/15 text-teal-800 dark:text-teal-200 opacity-60 cursor-not-allowed"
            }`}
          >
            <span>{actionLabel}</span>
            <span className="text-[10px] opacity-75 font-normal">
              {isInteractive ? "✦" : `[${t("common.pending_onchain")}]`}
            </span>
          </button>
        );
      })}
    </div>
  );

  return (
    <div className="min-h-screen font-sans antialiased selection:bg-teal-500/20">
      {/* Top Navbar */}
      <header className="sticky top-0 z-30 pointer-events-none">
        <div
          aria-hidden="true"
          className="absolute inset-x-0 top-0 h-24 pointer-events-none backdrop-blur-md bg-white/85 dark:bg-neutral-900/85 header-fade-mask"
        />

        <div className="pointer-events-auto relative z-10 max-w-5xl mx-auto px-4 sm:px-6 h-16 flex items-center justify-between">
          <div className="flex items-center gap-3">
            <CanguPayLogo className="h-7 sm:h-8 w-auto" />
            <span className="hidden sm:inline-block text-neutral-300 dark:text-neutral-700 font-light">
              |
            </span>
            <div
              className="hidden sm:flex items-center gap-1.5 px-2.5 py-1 rounded-lg border border-neutral-200/80 dark:border-neutral-800/80 bg-neutral-100/70 dark:bg-neutral-900/70 text-neutral-700 dark:text-neutral-300 font-mono text-[11px] font-bold tracking-wider uppercase shadow-2xs"
              title={
                derivedRole
                  ? `${t("header.role")}: ${derivedRole.toUpperCase()}`
                  : t("header.no_wallet")
              }
            >
              <span>{t("header.role")}: {derivedRole ? derivedRole.toUpperCase() : "—"}</span>
              {derivedRole && (
                <RoleIcon role={derivedRole} size={13} className="text-teal-600 dark:text-teal-400" />
              )}
            </div>
          </div>

          {/* Header Controls: Wallet Connection, Theme Mode & Dynamic Network */}
          <div className="flex items-center gap-2 sm:gap-3">
            <WalletConnection />
            <ThemeSwitcher />
            <LanguageSwitcher />

            <div
              className={`flex items-center font-mono text-xs border rounded-lg overflow-hidden shadow-2xs ${
                !isFreighterInstalled
                  ? "border-neutral-200 dark:border-neutral-800 bg-neutral-100/60 dark:bg-neutral-900/60 text-neutral-400"
                  : isExactTestnet
                    ? "border-teal-500/30 bg-teal-500/5 text-teal-700 dark:text-teal-300"
                    : "border-red-500 bg-red-500/10 text-red-600 dark:text-red-400"
              }`}
              title={
                !isFreighterInstalled
                  ? t("header.not_detected")
                  : isExactTestnet
                    ? `${t("header.network")}: ${network} (${networkPassphrase || ""})`
                    : `${t("header.blocked")}: ${network}`
              }
            >
              <span
                className="px-2 py-1.5 bg-neutral-200/50 dark:bg-neutral-800 text-neutral-500 dark:text-neutral-400 border-r border-neutral-200 dark:border-neutral-800 flex items-center justify-center"
                aria-label={t("header.network")}
              >
                <NetworkIcon size={13} />
              </span>
              <span
                className={`px-2.5 py-1 text-[11px] font-bold tracking-wider ${
                  !isFreighterInstalled
                    ? "text-neutral-400"
                    : isExactTestnet
                      ? "text-teal-700 dark:text-teal-300"
                      : "text-red-600 dark:text-red-400 animate-pulse"
                }`}
              >
                {!isFreighterInstalled
                  ? t("header.not_detected").toUpperCase()
                  : isExactTestnet
                    ? t("header.testnet").toUpperCase()
                    : t("header.blocked").toUpperCase()}
              </span>
            </div>
          </div>
        </div>
      </header>

      {/* Main Content */}
      <main className="max-w-5xl mx-auto px-4 sm:px-6 py-8">
        <div className="bg-white/95 dark:bg-neutral-900/85 backdrop-blur-xs border border-neutral-200/80 dark:border-neutral-800/80 rounded-2xl shadow-xs">
          <EscrowDetails
            data={currentData}
            isLoading={isLoading}
            error={hasError ? t("alerts.contract_error") : null}
            onRefresh={handleRefresh}
            actionSlot={actionSlot}
            viewerRole={derivedRole}
            canFinalize={isSimulatingExpired}
            onCreateEscrow={() => setIsCreateModalOpen(true)}
          />
        </div>
      </main>

      {/* Discreet Floating Dev Toolbar */}
      <div className="fixed bottom-4 right-4 z-40">
        {!isDevToolbarOpen ? (
          <button
            type="button"
            onClick={() => setIsDevToolbarOpen(true)}
            className="flex items-center gap-2 px-3 py-1.5 rounded-full bg-neutral-900/90 dark:bg-neutral-800/90 hover:bg-neutral-800 dark:hover:bg-neutral-700/90 text-neutral-200 border border-amber-500/30 shadow-lg backdrop-blur-md font-mono text-xs cursor-pointer transition-all duration-200 group"
            title={t("alerts.dev_preview")}
          >
            <span className="h-2 w-2 rounded-full bg-amber-400 animate-pulse" />
            <span className="font-semibold tracking-wider text-[11px] text-amber-300">
              DEV · MOCK
            </span>
            <span className="text-[10px] text-neutral-400 font-normal">
              [{activeScenario}]
            </span>
          </button>
        ) : (
          <div className="flex flex-wrap items-center gap-2.5 px-3.5 py-2 rounded-xl bg-neutral-900/95 dark:bg-neutral-900/95 text-neutral-100 border border-amber-500/40 shadow-2xl backdrop-blur-md font-mono text-xs">
            <div className="flex items-center gap-1.5 shrink-0">
              <span className="h-2 w-2 rounded-full bg-amber-400 animate-pulse" />
              <span className="text-[10px] font-bold tracking-widest text-amber-400 uppercase">
                {t("alerts.dev_preview")}
              </span>
            </div>

            <div className="h-4 w-px bg-neutral-700 shrink-0" />

            {/* Scenario Selector */}
            <select
              value={activeScenario}
              onChange={(e) => setActiveScenario(e.target.value as EscrowStatus)}
              className="text-[11px] bg-neutral-800 border border-neutral-700 hover:border-amber-500/50 text-neutral-200 rounded-md px-2 py-1 font-mono cursor-pointer focus:outline-none focus:ring-1 focus:ring-amber-500"
              title="Escenario de desarrollo"
              aria-label="Escenario de desarrollo"
            >
              {Object.keys(mockEscrows).map((status) => (
                <option key={status} value={status}>
                  {status}
                </option>
              ))}
            </select>

            {/* Simular Vencimiento */}
            <label className="flex items-center gap-1.5 cursor-pointer text-[11px] bg-neutral-800/80 hover:bg-neutral-800 border border-neutral-700 rounded-md px-2 py-1 select-none transition-colors">
              <input
                type="checkbox"
                checked={isSimulatingExpired}
                onChange={(e) => setIsSimulatingExpired(e.target.checked)}
                className="rounded border-neutral-600 text-teal-500 focus:ring-0 cursor-pointer h-3.5 w-3.5"
              />
              <span className="text-neutral-300 font-medium text-[10px] tracking-wide">
                {t("alerts.simulate_expiry")}
              </span>
            </label>

            {/* Collapse button */}
            <button
              type="button"
              onClick={() => setIsDevToolbarOpen(false)}
              className="p-1 rounded text-neutral-400 hover:text-white hover:bg-neutral-800 transition-colors cursor-pointer ml-1"
              title={t("common.close")}
              aria-label={t("common.close")}
            >
              <CloseIcon size={14} />
            </button>
          </div>
        )}
      </div>

      {/* Create Escrow Modal */}
      <CreateEscrowModal
        isOpen={isCreateModalOpen}
        onClose={() => setIsCreateModalOpen(false)}
        buyerAddress={address}
        onCreated={() => {
          setActiveScenario("CREATED");
        }}
      />
    </div>
  );
}
