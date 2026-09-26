"use client";

import React, { useState, useEffect } from "react";
import Image from "next/image";
import {
  CheckIcon,
  CopyIcon,
  ClockIcon,
  CpuIcon,
  UserIcon,
  PackageIcon,
} from "@/components/icons";
import { useLanguage } from "@/providers/LanguageProvider";

type RailStep = 1 | 2 | 3;

interface StepData {
  step: RailStep;
  status: string;
  statusLabelKey: string;
  badgeColor: string;
  headlineKey: string;
  descKey: string;
  actor: string;
  actorRole: "buyer" | "engine" | "supplier";
  actorKey: string;
  hashLabelKey: string;
  hashValue: string;
  timestamp: string;
}

export function HeroEscrowRail() {
  const { t } = useLanguage();
  const [activeStep, setActiveStep] = useState<RailStep>(2);
  const [isPaused, setIsPaused] = useState<boolean>(false);
  const [copiedHash, setCopiedHash] = useState<string | null>(null);

  // Auto-advance through states to demonstrate the living trust rail
  useEffect(() => {
    if (isPaused) return;

    const interval = setInterval(() => {
      setActiveStep((prev) => (prev === 3 ? 1 : ((prev + 1) as RailStep)));
    }, 4200);

    return () => clearInterval(interval);
  }, [isPaused]);

  const steps: StepData[] = [
    {
      step: 1,
      status: "FUNDED",
      statusLabelKey: "status.FUNDED",
      badgeColor: "border-teal-500/30 text-teal-700 dark:text-teal-400 bg-teal-500/10",
      headlineKey: "landing.hero.step_1_title",
      descKey: "landing.hero.step_1_desc",
      actor: "GDWTZTTY...9W4E",
      actorRole: "buyer",
      actorKey: "landing.hero.parties_buyer",
      hashLabelKey: "landing.hero.tx_digest",
      hashValue: "c18dea5142003778ac4bd3aafda31bebd5cf9651",
      timestamp: "Ledger #5128790 · 14:20:00",
    },
    {
      step: 2,
      status: "ATTESTED_PASS",
      statusLabelKey: "status.ATTESTED_PASS",
      badgeColor: "border-emerald-500/30 text-emerald-700 dark:text-emerald-400 bg-emerald-500/10",
      headlineKey: "landing.hero.step_2_title",
      descKey: "landing.hero.step_2_desc",
      actor: "CCGZQCV...J2OF",
      actorRole: "engine",
      actorKey: "landing.hero.parties_engine",
      hashLabelKey: "landing.hero.evidence_hash",
      hashValue: "4f2e8d65342a9812bf7c83da0129bc348e7189a0",
      timestamp: "Ledger #5128850 · 14:21:15",
    },
    {
      step: 3,
      status: "RELEASED",
      statusLabelKey: "status.RELEASED",
      badgeColor: "border-emerald-500/30 text-emerald-700 dark:text-emerald-400 bg-emerald-500/10",
      headlineKey: "landing.hero.step_3_title",
      descKey: "landing.hero.step_3_desc",
      actor: "CAVLH35...4MRH",
      actorRole: "supplier",
      actorKey: "landing.hero.parties_supplier",
      hashLabelKey: "landing.hero.tx_digest",
      hashValue: "a093edc784f45bc76c128e17c8c1e31c223db2f5",
      timestamp: "Ledger #5128911 · 14:22:40",
    },
  ];

  const current = steps[activeStep - 1];

  const handleCopy = (val: string) => {
    navigator.clipboard?.writeText(val);
    setCopiedHash(val);
    setTimeout(() => setCopiedHash(null), 1800);
  };

  return (
    <div
      onMouseEnter={() => setIsPaused(true)}
      onMouseLeave={() => setIsPaused(false)}
      className="relative rounded-2xl border border-neutral-200 dark:border-neutral-800 bg-white/90 dark:bg-neutral-900/90 shadow-xl overflow-hidden backdrop-blur-sm transition-all duration-300"
    >
      {/* Background visual texture with generated asset (dark mode only) */}
      <div className="absolute inset-0 pointer-events-none overflow-hidden opacity-0 dark:opacity-25 transition-opacity">
        <Image
          src="/images/ledger-topology.jpg"
          alt=""
          fill
          className="object-cover object-center mix-blend-screen [mask-image:radial-gradient(ellipse_at_top_right,black_30%,transparent_80%)]"
          priority
        />
      </div>

      {/* Top Header Card Info */}
      <div className="relative z-10 px-5 sm:px-6 pt-5 pb-4 border-b border-neutral-200/80 dark:border-neutral-800/80 flex items-center justify-between gap-3">
        <div className="flex items-center gap-2">
          <span className="relative flex h-2.5 w-2.5">
            <span className="animate-ping absolute inline-flex h-full w-full rounded-full bg-teal-400 opacity-75" />
            <span className="relative inline-flex rounded-full h-2.5 w-2.5 bg-teal-500" />
          </span>
          <span className="font-mono text-xs font-semibold text-neutral-800 dark:text-neutral-200 tracking-wider">
            {t("landing.hero.contract_id")}
          </span>
        </div>

        <div className="flex items-center gap-2">
          <span className={`inline-flex items-center px-2 py-0.5 rounded text-[11px] font-mono font-medium border ${current.badgeColor}`}>
            {t(current.statusLabelKey)}
          </span>
        </div>
      </div>

      {/* Hero Financial Amount Block */}
      <div className="relative z-10 px-5 sm:px-6 pt-6 pb-5">
        <p className="text-xs font-mono font-medium text-neutral-500 dark:text-neutral-400 uppercase tracking-wider">
          {t("landing.hero.rail_title")}
        </p>

        <div className="mt-2 flex items-baseline flex-wrap gap-2.5">
          <span className="font-mono font-bold text-3xl sm:text-4xl text-neutral-950 dark:text-neutral-50 tracking-tight">
            15,000.00
          </span>
          <span className="font-mono font-semibold text-lg text-teal-700 dark:text-teal-400">
            CPUSD
          </span>
        </div>
        <p className="mt-1 text-[11px] font-mono text-neutral-500 dark:text-neutral-400">
          {t("landing.hero.asset_note")}
        </p>
      </div>

      {/* The 3-Step Interactive State Rail */}
      <div className="relative z-10 px-4 sm:px-6 py-3.5 sm:py-4 bg-neutral-50/70 dark:bg-neutral-950/40 border-t border-b border-neutral-200/80 dark:border-neutral-800/80">
        <div className="grid grid-cols-3 gap-1.5 sm:gap-2">
          {steps.map((s) => {
            const isActive = activeStep === s.step;
            const isCompleted = activeStep > s.step;

            return (
              <button
                key={s.step}
                type="button"
                onClick={() => setActiveStep(s.step)}
                aria-pressed={isActive}
                className={`relative text-left p-2 sm:p-2.5 rounded-lg border transition-all duration-200 cursor-pointer ${
                  isActive
                    ? "border-teal-500/60 dark:border-teal-400/60 bg-white dark:bg-neutral-800/90 shadow-xs"
                    : isCompleted
                      ? "border-neutral-300/80 dark:border-neutral-700/80 bg-neutral-100/50 dark:bg-neutral-900/50 opacity-90 hover:opacity-100"
                      : "border-neutral-200 dark:border-neutral-800 bg-transparent opacity-60 hover:opacity-90"
                }`}
              >
                {/* Node indicator */}
                <div className="flex items-center justify-between mb-1">
                  <span className={`text-[10px] font-mono font-bold ${isActive ? "text-teal-600 dark:text-teal-400" : "text-neutral-500"}`}>
                    0{s.step}
                  </span>
                  {isCompleted ? (
                    <CheckIcon size={11} className="text-teal-600 dark:text-teal-400" />
                  ) : isActive ? (
                    <div className="w-1.5 h-1.5 rounded-full bg-teal-500 animate-pulse" />
                  ) : null}
                </div>
                <div className="font-semibold text-[11px] sm:text-xs text-neutral-900 dark:text-neutral-100 truncate">
                  {s.status}
                </div>
              </button>
            );
          })}
        </div>
      </div>

      {/* Dynamic Detail Panel for Current Selected State */}
      <div className="relative z-10 px-5 sm:px-6 py-4 space-y-3">
        <div className="flex items-start justify-between gap-3">
          <div>
            <h4 className="text-sm font-semibold text-neutral-900 dark:text-neutral-100">
              {t(current.headlineKey)}
            </h4>
            <p className="text-xs text-neutral-600 dark:text-neutral-400 mt-0.5">
              {t(current.descKey)}
            </p>
          </div>
        </div>

        {/* Actor and Hash verification row */}
        <div className="p-3 rounded-lg bg-neutral-100/80 dark:bg-neutral-800/50 border border-neutral-200/60 dark:border-neutral-700/60 text-xs font-mono space-y-2">
          <div className="flex items-center justify-between text-neutral-600 dark:text-neutral-400">
            <span className="flex items-center gap-1.5">
              {current.actorRole === "buyer" && <UserIcon size={13} />}
              {current.actorRole === "engine" && <CpuIcon size={13} />}
              {current.actorRole === "supplier" && <PackageIcon size={13} />}
              <span>{t(current.actorKey)}:</span>
            </span>
            <span className="font-bold text-neutral-900 dark:text-neutral-200">
              {current.actor}
            </span>
          </div>

          <div className="flex items-center justify-between text-neutral-600 dark:text-neutral-400 pt-1.5 border-t border-neutral-200/50 dark:border-neutral-700/50">
            <span>{t(current.hashLabelKey)}:</span>
            <div className="flex items-center gap-1">
              <span className="text-neutral-800 dark:text-neutral-200 truncate max-w-[120px] sm:max-w-[160px]">
                {current.hashValue.slice(0, 10)}...{current.hashValue.slice(-6)}
              </span>
              <button
                type="button"
                onClick={() => handleCopy(current.hashValue)}
                className="p-1 text-neutral-400 hover:text-neutral-700 dark:hover:text-neutral-200 transition-colors"
                title="Copiar"
                aria-label="Copiar hash"
              >
                {copiedHash === current.hashValue ? (
                  <CheckIcon size={12} className="text-teal-600 dark:text-teal-400" />
                ) : (
                  <CopyIcon size={12} />
                )}
              </button>
            </div>
          </div>
        </div>

        {/* Footer timestamp */}
        <div className="flex items-center justify-between text-[11px] font-mono text-neutral-500 pt-1">
          <span className="flex items-center gap-1">
            <ClockIcon size={12} />
            <span>{current.timestamp}</span>
          </span>
          <span className="text-[10px] text-neutral-400 dark:text-neutral-500">
            {isPaused ? "Pausado (hover)" : "Rotación automática"}
          </span>
        </div>
      </div>
    </div>
  );
}
