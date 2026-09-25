"use client";

import { useEffect, useState } from "react";
import {
  fetchPublicEscrowTimeline,
  type EscrowTimelineRead,
} from "@/lib/escrow-timeline";
import { useLanguage } from "@/providers/LanguageProvider";
import { truncateHash } from "./helpers";

const hashLabels: Record<string, string> = {
  evidenceBundleHash: "hashes.evidence",
  reportHash: "hashes.report",
  reasonHash: "hashes.reason",
  disputeEvidenceHash: "hashes.dispute",
};

export function EscrowEventTimeline({ contractId, config }: {
  contractId: string;
  config: Record<string, unknown>;
}) {
  const { t, language } = useLanguage();
  const [read, setRead] = useState<EscrowTimelineRead | null>(null);

  useEffect(() => {
    let current = true;
    const timer = setTimeout(() => {
      void fetchPublicEscrowTimeline(contractId, {
        attestationPeriod: config.attestationPeriod,
        objectionPeriod: config.objectionPeriod,
        correctionPeriod: config.correctionPeriod,
        resolutionPeriod: config.resolutionPeriod,
      }).then((response) => {
        if (current) setRead(response);
      });
    }, 0);
    return () => { current = false; clearTimeout(timer); };
  }, [contractId, config]);

  const locale = language === "es" ? "es-PE" : "en-US";
  const formatTime = (value: string | number) => new Intl.DateTimeFormat(locale, {
    dateStyle: "medium", timeStyle: "short", timeZone: "UTC",
  }).format(typeof value === "number" ? value * 1000 : new Date(value));

  return (
    <section aria-labelledby="escrow-event-timeline" className="rounded-xl border border-neutral-200 bg-white p-5 dark:border-neutral-800 dark:bg-neutral-900/60">
      <h2 id="escrow-event-timeline" className="text-sm font-semibold text-neutral-900 dark:text-neutral-100">
        {t("p0_10.timeline_title")}
      </h2>
      {!read ? (
        <p role="status" className="mt-3 text-sm text-neutral-600 dark:text-neutral-300">{t("common.loading")}</p>
      ) : read.error ? (
        <p role="alert" className="mt-3 text-sm text-rose-700 dark:text-rose-300">{t("p0_10.timeline_error")}</p>
      ) : (
        <>
          {read.ledgerTime !== null && (
            <p className="mt-2 text-xs text-neutral-600 dark:text-neutral-300">
              {t("p0_10.latest_ledger_time", { time: `${formatTime(read.ledgerTime)} UTC` })}
            </p>
          )}
          {read.events.length === 0 ? (
            <p className="mt-3 text-sm text-neutral-600 dark:text-neutral-300">{t("p0_10.timeline_empty")}</p>
          ) : (
            <ol className="mt-4 space-y-3">
              {read.events.map((event) => (
                <li key={event.id} className="rounded-lg border border-neutral-200 p-3 dark:border-neutral-800">
                  <div className="flex flex-wrap items-center justify-between gap-2">
                    <strong className="text-sm text-neutral-900 dark:text-neutral-100">{t(`p0_10.events.${event.kind}`)}</strong>
                    <span className="text-xs text-neutral-600 dark:text-neutral-300">{formatTime(event.occurredAt)} UTC</span>
                  </div>
                  <p className="mt-1 text-xs text-neutral-600 dark:text-neutral-300">
                    {t("p0_10.ledger_number", { ledger: event.ledger })}
                    {event.deadline !== undefined && ` · ${t(event.deadlineSource === "event" ? "p0_10.deadline_recorded" : "p0_10.deadline_derived", { time: `${formatTime(event.deadline)} UTC` })}`}
                  </p>
                  {event.hashes && (
                    <dl className="mt-2 space-y-1 text-xs text-neutral-700 dark:text-neutral-300">
                      {Object.entries(event.hashes).map(([key, hash]) => hash && (
                        <div key={key}>
                          <dt className="font-semibold">{t(hashLabels[key])}</dt>
                          <dd><code className="break-all font-mono">{hash}</code></dd>
                        </div>
                      ))}
                    </dl>
                  )}
                  <a href={`https://stellar.expert/explorer/testnet/tx/${event.txHash}`} target="_blank" rel="noopener noreferrer"
                    className="mt-2 inline-flex min-h-8 items-center text-xs font-medium text-teal-700 underline-offset-2 hover:underline focus-visible:outline-2 focus-visible:outline-teal-600 dark:text-teal-300">
                    {t("p0_10.view_transaction", { hash: truncateHash(event.txHash, 8, 6) })}
                  </a>
                </li>
              ))}
            </ol>
          )}
          {read.limitedHistory && (
            <p className="mt-3 text-xs text-neutral-600 dark:text-neutral-300">{t("p0_10.history_partial")}</p>
          )}
          <p className="mt-2 text-xs text-neutral-600 dark:text-neutral-300">{t("p0_10.ledger_authority")}</p>
        </>
      )}
    </section>
  );
}
