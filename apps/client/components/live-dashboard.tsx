"use client";

import Link from "next/link";

import type { DashboardStats, Fill, Intent } from "@/lib/data";
import { useLiveResource } from "@/hooks/use-live-list";
import { PageHeader } from "@/components/page-header";
import { StatCell } from "@/components/stat-card";
import { StatusPill } from "@/components/status-pill";
import {
  explorerTxUrl,
  formatPrice,
  formatUsdc,
  shortPk,
  timeAgo,
} from "@/lib/format";

interface Props {
  initialStats: DashboardStats;
  initialFills: Fill[];
  initialIntents: Intent[];
}

/**
 * Dashboard for an OTC RFQ venue, not a marketing landing page.
 * Three lanes: a stat strip, a live intents column (top 6), and a
 * live fills column (top 6). No subhead, no architecture diagram,
 * no "no data yet :)" emoji garbage.
 *
 * Common Region: each lane is a `card` with a single hairline; the
 * stat strip is one card subdivided by hairlines so the four numbers
 * read as a single dense unit.
 */
export function LiveDashboard({
  initialStats,
  initialFills,
  initialIntents,
}: Props) {
  const { data: stats } = useLiveResource<DashboardStats>(
    "/api/dashboard",
    initialStats,
    (env) =>
      env.event.kind !== "quote_submitted" &&
      env.event.kind !== "quote_revealed",
  );
  const { data: fills } = useLiveResource<Fill[]>(
    "/api/fills",
    initialFills,
    (env) => env.event.kind === "settled",
  );
  const { data: intents } = useLiveResource<Intent[]>(
    "/api/intents",
    initialIntents,
  );

  return (
    <>
      <PageHeader
        title="Dashboard"
        eyebrow={`Devnet · ${intents.length} intents · ${fills.length} fills`}
      />

      <div className="card mt-8 grid grid-cols-2 divide-x divide-[var(--hairline)] sm:grid-cols-4">
        <StatCell
          label="Open"
          value={String(stats.open_intents)}
        />
        <StatCell
          label="Resolved"
          value={String(stats.resolved_intents)}
        />
        <StatCell
          label="Fills"
          value={String(stats.total_fills)}
        />
        <StatCell
          label="24h notional"
          value={formatUsdc(stats.notional_24h)}
        />
      </div>

      <div className="mt-6 grid gap-6 lg:grid-cols-2">
        <Lane
          title="Live intents"
          eyebrow={`${intents.filter((i) => i.status === "open").length} open`}
          href="/intents"
        >
          {intents.length === 0 ? (
            <Empty href="/trade" cta="Post the first intent" />
          ) : (
            <ul className="divide-y divide-[var(--hairline)]">
              {intents.slice(0, 6).map((i) => (
                <li key={i.id}>
                  <Link
                    href={`/intents/${i.id}`}
                    className="flex items-center justify-between gap-4 px-5 py-3 transition-colors hover:bg-[var(--surface-2)]"
                  >
                    <div className="min-w-0">
                      <div className="flex items-center gap-2">
                        <span
                          className={`font-mono text-[10px] uppercase tracking-[0.14em] ${
                            i.side === "buy"
                              ? "text-[var(--buy)]"
                              : "text-[var(--sell)]"
                          }`}
                        >
                          {i.side}
                        </span>
                        <span className="font-mono text-[12px] text-foreground">
                          {shortPk(i.id)}
                        </span>
                      </div>
                      <p className="mt-0.5 font-mono text-[11px] tabular-nums text-muted">
                        {i.size} @ {formatPrice(i.limit_price)}
                      </p>
                    </div>
                    <StatusPill status={i.status} />
                  </Link>
                </li>
              ))}
            </ul>
          )}
        </Lane>

        <Lane
          title="Recent fills"
          eyebrow={`${fills.length} total`}
          href="/fills"
        >
          {fills.length === 0 ? (
            <Empty href="/trade" cta="Trade to seed activity" />
          ) : (
            <ul className="divide-y divide-[var(--hairline)]">
              {fills.slice(0, 6).map((f) => (
                <li
                  key={f.id}
                  className="flex items-center justify-between gap-4 px-5 py-3"
                >
                  <div className="min-w-0">
                    <Link
                      href={`/intents/${f.intent_id}`}
                      className="font-mono text-[12px] text-foreground hover:underline"
                    >
                      {shortPk(f.intent_id)}
                    </Link>
                    <p className="mt-0.5 font-mono text-[11px] tabular-nums text-muted">
                      {f.size} @ {formatPrice(f.price)}
                    </p>
                  </div>
                  <div className="text-right">
                    {f.tx_signature ? (
                      <a
                        href={explorerTxUrl(f.tx_signature)}
                        target="_blank"
                        rel="noopener noreferrer"
                        className="font-mono text-[11px] text-[var(--accent)] hover:underline"
                      >
                        {f.tx_signature.slice(0, 8)}…
                      </a>
                    ) : (
                      <span className="font-mono text-[11px] text-faint">—</span>
                    )}
                    <p className="mt-0.5 font-mono text-[10px] uppercase tracking-[0.14em] text-faint">
                      {timeAgo(f.settled_at)}
                    </p>
                  </div>
                </li>
              ))}
            </ul>
          )}
        </Lane>
      </div>
    </>
  );
}

function Lane({
  title,
  eyebrow,
  href,
  children,
}: {
  title: string;
  eyebrow?: string;
  href: string;
  children: React.ReactNode;
}) {
  return (
    <div className="card overflow-hidden">
      <div className="flex items-center justify-between border-b border-[var(--hairline)] px-5 py-3">
        <div>
          <h2 className="text-[13px] font-medium text-foreground">{title}</h2>
          {eyebrow && (
            <p className="mt-0.5 font-mono text-[10px] uppercase tracking-[0.14em] text-faint">
              {eyebrow}
            </p>
          )}
        </div>
        <Link
          href={href}
          className="font-mono text-[11px] uppercase tracking-[0.14em] text-muted hover:text-foreground"
        >
          all →
        </Link>
      </div>
      {children}
    </div>
  );
}

function Empty({ href, cta }: { href: string; cta: string }) {
  return (
    <div className="px-5 py-12 text-center">
      <p className="font-mono text-[11px] uppercase tracking-[0.14em] text-faint">
        no data
      </p>
      <Link
        href={href}
        className="mt-3 inline-flex h-8 items-center rounded-[var(--r-sm)] border border-[var(--hairline-strong)] px-3 text-[12px] font-medium text-foreground hover:bg-[var(--surface-2)]"
      >
        {cta}
      </Link>
    </div>
  );
}
