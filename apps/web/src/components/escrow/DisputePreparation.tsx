"use client";

import { useState, type FormEvent } from "react";
import type { EscrowStatus, FallbackOutcome } from "@/types/escrow";
import type { UserRole } from "@/types/wallet";
import {
  canRaiseDispute,
  canResolveDispute,
  validateDisputeRequest,
  validateResolutionRequest,
} from "@/lib/dispute";
import { useLanguage } from "@/providers/LanguageProvider";

const fieldStyle = "w-full min-h-11 rounded-lg border border-neutral-300 bg-white px-3 py-2 font-mono text-sm text-neutral-900 focus-visible:outline-2 focus-visible:outline-teal-600 dark:border-neutral-700 dark:bg-neutral-950 dark:text-neutral-100";

export function DisputePreparation({ role, status }: {
  role: UserRole | null;
  status: EscrowStatus;
}) {
  const { t } = useLanguage();
  const [reasonHash, setReasonHash] = useState("");
  const [evidenceHash, setEvidenceHash] = useState("");
  const [outcome, setOutcome] = useState<FallbackOutcome>("RELEASE");
  const [splitBps, setSplitBps] = useState("");
  const [checked, setChecked] = useState(false);

  const disputing = canRaiseDispute(role, status);
  const resolving = canResolveDispute(role, status);
  if (!disputing && !resolving) return null;

  const validation = disputing
    ? validateDisputeRequest({ role, status, reasonHash, disputeEvidenceHash: evidenceHash })
    : validateResolutionRequest({ role, status, outcome, splitBps });
  const issues = checked && !validation.ok ? validation.issues : [];

  function handleCheck(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    // This checks public arguments locally. No transaction is built or submitted.
    setChecked(true);
  }

  return (
    <section aria-labelledby="dispute-preparation-title" className="rounded-xl border border-neutral-200 bg-white p-5 text-neutral-900 dark:border-neutral-800 dark:bg-neutral-900/60 dark:text-neutral-100">
      <div className="mb-4 space-y-1">
        <h2 id="dispute-preparation-title" className="text-base font-semibold">
          {t(disputing ? "p0_10.dispute_title" : "p0_10.resolve_title")}
        </h2>
        <p className="text-sm text-neutral-700 dark:text-neutral-300">{t("p0_10.local_only")}</p>
      </div>

      <form onSubmit={handleCheck} noValidate className="space-y-4">
        {disputing ? (
          <>
            <div className="space-y-1.5">
              <label htmlFor="reason-hash" className="block text-sm font-medium">{t("p0_10.reason_hash")}</label>
              <input id="reason-hash" className={fieldStyle} autoComplete="off" spellCheck={false}
                value={reasonHash} onChange={(event) => { setReasonHash(event.target.value); setChecked(false); }}
                aria-invalid={issues.includes("reasonHash")} aria-describedby={issues.includes("reasonHash") ? "reason-hash-error" : undefined}
                placeholder={t("p0_10.hash_hint")} />
              {issues.includes("reasonHash") && <p id="reason-hash-error" className="text-sm text-rose-700 dark:text-rose-300">{t("p0_10.invalid_hash")}</p>}
            </div>
            <div className="space-y-1.5">
              <label htmlFor="dispute-evidence-hash" className="block text-sm font-medium">{t("p0_10.evidence_hash")}</label>
              <input id="dispute-evidence-hash" className={fieldStyle} autoComplete="off" spellCheck={false}
                value={evidenceHash} onChange={(event) => { setEvidenceHash(event.target.value); setChecked(false); }}
                aria-invalid={issues.includes("disputeEvidenceHash")}
                aria-describedby={issues.includes("disputeEvidenceHash") ? "dispute-evidence-hash-error" : undefined}
                placeholder={t("p0_10.hash_hint")} />
              {issues.includes("disputeEvidenceHash") && <p id="dispute-evidence-hash-error" className="text-sm text-rose-700 dark:text-rose-300">{t("p0_10.invalid_hash")}</p>}
            </div>
            <p className="text-xs text-neutral-700 dark:text-neutral-300">{t("p0_10.hash_privacy")}</p>
          </>
        ) : (
          <>
            <div className="space-y-1.5">
              <label htmlFor="resolution-outcome" className="block text-sm font-medium">{t("p0_10.outcome")}</label>
              <select id="resolution-outcome" className={fieldStyle} value={outcome}
                onChange={(event) => { setOutcome(event.target.value as FallbackOutcome); setSplitBps(""); setChecked(false); }}>
                {(["RELEASE", "REFUND", "SPLIT"] as const).map((value) => (
                  <option key={value} value={value}>{t(`fallback.outcomes.${value}`)}</option>
                ))}
              </select>
            </div>
            {outcome === "SPLIT" && (
              <div className="space-y-1.5">
                <label htmlFor="resolution-split-bps" className="block text-sm font-medium">{t("p0_10.split_bps")}</label>
                <input id="resolution-split-bps" type="number" inputMode="numeric" min="1" max="9999" step="1"
                  className={fieldStyle} value={splitBps}
                  onChange={(event) => { setSplitBps(event.target.value); setChecked(false); }}
                  aria-invalid={issues.includes("splitBps")}
                  aria-describedby={issues.includes("splitBps") ? "resolution-split-bps-error" : undefined} />
                {issues.includes("splitBps") && <p id="resolution-split-bps-error" className="text-sm text-rose-700 dark:text-rose-300">{t("p0_10.invalid_split")}</p>}
              </div>
            )}
          </>
        )}

        <div className="flex flex-wrap items-center gap-3">
          <button type="submit" className="min-h-11 rounded-lg bg-teal-700 px-4 py-2 text-sm font-semibold text-white hover:bg-teal-800 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-teal-700 dark:bg-teal-600 dark:hover:bg-teal-700">
            {t("p0_10.check_inputs")}
          </button>
          {checked && validation.ok && <p role="status" className="text-sm text-teal-800 dark:text-teal-200">{t("p0_10.validated_only")}</p>}
          {issues.includes("action") && <p role="alert" className="text-sm text-rose-700 dark:text-rose-300">{t("p0_10.ineligible")}</p>}
        </div>
      </form>
    </section>
  );
}
