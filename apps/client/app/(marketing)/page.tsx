import Link from "next/link";

const features = [
  {
    title: "Sealed-bid RFQ",
    description:
      "Makers submit commitments, not prices. Nobody sees the book — not even the venue. Price discovery stays private until resolution.",
    icon: (
      <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round">
        <rect x="4" y="10" width="16" height="10" rx="2" />
        <path d="M8 10V7a4 4 0 018 0v3" />
      </svg>
    ),
  },
  {
    title: "Atomic settlement",
    description:
      "Winning quote is revealed and settled in a single Solana transaction. HTLC-style escrow guarantees no half-fills and no frontrunning.",
    icon: (
      <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round">
        <path d="M12 2v20M4 12h16" />
      </svg>
    ),
  },
  {
    title: "Agent-native via MCP",
    description:
      "The full intent lifecycle is exposed as Model Context Protocol tools. AI agents quote, auction and settle without glue code.",
    icon: (
      <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round">
        <circle cx="12" cy="12" r="3" />
        <path d="M12 2v4M12 18v4M2 12h4M18 12h4M5 5l3 3M16 16l3 3M5 19l3-3M16 8l3-3" />
      </svg>
    ),
  },
];

const steps = [
  {
    step: "01",
    title: "Taker posts an intent",
    description:
      "A taker broadcasts `buy 50k SOL up to $195` as an intent PDA. No book reveals. A reveal deadline is set.",
  },
  {
    step: "02",
    title: "Makers commit sealed quotes",
    description:
      "Makers submit hash commitments of (price, size, nonce). Nothing is readable on-chain until reveal. No frontrunning surface.",
  },
  {
    step: "03",
    title: "Resolve + settle atomically",
    description:
      "At the deadline, makers reveal. The program picks the best valid quote and settles via HTLC-style escrow in one transaction.",
  },
];

export default function LandingPage() {
  return (
    <>
      <section className="mx-auto max-w-3xl px-6 pb-20 pt-24 text-center">
        <p className="text-sm font-medium tracking-wide text-accent">
          OTC infrastructure for Solana
        </p>
        <h1 className="mt-4 text-4xl font-bold tracking-tight sm:text-5xl">
          Sealed-bid RFQ.
          <br />
          Atomic settlement.
        </h1>
        <p className="mx-auto mt-6 max-w-xl text-lg text-muted">
          Nyxbid is a private venue for OTC-size trades on Solana. Sealed bids,
          atomic settlement, agent-native via MCP.
        </p>
        <div className="mt-10 flex items-center justify-center gap-4">
          <Link
            href="/docs"
            className="rounded-md bg-foreground px-5 py-2.5 text-sm font-medium text-background hover:bg-foreground/90"
          >
            Read the docs
          </Link>
          <Link
            href="/dashboard"
            className="rounded-md border border-border px-5 py-2.5 text-sm font-medium text-foreground hover:bg-accent/5"
          >
            Open the app
          </Link>
        </div>
      </section>

      <section className="border-t border-border bg-card/50">
        <div className="mx-auto max-w-5xl px-6 py-20">
          <div className="grid gap-12 md:grid-cols-3">
            {features.map((f) => (
              <div key={f.title}>
                <div className="flex h-10 w-10 items-center justify-center rounded-lg border border-border bg-card text-muted">
                  {f.icon}
                </div>
                <h3 className="mt-4 font-semibold">{f.title}</h3>
                <p className="mt-2 text-sm leading-relaxed text-muted">
                  {f.description}
                </p>
              </div>
            ))}
          </div>
        </div>
      </section>

      <section className="border-t border-border">
        <div className="mx-auto max-w-5xl px-6 py-20">
          <h2 className="text-2xl font-bold tracking-tight">How it works</h2>
          <p className="mt-2 text-sm text-muted">
            Three phases from intent to settlement.
          </p>
          <div className="mt-12 grid gap-10 md:grid-cols-3">
            {steps.map((s) => (
              <div key={s.step}>
                <p className="font-mono text-xs text-accent">{s.step}</p>
                <h3 className="mt-2 font-semibold">{s.title}</h3>
                <p className="mt-2 text-sm leading-relaxed text-muted">
                  {s.description}
                </p>
              </div>
            ))}
          </div>
        </div>
      </section>

      <section className="border-t border-border bg-card/50">
        <div className="mx-auto max-w-5xl px-6 py-20">
          <h2 className="text-2xl font-bold tracking-tight">Architecture</h2>
          <p className="mt-2 text-sm text-muted">
            Anchor program + Rust server + Next.js surface.
          </p>
          <div className="mt-8 overflow-hidden rounded-lg border border-border bg-card">
            <pre className="overflow-x-auto px-6 py-5 font-mono text-sm leading-relaxed text-muted">
{`taker ─────────────┐                              ┌──── Anchor program
                   ▼                              ▼     Intent / Quote /
      ┌────────────────────────┐   settle tx    Escrow / Receipt
      │    nyxbid-server       │ ───────────────►
      │    (Rust / Axum)       │
      │                        │
      │  intent  auction  mcp  │ ◀── MCP tools ── AI maker/taker agents
      └────────────────────────┘
                   ▲
                   │ SSE
              nyxbid-client (Next.js)`}
            </pre>
          </div>
        </div>
      </section>

      <section className="border-t border-border">
        <div className="mx-auto max-w-5xl px-6 py-20">
          <h2 className="text-2xl font-bold tracking-tight">Stack</h2>
          <div className="mt-8 grid gap-px overflow-hidden rounded-lg border border-border bg-border sm:grid-cols-2 lg:grid-cols-4">
            {[
              { label: "Server", value: "Rust / Axum" },
              { label: "Client", value: "Next.js / Bun" },
              { label: "Chain", value: "Anchor 1.0" },
              { label: "Settlement", value: "Solana / USDC" },
            ].map((item) => (
              <div key={item.label} className="bg-card px-5 py-4">
                <p className="text-xs font-medium uppercase tracking-wide text-muted">
                  {item.label}
                </p>
                <p className="mt-1 font-mono text-sm">{item.value}</p>
              </div>
            ))}
          </div>
        </div>
      </section>
    </>
  );
}
