import Image from "next/image";
import Link from "next/link";

import { GithubIcon } from "@/components/icons";

/**
 * Landing.
 *
 * Composition:
 *   1. Hero        cosmos · wordmark+github float in hero corners ·
 *                  serif headline · pill CTA · grain
 *   2. Edge        cream paper · 3 hairline rows · serif headline
 *   3. Movements   cosmos · 3 numbered movements
 *   4. Builders    cosmos · 2-col with sunsetdesk image
 *   5. CTA         full-bleed cloudtop · final pill
 *
 * Why no <header>: the user repeatedly read any persistent top
 * element as a "solid navbar". Rendering the brand and icon as
 * absolute children of the hero section solves that — they live
 * inside the hero's grain/cosmos surface, so they belong to the
 * hero rather than to a separate bar.
 */
export default function LandingPage() {
  return (
    <>
      <Hero />
      <Edge />
      <Movements />
      <Builders />
      <CTA />
    </>
  );
}

function ArrowRight() {
  return (
    <svg
      width="18"
      height="18"
      viewBox="0 0 18 18"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.6"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden
    >
      <path d="M4 9h10M9 4l5 5-5 5" />
    </svg>
  );
}

function HeroChrome() {
  return (
    <div className="pointer-events-none absolute inset-x-0 top-0 z-10 px-6 pt-7 md:px-10 md:pt-9">
      <div className="pointer-events-auto mx-auto flex max-w-7xl items-center justify-between">
        <Link
          href="/"
          className="group flex items-center gap-3 text-[26px] tracking-tight text-foreground/95 hover:text-foreground sm:text-[28px] md:gap-3.5 md:text-[32px]"
          style={{ fontFamily: "var(--font-serif)" }}
        >
          <Image
            src="/logo.png"
            alt=""
            width={40}
            height={40}
            className="h-9 w-9 shrink-0 object-contain sm:h-10 sm:w-10 md:h-11 md:w-11"
            priority
          />
          <span>Nyxbid</span>
        </Link>
        <Link
          href="https://github.com/Nyxbid/Nyxbid"
          target="_blank"
          rel="noopener noreferrer"
          aria-label="Nyxbid on GitHub"
          className="inline-flex h-9 w-9 items-center justify-center rounded-full text-[color-mix(in_srgb,var(--fg)_80%,transparent)] transition-colors hover:bg-[color-mix(in_srgb,var(--fg)_8%,transparent)] hover:text-foreground"
        >
          <GithubIcon size={19} />
        </Link>
      </div>
    </div>
  );
}

function Hero() {
  return (
    <section className="lp-cosmos">
      <HeroChrome />
      <div className="relative mx-auto max-w-5xl px-6 pb-32 pt-32 text-center md:px-10 md:pb-40 md:pt-40">
        <p className="lp-eyebrow">Sealed-bid OTC · Solana</p>
        <h1 className="lp-display mt-6 text-[52px] sm:text-[76px] md:text-[96px]">
          Trade in size.
          <br />
          <em>Without showing&nbsp;your hand.</em>
        </h1>
        <p
          className="mx-auto mt-8 max-w-xl text-balance text-[16px] leading-[1.55] text-[color-mix(in_srgb,var(--fg)_72%,transparent)]"
          style={{ fontFamily: "var(--font-geist-sans)" }}
        >
          A private RFQ venue for OTC-size trades. Sealed bids, atomic
          settlement, agent-native via A2A.
        </p>

        <div className="mt-12 flex flex-col items-center gap-5 sm:flex-row sm:justify-center sm:gap-7">
          <Link href="/trade" className="lp-pill">
            <span className="lp-pill__disc">
              <ArrowRight />
            </span>
            Open the app
          </Link>
          <Link href="/docs" className="lp-link">
            Read the docs
          </Link>
        </div>
      </div>
    </section>
  );
}

function Edge() {
  return (
    <section className="lp-cream">
      <div className="mx-auto grid max-w-6xl gap-16 px-6 py-32 md:grid-cols-[1.1fr_1fr] md:px-10 md:py-40">
        <div>
          <p className="lp-eyebrow">The edge</p>
          <h2 className="lp-display mt-6 text-[40px] sm:text-[52px] md:text-[64px]">
            Built for the way{" "}
            <em style={{ color: "var(--accent)" }}>agents trade.</em>
          </h2>
        </div>

        <div className="space-y-8 self-end pt-2 text-[15px] leading-[1.65] text-[color-mix(in_srgb,var(--lp-ink)_72%,transparent)]">
          <CreamRow
            title="Sealed bids"
            body="Makers commit hashes, not prices. Nothing leaks on-chain until reveal — no quote-sniping, no copytraders shadowing the book."
          />
          <CreamRow
            title="Atomic settlement"
            body="Both legs swap inside a single Solana transaction. No half-fills, no settlement risk, no relayer trust."
          />
          <CreamRow
            title="Agent-native"
            body="Discovery and the task lifecycle ride on Google's A2A v1: a signed agent card, a JSON-RPC endpoint, and live SSE event streams. Drop in any A2A client — no SDK, no API key."
          />
        </div>
      </div>
    </section>
  );
}

function CreamRow({ title, body }: { title: string; body: string }) {
  return (
    <div className="border-t border-[color-mix(in_srgb,var(--lp-ink)_15%,transparent)] pt-7">
      <h3 className="font-mono text-[11px] uppercase tracking-[0.22em] text-[var(--lp-ink)]">
        {title}
      </h3>
      <p className="mt-3">{body}</p>
    </div>
  );
}

function Movements() {
  const steps = [
    {
      n: "I",
      title: "Post intent",
      body: "Taker broadcasts a sealed RFQ with a reveal deadline. Their leg is locked into escrow on the spot.",
    },
    {
      n: "II",
      title: "Sealed quotes",
      body: "Makers submit hash commitments of (price, size, nonce). The book stays private until the auction closes.",
    },
    {
      n: "III",
      title: "Reveal & settle",
      body: "Best valid quote wins. Both legs swap atomically through HTLC-style escrow. Receipt is on-chain.",
    },
  ];
  return (
    <section className="lp-cosmos">
      <div className="mx-auto max-w-6xl px-6 py-28 md:px-10 md:py-36">
        <p className="lp-eyebrow">How it works</p>
        <h2 className="lp-display mt-6 text-[40px] sm:text-[52px] md:text-[64px]">
          Three movements,
          <br />
          <em>one transaction.</em>
        </h2>

        <div className="mt-20 grid gap-12 md:grid-cols-3 md:gap-10">
          {steps.map((s) => (
            <div
              key={s.n}
              className="border-t border-[color-mix(in_srgb,var(--fg)_18%,transparent)] pt-7"
            >
              <p
                className="text-[28px] leading-none"
                style={{
                  fontFamily: "var(--font-serif)",
                  color: "var(--accent-soft)",
                }}
              >
                {s.n}
              </p>
              <h3
                className="mt-5 text-[22px] text-foreground"
                style={{ fontFamily: "var(--font-serif)" }}
              >
                {s.title}
              </h3>
              <p className="mt-3 text-[14px] leading-[1.65] text-[color-mix(in_srgb,var(--fg)_72%,transparent)]">
                {s.body}
              </p>
            </div>
          ))}
        </div>
      </div>
    </section>
  );
}

function Builders() {
  return (
    <section className="lp-cosmos">
      <div className="mx-auto grid max-w-6xl items-center gap-16 px-6 py-28 md:grid-cols-[1fr_1.05fr] md:px-10 md:py-36">
        <FeatureImage
          src="/sunsetdesk.png"
          alt="A solo builder watching markets and a sunset roll across mountain valleys."
          aspect="4 / 5"
        />

        <div>
          <p className="lp-eyebrow">For agents &amp; builders</p>
          <h2 className="lp-display mt-6 text-[40px] sm:text-[52px] md:text-[64px]">
            Spec-compliant.
            <br />
            <em style={{ color: "var(--accent-soft)" }}>Agent-native.</em>
          </h2>
          <p className="mt-8 max-w-md text-[15px] leading-[1.65] text-[color-mix(in_srgb,var(--fg)_75%,transparent)]">
            Fetch the signed{" "}
            <code className="font-mono text-[13px] text-[var(--accent-soft)]">
              agent-card.json
            </code>
            , open one{" "}
            <code className="font-mono text-[13px] text-[var(--accent-soft)]">
              message/stream
            </code>{" "}
            SSE call, and your bot is live on the venue. JSON-RPC for
            send / cancel / resubscribe, push webhooks for offline
            agents, JWS-verified identity end-to-end.
          </p>
          <ul className="mt-7 space-y-2 text-[13px] font-mono leading-[1.7] text-[color-mix(in_srgb,var(--fg)_72%,transparent)]">
            <li>· Google A2A v1, not a custom protocol</li>
            <li>· 9 well-known skills, all map to unsigned txs</li>
            <li>· No API key, no rate-limit handshake</li>
          </ul>
          <div className="mt-10 flex flex-wrap items-center gap-6">
            <Link href="/docs/agents" className="lp-link">
              Agent integration
            </Link>
            <Link href="/docs/makers" className="lp-link">
              Maker mechanics
            </Link>
          </div>
        </div>
      </div>
    </section>
  );
}

function CTA() {
  return (
    <section className="relative isolate overflow-hidden">
      <div className="absolute inset-0 -z-10">
        <Image
          src="/cloudtop.png"
          alt=""
          fill
          priority={false}
          sizes="100vw"
          className="object-cover"
        />
        <div
          aria-hidden
          className="absolute inset-0"
          style={{
            background:
              "linear-gradient(180deg, color-mix(in srgb, var(--bg) 60%, transparent) 0%, color-mix(in srgb, var(--bg) 30%, transparent) 50%, color-mix(in srgb, var(--bg) 80%, transparent) 100%)",
          }}
        />
        <div
          aria-hidden
          className="absolute inset-0 opacity-30 mix-blend-soft-light"
          style={{
            backgroundImage:
              "url(\"data:image/svg+xml;utf8,<svg xmlns='http://www.w3.org/2000/svg' width='320' height='320'><filter id='n'><feTurbulence type='fractalNoise' baseFrequency='0.78' numOctaves='2' stitchTiles='stitch'/><feColorMatrix values='0 0 0 0 1  0 0 0 0 1  0 0 0 0 1  0 0 0 0.55 0'/></filter><rect width='100%' height='100%' filter='url(%23n)'/></svg>\")",
            backgroundSize: "320px 320px",
          }}
        />
      </div>

      <div className="relative mx-auto max-w-3xl px-6 py-40 text-center md:px-10 md:py-52">
        <p className="lp-eyebrow">The venue</p>
        <h2 className="lp-display mt-6 text-[44px] sm:text-[60px] md:text-[80px]">
          Open the <em>venue.</em>
        </h2>
        <p className="mx-auto mt-6 max-w-md text-[15px] leading-[1.65] text-[color-mix(in_srgb,var(--fg)_82%,transparent)]">
          No custody. No relayer keys. Sign once with Phantom,
          Solflare, or Backpack.
        </p>
        <div className="mt-12 flex justify-center">
          <Link href="/trade" className="lp-pill">
            <span className="lp-pill__disc">
              <ArrowRight />
            </span>
            Launch app
          </Link>
        </div>
      </div>
    </section>
  );
}

function FeatureImage({
  src,
  alt,
  aspect,
}: {
  src: string;
  alt: string;
  aspect: string;
}) {
  return (
    <div
      className="relative overflow-hidden rounded-[10px] border border-[color-mix(in_srgb,var(--fg)_12%,transparent)]"
      style={{ aspectRatio: aspect }}
    >
      <Image
        src={src}
        alt={alt}
        fill
        sizes="(min-width: 768px) 50vw, 100vw"
        className="object-cover"
      />
      <div
        aria-hidden
        className="pointer-events-none absolute inset-0"
        style={{
          background:
            "linear-gradient(180deg, color-mix(in srgb, var(--bg) 18%, transparent), transparent 40%, color-mix(in srgb, var(--bg) 25%, transparent))",
        }}
      />
    </div>
  );
}
