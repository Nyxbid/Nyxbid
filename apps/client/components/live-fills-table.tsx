"use client";

import Link from "next/link";

import type { Fill } from "@/lib/data";
import { useLiveResource } from "@/hooks/use-live-list";
import { Paginator, usePagination } from "@/components/pagination";
import {
  explorerTxUrl,
  formatPrice,
  shortPk,
  timeAgo,
} from "@/lib/format";

export function LiveFillsTable({ initial }: { initial: Fill[] }) {
  const { data } = useLiveResource<Fill[]>(
    "/api/fills",
    initial,
    (env) => env.event.kind === "settled",
  );
  const pager = usePagination(data, 20);

  if (data.length === 0) {
    return (
      <div className="card flex flex-col items-center justify-center py-16">
        <p className="font-mono text-[11px] uppercase tracking-[0.14em] text-faint">
          no fills
        </p>
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
              <Th>Taker</Th>
              <Th>Maker</Th>
              <Th align="right">Size</Th>
              <Th align="right">Price</Th>
              <Th>Tx</Th>
              <Th align="right">When</Th>
            </tr>
          </thead>
          <tbody>
            {pager.rows.map((f) => (
            <tr
              key={f.id}
              className="border-b border-[var(--hairline)] last:border-0 hover:bg-[var(--surface-2)]"
            >
              <Td>
                <Link
                  href={`/intents/${f.intent_id}`}
                  className="font-mono text-[12px] text-foreground hover:underline"
                >
                  {shortPk(f.intent_id)}
                </Link>
              </Td>
              <Td>
                <span className="font-mono text-[12px] text-muted">
                  {shortPk(f.taker)}
                </span>
              </Td>
              <Td>
                <span className="font-mono text-[12px] text-muted">
                  {shortPk(f.maker)}
                </span>
              </Td>
              <Td align="right">
                <span className="font-mono text-[12px] tabular-nums">
                  {f.size}
                </span>
              </Td>
              <Td align="right">
                <span className="font-mono text-[12px] tabular-nums">
                  {formatPrice(f.price, f.base_mint, f.quote_mint)}
                </span>
              </Td>
              <Td>
                {f.tx_signature ? (
                  <a
                    href={explorerTxUrl(f.tx_signature)}
                    target="_blank"
                    rel="noopener noreferrer"
                    className="font-mono text-[12px] text-[var(--accent)] hover:underline"
                  >
                    {f.tx_signature.slice(0, 8)}…
                  </a>
                ) : (
                  <span className="font-mono text-[12px] text-faint">—</span>
                )}
              </Td>
              <Td align="right">
                <span className="font-mono text-[11px] text-muted">
                  {timeAgo(f.settled_at)}
                </span>
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
        noun="fills"
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
