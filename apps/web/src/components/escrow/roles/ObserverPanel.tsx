"use client";

import React from "react";
import { EyeIcon } from "@/components/icons";
import { useLanguage } from "@/providers/LanguageProvider";

export const ObserverPanel: React.FC = () => {
  const { t } = useLanguage();

  return (
    <div className="p-3 rounded-xl border border-neutral-200 dark:border-neutral-800 bg-neutral-100/60 dark:bg-neutral-900/80 shadow-xs font-mono text-xs flex items-center gap-2.5">
      <div className="p-1.5 rounded-lg bg-neutral-200/80 dark:bg-neutral-800 text-neutral-600 dark:text-neutral-400 shrink-0">
        <EyeIcon size={16} />
      </div>
      <span className="text-xs text-neutral-600 dark:text-neutral-400">{t("roles.observer.banner")}</span>
    </div>
  );
};
