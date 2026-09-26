"use client";

import React from "react";
import { useLanguage } from "@/providers/LanguageProvider";

export function LandingStellar() {
  const { t } = useLanguage();

  const tech = [
    {
      name: "STELLAR",
      protocol: "PROTOCOL 28",
      desc: t("landing.stellar.stellar_desc"),
    },
    {
      name: "SOROBAN",
      protocol: "RUST / WASM",
      desc: t("landing.stellar.soroban_desc"),
    },
    {
      name: "FREIGHTER",
      protocol: "ROLE WALLET",
      desc: t("landing.stellar.freighter_desc"),
    },
    {
      name: "TESTNET",
      protocol: "CPUSD TOKEN",
      desc: t("landing.stellar.testnet_desc"),
    },
  ];

  return (
    <section className="py-16 sm:py-20 border-t border-neutral-200/60 dark:border-neutral-800/60 bg-neutral-100/40 dark:bg-neutral-900/30">
      <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8">
        <div className="flex flex-col md:flex-row md:items-center justify-between gap-6 mb-10">
          <div>
            <div className="inline-flex items-center gap-2 px-2 py-0.5 rounded font-mono text-[10px] text-teal-700 dark:text-teal-400 border border-teal-500/20 bg-teal-500/10 uppercase tracking-widest mb-2">
              <span>{t("landing.stellar.tag")}</span>
            </div>
            <h2 className="text-xl sm:text-2xl font-extrabold text-neutral-950 dark:text-white">
              {t("landing.stellar.title")}
            </h2>
          </div>
          <p className="text-xs font-mono text-neutral-500 max-w-sm">
            Auditable smart contract instances running on official Soroban Testnet RPC.
          </p>
        </div>

        <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4 sm:gap-6">
          {tech.map((item) => (
            <div
              key={item.name}
              className="p-5 rounded-xl border border-neutral-200/80 dark:border-neutral-800/80 bg-white/70 dark:bg-neutral-900/70 backdrop-blur-xs flex flex-col justify-between"
            >
              <div>
                <div className="flex items-center justify-between mb-3">
                  <span className="font-mono text-sm font-extrabold tracking-wider text-neutral-900 dark:text-neutral-100">
                    {item.name}
                  </span>
                  <span className="font-mono text-[10px] font-semibold text-teal-700 dark:text-teal-400 px-1.5 py-0.5 rounded bg-teal-500/10 border border-teal-500/20">
                    {item.protocol}
                  </span>
                </div>
                <p className="text-xs text-neutral-600 dark:text-neutral-400 leading-relaxed">
                  {item.desc}
                </p>
              </div>
            </div>
          ))}
        </div>
      </div>
    </section>
  );
}
