"use client";

import React, { useState } from "react";
import Link from "next/link";
import { CanguPayLogo, ExternalLinkIcon } from "@/components/icons";
import { ThemeSwitcher } from "@/components/theme/ThemeSwitcher";
import { LanguageSwitcher } from "@/components/theme/LanguageSwitcher";
import { useLanguage } from "@/providers/LanguageProvider";

export function LandingHeader() {
  const { t } = useLanguage();
  const [mobileMenuOpen, setMobileMenuOpen] = useState(false);

  return (
    <header className="sticky top-0 z-50 w-full transition-all duration-300">
      {/* Background with blur and subtle border */}
      <div className="absolute inset-0 bg-white/75 dark:bg-neutral-950/80 backdrop-blur-md border-b border-neutral-200/60 dark:border-neutral-800/60 transition-colors" />

      <div className="relative max-w-7xl mx-auto px-3.5 sm:px-6 lg:px-8 h-16 flex items-center justify-between gap-2 sm:gap-4">
        {/* Brand identity */}
        <div className="flex items-center gap-2 sm:gap-3 shrink-0">
          <Link
            href="/landing"
            className="flex items-center gap-2 focus:outline-none focus-visible:ring-2 focus-visible:ring-teal-500 rounded-md p-0.5"
            aria-label="CanguPay Home"
          >
            <CanguPayLogo className="h-6 sm:h-7 w-auto" />
          </Link>
          <span className="hidden sm:inline-flex items-center px-2 py-0.5 rounded text-[11px] font-mono font-medium border border-teal-500/20 bg-teal-500/10 text-teal-700 dark:text-teal-400">
            TESTNET // P-28
          </span>
        </div>

        {/* Desktop Navigation */}
        <nav
          className="hidden md:flex items-center gap-6 text-sm font-medium text-neutral-600 dark:text-neutral-400"
          aria-label="Navegación principal"
        >
          <a
            href="#flow"
            className="hover:text-neutral-950 dark:hover:text-neutral-100 transition-colors"
          >
            {t("landing.nav.flow")}
          </a>
          <a
            href="#architecture"
            className="hover:text-neutral-950 dark:hover:text-neutral-100 transition-colors"
          >
            {t("landing.nav.architecture")}
          </a>
          <a
            href="#simulation"
            className="hover:text-neutral-950 dark:hover:text-neutral-100 transition-colors"
          >
            {t("landing.nav.simulation")}
          </a>
          <a
            href="https://github.com/gonnnzaDev/CanguPay"
            target="_blank"
            rel="noopener noreferrer"
            className="inline-flex items-center gap-1.5 hover:text-neutral-950 dark:hover:text-neutral-100 transition-colors"
          >
            <span>{t("landing.nav.github")}</span>
            <ExternalLinkIcon size={12} className="opacity-70" />
          </a>
        </nav>

        {/* Right utilities & primary action */}
        <div className="flex items-center gap-2 sm:gap-3 shrink-0">
          <div className="hidden sm:flex items-center gap-2">
            <ThemeSwitcher />
            <LanguageSwitcher />
          </div>

          <Link
            href="/"
            className="inline-flex items-center justify-center px-3 py-1.5 sm:px-4 sm:py-2 text-xs sm:text-sm font-semibold rounded-lg bg-[#0E2F46] hover:bg-[#0c273a] text-white dark:bg-[#0EAFA2] dark:hover:bg-[#0d9d92] dark:text-neutral-950 shadow-xs transition-all duration-150 active:scale-[0.98] focus:outline-none focus-visible:ring-2 focus-visible:ring-teal-500"
          >
            <span>{t("landing.nav.open_app")}</span>
            <span className="ml-1 sm:ml-1.5 font-mono text-xs">→</span>
          </Link>

          {/* Mobile hamburger toggle */}
          <button
            type="button"
            onClick={() => setMobileMenuOpen(!mobileMenuOpen)}
            className="md:hidden p-1.5 rounded-md text-neutral-600 dark:text-neutral-400 hover:text-neutral-900 dark:hover:text-neutral-100 focus:outline-none cursor-pointer"
            aria-label="Abrir menú"
          >
            <svg
              className="w-5 h-5"
              fill="none"
              viewBox="0 0 24 24"
              stroke="currentColor"
            >
              {mobileMenuOpen ? (
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
              ) : (
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M4 6h16M4 12h16M4 18h16" />
              )}
            </svg>
          </button>
        </div>
      </div>

      {/* Mobile drop menu */}
      {mobileMenuOpen && (
        <div className="md:hidden relative bg-white/95 dark:bg-neutral-950/95 backdrop-blur-md border-b border-neutral-200 dark:border-neutral-800 px-4 pt-3 pb-5 space-y-3 text-sm font-medium">
          <a
            href="#flow"
            onClick={() => setMobileMenuOpen(false)}
            className="block py-1.5 text-neutral-700 dark:text-neutral-300 hover:text-teal-600 dark:hover:text-teal-400"
          >
            {t("landing.nav.flow")}
          </a>
          <a
            href="#architecture"
            onClick={() => setMobileMenuOpen(false)}
            className="block py-1.5 text-neutral-700 dark:text-neutral-300 hover:text-teal-600 dark:hover:text-teal-400"
          >
            {t("landing.nav.architecture")}
          </a>
          <a
            href="#simulation"
            onClick={() => setMobileMenuOpen(false)}
            className="block py-1.5 text-neutral-700 dark:text-neutral-300 hover:text-teal-600 dark:hover:text-teal-400"
          >
            {t("landing.nav.simulation")}
          </a>
          <a
            href="https://github.com/gonnnzaDev/CanguPay"
            target="_blank"
            rel="noopener noreferrer"
            className="flex items-center gap-1.5 py-1.5 text-neutral-700 dark:text-neutral-300 hover:text-teal-600 dark:hover:text-teal-400"
          >
            <span>{t("landing.nav.github")}</span>
            <ExternalLinkIcon size={12} className="opacity-70" />
          </a>

          <div className="pt-3 border-t border-neutral-200 dark:border-neutral-800 flex items-center justify-between">
            <span className="text-xs font-mono text-neutral-500">Preferencias:</span>
            <div className="flex items-center gap-2">
              <ThemeSwitcher />
              <LanguageSwitcher />
            </div>
          </div>
        </div>
      )}
    </header>
  );
}
