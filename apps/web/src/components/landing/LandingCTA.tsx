"use client";

import React from "react";
import Link from "next/link";
import { useLanguage } from "@/providers/LanguageProvider";
import { CanguPayLogo, ExternalLinkIcon } from "@/components/icons";

export function LandingCTA() {
  const { t } = useLanguage();

  return (
    <footer className="border-t border-neutral-200/60 dark:border-neutral-800/60 relative bg-white/40 dark:bg-neutral-950/40">
      {/* Final Call To Action Card */}
      <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 pt-20 pb-16">
        <div className="relative rounded-3xl border border-neutral-200 dark:border-neutral-800 bg-neutral-900 dark:bg-neutral-900/90 text-white p-8 sm:p-14 overflow-hidden shadow-2xl">
          {/* Subtle brand glow in dark card */}
          <div className="absolute -right-20 -bottom-20 w-80 h-80 bg-teal-500/15 rounded-full blur-3xl pointer-events-none" />

          <div className="relative z-10 max-w-2xl space-y-4">
            <span className="font-mono text-xs uppercase tracking-widest text-[#0EAFA2]">
              TESTNET READY // STELLAR ODYSSEY & BUILDER CHALLENGE
            </span>
            <h2 className="text-3xl sm:text-4xl lg:text-5xl font-extrabold tracking-tight leading-tight">
              {t("landing.cta.title")}
            </h2>
            <p className="text-sm sm:text-base text-neutral-300 leading-relaxed max-w-lg">
              {t("landing.cta.subtitle")}
            </p>

            <div className="pt-4 flex flex-wrap items-center gap-3 sm:gap-4">
              <Link
                href="/"
                className="inline-flex items-center justify-center px-5 py-3 text-sm sm:text-base font-semibold rounded-xl bg-[#0EAFA2] hover:bg-[#0d9d92] text-neutral-950 shadow-md transition-all duration-150 active:scale-[0.98]"
              >
                <span>{t("landing.cta.launch")}</span>
                <span className="ml-2 font-mono">→</span>
              </Link>

              <a
                href="https://github.com/gonnnzaDev/CanguPay"
                target="_blank"
                rel="noopener noreferrer"
                className="inline-flex items-center justify-center px-4 py-3 text-sm sm:text-base font-semibold rounded-xl border border-neutral-700 bg-neutral-800 text-neutral-100 hover:bg-neutral-750 transition-all duration-150 active:scale-[0.98]"
              >
                <span>{t("landing.cta.repo")}</span>
                <ExternalLinkIcon size={14} className="ml-2 opacity-70" />
              </a>
            </div>
          </div>
        </div>

        {/* Minimal Footer */}
        <div className="mt-16 pt-8 border-t border-neutral-200 dark:border-neutral-800 flex flex-col sm:flex-row items-center justify-between gap-6 text-xs text-neutral-500 dark:text-neutral-400 font-mono">
          <div className="flex items-center gap-3">
            <CanguPayLogo className="h-5 w-auto" />
            <span>{t("landing.footer.rights")}</span>
          </div>

          <div className="flex flex-wrap items-center gap-6">
            <a
              href="https://github.com/gonnnzaDev/CanguPay/blob/main/docs/ruleset.md"
              target="_blank"
              rel="noopener noreferrer"
              className="hover:text-neutral-900 dark:hover:text-neutral-100 transition-colors"
            >
              {t("landing.footer.ruleset")}
            </a>
            <a
              href="https://github.com/gonnnzaDev/CanguPay/blob/main/docs/state-machine.md"
              target="_blank"
              rel="noopener noreferrer"
              className="hover:text-neutral-900 dark:hover:text-neutral-100 transition-colors"
            >
              {t("landing.footer.statemachine")}
            </a>
            <span className="text-neutral-400 dark:text-neutral-600">
              {t("landing.footer.mit")}
            </span>
          </div>
        </div>
      </div>
    </footer>
  );
}
