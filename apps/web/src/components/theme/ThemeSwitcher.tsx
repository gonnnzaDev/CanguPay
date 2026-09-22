"use client";

import React, { useState } from "react";
import { useTheme, Theme } from "@/providers/ThemeProvider";
import { SunIcon, MoonIcon, MonitorIcon } from "@/components/icons";

export function ThemeSwitcher() {
  const { theme, setTheme } = useTheme();
  const [isHovered, setIsHovered] = useState(false);

  const options: { value: Theme; label: string; icon: React.ReactNode }[] = [
    { value: "light", label: "Claro", icon: <SunIcon size={13} /> },
    { value: "system", label: "Auto", icon: <MonitorIcon size={13} /> },
    { value: "dark", label: "Oscuro", icon: <MoonIcon size={13} /> },
  ];

  const currentOption = options.find((o) => o.value === theme) || options[1];

  return (
    <div
      role="radiogroup"
      aria-label="Selector de tema"
      onMouseEnter={() => setIsHovered(true)}
      onMouseLeave={() => setIsHovered(false)}
      onFocus={() => setIsHovered(true)}
      onBlur={(e) => {
        if (!e.currentTarget.contains(e.relatedTarget)) {
          setIsHovered(false);
        }
      }}
      tabIndex={0}
      className="flex items-center p-0.5 rounded-lg border border-neutral-200 dark:border-neutral-800 bg-neutral-100/70 dark:bg-neutral-900/70 shadow-2xs font-mono transition-all duration-300 ease-out cursor-pointer select-none"
    >
      {/* Minimized Icon (Visible when not hovered) */}
      <div
        className={`flex items-center justify-center transition-all duration-300 overflow-hidden ${
          isHovered
            ? "w-0 opacity-0 pointer-events-none"
            : "w-7 h-7 opacity-100 text-neutral-700 dark:text-neutral-200"
        }`}
        title={`Tema: ${currentOption.label} (pasar el cursor para cambiar)`}
      >
        {currentOption.icon}
      </div>

      {/* Maximized Horizontal Menu (Reveals on hover: icons only) */}
      <div
        className={`flex items-center gap-0.5 transition-all duration-300 overflow-hidden ${
          isHovered
            ? "max-w-[120px] opacity-100"
            : "max-w-0 opacity-0 pointer-events-none"
        }`}
      >
        {options.map((option) => {
          const isActive = theme === option.value;
          return (
            <button
              key={option.value}
              type="button"
              role="radio"
              aria-checked={isActive}
              aria-label={`Tema ${option.label}`}
              title={`Tema ${option.label}`}
              onClick={(e) => {
                e.stopPropagation();
                setTheme(option.value);
              }}
              className={`flex items-center justify-center w-7 h-7 rounded-md transition-all cursor-pointer ${
                isActive
                  ? "bg-white dark:bg-neutral-800 text-neutral-950 dark:text-neutral-50 shadow-xs"
                  : "text-neutral-400 hover:text-neutral-900 dark:hover:text-neutral-100 hover:bg-neutral-200/50 dark:hover:bg-neutral-800/50"
              }`}
            >
              {option.icon}
            </button>
          );
        })}
      </div>
    </div>
  );
}
