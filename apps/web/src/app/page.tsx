"use client";

import React, { useCallback, useEffect, useState } from "react";
import { useRouter } from "next/navigation";
import {
  CanguPayLogo,
  NetworkIcon,
  RoleIcon,
} from "@/components/icons";
import { ThemeSwitcher } from "@/components/theme/ThemeSwitcher";
import { LanguageSwitcher } from "@/components/theme/LanguageSwitcher";
import { WalletConnection } from "@/components/wallet/WalletConnection";
import { useWallet } from "@/providers/WalletProvider";
import { useLanguage } from "@/providers/LanguageProvider";
import { EscrowDetails } from "@/components/escrow/EscrowDetails";
import styles from "@/components/escrow/EscrowVisuals.module.css";
import { CreateEscrowModal } from "@/components/escrow/CreateEscrowModal";
import {
  EscrowStatus,
  deriveWalletRole,
  getAvailableActions,
  mapOnChainEscrow,
} from "@/types/escrow";

import { mockEscrows } from "@/dev/mockEscrow";
import { fetchOnChainEscrow } from "@/lib/soroban";

export default function Home() {
  const { t } = useLanguage();
  const router = useRouter();
  const [activeScenario, setActiveScenario] = useState<EscrowStatus>("FUNDED");
  const [isLoading, setIsLoading] = useState<boolean>(false);
  const [hasError, setHasError] = useState<boolean>(false);
  const [isSimulatingExpired, setIsSimulatingExpired] = useState<boolean>(false);
  const [isCreateModalOpen, setIsCreateModalOpen] = useState<boolean>(false);
  const [onChainState, setOnChainState] = useState<string | null>(null);
  const [onChainConfig, setOnChainConfig] = useState<Record<string, unknown> | null>(null);
  const [rpcError, setRpcError] = useState<string | null>(null);

  const {
    address,
    isConnected,
    network,
    networkPassphrase,
    isExactTestnet,
    isFreighterInstalled,
  } = useWallet();

  const contractId = process.env.NEXT_PUBLIC_ESCROW_CONTRACT_ID || "";

  const handleRefresh = useCallback(async () => {
    setIsLoading(true);
    setHasError(false);
    setRpcError(null);
    setOnChainState(null);
    setOnChainConfig(null);
    if (contractId) {
      try {
        const res = await fetchOnChainEscrow(contractId);
        if (res.error) {
          setRpcError(res.error);
          setHasError(true);
        } else {
          setOnChainState(res.state);
          setOnChainConfig(res.config as Record<string, unknown> | null);
        }
      } catch (e) {
        setRpcError(e instanceof Error ? e.message : String(e));
        setHasError(true);
      }
    }
    setIsLoading(false);
  }, [contractId]);

  useEffect(() => {
    if (!contractId) return;
    const timer = setTimeout(() => void handleRefresh(), 0);
    return () => clearTimeout(timer);
  }, [contractId, handleRefresh]);

  const handleLogoClick = useCallback(() => {
    if (!isConnected) {
      router.push("/landing");
    } else {
      void handleRefresh();
    }
  }, [isConnected, router, handleRefresh]);

  // A configured contract never falls back to preview data, even for partial RPC reads.
  const isMock = !contractId;
  const currentData = isMock
    ? mockEscrows[activeScenario]
    : mapOnChainEscrow(onChainState, onChainConfig, contractId);
  // A preview timeout is explicit and only applies to a known, eligible deadline.
  const canPreviewFinalize = isMock && isSimulatingExpired && Boolean(address) &&
    Boolean(currentData?.activeDeadline?.timestamp) && currentData?.status !== "CREATED";

  const derivedRole = deriveWalletRole(address, currentData?.parties);
  const availableActions = currentData
    ? getAvailableActions(derivedRole, currentData.status, canPreviewFinalize, t, currentData.fallbackOutcome)
      .filter((action) => isMock || action.id !== "create")
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

  const bannerSlot = isMock ? (
    <div
      role="status"
      className={styles.previewBanner}
    >
      <div className={styles.bannerIntro}>
        <strong className={styles.bannerBadge}>{t("alerts.mock_data_badge")}</strong>
        <span className={styles.bannerMessage}>{t("alerts.mock_only")}</span>
      </div>
      <div className={styles.bannerControls}>
        <label className={styles.expiryToggle}>
          <input
            type="checkbox"
            checked={isSimulatingExpired}
            onChange={(e) => setIsSimulatingExpired(e.target.checked)}
            className={styles.expiryCheckbox}
          />
          {t("alerts.simulate_expiry")}
        </label>
        <label htmlFor="preview-state" className={styles.scenarioLabel}>{t("alerts.scenario_label")}</label>
        <select
          id="preview-state"
          value={activeScenario}
          onChange={(e) => setActiveScenario(e.target.value as EscrowStatus)}
          className={styles.scenarioSelect}
        >
          {Object.keys(mockEscrows).map((status) => (
            <option key={status} value={status}>
              {t(`status.${status}`)}
            </option>
          ))}
        </select>
      </div>
    </div>
  ) : (
    <div role="status" className={styles.onchainBanner}>
      <strong className={styles.bannerBadge}>{t("alerts.onchain_data_badge")}</strong>
      <span>{t("alerts.onchain_partial")}</span>
      {rpcError && <span className={styles.onchainError}>{rpcError}</span>}
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

        <div className="pointer-events-auto relative z-10 max-w-5xl mx-auto px-4 sm:px-6 min-h-16 py-2 flex flex-wrap items-center justify-between gap-2">
          <div className="flex flex-wrap items-center gap-2 sm:gap-3">
            <button
              type="button"
              onClick={handleLogoClick}
              className="flex items-center cursor-pointer focus:outline-none focus-visible:ring-2 focus-visible:ring-teal-500 rounded-md p-0.5 -m-0.5 transition-opacity hover:opacity-85"
              title={isConnected ? t("common.refresh") : "CanguPay Landing"}
              aria-label={isConnected ? t("common.refresh") : "Ir a landing"}
            >
              <CanguPayLogo className="h-7 sm:h-8 w-auto" />
            </button>
            <span className="hidden sm:inline-block text-neutral-300 dark:text-neutral-700 font-light">
              |
            </span>
            <div
              className="flex items-center gap-1.5 px-2.5 py-1 rounded-lg border border-neutral-200/80 dark:border-neutral-800/80 bg-neutral-100/70 dark:bg-neutral-900/70 text-neutral-700 dark:text-neutral-300 font-mono text-[11px] font-bold shadow-2xs"
              title={!address ? t("header.no_wallet") : derivedRole ? `${t("header.role")}: ${t(`header.roles.${derivedRole}`)}` : t("header.role_unavailable")}
            >
              <span>{t("header.role")}: {derivedRole ? t(`header.roles.${derivedRole}`) : "—"}</span>
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

            {isConnected && isFreighterInstalled && <div
              className={`flex items-center font-mono text-xs border rounded-lg overflow-hidden shadow-2xs ${
                isExactTestnet
                    ? "border-teal-500/30 bg-teal-500/5 text-teal-700 dark:text-teal-300"
                    : "border-red-500 bg-red-500/10 text-red-600 dark:text-red-400"
              }`}
              title={
                isExactTestnet
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
                  isExactTestnet
                      ? "text-teal-700 dark:text-teal-300"
                      : "text-red-600 dark:text-red-400 animate-pulse"
                }`}
              >
                {isExactTestnet
                    ? t("header.testnet").toUpperCase()
                    : t("header.blocked").toUpperCase()}
              </span>
            </div>}
          </div>
        </div>
      </header>

      {/* Main Content */}
      <main className="max-w-5xl mx-auto px-4 sm:px-6 py-8">
        <div className="bg-white/95 dark:bg-neutral-900/85 backdrop-blur-xs border border-neutral-200/80 dark:border-neutral-800/80 rounded-2xl shadow-xs">
          <EscrowDetails
            data={currentData}
            isLoading={isLoading}
            error={hasError ? `${t("alerts.contract_error")}: ${rpcError || t("alerts.unavailable")}` : null}
            emptyTitle={isMock ? undefined : t("alerts.onchain_unavailable_title")}
            emptyDescription={isMock ? undefined : t("alerts.onchain_unavailable_desc")}
            onRefresh={handleRefresh}
            actionSlot={actionSlot}
            bannerSlot={bannerSlot}
            viewerRole={derivedRole}
            canFinalize={canPreviewFinalize}
            onCreateEscrow={isMock ? () => setIsCreateModalOpen(true) : undefined}
          />
        </div>
      </main>

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
