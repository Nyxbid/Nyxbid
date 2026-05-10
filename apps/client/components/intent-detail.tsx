"use client";

import { useEffect, useMemo, useState } from "react";
import { useWallet } from "@solana/wallet-adapter-react";

import type { Intent, Quote } from "@/lib/data";
import { tx as txApi } from "@/lib/api";
import {
  bytesToHex,
  commitmentHex,
  hexToBytes,
  randomBytes,
} from "@/lib/commitment";
import {
  explorerAccountUrl,
  explorerTxUrl,
  formatPrice,
  priceToScaled,
  shortPk,
  timeAgo,
} from "@/lib/format";
import { friendlyTxError } from "@/lib/tx-errors";
import { useLiveResource } from "@/hooks/use-live-list";
import { useNyxbidTx } from "@/hooks/use-nyxbid-tx";
import { useToast } from "@/components/toast";
import { ActionButton } from "@/components/action-button";
import { Countdown } from "@/components/countdown";
import { PhaseStrip } from "@/components/phase";
import { StatusPill } from "@/components/status-pill";

interface Props {
  initialIntent: Intent;
  initialQuotes: Quote[];
}

/**
 * Intent detail surface. Single page that renders the right action
 * based on (wallet identity, intent phase). The four action paths:
 *
 *   1) **Maker before reveal_deadline**: submit a sealed quote.
 *      We compute sha256(price_le||size_le||nonce32) entirely on the
 *      client; the secret stays in `localStorage` so the same browser
 *      can reveal it later (and we surface the secret in a copy-out
 *      box so they can move to a different machine if they want).
 *
 *   2) **Maker between reveal_deadline and resolve_deadline**: reveal
 *      the secret, which the chain checks against the on-chain
 *      commitment.
 *
 *   3) **Winning maker after resolve_deadline**: fund their leg into
 *      the per-intent maker vault.
 *
 *   4) **Anyone after fund**: settle the trade.
 *
 *   5) **Taker before reveal_deadline**: cancel.
 *
 * After settle/cancel the page transitions into a read-only summary
 * with a Solscan link to the Receipt.
 *
 * Peak-End Rule: the final settled state shows a clear success card
 * with the receipt + amount transferred, so the *end* of the flow
 * leaves a positive memory.
 */
export function IntentDetail({ initialIntent, initialQuotes }: Props) {
  const { publicKey } = useWallet();

  // Live-update both intent and its quotes when relevant events fire.
  // All ChainEvent variants currently carry an `intent` field, so a
  // single membership check is enough to scope the refetch.
  const matchesThisIntent = (env: { event: { intent?: string } }) =>
    env.event.intent === initialIntent.id;
  const { data: intent } = useLiveResource<Intent>(
    `/api/intents/${initialIntent.id}`,
    initialIntent,
    matchesThisIntent,
  );
  const { data: quotes } = useLiveResource<Quote[]>(
    `/api/intents/${initialIntent.id}/quotes`,
    initialQuotes,
    matchesThisIntent,
  );

  const me = publicKey?.toBase58() ?? null;
  const isTaker = me === intent.taker;
  const myQuote = useMemo(
    () => quotes.find((q) => q.maker === me) ?? null,
    [quotes, me],
  );

  // Drive deadline checks off a 1Hz state ticker so the action card
  // flips correctly the moment a window closes mid-session, instead
  // of relying on `Date.now()` during render (which is impure and
  // also won't trigger a re-render on its own).
  const now = useNow(1000);
  const beforeReveal = new Date(intent.reveal_deadline).getTime() > now;
  const beforeResolve = new Date(intent.resolve_deadline).getTime() > now;

  const winningQuote = useMemo(
    () =>
      intent.winning_quote
        ? quotes.find((q) => q.id === intent.winning_quote) ?? null
        : null,
    [quotes, intent.winning_quote],
  );

  return (
    <div className="space-y-6">
      <Header intent={intent} />

      <PhaseStrip
        status={intent.status}
        marks={{
          open: `reveal in ${beforeReveal ? "..." : "expired"}`,
          resolved: intent.winning_quote ? "winner picked" : undefined,
          settled: winningQuote
            ? `@ ${formatPrice(winningQuote.revealed_price ?? 0, intent.base_mint, intent.quote_mint)}`
            : undefined,
        }}
      />

      <div className="grid gap-6 lg:grid-cols-[1fr_360px]">
        <QuoteList
          quotes={quotes}
          winningId={intent.winning_quote}
          mePk={me}
          baseMint={intent.base_mint}
          quoteMint={intent.quote_mint}
        />

        <div className="space-y-4">
          {/* Action surface — exactly one card visible at a time. */}
          {intent.status === "open" && isTaker && beforeReveal && (
            <CancelCard intentId={intent.id} />
          )}
          {intent.status === "open" && !isTaker && beforeReveal && (
            <SubmitQuoteCard intent={intent} myQuote={myQuote} />
          )}
          {intent.status === "open" &&
            !isTaker &&
            !beforeReveal &&
            beforeResolve &&
            myQuote &&
            !myQuote.revealed && (
              <RevealQuoteCard intent={intent} myQuote={myQuote} />
            )}
          {intent.status === "resolved" &&
            winningQuote &&
            me === winningQuote.maker && (
              <FundEscrowCard
                intent={intent}
                quote={winningQuote}
              />
            )}
          {intent.status === "resolved" && winningQuote && (
            <SettleCard intent={intent} />
          )}
          {(intent.status === "settled" ||
            intent.status === "cancelled" ||
            intent.status === "expired") && (
            <TerminalCard intent={intent} winning={winningQuote} />
          )}

          <MetaCard intent={intent} />
        </div>
      </div>
    </div>
  );
}

// ----- header -------------------------------------------------------

function useNow(intervalMs: number): number {
  const [now, setNow] = useState(() => Date.now());
  useEffect(() => {
    const id = setInterval(() => setNow(Date.now()), intervalMs);
    return () => clearInterval(id);
  }, [intervalMs]);
  return now;
}

function Header({ intent }: { intent: Intent }) {
  return (
    <div className="card p-5">
      <div className="flex flex-wrap items-center gap-3">
        <span
          className={`font-mono text-[10px] uppercase tracking-[0.18em] ${
            intent.side === "buy" ? "text-[var(--buy)]" : "text-[var(--sell)]"
          }`}
        >
          {intent.side}
        </span>
        <h1 className="font-mono text-[14px] text-foreground">
          <a
            href={explorerAccountUrl(intent.id)}
            target="_blank"
            rel="noopener noreferrer"
            className="hover:underline"
          >
            {shortPk(intent.id, 8, 6)}
          </a>
        </h1>
        <StatusPill status={intent.status} />
      </div>
      <div className="mt-4 grid grid-cols-2 divide-x divide-[var(--hairline)] border-t border-[var(--hairline)] sm:grid-cols-4">
        <Stat label="Size" value={String(intent.size)} />
        <Stat
          label="Limit"
          value={formatPrice(intent.limit_price, intent.base_mint, intent.quote_mint)}
        />
        <Stat label="Reveal" value={<Countdown iso={intent.reveal_deadline} />} />
        <Stat label="Resolve" value={<Countdown iso={intent.resolve_deadline} />} />
      </div>
    </div>
  );
}

function Stat({
  label,
  value,
}: {
  label: string;
  value: React.ReactNode;
}) {
  return (
    <div className="px-4 py-3 first:pl-0 last:pr-0">
      <p className="font-mono text-[10px] uppercase tracking-[0.18em] text-muted">
        {label}
      </p>
      <p className="mt-1 font-mono text-[14px] tabular-nums text-foreground">
        {value}
      </p>
    </div>
  );
}

// ----- quote list ---------------------------------------------------

function QuoteList({
  quotes,
  winningId,
  mePk,
  baseMint,
  quoteMint,
}: {
  quotes: Quote[];
  winningId: string | null;
  mePk: string | null;
  baseMint: string;
  quoteMint: string;
}) {
  return (
    <div className="card">
      <div className="flex items-center justify-between border-b border-[var(--hairline)] px-5 py-3">
        <h2 className="text-[13px] font-medium text-foreground">
          Quotes
        </h2>
        <p className="font-mono text-[10px] uppercase tracking-[0.14em] text-faint">
          {quotes.length} sealed
        </p>
      </div>
      {quotes.length === 0 ? (
        <p className="px-5 py-12 text-center font-mono text-[11px] uppercase tracking-[0.14em] text-faint">
          no quotes
        </p>
      ) : (
        <ul className="divide-y divide-[var(--hairline)]">
          {quotes.map((q) => {
            const isWinner = q.id === winningId;
            const isMine = mePk && q.maker === mePk;
            return (
              <li
                key={q.id}
                className={`px-5 py-3 ${
                  isWinner ? "bg-[var(--buy)]/[0.04]" : ""
                } ${isMine ? "border-l-2 border-l-[var(--accent)] pl-[18px]" : ""}`}
              >
                <div className="flex items-center justify-between gap-3">
                  <div className="min-w-0">
                    <p className="font-mono text-[12px] text-foreground">
                      {shortPk(q.maker)}
                      {isMine && (
                        <span className="ml-2 font-mono text-[10px] uppercase tracking-[0.14em] text-[var(--accent)]">
                          you
                        </span>
                      )}
                      {isWinner && (
                        <span className="ml-2 font-mono text-[10px] uppercase tracking-[0.14em] text-[var(--buy)]">
                          winner
                        </span>
                      )}
                    </p>
                    <p className="mt-0.5 font-mono text-[10px] uppercase tracking-[0.14em] text-faint">
                      {timeAgo(q.created_at)}
                    </p>
                  </div>
                  <div className="text-right">
                    {q.revealed && q.revealed_price != null ? (
                      <p className="font-mono text-[13px] tabular-nums text-foreground">
                        {formatPrice(q.revealed_price, baseMint, quoteMint)} ×{" "}
                        {String(q.revealed_size ?? 0)}
                      </p>
                    ) : (
                      <p className="font-mono text-[11px] uppercase tracking-[0.14em] text-faint">
                        sealed
                      </p>
                    )}
                  </div>
                </div>
              </li>
            );
          })}
        </ul>
      )}
    </div>
  );
}

// ----- action cards -------------------------------------------------

function CancelCard({ intentId }: { intentId: string }) {
  const { publicKey } = useWallet();
  const toast = useToast();
  const { state, run } = useNyxbidTx(txApi.cancel);

  const onClick = async () => {
    if (!publicKey) return;
    const tid = toast.push({ kind: "info", title: "Cancelling intent" });
    try {
      const { signature: sig } = await run({
        taker: publicKey.toBase58(),
        intent: intentId,
      });
      toast.update(tid, {
        kind: "success",
        title: "Cancelled",
        href: explorerTxUrl(sig),
        hrefLabel: "View tx",
      });
    } catch (e) {
      toast.update(tid, { kind: "error", ...friendlyTxError(e) });
    }
  };

  return (
    <Card title="Cancel intent">
      <ActionButton
        onClick={onClick}
        phase={state.phase}
        variant="danger"
        className="w-full"
      >
        Cancel
      </ActionButton>
    </Card>
  );
}

function SubmitQuoteCard({
  intent,
  myQuote,
}: {
  intent: Intent;
  myQuote: Quote | null;
}) {
  const { publicKey } = useWallet();
  const toast = useToast();
  const { state, run } = useNyxbidTx(txApi.submitQuote);

  const [price, setPrice] = useState("");
  const [size, setSize] = useState(String(intent.size));

  if (myQuote) {
    return (
      <Card title="Quote sealed">
        <p className="font-mono text-[11px] uppercase tracking-[0.14em] text-faint">
          waiting for reveal window
        </p>
      </Card>
    );
  }

  const onClick = async () => {
    if (!publicKey) return;
    const priceScaled = priceToScaled(
      parseFloat(price),
      intent.base_mint,
      intent.quote_mint,
    );
    const sizeMinor = parseInt(size, 10);
    if (!Number.isFinite(priceScaled) || priceScaled <= 0) {
      toast.push({ kind: "error", title: "Invalid price" });
      return;
    }
    if (!Number.isFinite(sizeMinor) || sizeMinor <= 0) {
      toast.push({ kind: "error", title: "Invalid size" });
      return;
    }

    const commitNonce = randomBytes(32);
    const submitNonce = randomBytes(16);
    const commitNonceHex = bytesToHex(commitNonce);
    const commitment = await commitmentHex(
      priceScaled,
      sizeMinor,
      commitNonce,
    );

    const tid = toast.push({
      kind: "info",
      title: "Submitting sealed quote",
    });
    try {
      const { signature: sig } = await run({
        maker: publicKey.toBase58(),
        intent: intent.id,
        commitment_hex: commitment,
        nonce_hex: bytesToHex(submitNonce),
      });

      // Stash the reveal secret keyed by intent so the maker can
      // reveal later without retyping. localStorage is fine for
      // hackathon scope; production would put this in a maker bot.
      if (typeof window !== "undefined") {
        const key = `nyxbid:reveal:${intent.id}:${publicKey.toBase58()}`;
        localStorage.setItem(
          key,
          JSON.stringify({
            price: priceScaled,
            size: sizeMinor,
            commit_nonce_hex: commitNonceHex,
          }),
        );
      }

      toast.update(tid, {
        kind: "success",
        title: "Quote sealed",
        body: `Commitment ${commitment.slice(0, 8)}…`,
        href: explorerTxUrl(sig),
        hrefLabel: "View tx",
      });
    } catch (e) {
      toast.update(tid, { kind: "error", ...friendlyTxError(e) });
    }
  };

  return (
    <Card title="Submit sealed quote">
      <div className="space-y-3">
        <Input
          label="Price"
          value={price}
          onChange={setPrice}
          placeholder="195.40"
          mono
        />
        <Input
          label="Size"
          value={size}
          onChange={setSize}
          placeholder={String(intent.size)}
          mono
        />
        <ActionButton
          onClick={onClick}
          phase={state.phase}
          variant="primary"
          className="w-full"
        >
          Seal quote
        </ActionButton>
      </div>
    </Card>
  );
}

function RevealQuoteCard({
  intent,
  myQuote,
}: {
  intent: Intent;
  myQuote: Quote;
}) {
  const { publicKey } = useWallet();
  const toast = useToast();
  const { state, run } = useNyxbidTx(txApi.revealQuote);

  const stored =
    typeof window !== "undefined" && publicKey
      ? localStorage.getItem(
          `nyxbid:reveal:${intent.id}:${publicKey.toBase58()}`,
        )
      : null;
  const parsed = stored
    ? (JSON.parse(stored) as {
        price: number;
        size: number;
        commit_nonce_hex: string;
      })
    : null;

  const [price, setPrice] = useState(
    parsed ? formatPrice(parsed.price, intent.base_mint, intent.quote_mint) : "",
  );
  const [size, setSize] = useState(parsed ? String(parsed.size) : "");
  const [nonce, setNonce] = useState(parsed ? parsed.commit_nonce_hex : "");

  const onClick = async () => {
    if (!publicKey) return;
    const priceScaled = priceToScaled(
      parseFloat(price),
      intent.base_mint,
      intent.quote_mint,
    );
    const sizeMinor = parseInt(size, 10);
    try {
      hexToBytes(nonce); // throws on bad hex
    } catch {
      toast.push({ kind: "error", title: "Invalid nonce" });
      return;
    }

    const tid = toast.push({ kind: "info", title: "Revealing quote" });
    try {
      const { signature: sig } = await run({
        maker: publicKey.toBase58(),
        intent: intent.id,
        quote: myQuote.id,
        revealed_price: priceScaled,
        revealed_size: sizeMinor,
        commit_nonce_hex: nonce,
      });
      toast.update(tid, {
        kind: "success",
        title: "Quote revealed",
        href: explorerTxUrl(sig),
        hrefLabel: "View tx",
      });
    } catch (e) {
      toast.update(tid, { kind: "error", ...friendlyTxError(e) });
    }
  };

  return (
    <Card title="Reveal quote">
      <div className="space-y-3">
        <Input label="Price" value={price} onChange={setPrice} mono />
        <Input label="Size" value={size} onChange={setSize} mono />
        <Input
          label="Commit nonce (32-byte hex)"
          value={nonce}
          onChange={setNonce}
          mono
          small
        />
        <ActionButton
          onClick={onClick}
          phase={state.phase}
          variant="primary"
          className="w-full"
        >
          Reveal
        </ActionButton>
      </div>
    </Card>
  );
}

function FundEscrowCard({
  intent,
  quote,
}: {
  intent: Intent;
  quote: Quote;
}) {
  const { publicKey } = useWallet();
  const toast = useToast();
  const { state, run } = useNyxbidTx(txApi.fundMakerEscrow);

  // Default amount: revealed_size for sells (maker delivers base);
  // size * limit / SCALE for buys (maker delivers quote).
  const defaultAmount =
    intent.side === "buy"
      ? String(quote.revealed_size ?? 0)
      : String(
          Math.floor(
            ((quote.revealed_size ?? 0) * (quote.revealed_price ?? 0)) /
              1_000_000,
          ),
        );
  const [amount, setAmount] = useState(defaultAmount);

  const onClick = async () => {
    if (!publicKey) return;
    const amt = parseInt(amount, 10);
    if (!Number.isFinite(amt) || amt <= 0) {
      toast.push({ kind: "error", title: "Invalid amount" });
      return;
    }
    const tid = toast.push({
      kind: "info",
      title: "Funding maker leg",
    });
    try {
      const { signature: sig } = await run({
        maker: publicKey.toBase58(),
        intent: intent.id,
        quote: quote.id,
        amount: amt,
      });
      toast.update(tid, {
        kind: "success",
        title: "Maker leg funded",
        href: explorerTxUrl(sig),
        hrefLabel: "View tx",
      });
    } catch (e) {
      toast.update(tid, { kind: "error", ...friendlyTxError(e) });
    }
  };

  return (
    <Card title="Fund maker leg">
      <div className="space-y-3">
        <Input
          label="Amount (raw units)"
          value={amount}
          onChange={setAmount}
          mono
        />
        <ActionButton
          onClick={onClick}
          phase={state.phase}
          variant="primary"
          className="w-full"
        >
          Lock funds
        </ActionButton>
      </div>
    </Card>
  );
}

function SettleCard({ intent }: { intent: Intent }) {
  const { publicKey } = useWallet();
  const toast = useToast();
  const { state, run } = useNyxbidTx(txApi.settle);

  const onClick = async () => {
    if (!publicKey) return;
    const tid = toast.push({ kind: "info", title: "Settling" });
    try {
      const { signature: sig } = await run({
        payer: publicKey.toBase58(),
        intent: intent.id,
      });
      toast.update(tid, {
        kind: "success",
        title: "Settled",
        body: "Atomic SPL transfer + receipt minted",
        href: explorerTxUrl(sig),
        hrefLabel: "View tx",
      });
    } catch (e) {
      toast.update(tid, { kind: "error", ...friendlyTxError(e) });
    }
  };

  return (
    <Card title="Settle">
      <ActionButton
        onClick={onClick}
        phase={state.phase}
        variant="primary"
        className="w-full"
      >
        Settle atomically
      </ActionButton>
    </Card>
  );
}

function TerminalCard({
  intent,
  winning,
}: {
  intent: Intent;
  winning: Quote | null;
}) {
  if (intent.status === "settled" && winning) {
    return (
      <Card title="Settled">
        <div className="space-y-2.5">
          <Row
            label="Price"
            value={formatPrice(
              winning.revealed_price ?? 0,
              intent.base_mint,
              intent.quote_mint,
            )}
          />
          <Row label="Size" value={String(winning.revealed_size ?? 0)} />
          <Row label="Maker" value={shortPk(winning.maker)} />
        </div>
      </Card>
    );
  }
  if (intent.status === "expired") {
    return (
      <Card title="Expired">
        <p className="font-mono text-[11px] uppercase tracking-[0.14em] text-faint">
          refunded
        </p>
      </Card>
    );
  }
  return (
    <Card title="Cancelled">
      <p className="font-mono text-[11px] uppercase tracking-[0.14em] text-faint">
        by taker
      </p>
    </Card>
  );
}

function MetaCard({ intent }: { intent: Intent }) {
  return (
    <Card title="Detail">
      <div className="space-y-2.5">
        <Row label="Taker" value={shortPk(intent.taker)} />
        <Row label="Base" value={shortPk(intent.base_mint)} />
        <Row label="Quote" value={shortPk(intent.quote_mint)} />
        <Row label="Created" value={timeAgo(intent.created_at)} />
      </div>
      <a
        href={explorerAccountUrl(intent.id)}
        target="_blank"
        rel="noopener noreferrer"
        className="mt-4 inline-flex font-mono text-[11px] uppercase tracking-[0.14em] text-[var(--accent)] hover:underline"
      >
        Solana Explorer →
      </a>
    </Card>
  );
}

function Card({
  title,
  children,
}: {
  title: string;
  children: React.ReactNode;
}) {
  return (
    <div className="card p-5">
      <h3 className="font-mono text-[10px] uppercase tracking-[0.18em] text-muted">
        {title}
      </h3>
      <div className="mt-4">{children}</div>
    </div>
  );
}

function Input({
  label,
  value,
  onChange,
  placeholder,
  mono,
  small,
}: {
  label: string;
  value: string;
  onChange: (v: string) => void;
  placeholder?: string;
  mono?: boolean;
  small?: boolean;
}) {
  return (
    <label className="block">
      <span className="font-mono text-[10px] uppercase tracking-[0.18em] text-muted">
        {label}
      </span>
      <input
        value={value}
        onChange={(e) => onChange(e.target.value)}
        placeholder={placeholder}
        className={`input mt-1.5 ${mono ? "font-mono" : ""} ${small ? "text-[11px]" : ""}`}
      />
    </label>
  );
}

function Row({
  label,
  value,
}: {
  label: string;
  value: React.ReactNode;
}) {
  return (
    <div className="flex items-center justify-between">
      <span className="font-mono text-[10px] uppercase tracking-[0.18em] text-muted">
        {label}
      </span>
      <span className="font-mono text-[12px] tabular-nums text-foreground">
        {value}
      </span>
    </div>
  );
}
