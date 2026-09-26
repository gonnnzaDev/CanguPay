"use client";

import React from "react";
import Link from "next/link";
import { useLanguage } from "@/providers/LanguageProvider";
import { HeroEscrowRail } from "./HeroEscrowRail";
import { ExternalLinkIcon, ShieldLockIcon, CpuIcon, ScaleIcon } from "@/components/icons";

export function LandingHero() {
  const { t } = useLanguage();

  return (
    <section className="relative pt-12 pb-16 sm:pt-20 sm:pb-24 lg:pt-28 lg:pb-32 overflow-hidden">
      <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8">
        <div className="grid grid-cols-1 lg:grid-cols-12 gap-12 lg:gap-8 items-center">
          {/* Left Column: Asymmetrical Typography & Direct Action */}
          <div className="lg:col-span-6 space-y-6 sm:space-y-8 text-left">
            {/* Engineering Badge */}
            <div className="inline-flex items-center gap-2 px-2.5 py-1 rounded-md border border-neutral-300 dark:border-neutral-800 bg-neutral-100/70 dark:bg-neutral-900/70 font-mono text-[11px] text-neutral-600 dark:text-neutral-400">
              <span className="w-1.5 h-1.5 rounded-full bg-teal-500" />
              <span>{t("landing.hero.badge")}</span>
            </div>

            {/* Brutally concise headline */}
            <h1 className="text-3xl sm:text-5xl lg:text-6xl font-extrabold tracking-tight text-neutral-950 dark:text-white leading-[1.15] sm:leading-[1.1] text-balance">
              {t("landing.hero.title")}
            </h1>

            {/* Subheadline (strictly 1 sentence) */}
            <p className="text-sm sm:text-base lg:text-lg text-neutral-600 dark:text-neutral-400 max-w-xl leading-relaxed">
              {t("landing.hero.subtitle")}
            </p>

            {/* Primary & Secondary CTAs */}
            <div className="flex flex-wrap items-center gap-3 sm:gap-4 pt-1 sm:pt-2">
              <Link
                href="/"
                className="inline-flex items-center justify-center px-4 py-2.5 sm:px-5 sm:py-3 text-sm sm:text-base font-semibold rounded-xl bg-[#0E2F46] hover:bg-[#0c273a] text-white dark:bg-[#0EAFA2] dark:hover:bg-[#0d9d92] dark:text-neutral-950 shadow-md transition-all duration-150 active:scale-[0.98] focus:outline-none focus-visible:ring-2 focus-visible:ring-teal-500"
              >
                <span>{t("landing.hero.cta_primary")}</span>
                <span className="ml-2 font-mono">→</span>
              </Link>

              <a
                href="https://github.com/gonnnzaDev/CanguPay"
                target="_blank"
                rel="noopener noreferrer"
                className="inline-flex items-center justify-center px-4 py-2.5 sm:px-4 sm:py-3 text-sm sm:text-base font-semibold rounded-xl border border-neutral-300 dark:border-neutral-800 bg-white dark:bg-neutral-900 text-neutral-800 dark:text-neutral-200 hover:bg-neutral-50 dark:hover:bg-neutral-850 shadow-2xs transition-all duration-150 active:scale-[0.98] focus:outline-none focus-visible:ring-2 focus-visible:ring-teal-500"
              >
                <span>{t("landing.hero.cta_secondary")}</span>
                <ExternalLinkIcon size={14} className="ml-2 opacity-70" />
              </a>
            </div>

            {/* Micro Technical Markers */}
            <div className="pt-4 border-t border-neutral-200/60 dark:border-neutral-800/60 flex flex-wrap items-center gap-x-4 sm:gap-x-6 gap-y-2 text-[11px] sm:text-xs font-mono text-neutral-500 dark:text-neutral-400">
              <div className="flex items-center gap-1.5">
                <ShieldLockIcon size={13} className="text-teal-600 dark:text-teal-400 shrink-0" />
                <span>Non-custodial Soroban</span>
              </div>
              <div className="flex items-center gap-1.5">
                <CpuIcon size={13} className="text-teal-600 dark:text-teal-400 shrink-0" />
                <span>Deterministic Attestation</span>
              </div>
              <div className="flex items-center gap-1.5">
                <ScaleIcon size={13} className="text-teal-600 dark:text-teal-400 shrink-0" />
                <span>Human Resolution</span>
              </div>
            </div>
          </div>

          {/* Right Column: The Living Escrow Card */}
          <div className="lg:col-span-6 w-full max-w-lg mx-auto lg:max-w-none">
            <HeroEscrowRail />
          </div>
        </div>
      </div>
    </section>
  );
}
