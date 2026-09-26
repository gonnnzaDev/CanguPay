"use client";

import React from "react";
import { useLanguage } from "@/providers/LanguageProvider";
import { ShieldLockIcon, FileTextIcon, ScaleIcon } from "@/components/icons";

export function LandingFlow() {
  const { t } = useLanguage();

  const steps = [
    {
      num: "01",
      badge: t("landing.flow.step_1_name"),
      title: t("landing.flow.step_1_title"),
      desc: t("landing.flow.step_1_desc"),
      icon: <ShieldLockIcon size={20} className="text-teal-600 dark:text-teal-400" />,
      tag: "CPUSD SAC · Non-custodial lock",
    },
    {
      num: "02",
      badge: t("landing.flow.step_2_name"),
      title: t("landing.flow.step_2_title"),
      desc: t("landing.flow.step_2_desc"),
      icon: <FileTextIcon size={20} className="text-[#F3AC44]" />,
      tag: "SHA-256 Hash · Rule Engine PASS/FAIL",
    },
    {
      num: "03",
      badge: t("landing.flow.step_3_name"),
      title: t("landing.flow.step_3_title"),
      desc: t("landing.flow.step_3_desc"),
      icon: <ScaleIcon size={20} className="text-teal-600 dark:text-teal-400" />,
      tag: "Permissionless finalize() · Ledger Deadlines",
    },
  ];

  return (
    <section id="flow" className="py-20 sm:py-28 border-t border-neutral-200/60 dark:border-neutral-800/60 relative">
      <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8">
        {/* Section Header */}
        <div className="max-w-2xl">
          <div className="inline-flex items-center gap-2 px-2.5 py-1 rounded-md border border-teal-500/20 bg-teal-500/10 font-mono text-[11px] text-teal-700 dark:text-teal-400 uppercase tracking-wider mb-4">
            <span>{t("landing.flow.tag")}</span>
          </div>
          <h2 className="text-3xl sm:text-4xl lg:text-5xl font-extrabold tracking-tight text-neutral-950 dark:text-white leading-tight">
            {t("landing.flow.title")}
          </h2>
          <p className="mt-3 text-base sm:text-lg text-neutral-600 dark:text-neutral-400 leading-relaxed">
            {t("landing.flow.subtitle")}
          </p>
        </div>

        {/* The 3 Rail Cards */}
        <div className="mt-12 sm:mt-16 grid grid-cols-1 md:grid-cols-3 gap-6 sm:gap-8 relative">
          {steps.map((s) => (
            <div
              key={s.num}
              className="relative p-6 sm:p-8 rounded-2xl border border-neutral-200 dark:border-neutral-800 bg-white/60 dark:bg-neutral-900/60 backdrop-blur-xs shadow-xs hover:border-teal-500/40 dark:hover:border-teal-400/40 transition-all duration-200 group flex flex-col justify-between"
            >
              <div>
                {/* Step Top Bar */}
                <div className="flex items-center justify-between mb-6">
                  <span className="font-mono text-xs font-bold text-teal-600 dark:text-teal-400 px-2 py-0.5 rounded bg-teal-500/10 border border-teal-500/20">
                    {s.badge}
                  </span>
                  <span className="font-mono text-2xl font-black text-neutral-300 dark:text-neutral-700 select-none">
                    {s.num}
                  </span>
                </div>

                {/* Icon & Title */}
                <div className="flex items-center gap-3 mb-3">
                  <div className="p-2 rounded-lg bg-neutral-100 dark:bg-neutral-800 shrink-0">
                    {s.icon}
                  </div>
                  <h3 className="text-lg sm:text-xl font-bold text-neutral-900 dark:text-neutral-100">
                    {s.title}
                  </h3>
                </div>

                {/* Description */}
                <p className="text-sm text-neutral-600 dark:text-neutral-400 leading-relaxed">
                  {s.desc}
                </p>
              </div>

              {/* Technical Footnote Tag */}
              <div className="mt-6 pt-4 border-t border-neutral-200/50 dark:border-neutral-800/60 font-mono text-[11px] text-neutral-500 dark:text-neutral-400 flex items-center gap-1.5">
                <span className="w-1.5 h-1.5 rounded-full bg-neutral-400 dark:bg-neutral-600" />
                <span className="truncate">{s.tag}</span>
              </div>
            </div>
          ))}
        </div>
      </div>
    </section>
  );
}
