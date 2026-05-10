"use client";

import { useState } from "react";
import { useRouter } from "next/navigation";
import { useWallet } from "@solana/wallet-adapter-react";

import type { Market } from "@/lib/data";
import { tx as txApi } from "@/lib/api";
import { bytesToHex, randomBytes } from "@/lib/commitment";
import { priceToScaled, explorerTxUrl } from "@/lib/format";
import { useNyxbidTx } from "@/hooks/use-nyxbid-tx";
import { useToast } from "@/components/toast";
import { ActionButton } from "@/components/action-button";

interface Props {
  markets: Market[];
}

// Fallback used only when /api/markets fails. We pick the devnet
// USDC faucet mint here because the running stack defaults to devnet;
// in production the server's /api/markets endpoint is the source of
// truth for the active mint and this fallback never gets read.
const FALLBACK_MARKET: Market = {
  symbol: "SOL/USDC",
  base_mint: "So11111111111111111111111111111111111111112",
  quote_mint: "4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU",
  min_size: 100_000_000,
  base_decimals: 9,
  quote_decimals: 6,
};

export function TradeForm({ markets }: Props) {
  const router = useRouter();
  const { publicKey } = useWallet();
  const toast = useToast();

  const market = markets[0] ?? FALLBACK_MARKET;
  const [base, quote] = market.symbol.split("/");

  const [side, setSide] = useState<"buy" | "sell">("buy");
  const [sizeBaseUnits, setSizeBaseUnits] = useState("1");
  const [limitPrice, setLimitPrice] = useState("100");
  const [windowSecs, setWindowSecs] = useState(60);

  const { state, run } = useNyxbidTx(txApi.createIntent);

  const sizeNum = parseFloat(sizeBaseUnits || "0");
  const priceNum = parseFloat(limitPrice || "0");
  const lockingHuman =
    side === "buy"
      ? `${(sizeNum * priceNum).toLocaleString(undefined, { maximumFractionDigits: 2 })} ${quote}`
      : `${sizeBaseUnits || "0"} ${base}`;

  // Pre-compute encoded values so the diagnostic strip below can show
  // exactly what will be sent to the chain. The same numbers are
  // recomputed inside `submit()` — kept here only for display.
  const previewSizeMinor = Number.isFinite(sizeNum)
    ? Math.round(sizeNum * Math.pow(10, market.base_decimals))
    : 0;
  const previewPriceScaled =
    Number.isFinite(priceNum) && priceNum > 0
      ? priceToScaled(priceNum, market.base_mint, market.quote_mint)
      : 0;
  const previewLockMinor =
    side === "buy"
      ? Math.floor((previewSizeMinor * previewPriceScaled) / 1_000_000)
      : previewSizeMinor;
  const previewLockMint =
    side === "buy" ? market.quote_mint : market.base_mint;
  const previewLockToken = side === "buy" ? quote : base;
  const previewLockHuman =
    previewLockMinor /
    Math.pow(
      10,
      side === "buy" ? market.quote_decimals : market.base_decimals,
    );

  const submit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!publicKey) {
      toast.push({
        kind: "error",
        title: "Connect a wallet",
        body: "Phantom, Solflare, or Backpack on devnet.",
      });
      return;
    }
    const sizeMinor = Math.round(sizeNum * Math.pow(10, market.base_decimals));
    const priceScaled = priceToScaled(
      priceNum,
      market.base_mint,
      market.quote_mint,
    );
    if (!Number.isFinite(sizeMinor) || sizeMinor <= 0) {
      toast.push({ kind: "error", title: "Invalid size" });
      return;
    }
    if (!Number.isFinite(priceScaled) || priceScaled <= 0) {
      toast.push({ kind: "error", title: "Invalid price" });
      return;
    }

    // Diagnostic: verify the new decimal-aware encoding is loaded.
    // If priceScaled is in the hundreds-of-millions range for a "100"
    // input, the client is still on stale code (turbopack cache).
    console.log("[nyxbid] create_intent →", {
      side,
      sizeMinor,
      priceScaled,
      lockMint: side === "buy" ? market.quote_mint : market.base_mint,
      lockMinor:
        side === "buy"
          ? Math.floor((sizeMinor * priceScaled) / 1_000_000)
          : sizeMinor,
      base_decimals: market.base_decimals,
      quote_decimals: market.quote_decimals,
    });

    const now = Math.floor(Date.now() / 1000);
    const reveal = now + windowSecs;
    const resolve = reveal + Math.max(15, Math.floor(windowSecs / 2));
    const settle = resolve + Math.max(60, windowSecs);
    const nonceHex = bytesToHex(randomBytes(16));

    const tid = toast.push({
      kind: "info",
      title: `${side === "buy" ? "Buy" : "Sell"} intent`,
      body: `${sizeBaseUnits} ${base} @ ${limitPrice}`,
    });

    try {
      // `run` returns the prepared tx alongside the signature so we
      // can read the intent PDA without waiting for `state.prepared`
      // to be flushed by React's async-state batching.
      const { signature, prepared } = await run({
        taker: publicKey.toBase58(),
        side,
        base_mint: market.base_mint,
        quote_mint: market.quote_mint,
        size: sizeMinor,
        limit_price: priceScaled,
        reveal_deadline: reveal,
        resolve_deadline: resolve,
        settle_deadline: settle,
        nonce_hex: nonceHex,
      });
      const intentPda = prepared.accounts.intent;
      toast.update(tid, {
        kind: "success",
        title: "Intent posted",
        body: intentPda ? `${intentPda.slice(0, 8)}…` : undefined,
        href: explorerTxUrl(signature),
        hrefLabel: "View tx",
      });
      if (intentPda) router.push(`/intents/${intentPda}`);
    } catch (err) {
      toast.update(tid, {
        kind: "error",
        title: "Failed",
        body: err instanceof Error ? err.message : "try again",
      });
    }
  };

  return (
    <form
      onSubmit={submit}
      className="card grid gap-0 lg:grid-cols-[1fr_320px]"
    >
      {/* left — inputs */}
      <div className="space-y-6 p-6 lg:border-r lg:border-[var(--hairline)]">
        <SideToggle value={side} onChange={setSide} />

        <Field label="Size" hint={base}>
          <input
            inputMode="decimal"
            value={sizeBaseUnits}
            onChange={(e) => setSizeBaseUnits(e.target.value)}
            className="input"
            placeholder="0.00"
          />
        </Field>

        <Field label="Limit" hint={`${quote} per ${base}`}>
          <input
            inputMode="decimal"
            value={limitPrice}
            onChange={(e) => setLimitPrice(e.target.value)}
            className="input"
            placeholder="0.00"
          />
        </Field>

        <Field label="Auction window" hint={`${windowSecs}s`}>
          <div className="flex items-center gap-3">
            <input
              type="range"
              min={15}
              max={300}
              step={5}
              value={windowSecs}
              onChange={(e) => setWindowSecs(Number(e.target.value))}
              className="flex-1 accent-[var(--accent)]"
            />
            <span className="w-12 text-right font-mono text-[12px] tabular-nums text-muted">
              {windowSecs}s
            </span>
          </div>
        </Field>
      </div>

      {/* right — summary + cta */}
      <div className="flex flex-col justify-between gap-6 bg-[var(--surface-2)] p-6">
        <div className="space-y-2.5">
          <Row label="Market" value={market.symbol} />
          <Row
            label="Side"
            value={
              <span
                className={
                  side === "buy" ? "text-[var(--buy)]" : "text-[var(--sell)]"
                }
              >
                {side.toUpperCase()}
              </span>
            }
          />
          <Row label="Locking" value={lockingHuman} />
          <Row label="Reveal" value={`now + ${windowSecs}s`} />
        </div>

        {/* Diagnostic strip — surfaces the exact integers the program
         * will see. If "Lock (minor)" is wildly larger than your wallet
         * balance, the client is encoding price wrong (stale build). */}
        <div className="space-y-1.5 rounded-[var(--r-sm)] border border-dashed border-[var(--hairline-strong)] bg-[var(--surface)] p-3 font-mono text-[10px] leading-[1.6] text-faint">
          <div className="flex justify-between gap-3">
            <span>size (minor)</span>
            <span className="tabular-nums text-muted">{previewSizeMinor.toLocaleString()}</span>
          </div>
          <div className="flex justify-between gap-3">
            <span>price (scaled)</span>
            <span className="tabular-nums text-muted">{previewPriceScaled.toLocaleString()}</span>
          </div>
          <div className="flex justify-between gap-3">
            <span>lock (minor)</span>
            <span className="tabular-nums text-muted">
              {previewLockMinor.toLocaleString()}
            </span>
          </div>
          <div className="flex justify-between gap-3">
            <span>lock (human)</span>
            <span className="tabular-nums text-muted">
              {previewLockHuman.toLocaleString(undefined, {
                maximumFractionDigits: 6,
              })}{" "}
              {previewLockToken}
            </span>
          </div>
          <div className="flex justify-between gap-3">
            <span>lock mint</span>
            <span className="truncate text-muted" title={previewLockMint}>
              {previewLockMint.slice(0, 4)}…{previewLockMint.slice(-4)}
            </span>
          </div>
        </div>

        <ActionButton
          type="submit"
          phase={state.phase}
          variant="primary"
          className="w-full"
        >
          {publicKey ? "Post intent" : "Connect wallet"}
        </ActionButton>
      </div>
    </form>
  );
}

function SideToggle({
  value,
  onChange,
}: {
  value: "buy" | "sell";
  onChange: (v: "buy" | "sell") => void;
}) {
  return (
    <div
      role="radiogroup"
      aria-label="Side"
      className="inline-flex rounded-[var(--r-sm)] border border-[var(--hairline-strong)] bg-[var(--surface-2)] p-0.5"
    >
      {(["buy", "sell"] as const).map((s) => (
        <button
          type="button"
          key={s}
          role="radio"
          aria-checked={value === s}
          onClick={() => onChange(s)}
          className={`h-8 min-w-[78px] rounded-[var(--r-xs)] px-4 font-mono text-[11px] uppercase tracking-[0.14em] transition-colors ${
            value === s
              ? s === "buy"
                ? "bg-[var(--buy)]/12 text-[var(--buy)]"
                : "bg-[var(--sell)]/12 text-[var(--sell)]"
              : "text-muted hover:text-foreground"
          }`}
        >
          {s}
        </button>
      ))}
    </div>
  );
}

function Field({
  label,
  hint,
  children,
}: {
  label: string;
  hint?: string;
  children: React.ReactNode;
}) {
  return (
    <label className="block">
      <span className="flex items-baseline justify-between">
        <span className="font-mono text-[10px] uppercase tracking-[0.18em] text-muted">
          {label}
        </span>
        {hint && (
          <span className="font-mono text-[10px] tracking-tight text-faint">
            {hint}
          </span>
        )}
      </span>
      <span className="mt-1.5 block">{children}</span>
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
    <div className="flex items-center justify-between gap-4">
      <span className="font-mono text-[10px] uppercase tracking-[0.18em] text-muted">
        {label}
      </span>
      <span className="font-mono text-[12px] tabular-nums text-foreground">
        {value}
      </span>
    </div>
  );
}
