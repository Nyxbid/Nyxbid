"use client";

import Link from "next/link";
import { useEffect, useState } from "react";
import { useWallet } from "@solana/wallet-adapter-react";

import type { Intent } from "@/lib/data";
import { useLiveResource } from "@/hooks/use-live-list";
import { Countdown } from "@/components/countdown";
import { StatusPill } from "@/components/status-pill";
import { formatPrice, shortPk } from "@/lib/format";

export function MakerInbox({ initial }: { initial: Intent[] }) {
  const { data: intents } = useLiveResource<Intent[]>("/api/intents", initial);
  const { publicKey } = useWallet();
  const [myIntentIds, setMyIntentIds] = useState<string[]>([]);
  // Drive the deadline check off a state ticker rather than
  // `Date.now()` directly. Calling `Date.now()` during render is
  // impure (React 19 rule) — the ticker re-renders every 5s so the
  // "live" set drops expired intents on the same cadence the
  // countdowns already update at.
  const [now, setNow] = useState<number>(() => Date.now());

  /* eslint-disable react-hooks/set-state-in-effect --
   * Reading from localStorage in response to a wallet change is an
   * external-storage-sync effect (localStorage is the external
   * store). The lint rule can't distinguish that from a derived-state
   * loop, so we suppress it for this block.
   */
  useEffect(() => {
    if (!publicKey || typeof window === "undefined") {
      setMyIntentIds([]);
      return;
    }
    const me = publicKey.toBase58();
    const prefix = "nyxbid:reveal:";
    const out: string[] = [];
    for (let i = 0; i < localStorage.length; i++) {
      const k = localStorage.key(i);
      if (!k || !k.startsWith(prefix)) continue;
      const [, , intent, maker] = k.split(":");
      if (maker === me && intent) out.push(intent);
    }
    setMyIntentIds(out);
  }, [publicKey]);
  /* eslint-enable react-hooks/set-state-in-effect */

  useEffect(() => {
    const t = setInterval(() => setNow(Date.now()), 5_000);
    return () => clearInterval(t);
  }, []);

  const live = intents.filter(
    (i) =>
      i.status === "open" &&
      new Date(i.reveal_deadline).getTime() > now,
  );
  const mine = intents.filter((i) => myIntentIds.includes(i.id));

  return (
    <div className="space-y-8">
      <Section title="Live RFQs" eyebrow={`${live.length} open`}>
        {live.length === 0 ? (
          <Empty>nothing to quote on right now</Empty>
        ) : (
          <RfqList intents={live} />
        )}
      </Section>

      <Section
        title="My quotes"
        eyebrow={publicKey ? `${mine.length} sealed` : "wallet"}
      >
        {!publicKey ? (
          <Empty>connect a wallet to track your sealed quotes</Empty>
        ) : mine.length === 0 ? (
          <Empty>no quotes sealed from this browser</Empty>
        ) : (
          <RfqList intents={mine} />
        )}
      </Section>
    </div>
  );
}

function Section({
  title,
  eyebrow,
  children,
}: {
  title: string;
  eyebrow?: string;
  children: React.ReactNode;
}) {
  return (
    <section>
      <div className="flex items-baseline justify-between border-b border-[var(--hairline)] pb-2">
        <h2 className="text-[13px] font-medium text-foreground">{title}</h2>
        {eyebrow && (
          <p className="font-mono text-[10px] uppercase tracking-[0.14em] text-faint">
            {eyebrow}
          </p>
        )}
      </div>
      <div className="mt-3">{children}</div>
    </section>
  );
}

function Empty({ children }: { children: React.ReactNode }) {
  return (
    <div className="card flex items-center justify-center py-12">
      <p className="font-mono text-[11px] uppercase tracking-[0.14em] text-faint">
        {children}
      </p>
    </div>
  );
}

function RfqList({ intents }: { intents: Intent[] }) {
  return (
    <div className="card overflow-x-auto">
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
          {intents.map((i) => (
            <tr
              key={i.id}
              className="border-b border-[var(--hairline)] last:border-0 hover:bg-[var(--surface-2)]"
            >
              <Td>
                <Link
                  href={`/intents/${i.id}`}
                  className="font-mono text-[12px] text-foreground hover:underline"
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
                  {formatPrice(i.limit_price)}
                </span>
              </Td>
              <Td>
                <StatusPill status={i.status} />
              </Td>
              <Td align="right">
                <Countdown
                  iso={i.reveal_deadline}
                  className="font-mono text-[11px] text-muted"
                />
              </Td>
            </tr>
          ))}
        </tbody>
      </table>
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
    <td className={`px-5 py-3 ${align === "right" ? "text-right" : ""}`}>
      {children}
    </td>
  );
}
