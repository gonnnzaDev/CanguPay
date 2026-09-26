"use client";

import React from "react";
import { useLanguage } from "@/providers/LanguageProvider";
import { CpuIcon, ShieldLockIcon, ScaleIcon } from "@/components/icons";

export function LandingTriad() {
  const { t } = useLanguage();

  const triad = [
    {
      code: "01",
      title: t("landing.triad.agent_title"),
      role: t("landing.triad.agent_role"),
      desc: t("landing.triad.agent_desc"),
      icon: <CpuIcon size={24} className="text-teal-600 dark:text-teal-400" />,
      accent: "border-teal-500/30 text-teal-600 dark:text-teal-400",
      spec: "Rule engine · SHA-256 matching · Zero custody",
    },
    {
      code: "02",
      title: t("landing.triad.contract_title"),
      role: t("landing.triad.contract_role"),
      desc: t("landing.triad.contract_desc"),
      icon: <ShieldLockIcon size={24} className="text-teal-600 dark:text-teal-400" />,
      accent: "border-teal-500/30 text-teal-600 dark:text-teal-400",
      spec: "Soroban / Rust · SAC CPUSD · env.ledger().timestamp()",
    },
    {
      code: "03",
      title: t("landing.triad.human_title"),
      role: t("landing.triad.human_role"),
      desc: t("landing.triad.human_desc"),
      icon: <ScaleIcon size={24} className="text-[#F3AC44]" />,
      accent: "border-amber-500/30 text-[#F3AC44]",
      spec: "Independent arbiter · 1 dispute max · Release / Refund / Split",
    },
  ];

  return (
    <section id="architecture" className="py-20 sm:py-28 border-t border-neutral-200/60 dark:border-neutral-800/60 relative">
      <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8">
        {/* Section Header */}
        <div className="text-center max-w-3xl mx-auto">
          <div className="inline-flex items-center gap-2 px-2.5 py-1 rounded-md border border-neutral-300 dark:border-neutral-800 bg-neutral-100/70 dark:bg-neutral-900/70 font-mono text-[11px] text-neutral-600 dark:text-neutral-400 uppercase tracking-wider mb-4">
            <span>{t("landing.triad.tag")}</span>
          </div>
          <blockquote className="text-2xl sm:text-3xl lg:text-4xl font-extrabold text-neutral-950 dark:text-white leading-tight font-sans">
            &ldquo;{t("landing.triad.quote")}&rdquo;
          </blockquote>
        </div>

        {/* 3 Pillars */}
        <div className="mt-14 sm:mt-20 grid grid-cols-1 md:grid-cols-3 gap-8">
          {triad.map((pillar) => (
            <div
              key={pillar.code}
              className="p-8 rounded-2xl border border-neutral-200 dark:border-neutral-800 bg-white/70 dark:bg-neutral-900/70 backdrop-blur-xs flex flex-col justify-between hover:border-neutral-300 dark:hover:border-neutral-700 transition-colors"
            >
              <div>
                <div className="flex items-center justify-between mb-6">
                  <div className="p-3 rounded-xl bg-neutral-100 dark:bg-neutral-800 shrink-0">
                    {pillar.icon}
                  </div>
                  <span className="font-mono text-xs font-bold text-neutral-400 dark:text-neutral-500 tracking-wider">
                    ROLE // {pillar.code}
                  </span>
                </div>

                <div className="flex items-baseline gap-2 mb-2">
                  <h3 className="text-xl font-extrabold text-neutral-950 dark:text-neutral-50 font-mono tracking-tight">
                    {pillar.title}
                  </h3>
                  <span className={`text-xs font-mono font-bold uppercase ${pillar.accent}`}>
                    · {pillar.role}
                  </span>
                </div>

                <p className="text-sm text-neutral-600 dark:text-neutral-400 leading-relaxed mt-3">
                  {pillar.desc}
                </p>
              </div>

              <div className="mt-8 pt-4 border-t border-neutral-200/50 dark:border-neutral-800/60 font-mono text-[11px] text-neutral-500">
                {pillar.spec}
              </div>
            </div>
          ))}
        </div>
      </div>
    </section>
  );
}
