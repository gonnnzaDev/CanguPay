"use client";

import React from "react";
import { useLanguage, Language } from "@/providers/LanguageProvider";

export function LanguageSwitcher() {
  const { language, setLanguage, t } = useLanguage();

  const options: { value: Language; label: string; ariaLabel: string }[] = [
    { value: "es", label: "ES", ariaLabel: t("language.es") || "Español" },
    { value: "en", label: "EN", ariaLabel: t("language.en") || "English" },
  ];

  const handleKeyDown = (
    e: React.KeyboardEvent<HTMLButtonElement>,
    currentIndex: number
  ) => {
    if (e.key === "ArrowRight" || e.key === "ArrowLeft") {
      e.preventDefault();
      const nextIndex = currentIndex === 0 ? 1 : 0;
      setLanguage(options[nextIndex].value);
    }
  };

  return (
    <div
      role="radiogroup"
      aria-label={t("language.switch_language") || "Switch language"}
      className="flex items-center p-0.5 rounded-lg border border-neutral-200 dark:border-neutral-800 bg-neutral-100/70 dark:bg-neutral-900/70 shadow-2xs font-mono text-xs select-none transition-colors"
    >
      {options.map((option, idx) => {
        const isActive = language === option.value;
        return (
          <button
            key={option.value}
            type="button"
            role="radio"
            aria-checked={isActive}
            aria-label={option.ariaLabel}
            title={option.ariaLabel}
            onClick={() => setLanguage(option.value)}
            onKeyDown={(e) => handleKeyDown(e, idx)}
            className={`px-2 py-1 text-[11px] font-bold rounded-md transition-all duration-150 hover:scale-105 active:scale-95 cursor-pointer focus-visible:ring-2 focus-visible:ring-teal-500/50 focus-visible:outline-none ${
              isActive
                ? "bg-white dark:bg-neutral-800 text-neutral-950 dark:text-neutral-50 shadow-xs"
                : "text-neutral-400 hover:text-neutral-800 dark:hover:text-neutral-200 hover:bg-neutral-200/50 dark:hover:bg-neutral-800/50"
            }`}
          >
            {option.label}
          </button>
        );
      })}
    </div>
  );
}
