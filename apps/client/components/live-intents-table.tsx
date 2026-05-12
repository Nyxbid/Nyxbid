"use client";

import Link from "next/link";

import type { Intent } from "@/lib/data";
import { useLiveResource } from "@/hooks/use-live-list";
import { StatusPill } from "@/components/status-pill";
import { Countdown } from "@/components/countdown";
import { Paginator, usePagination } from "@/components/pagination";
import { formatPrice, shortPk } from "@/lib/format";

export function LiveIntentsTable({ initial }: { initial: Intent[] }) {
  const { data } = useLiveResource<Intent[]>("/api/intents", initial);
  // 20 rows per page keeps the table inside a 1080p viewport without
  // the parent div needing to scroll independently of the page.
  const pager = usePagination(data, 20);

  if (data.length === 0) {
    return (
      <div className="card flex flex-col items-center justify-center py-16">
        <p className="font-mono text-[11px] uppercase tracking-[0.14em] text-faint">
          no intents
        </p>
        <Link
          href="/trade"
          className="mt-4 inline-flex h-9 items-center rounded-[var(--r-sm)] bg-[var(--accent)] px-4 text-[12px] font-medium text-[var(--accent-fg)] hover:bg-[var(--accent-soft)]"
        >
          Post the first
        </Link>
      </div>
    );
  }

  return (
    <div className="card">
      <div className="overflow-x-auto">
        <table className="w-full text-left">
          <thead>
            <tr className="border-b border-[var(--hairline)]">
              <Th>Intent</Th>
              <Th>Side</Th>
              <Th align="right">Size</Th>
              <Th align="right">Limit</Th>
              <Th>Status</Th>
              <Th align="right">Reveal</Th>
            </tr>
          </thead>
          <tbody>
            {pager.rows.map((i) => (
            <tr
              key={i.id}
              className="border-b border-[var(--hairline)] last:border-0 hover:bg-[var(--surface-2)]"
            >
              <Td>
                <Link
                  href={`/intents/${i.id}`}
                  className="font-mono text-[12px] text-foreground hover:underline"
                  prefetch={false}
                >
                  {shortPk(i.id)}
                </Link>
              </Td>
              <Td>
                <span
                  className={`font-mono text-[10px] uppercase tracking-[0.14em] ${
                    i.side === "buy" ? "text-[var(--buy)]" : "text-[var(--sell)]"
                  }`}
                >
                  {i.side}
                </span>
              </Td>
              <Td align="right">
                <span className="font-mono text-[12px] tabular-nums">
                  {i.size}
                </span>
              </Td>
              <Td align="right">
                <span className="font-mono text-[12px] tabular-nums">
                  {formatPrice(i.limit_price, i.base_mint, i.quote_mint)}
                </span>
              </Td>
              <Td>
                <StatusPill status={i.status} />
              </Td>
              <Td align="right">
                {i.status === "open" ? (
                  <Countdown
                    iso={i.reveal_deadline}
                    className="font-mono text-[11px] text-muted"
                  />
                ) : (
                  <span className="font-mono text-[11px] text-faint">—</span>
                )}
              </Td>
            </tr>
            ))}
          </tbody>
        </table>
      </div>
      <Paginator
        page={pager.page}
        pageCount={pager.pageCount}
        from={pager.from}
        to={pager.to}
        total={pager.total}
        onPrev={pager.prev}
        onNext={pager.next}
        canPrev={pager.canPrev}
        canNext={pager.canNext}
        noun="intents"
      />
    </div>
  );
}

function Th({
  children,
  align,
}: {
  children: React.ReactNode;
  align?: "right";
}) {
  return (
    <th
      className={`px-5 py-2.5 font-mono text-[10px] font-medium uppercase tracking-[0.14em] text-muted ${
        align === "right" ? "text-right" : ""
      }`}
    >
      {children}
    </th>
  );
}

function Td({
  children,
  align,
}: {
  children: React.ReactNode;
  align?: "right";
}) {
  return (
    <td
      className={`px-5 py-3 ${align === "right" ? "text-right" : ""}`}
    >
      {children}
    </td>
  );
}
