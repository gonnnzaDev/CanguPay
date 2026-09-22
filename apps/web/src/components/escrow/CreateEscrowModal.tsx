"use client";

import React, { useState, useEffect } from "react";
import {
  CloseIcon,
  ShieldLockIcon,
  AlertCircleIcon,
  CheckIcon,
} from "@/components/icons";
import { FallbackOutcome } from "@/types/escrow";

interface CreateEscrowModalProps {
  isOpen: boolean;
  onClose: () => void;
  buyerAddress?: string | null;
  onCreated?: (newEscrowParams: {
    buyer: string;
    supplier: string;
    resolver: string;
    engine: string;
    token: string;
    amount: string;
    fallbackOutcome: FallbackOutcome;
    fallbackSplitBps: number;
  }) => void;
}

function isValidStellarAddress(addr: string): boolean {
  if (!addr) return false;
  const trimmed = addr.trim();
  // Valid Stellar Public Key starts with 'G', uppercase alphanumeric, typically 56 chars (supports >=40 for dev keys)
  return trimmed.startsWith("G") && (trimmed.length === 56 || trimmed.length >= 40) && /^[A-Z0-9]+$/.test(trimmed);
}

export const CreateEscrowModal: React.FC<CreateEscrowModalProps> = ({
  isOpen,
  onClose,
  buyerAddress,
  onCreated,
}) => {
  const defaultBuyer = buyerAddress || "GBUYER4X9Z2K1L3M4N5O6P7Q8R9S0T1U2V3W4X9Z";
  const [buyerInput, setBuyerInput] = useState<string | null>(null);
  const buyer = buyerInput ?? defaultBuyer;

  const [supplier, setSupplier] = useState("GSUPPLIER8K2L3M4N5O6P7Q8R9S0T1U2V3W4X8K2L");
  const [resolver, setResolver] = useState("GRESOLVER9P0R1S2T3U4V5W6X7Y8Z9A0B1C2D9P0R");
  const [engine, setEngine] = useState("GENGINE1V3M4N5O6P7Q8R9S0T1U2V3W4X5Y6Z1V3M");
  const [token, setToken] = useState("CCPUSDT5K7SOROBANTOKENIDTESTNETCANGUPAY2026");
  const [amount, setAmount] = useState("10,000.0000000");
  const [fallbackOutcome, setFallbackOutcome] = useState<FallbackOutcome>("SPLIT");
  const [fallbackSplitBps, setFallbackSplitBps] = useState<number>(5000);
  const [submitted, setSubmitted] = useState(false);

  // Handle ESC key
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape" && isOpen) {
        onClose();
      }
    };
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [isOpen, onClose]);

  if (!isOpen) return null;

  // Validation
  const errors: string[] = [];

  if (!isValidStellarAddress(buyer)) {
    errors.push("Comprador: Dirección pública inválida (debe iniciar con 'G' y contener caracteres válidos).");
  }
  if (!isValidStellarAddress(supplier)) {
    errors.push("Proveedor: Dirección pública inválida (debe iniciar con 'G' y contener caracteres válidos).");
  }
  if (!isValidStellarAddress(resolver)) {
    errors.push("Árbitro: Dirección pública inválida (debe iniciar con 'G' y contener caracteres válidos).");
  }
  if (!isValidStellarAddress(engine)) {
    errors.push("Motor: Dirección pública inválida (debe iniciar con 'G' y contener caracteres válidos).");
  }

  // Parties distinct
  const partiesList = [buyer.trim(), supplier.trim(), resolver.trim()].filter(Boolean);
  const distinctParties = new Set(partiesList);
  if (partiesList.length === 3 && distinctParties.size < 3) {
    errors.push("Comprador, Proveedor y Árbitro deben ser cuentas distintas.");
  }

  const numericAmount = parseFloat(amount.replace(/,/g, ""));
  if (isNaN(numericAmount) || numericAmount <= 0) {
    errors.push("Monto: Debe ser un valor numérico mayor a 0.");
  }

  if (fallbackOutcome === "SPLIT" && (fallbackSplitBps <= 0 || fallbackSplitBps >= 10000)) {
    errors.push("División Fallback: Los puntos básicos deben estar entre 1 y 9999 (e.g. 5000 = 50%).");
  }

  const isValid = errors.length === 0;

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (!isValid) return;
    setSubmitted(true);
    if (onCreated) {
      onCreated({
        buyer: buyer.trim(),
        supplier: supplier.trim(),
        resolver: resolver.trim(),
        engine: engine.trim(),
        token: token.trim(),
        amount: amount.trim(),
        fallbackOutcome,
        fallbackSplitBps,
      });
    }
  };

  const handleModalClose = () => {
    setBuyerInput(null);
    setSubmitted(false);
    onClose();
  };

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center p-4">
      {/* Backdrop */}
      <div
        className="fixed inset-0 bg-neutral-950/70 backdrop-blur-xs transition-opacity"
        onClick={handleModalClose}
        aria-hidden="true"
      />

      {/* Modal Dialog */}
      <div
        role="dialog"
        aria-modal="true"
        aria-labelledby="create-escrow-title"
        className="relative w-full max-w-2xl bg-white dark:bg-neutral-900 border border-neutral-200 dark:border-neutral-800 rounded-2xl shadow-2xl overflow-hidden z-10 flex flex-col max-h-[90vh]"
      >
        {/* Header */}
        <div className="flex items-center justify-between px-6 py-4 border-b border-neutral-200/80 dark:border-neutral-800/80">
          <div className="flex items-center gap-2.5">
            <div className="p-2 rounded-lg bg-teal-500/10 text-teal-600 dark:text-teal-400">
              <ShieldLockIcon size={18} />
            </div>
            <div>
              <h2
                id="create-escrow-title"
                className="text-base font-bold font-mono tracking-tight text-neutral-950 dark:text-white"
              >
                Crear Nuevo Escrow Comercial
              </h2>
            </div>
          </div>
          <button
            onClick={handleModalClose}
            className="p-1.5 rounded-lg text-neutral-400 hover:text-neutral-700 dark:hover:text-neutral-200 hover:bg-neutral-100 dark:hover:bg-neutral-800 transition-colors cursor-pointer"
            aria-label="Cerrar modal"
          >
            <CloseIcon size={16} />
          </button>
        </div>

        {/* Form Body */}
        <form onSubmit={handleSubmit} className="overflow-y-auto px-6 py-5 space-y-4 font-mono text-xs">
          {submitted ? (
            <div className="p-6 text-center space-y-3">
              <div className="mx-auto w-12 h-12 rounded-full bg-emerald-500/10 text-emerald-600 dark:text-emerald-400 flex items-center justify-center">
                <CheckIcon size={24} />
              </div>
              <h3 className="text-sm font-bold text-neutral-900 dark:text-neutral-100">
                Parámetros de Escrow Validados
              </h3>
              <p className="text-xs text-neutral-500 max-w-md mx-auto">
                La invocación on-chain <code className="text-teal-600 dark:text-teal-400">initialize()</code> está lista para ejecutarse cuando se active la conexión RPC de Soroban.
              </p>
              <div className="pt-4">
                <button
                  type="button"
                  onClick={handleModalClose}
                  className="px-4 py-2 rounded-lg bg-neutral-900 text-white dark:bg-neutral-100 dark:text-neutral-900 font-semibold cursor-pointer"
                >
                  Entendido / Volver al Panel
                </button>
              </div>
            </div>
          ) : (
            <>
              {/* Validation Alert */}
              {errors.length > 0 && (
                <div className="p-3 rounded-xl border border-rose-500/30 bg-rose-500/10 text-rose-800 dark:text-rose-200 space-y-1">
                  <div className="flex items-center gap-1.5 font-bold text-[11px]">
                    <AlertCircleIcon size={14} className="shrink-0" />
                    <span>Errores de validación en los parámetros:</span>
                  </div>
                  <ul className="list-disc list-inside space-y-0.5 text-[10px] pl-1">
                    {errors.map((err, i) => (
                      <li key={i}>{err}</li>
                    ))}
                  </ul>
                </div>
              )}

              {/* Field: Buyer */}
              <div className="space-y-1">
                <label className="block text-[11px] font-bold text-neutral-700 dark:text-neutral-300">
                  Comprador (Buyer Address)
                  <span className="text-teal-600 dark:text-teal-400 ml-1.5 font-normal">
                    {buyerAddress && buyer === buyerAddress ? "(Tu wallet conectada)" : ""}
                  </span>
                </label>
                <input
                  type="text"
                  value={buyer}
                  onChange={(e) => setBuyerInput(e.target.value)}
                  placeholder="G..."
                  className="w-full px-3 py-2 rounded-lg border border-neutral-200 dark:border-neutral-800 bg-neutral-50 dark:bg-neutral-900 text-neutral-900 dark:text-neutral-100 focus:outline-none focus:ring-1 focus:ring-teal-500 font-mono text-xs"
                />
              </div>

              {/* Field: Supplier */}
              <div className="space-y-1">
                <label className="block text-[11px] font-bold text-neutral-700 dark:text-neutral-300">
                  Proveedor (Supplier Address)
                </label>
                <input
                  type="text"
                  value={supplier}
                  onChange={(e) => setSupplier(e.target.value)}
                  placeholder="GSUPPLIER..."
                  className="w-full px-3 py-2 rounded-lg border border-neutral-200 dark:border-neutral-800 bg-neutral-50 dark:bg-neutral-900 text-neutral-900 dark:text-neutral-100 focus:outline-none focus:ring-1 focus:ring-teal-500 font-mono text-xs"
                />
              </div>

              {/* Field: Resolver & Engine */}
              <div className="grid grid-cols-1 sm:grid-cols-2 gap-3">
                <div className="space-y-1">
                  <label className="block text-[11px] font-bold text-neutral-700 dark:text-neutral-300">
                    Árbitro (Resolver / Arbiter)
                  </label>
                  <input
                    type="text"
                    value={resolver}
                    onChange={(e) => setResolver(e.target.value)}
                    placeholder="GRESOLVER..."
                    className="w-full px-3 py-2 rounded-lg border border-neutral-200 dark:border-neutral-800 bg-neutral-50 dark:bg-neutral-900 text-neutral-900 dark:text-neutral-100 focus:outline-none focus:ring-1 focus:ring-teal-500 font-mono text-xs"
                  />
                </div>
                <div className="space-y-1">
                  <label className="block text-[11px] font-bold text-neutral-700 dark:text-neutral-300">
                    Motor (Attestation Engine)
                  </label>
                  <input
                    type="text"
                    value={engine}
                    onChange={(e) => setEngine(e.target.value)}
                    placeholder="GENGINE..."
                    className="w-full px-3 py-2 rounded-lg border border-neutral-200 dark:border-neutral-800 bg-neutral-50 dark:bg-neutral-900 text-neutral-900 dark:text-neutral-100 focus:outline-none focus:ring-1 focus:ring-teal-500 font-mono text-xs"
                  />
                </div>
              </div>

              {/* Field: Token Contract & Amount */}
              <div className="grid grid-cols-1 sm:grid-cols-2 gap-3">
                <div className="space-y-1">
                  <label className="block text-[11px] font-bold text-neutral-700 dark:text-neutral-300">
                    Contrato del Token (SAC / Asset)
                  </label>
                  <input
                    type="text"
                    value={token}
                    onChange={(e) => setToken(e.target.value)}
                    placeholder="C..."
                    className="w-full px-3 py-2 rounded-lg border border-neutral-200 dark:border-neutral-800 bg-neutral-50 dark:bg-neutral-900 text-neutral-900 dark:text-neutral-100 focus:outline-none focus:ring-1 focus:ring-teal-500 font-mono text-xs"
                  />
                </div>
                <div className="space-y-1">
                  <label className="block text-[11px] font-bold text-neutral-700 dark:text-neutral-300">
                    Monto en CPUSD
                  </label>
                  <input
                    type="text"
                    value={amount}
                    onChange={(e) => setAmount(e.target.value)}
                    placeholder="10,000.0000000"
                    className="w-full px-3 py-2 rounded-lg border border-neutral-200 dark:border-neutral-800 bg-neutral-50 dark:bg-neutral-900 text-neutral-900 dark:text-neutral-100 focus:outline-none focus:ring-1 focus:ring-teal-500 font-mono text-xs"
                  />
                </div>
              </div>

              {/* Field: Fallback Configuration */}
              <div className="p-3.5 rounded-xl border border-neutral-200 dark:border-neutral-800 bg-neutral-50/50 dark:bg-neutral-900/50 space-y-3">
                <span className="block text-[11px] font-bold uppercase tracking-wider text-neutral-500">
                  Configuración de Fallback Contractual
                </span>
                <div className="grid grid-cols-1 sm:grid-cols-2 gap-3">
                  <div>
                    <label className="block text-[10px] text-neutral-500 mb-1">
                      Destino de Liquidación Automática
                    </label>
                    <select
                      value={fallbackOutcome}
                      onChange={(e) => setFallbackOutcome(e.target.value as FallbackOutcome)}
                      className="w-full px-3 py-2 rounded-lg border border-neutral-200 dark:border-neutral-800 bg-white dark:bg-neutral-900 text-neutral-900 dark:text-neutral-100 font-mono text-xs cursor-pointer"
                    >
                      <option value="SPLIT">SPLIT (División Porcentual)</option>
                      <option value="RELEASE">RELEASE (Liberar al Proveedor)</option>
                      <option value="REFUND">REFUND (Reembolsar al Comprador)</option>
                    </select>
                  </div>

                  {fallbackOutcome === "SPLIT" && (
                    <div>
                      <label className="block text-[10px] text-neutral-500 mb-1">
                        Puntos Básicos División (BPS: {fallbackSplitBps / 100}%)
                      </label>
                      <input
                        type="number"
                        min="1"
                        max="9999"
                        step="100"
                        value={fallbackSplitBps}
                        onChange={(e) => setFallbackSplitBps(parseInt(e.target.value) || 5000)}
                        className="w-full px-3 py-2 rounded-lg border border-neutral-200 dark:border-neutral-800 bg-white dark:bg-neutral-900 text-neutral-900 dark:text-neutral-100 font-mono text-xs"
                      />
                    </div>
                  )}
                </div>
                <p className="text-[10px] text-neutral-500 dark:text-neutral-400">
                  {fallbackOutcome === "SPLIT"
                    ? `En caso de expiración o arbitraje por defecto, los fondos se dividirán ${(fallbackSplitBps / 100).toFixed(0)}% Proveedor / ${(100 - fallbackSplitBps / 100).toFixed(0)}% Comprador.`
                    : fallbackOutcome === "RELEASE"
                      ? "En caso de expiración, el 100% de los fondos se liberará al proveedor."
                      : "En caso de expiración, el 100% de los fondos se reembolsará al comprador."}
                </p>
              </div>

              {/* Action Buttons */}
              <div className="pt-3 flex items-center justify-end gap-3 border-t border-neutral-200/80 dark:border-neutral-800/80">
                <button
                  type="button"
                  onClick={handleModalClose}
                  className="px-3.5 py-2 rounded-lg border border-neutral-200 dark:border-neutral-800 text-neutral-700 dark:text-neutral-300 hover:bg-neutral-100 dark:hover:bg-neutral-800 transition-colors cursor-pointer"
                >
                  Cancelar
                </button>
                <button
                  type="submit"
                  disabled={!isValid}
                  className="px-4 py-2 rounded-lg bg-teal-600 hover:bg-teal-700 disabled:opacity-40 disabled:cursor-not-allowed text-white font-semibold transition-colors flex items-center gap-1.5 shadow-2xs cursor-pointer"
                >
                  <span>Inicializar Escrow</span>
                  <span className="text-[10px] opacity-75 font-normal">
                    [Pendiente on-chain]
                  </span>
                </button>
              </div>
            </>
          )}
        </form>
      </div>
    </div>
  );
};
