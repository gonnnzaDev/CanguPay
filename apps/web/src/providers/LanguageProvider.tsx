"use client";

import React, {
  createContext,
  useContext,
  useEffect,
  useCallback,
  useMemo,
  useSyncExternalStore,
} from "react";
import { es, en, getTranslation, LocaleDictionary, Language } from "@/locales";

export type { Language, LocaleDictionary };

export interface LanguageContextType {
  language: Language;
  setLanguage: (lang: Language) => void;
  t: (path: string, params?: Record<string, string | number>) => string;
  dict: LocaleDictionary;
}

const LanguageContext = createContext<LanguageContextType | undefined>(undefined);

const STORAGE_KEY = "cangupay_lang";

let languageListeners: Array<() => void> = [];

function emitLanguageChange() {
  for (const listener of languageListeners) {
    listener();
  }
}

function subscribeToLanguage(callback: () => void): () => void {
  languageListeners.push(callback);

  const handleStorage = (e: StorageEvent) => {
    if (e.key === STORAGE_KEY) {
      callback();
    }
  };
  window.addEventListener("storage", handleStorage);

  return () => {
    languageListeners = languageListeners.filter((l) => l !== callback);
    window.removeEventListener("storage", handleStorage);
  };
}

function getLanguageSnapshot(): Language {
  try {
    const stored = localStorage.getItem(STORAGE_KEY);
    if (stored === "es" || stored === "en") {
      return stored;
    }

    if (typeof navigator !== "undefined" && navigator.language) {
      const browserLang = navigator.language.toLowerCase();
      if (browserLang.startsWith("en")) {
        return "en";
      }
    }
  } catch {
    // Ignore storage access errors
  }
  return "es";
}

function getServerSnapshot(): Language {
  return "es";
}

export function LanguageProvider({ children }: { children: React.ReactNode }) {
  const language = useSyncExternalStore(
    subscribeToLanguage,
    getLanguageSnapshot,
    getServerSnapshot
  );

  useEffect(() => {
    if (typeof document !== "undefined") {
      document.documentElement.lang = language;
    }
  }, [language]);

  const setLanguage = useCallback((newLang: Language) => {
    try {
      localStorage.setItem(STORAGE_KEY, newLang);
    } catch {
      // Ignore storage access errors
    }
    if (typeof document !== "undefined") {
      document.documentElement.lang = newLang;
    }
    emitLanguageChange();
  }, []);

  const dict = useMemo(() => (language === "en" ? en : es), [language]);

  const t = useCallback(
    (path: string, params?: Record<string, string | number>): string => {
      return getTranslation(dict, path, params);
    },
    [dict]
  );

  const value = useMemo(
    () => ({
      language,
      setLanguage,
      t,
      dict,
    }),
    [language, setLanguage, t, dict]
  );

  return (
    <LanguageContext.Provider value={value}>
      {children}
    </LanguageContext.Provider>
  );
}

export function useLanguage(): LanguageContextType {
  const context = useContext(LanguageContext);
  if (!context) {
    throw new Error("useLanguage must be used within a LanguageProvider");
  }
  return context;
}
