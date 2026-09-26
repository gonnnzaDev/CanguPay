"use client";

import React, { useState } from "react";
import Link from "next/link";
import { useLanguage } from "@/providers/LanguageProvider";
import {
  ClockIcon,
  UserIcon,
  PackageIcon,
  ScaleIcon,
} from "@/components/icons";

interface ScenarioState {
  id: "pass" | "ghost" | "dispute";
  labelKey: string;
  contractId: string;
  status: string;
  statusColor: string;
  amount: string;
  buyer: string;
  supplier: string;
  resolver: string;
  activeDeadline: string;
  outcomeDesc: string;
}

export function LandingProductPreview() {
  const { t } = useLanguage();
  const [activeScenario, setActiveScenario] = useState<"pass" | "ghost" | "dispute">("pass");

  const scenarios: Record<"pass" | "ghost" | "dispute", ScenarioState> = {
    pass: {
      id: "pass",
      labelKey: "landing.preview.scenario_pass",
      contractId: "CAVLH35BXYEMBIF6WAPVEMWZQJ2MLEH6HB5SQE4IX77ABMBUOOUS4MRH",
      status: "RELEASED",
      statusColor: "border-emerald-500/30 text-emerald-700 dark:text-emerald-400 bg-emerald-500/10",
      amount: "15,000.00 CPUSD",
      buyer: "GDWTZTTYQY3KDRNMSOGEQ5W2K7SFFJ35X6H2G3AILXJ5SCL9W4EDX",
      supplier: "CAVLH35BXYEMBIF6WAPVEMWZQJ2MLEH6HB5SQE4IX77ABMBUOOUS4",
      resolver: "GB35UO2BS67T5G7UQQ7X5P4J56S7Z2ZKJ4H8L4B9X3D2R7S4D9",
      activeDeadline: "Cumplido (Aprobación sin objeción)",
      outcomeDesc: "Fondo liberado al proveedor por atestación exitosa y expiración de plazo sin controversia.",
    },
    ghost: {
      id: "ghost",
      labelKey: "landing.preview.scenario_ghost",
      contractId: "CD4GXNR7F5MSFZGJ6SMMNRXTICTUYRRM2UV5UNDKXZXDW3ORKR275YXD",
      status: "REFUNDED",
      statusColor: "border-amber-500/30 text-amber-700 dark:text-amber-400 bg-amber-500/10",
      amount: "15,000.00 CPUSD",
      buyer: "GDWTZTTYQY3KDRNMSOGEQ5W2K7SFFJ35X6H2G3AILXJ5SCL9W4EDX",
      supplier: "CAVLH35BXYEMBIF6WAPVEMWZQJ2MLEH6HB5SQE4IX77ABMBUOOUS4",
      resolver: "GB35UO2BS67T5G7UQQ7X5P4J56S7Z2ZKJ4H8L4B9X3D2R7S4D9",
      activeDeadline: "Vencido en Ledger #5128950 (SUBMISSION_TIMEOUT)",
      outcomeDesc: "El proveedor no presentó evidencia en plazo; finalize() reembolsó el 100% al comprador.",
    },
    dispute: {
      id: "dispute",
      labelKey: "landing.preview.scenario_dispute",
      contractId: "CB6SQ2N2IWDS36HVDF7CXB2RPVJVJLCYEKERR5EOCEEJR656F7TH4OJY",
      status: "SPLIT",
      statusColor: "border-purple-500/30 text-purple-700 dark:text-purple-400 bg-purple-500/10",
      amount: "15,000.00 CPUSD (Reparto 70 / 30)",
      buyer: "GDWTZTTYQY3KDRNMSOGEQ5W2K7SFFJ35X6H2G3AILXJ5SCL9W4EDX",
      supplier: "CAVLH35BXYEMBIF6WAPVEMWZQJ2MLEH6HB5SQE4IX77ABMBUOOUS4",
      resolver: "GB35UO2BS67T5G7UQQ7X5P4J56S7Z2ZKJ4H8L4B9X3D2R7S4D9",
      activeDeadline: "Resuelto por Árbitro en Ledger #5128920",
      outcomeDesc: "Disputa resuelta con distribución: 10,500 CPUSD a proveedor y 4,500 CPUSD reembolsados a comprador.",
    },
  };

  const current = scenarios[activeScenario];

  return (
    <section id="simulation" className="py-20 sm:py-28 border-t border-neutral-200/60 dark:border-neutral-800/60 relative">
      <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8">
        {/* Section Header */}
        <div className="max-w-2xl mb-10">
          <div className="inline-flex items-center gap-2 px-2.5 py-1 rounded-md border border-neutral-300 dark:border-neutral-800 bg-neutral-100/70 dark:bg-neutral-900/70 font-mono text-[11px] text-neutral-600 dark:text-neutral-400 uppercase tracking-wider mb-4">
            <span>{t("landing.preview.tag")}</span>
          </div>
          <h2 className="text-3xl sm:text-4xl lg:text-5xl font-extrabold tracking-tight text-neutral-950 dark:text-white leading-tight">
            {t("landing.preview.title")}
          </h2>
          <p className="mt-3 text-base sm:text-lg text-neutral-600 dark:text-neutral-400 leading-relaxed">
            {t("landing.preview.subtitle")}
          </p>
        </div>

        {/* Scenario Switcher Tabs */}
        <div className="flex flex-wrap gap-2 mb-8">
          {(["pass", "ghost", "dispute"] as const).map((scKey) => {
            const isActive = activeScenario === scKey;
            return (
              <button
                key={scKey}
                type="button"
                onClick={() => setActiveScenario(scKey)}
                className={`px-4 py-2 rounded-xl text-xs sm:text-sm font-semibold font-mono transition-all duration-200 cursor-pointer ${
                  isActive
                    ? "bg-[#0E2F46] text-white dark:bg-[#0EAFA2] dark:text-neutral-950 shadow-xs"
                    : "border border-neutral-200 dark:border-neutral-800 bg-white/70 dark:bg-neutral-900/70 text-neutral-600 dark:text-neutral-400 hover:text-neutral-900 dark:hover:text-neutral-100"
                }`}
              >
                {t(scenarios[scKey].labelKey)}
              </button>
            );
          })}
        </div>

        {/* Blueprint Visual Card */}
        <div className="rounded-2xl border border-neutral-200 dark:border-neutral-800 bg-white/80 dark:bg-neutral-900/80 shadow-md backdrop-blur-xs p-6 sm:p-8">
          <div className="flex flex-col lg:flex-row lg:items-center justify-between gap-4 pb-6 border-b border-neutral-200/80 dark:border-neutral-800/80">
            <div>
              <div className="flex items-center gap-2 mb-2">
                <span className={`inline-flex items-center px-2 py-0.5 rounded text-xs font-mono font-semibold border ${current.statusColor}`}>
                  {current.status}
                </span>
                <span className="font-mono text-xs text-neutral-500">
                  Stellar Testnet Instance
                </span>
              </div>
              <p className="font-mono text-xs text-neutral-600 dark:text-neutral-400 break-all select-all">
                ID: {current.contractId}
              </p>
            </div>

            <div className="text-left lg:text-right">
              <span className="font-mono text-xs text-neutral-500 uppercase tracking-wider block">
                Monto Custodiado
              </span>
              <span className="font-mono text-2xl font-bold text-neutral-950 dark:text-white">
                {current.amount}
              </span>
            </div>
          </div>

          {/* Grid of contract parameters */}
          <div className="grid grid-cols-1 md:grid-cols-3 gap-6 py-6 border-b border-neutral-200/80 dark:border-neutral-800/80 text-xs font-mono">
            <div className="p-4 rounded-xl bg-neutral-50 dark:bg-neutral-950/60 border border-neutral-200/60 dark:border-neutral-800/60 space-y-1">
              <span className="flex items-center gap-1.5 text-neutral-500">
                <UserIcon size={14} className="text-teal-600 dark:text-teal-400" />
                <span>COMPRADOR (BUYER)</span>
              </span>
              <p className="font-semibold text-neutral-900 dark:text-neutral-100 truncate">
                {current.buyer.slice(0, 12)}...{current.buyer.slice(-8)}
              </p>
            </div>

            <div className="p-4 rounded-xl bg-neutral-50 dark:bg-neutral-950/60 border border-neutral-200/60 dark:border-neutral-800/60 space-y-1">
              <span className="flex items-center gap-1.5 text-neutral-500">
                <PackageIcon size={14} className="text-teal-600 dark:text-teal-400" />
                <span>PROVEEDOR (SUPPLIER)</span>
              </span>
              <p className="font-semibold text-neutral-900 dark:text-neutral-100 truncate">
                {current.supplier.slice(0, 12)}...{current.supplier.slice(-8)}
              </p>
            </div>

            <div className="p-4 rounded-xl bg-neutral-50 dark:bg-neutral-950/60 border border-neutral-200/60 dark:border-neutral-800/60 space-y-1">
              <span className="flex items-center gap-1.5 text-neutral-500">
                <ScaleIcon size={14} className="text-[#F3AC44]" />
                <span>ÁRBITRO (RESOLVER)</span>
              </span>
              <p className="font-semibold text-neutral-900 dark:text-neutral-100 truncate">
                {current.resolver.slice(0, 12)}...{current.resolver.slice(-8)}
              </p>
            </div>
          </div>

          {/* Outcome explanation & CTA */}
          <div className="pt-6 flex flex-col sm:flex-row sm:items-center justify-between gap-4">
            <div className="max-w-xl">
              <span className="text-xs font-mono text-neutral-500 flex items-center gap-1.5 mb-1">
                <ClockIcon size={12} />
                <span>{current.activeDeadline}</span>
              </span>
              <p className="text-sm text-neutral-700 dark:text-neutral-300">
                {current.outcomeDesc}
              </p>
            </div>

            <Link
              href="/"
              className="inline-flex items-center justify-center px-4 py-2.5 rounded-xl font-semibold text-xs sm:text-sm bg-neutral-950 hover:bg-neutral-800 text-white dark:bg-neutral-100 dark:hover:bg-neutral-200 dark:text-neutral-950 transition-colors shrink-0"
            >
              <span>{t("landing.preview.open_in_app")}</span>
              <span className="ml-2 font-mono">→</span>
            </Link>
          </div>
        </div>
      </div>
    </section>
  );
}
