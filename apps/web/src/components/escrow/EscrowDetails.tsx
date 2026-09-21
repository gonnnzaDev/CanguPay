"use client";

import React, { useState, useEffect } from "react";
import {
  EscrowDetailsProps,
  EscrowStatus,
  EscrowActor,
  getActiveActor,
  isTerminalStatus,
} from "@/types/escrow";

/**
 * Truncates an address or cryptographic hash for concise display.
 * e.g., 0x4f82a...9b12
 */
function truncateHash(hash: string, start = 8, end = 6): string {
  if (!hash) return "—";
  if (hash.length <= start + end) return hash;
  return `${hash.slice(0, start)}...${hash.slice(-end)}`;
}

/**
 * Maps EscrowStatus to technical status badge styling.
 */
function getStatusBadgeConfig(status: EscrowStatus): {
  label: string;
  badgeClass: string;
  dotClass: string;
} {
  switch (status) {
    case "CREATED":
      return {
        label: "CREATED",
        badgeClass: "bg-amber-500/10 text-amber-600 dark:text-amber-400 border-amber-500/20",
        dotClass: "bg-amber-500",
      };
    case "FUNDED":
      return {
        label: "FUNDED",
        badgeClass: "bg-blue-500/10 text-blue-600 dark:text-blue-400 border-blue-500/20",
        dotClass: "bg-blue-500",
      };
    case "EVIDENCE_SUBMITTED":
      return {
        label: "EVIDENCE SUBMITTED",
        badgeClass: "bg-indigo-500/10 text-indigo-600 dark:text-indigo-400 border-indigo-500/20",
        dotClass: "bg-indigo-500",
      };
    case "ATTESTED_PASS":
      return {
        label: "ATTESTED (PASS)",
        badgeClass: "bg-emerald-500/10 text-emerald-600 dark:text-emerald-400 border-emerald-500/20",
        dotClass: "bg-emerald-500",
      };
    case "ATTESTED_FAIL":
      return {
        label: "ATTESTED (FAIL)",
        badgeClass: "bg-rose-500/10 text-rose-600 dark:text-rose-400 border-rose-500/20",
        dotClass: "bg-rose-500",
      };
    case "DISPUTED":
      return {
        label: "IN DISPUTE",
        badgeClass: "bg-purple-500/10 text-purple-600 dark:text-purple-400 border-purple-500/20",
        dotClass: "bg-purple-500 animate-pulse",
      };
    case "RELEASED":
      return {
        label: "RELEASED",
        badgeClass: "bg-teal-500/10 text-teal-600 dark:text-teal-400 border-teal-500/20",
        dotClass: "bg-teal-500",
      };
    case "REFUNDED":
      return {
        label: "REFUNDED",
        badgeClass: "bg-neutral-500/10 text-neutral-600 dark:text-neutral-400 border-neutral-500/20",
        dotClass: "bg-neutral-500",
      };
    case "SPLIT":
      return {
        label: "SPLIT SETTLED",
        badgeClass: "bg-cyan-500/10 text-cyan-600 dark:text-cyan-400 border-cyan-500/20",
        dotClass: "bg-cyan-500",
      };
    case "CANCELLED":
      return {
        label: "CANCELLED",
        badgeClass: "bg-neutral-500/10 text-neutral-500 dark:text-neutral-400 border-neutral-500/20",
        dotClass: "bg-neutral-400",
      };
    default:
      return {
        label: status,
        badgeClass: "bg-neutral-500/10 text-neutral-600 dark:text-neutral-400 border-neutral-500/20",
        dotClass: "bg-neutral-400",
      };
  }
}

/**
 * Human-readable actor descriptions.
 */
function getActorMeta(actor: EscrowActor): { label: string; description: string } {
  switch (actor) {
    case "buyer":
      return {
        label: "Comprador (Buyer)",
        description: "Debe fondear el escrow o aprobar la liberación tras atestación satisfactoria.",
      };
    case "supplier":
      return {
        label: "Proveedor (Supplier)",
        description: "Debe suministrar la evidencia documental o iniciar corrección/disputa tras fallo.",
      };
    case "engine":
      return {
        label: "Motor de Reglas (Engine)",
        description: "Evaluando evidencia documental contra reglas deterministas acordadas.",
      };
    case "resolver":
      return {
        label: "Árbitro / Resolvedor (Resolver)",
        description: "Revisión humana en curso para dictaminar liberación, reembolso o división proporcional.",
      };
    case "none":
    default:
      return {
        label: "Sin acción pendiente",
        description: "El contrato se encuentra en un estado terminal o sin turno activo de intervención.",
      };
  }
}

export const EscrowDetails: React.FC<EscrowDetailsProps> = ({
  data,
  isLoading = false,
  error = null,
  onRefresh,
}) => {
  const [copiedKey, setCopiedKey] = useState<string | null>(null);
  const [currentTime, setCurrentTime] = useState<number>(() =>
    Math.floor(Date.now() / 1000)
  );

  useEffect(() => {
    const interval = setInterval(() => {
      setCurrentTime(Math.floor(Date.now() / 1000));
    }, 1000);
    return () => clearInterval(interval);
  }, []);

  const handleCopy = (text: string, key: string) => {
    if (!text || text === "—") return;
    navigator.clipboard?.writeText(text);
    setCopiedKey(key);
    setTimeout(() => setCopiedKey(null), 1800);
  };

  // 1. Loading State (Sober Skeleton)
  if (isLoading) {
    return (
      <div className="w-full max-w-5xl mx-auto p-6 space-y-6 animate-pulse">
        <div className="h-10 bg-neutral-200 dark:bg-neutral-800 rounded-lg w-1/3" />
        <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
          <div className="h-28 bg-neutral-100 dark:bg-neutral-900 border border-neutral-200 dark:border-neutral-800 rounded-xl" />
          <div className="h-28 bg-neutral-100 dark:bg-neutral-900 border border-neutral-200 dark:border-neutral-800 rounded-xl" />
          <div className="h-28 bg-neutral-100 dark:bg-neutral-900 border border-neutral-200 dark:border-neutral-800 rounded-xl" />
        </div>
        <div className="h-64 bg-neutral-100 dark:bg-neutral-900 border border-neutral-200 dark:border-neutral-800 rounded-xl" />
      </div>
    );
  }

  // 2. Error State
  if (error) {
    return (
      <div className="w-full max-w-5xl mx-auto p-6">
        <div className="rounded-xl border border-rose-500/20 bg-rose-500/5 p-6 text-neutral-900 dark:text-neutral-100">
          <div className="flex items-center gap-3">
            <span className="flex h-3 w-3 rounded-full bg-rose-500" />
            <h3 className="text-base font-medium">Error al consultar escrow</h3>
          </div>
          <p className="mt-2 text-sm text-neutral-600 dark:text-neutral-400 font-mono">
            {error}
          </p>
          {onRefresh && (
            <button
              onClick={onRefresh}
              className="mt-4 inline-flex items-center text-xs font-semibold px-3 py-1.5 rounded-lg bg-neutral-900 text-white dark:bg-neutral-100 dark:text-neutral-900 hover:opacity-90 transition-opacity"
            >
              Reintentar consulta
            </button>
          )}
        </div>
      </div>
    );
  }

  // 3. Empty State
  if (!data) {
    return (
      <div className="w-full max-w-5xl mx-auto p-12 text-center border border-dashed border-neutral-300 dark:border-neutral-800 rounded-2xl">
        <div className="mx-auto w-10 h-10 rounded-full bg-neutral-100 dark:bg-neutral-900 flex items-center justify-center text-neutral-400 mb-3">
          <span className="font-mono text-sm">#</span>
        </div>
        <h3 className="text-base font-semibold text-neutral-900 dark:text-neutral-100">
          Sin datos de escrow
        </h3>
        <p className="mt-1 text-sm text-neutral-500 dark:text-neutral-400 max-w-sm mx-auto">
          No se encontró ninguna operación activa de CanguPay para mostrar en esta vista.
        </p>
      </div>
    );
  }

  const statusConfig = getStatusBadgeConfig(data.status);
  const activeActor = getActiveActor(data.status);
  const actorMeta = getActorMeta(activeActor);
  const isTerminal = isTerminalStatus(data.status);

  // Relative deadline estimation (Informational only)
  let deadlineDiffSeconds = 0;
  if (data.activeDeadline?.timestamp) {
    deadlineDiffSeconds = data.activeDeadline.timestamp - currentTime;
  }

  const formatCountdown = (diff: number) => {
    if (diff <= 0) return "Vencido en tiempo de reloj (verificar ledger)";
    const hours = Math.floor(diff / 3600);
    const minutes = Math.floor((diff % 3600) / 60);
    const seconds = diff % 60;
    return `~${hours}h ${minutes}m ${seconds}s restantes`;
  };

  const explorerUrl = data.transactionHash
    ? `${data.explorerBaseUrl || "https://stellar.expert/explorer/testnet"}/tx/${data.transactionHash}`
    : data.contractId
    ? `${data.explorerBaseUrl || "https://stellar.expert/explorer/testnet"}/contract/${data.contractId}`
    : null;

  return (
    <div className="w-full max-w-5xl mx-auto p-4 md:p-6 space-y-6 text-neutral-900 dark:text-neutral-100">
      {/* Header & Status */}
      <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-4 pb-4 border-b border-neutral-200 dark:border-neutral-800">
        <div>
          <div className="flex items-center gap-2">
            <span className="text-xs uppercase tracking-widest text-neutral-400 font-semibold">
              Operación CanguPay
            </span>
            <span className="text-xs font-mono px-2 py-0.5 rounded bg-neutral-100 dark:bg-neutral-800 text-neutral-600 dark:text-neutral-300">
              Testnet
            </span>
          </div>
          <div className="flex items-center gap-3 mt-1">
            <h1 className="text-xl sm:text-2xl font-semibold tracking-tight font-mono">
              {data.operationId}
            </h1>
            <button
              onClick={() => handleCopy(data.operationId, "operationId")}
              title="Copiar ID"
              className="text-xs text-neutral-400 hover:text-neutral-600 dark:hover:text-neutral-200"
            >
              {copiedKey === "operationId" ? "Copiado ✓" : "Copiar"}
            </button>
          </div>
        </div>

        <div className="flex items-center gap-3">
          <div
            className={`inline-flex items-center gap-2 px-3 py-1.5 rounded-full border text-xs font-mono font-medium tracking-wide ${statusConfig.badgeClass}`}
          >
            <span className={`w-2 h-2 rounded-full ${statusConfig.dotClass}`} />
            {statusConfig.label}
          </div>

          {onRefresh && (
            <button
              onClick={onRefresh}
              className="p-2 text-neutral-400 hover:text-neutral-700 dark:hover:text-neutral-200 transition-colors border border-neutral-200 dark:border-neutral-800 rounded-lg"
              title="Actualizar estado"
            >
              ↻
            </button>
          )}
        </div>
      </div>

      {/* Top Metrics Cards: Amount & Active Actor */}
      <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
        {/* Metric 1: Amount in Custody */}
        <div className="p-5 rounded-xl border border-neutral-200 dark:border-neutral-800 bg-white dark:bg-neutral-900/50 shadow-xs">
          <span className="text-xs font-medium text-neutral-500 dark:text-neutral-400 uppercase tracking-wider">
            Monto en Custodia
          </span>
          <div className="mt-2 flex items-baseline gap-2">
            <span className="text-2xl font-bold font-mono tracking-tight">
              {data.amount}
            </span>
            <span className="text-sm font-semibold text-neutral-500 font-mono">
              {data.asset}
            </span>
          </div>
          <p className="mt-1 text-xs text-neutral-400">
            Reserva atómica en contrato inteligente
          </p>
        </div>

        {/* Metric 2: Active Actor */}
        <div className="p-5 rounded-xl border border-neutral-200 dark:border-neutral-800 bg-white dark:bg-neutral-900/50 shadow-xs md:col-span-2">
          <div className="flex items-center justify-between">
            <span className="text-xs font-medium text-neutral-500 dark:text-neutral-400 uppercase tracking-wider">
              Turno Activo (UX Hint)
            </span>
            {!isTerminal && (
              <span className="text-xs px-2 py-0.5 rounded-full bg-blue-500/10 text-blue-600 dark:text-blue-400 font-medium">
                Esperando acción
              </span>
            )}
          </div>
          <div className="mt-2">
            <h4 className="text-base font-semibold tracking-tight text-neutral-900 dark:text-neutral-100">
              {actorMeta.label}
            </h4>
            <p className="text-xs text-neutral-500 dark:text-neutral-400 mt-0.5">
              {actorMeta.description}
            </p>
          </div>
        </div>
      </div>

      {/* Active Deadline Panel */}
      {data.activeDeadline && !isTerminal && (
        <div className="p-5 rounded-xl border border-amber-500/20 bg-amber-500/5 dark:bg-amber-500/[0.03]">
          <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-2">
            <div>
              <span className="text-xs font-semibold text-amber-700 dark:text-amber-400 uppercase tracking-wider">
                Plazo activo: {data.activeDeadline.label}
              </span>
              <p className="text-sm font-medium mt-0.5 text-neutral-800 dark:text-neutral-200">
                {formatCountdown(deadlineDiffSeconds)}
              </p>
            </div>
            <div className="text-right">
              <span className="text-xs text-neutral-400 font-mono block">
                Timestamp: {data.activeDeadline.timestamp}
              </span>
            </div>
          </div>
          <p className="mt-3 text-[11px] text-neutral-500 dark:text-neutral-400 italic border-t border-amber-500/10 pt-2">
            * El contador visible es informativo. El contrato usa el timestamp del
            ledger (<code className="font-mono">env.ledger().timestamp()</code>) como
            fuente de verdad indiscutible.
          </p>
        </div>
      )}

      {/* Parties / Roles Panel */}
      <div className="p-5 rounded-xl border border-neutral-200 dark:border-neutral-800 bg-white dark:bg-neutral-900/50 shadow-xs">
        <h3 className="text-sm font-semibold text-neutral-900 dark:text-neutral-100 mb-4 tracking-tight">
          Participantes del Escrow
        </h3>
        <div className="grid grid-cols-1 sm:grid-cols-2 gap-4 text-xs font-mono">
          <div className="p-3 rounded-lg bg-neutral-50 dark:bg-neutral-900 border border-neutral-100 dark:border-neutral-800">
            <span className="text-neutral-400 block mb-1 uppercase text-[10px] tracking-wider">
              Buyer (Comprador)
            </span>
            <div className="flex items-center justify-between gap-2">
              <span className="truncate">{data.parties.buyer}</span>
              <button
                onClick={() => handleCopy(data.parties.buyer, "buyer")}
                className="text-neutral-400 hover:text-neutral-600 dark:hover:text-neutral-200 shrink-0"
              >
                {copiedKey === "buyer" ? "✓" : "Copiar"}
              </button>
            </div>
          </div>

          <div className="p-3 rounded-lg bg-neutral-50 dark:bg-neutral-900 border border-neutral-100 dark:border-neutral-800">
            <span className="text-neutral-400 block mb-1 uppercase text-[10px] tracking-wider">
              Supplier (Proveedor)
            </span>
            <div className="flex items-center justify-between gap-2">
              <span className="truncate">{data.parties.supplier}</span>
              <button
                onClick={() => handleCopy(data.parties.supplier, "supplier")}
                className="text-neutral-400 hover:text-neutral-600 dark:hover:text-neutral-200 shrink-0"
              >
                {copiedKey === "supplier" ? "✓" : "Copiar"}
              </button>
            </div>
          </div>

          <div className="p-3 rounded-lg bg-neutral-50 dark:bg-neutral-900 border border-neutral-100 dark:border-neutral-800">
            <span className="text-neutral-400 block mb-1 uppercase text-[10px] tracking-wider">
              Engine (Motor de Reglas)
            </span>
            <div className="flex items-center justify-between gap-2">
              <span className="truncate">{data.parties.engine}</span>
              <button
                onClick={() => handleCopy(data.parties.engine, "engine")}
                className="text-neutral-400 hover:text-neutral-600 dark:hover:text-neutral-200 shrink-0"
              >
                {copiedKey === "engine" ? "✓" : "Copiar"}
              </button>
            </div>
          </div>

          <div className="p-3 rounded-lg bg-neutral-50 dark:bg-neutral-900 border border-neutral-100 dark:border-neutral-800">
            <span className="text-neutral-400 block mb-1 uppercase text-[10px] tracking-wider">
              Resolver (Árbitro)
            </span>
            <div className="flex items-center justify-between gap-2">
              <span className="truncate">{data.parties.resolver}</span>
              <button
                onClick={() => handleCopy(data.parties.resolver, "resolver")}
                className="text-neutral-400 hover:text-neutral-600 dark:hover:text-neutral-200 shrink-0"
              >
                {copiedKey === "resolver" ? "✓" : "Copiar"}
              </button>
            </div>
          </div>
        </div>
      </div>

      {/* Cryptographic Hashes Vault */}
      <div className="p-5 rounded-xl border border-neutral-200 dark:border-neutral-800 bg-white dark:bg-neutral-900/50 shadow-xs">
        <h3 className="text-sm font-semibold text-neutral-900 dark:text-neutral-100 mb-1 tracking-tight">
          Bóveda Criptográfica y Evidencia
        </h3>
        <p className="text-xs text-neutral-500 dark:text-neutral-400 mb-4">
          Hashes SHA-256 / BytesN&lt;32&gt; verificables contra reglas documentales.
        </p>

        <div className="space-y-2 text-xs font-mono">
          {/* Evidence Hash */}
          <div className="flex flex-col sm:flex-row sm:items-center justify-between p-2.5 rounded-lg bg-neutral-50 dark:bg-neutral-900 border border-neutral-100 dark:border-neutral-800 gap-1">
            <span className="text-neutral-500 font-sans text-xs">
              Evidence Bundle Hash:
            </span>
            <div className="flex items-center gap-2">
              <span className="truncate text-neutral-800 dark:text-neutral-200">
                {truncateHash(data.hashes.evidenceBundleHash || "")}
              </span>
              {data.hashes.evidenceBundleHash && (
                <button
                  onClick={() =>
                    handleCopy(data.hashes.evidenceBundleHash!, "evidenceHash")
                  }
                  className="text-neutral-400 hover:text-neutral-600 dark:hover:text-neutral-200"
                >
                  {copiedKey === "evidenceHash" ? "✓" : "Copiar"}
                </button>
              )}
            </div>
          </div>

          {/* Report Hash */}
          <div className="flex flex-col sm:flex-row sm:items-center justify-between p-2.5 rounded-lg bg-neutral-50 dark:bg-neutral-900 border border-neutral-100 dark:border-neutral-800 gap-1">
            <span className="text-neutral-500 font-sans text-xs">
              Engine Report Hash:
            </span>
            <div className="flex items-center gap-2">
              <span className="truncate text-neutral-800 dark:text-neutral-200">
                {truncateHash(data.hashes.reportHash || "")}
              </span>
              {data.hashes.reportHash && (
                <button
                  onClick={() =>
                    handleCopy(data.hashes.reportHash!, "reportHash")
                  }
                  className="text-neutral-400 hover:text-neutral-600 dark:hover:text-neutral-200"
                >
                  {copiedKey === "reportHash" ? "✓" : "Copiar"}
                </button>
              )}
            </div>
          </div>

          {/* Dispute Reason Hash */}
          <div className="flex flex-col sm:flex-row sm:items-center justify-between p-2.5 rounded-lg bg-neutral-50 dark:bg-neutral-900 border border-neutral-100 dark:border-neutral-800 gap-1">
            <span className="text-neutral-500 font-sans text-xs">
              Dispute Reason Hash:
            </span>
            <div className="flex items-center gap-2">
              <span className="truncate text-neutral-800 dark:text-neutral-200">
                {truncateHash(data.hashes.reasonHash || "")}
              </span>
              {data.hashes.reasonHash && (
                <button
                  onClick={() =>
                    handleCopy(data.hashes.reasonHash!, "reasonHash")
                  }
                  className="text-neutral-400 hover:text-neutral-600 dark:hover:text-neutral-200"
                >
                  {copiedKey === "reasonHash" ? "✓" : "Copiar"}
                </button>
              )}
            </div>
          </div>

          {/* Dispute Evidence Hash */}
          <div className="flex flex-col sm:flex-row sm:items-center justify-between p-2.5 rounded-lg bg-neutral-50 dark:bg-neutral-900 border border-neutral-100 dark:border-neutral-800 gap-1">
            <span className="text-neutral-500 font-sans text-xs">
              Dispute Evidence Hash:
            </span>
            <div className="flex items-center gap-2">
              <span className="truncate text-neutral-800 dark:text-neutral-200">
                {truncateHash(data.hashes.disputeEvidenceHash || "")}
              </span>
              {data.hashes.disputeEvidenceHash && (
                <button
                  onClick={() =>
                    handleCopy(
                      data.hashes.disputeEvidenceHash!,
                      "disputeEvidenceHash"
                    )
                  }
                  className="text-neutral-400 hover:text-neutral-600 dark:hover:text-neutral-200"
                >
                  {copiedKey === "disputeEvidenceHash" ? "✓" : "Copiar"}
                </button>
              )}
            </div>
          </div>
        </div>
      </div>

      {/* Explorer Link & Footer Meta */}
      <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-2 pt-2 text-xs text-neutral-500">
        <div>
          {data.contractId && (
            <span className="font-mono">
              Contract: {truncateHash(data.contractId, 10, 8)}
            </span>
          )}
        </div>

        {explorerUrl && (
          <a
            href={explorerUrl}
            target="_blank"
            rel="noopener noreferrer"
            className="inline-flex items-center gap-1 text-blue-600 dark:text-blue-400 hover:underline font-medium"
          >
            Ver en Stellar Expert Testnet ↗
          </a>
        )}
      </div>
    </div>
  );
};
