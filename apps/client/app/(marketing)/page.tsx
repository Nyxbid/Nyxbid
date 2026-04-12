import Link from "next/link";

const features = [
  {
    title: "On-chain policy",
    description:
      "Spend limits, per-transaction caps, and tool allowlists enforced by a Solana program. No trust assumptions.",
    icon: (
      <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round">
        <rect x="3" y="11" width="18" height="11" rx="2" ry="2" />
        <path d="M7 11V7a5 5 0 0110 0v4" />
      </svg>
    ),
  },
  {
    title: "Spend receipts",
    description:
      "Every API call produces a receipt with a hash-linked audit trail. Optionally anchored on-chain for immutability.",
    icon: (
      <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round">
        <path d="M14 2H6a2 2 0 00-2 2v16a2 2 0 002 2h12a2 2 0 002-2V8z" />
        <polyline points="14 2 14 8 20 8" />
        <line x1="16" y1="13" x2="8" y2="13" />
        <line x1="16" y1="17" x2="8" y2="17" />
        <polyline points="10 9 9 9 8 9" />
      </svg>
    ),
  },
  {
    title: "Multi-LLM routing",
    description:
      "Gemini, Groq, OpenAI. The server routes agent requests to the right provider and tracks cost per call.",
    icon: (
      <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round">
        <polyline points="16 18 22 12 16 6" />
        <polyline points="8 6 2 12 8 18" />
      </svg>
    ),
  },
];

const steps = [
  {
    step: "01",
    title: "Agent submits a proposal",
    description:
      "An agent sends a tool request with its ID and prompt. The server checks the policy on-chain before proceeding.",
  },
  {
    step: "02",
    title: "Tool executes, cost recorded",
    description:
      "The server calls the tool (LLM, oracle, API), estimates cost in USDC, and creates a spend receipt with a SHA-256 hash.",
  },
  {
    step: "03",
    title: "Receipt anchored on Solana",
    description:
      "The receipt is written to a PDA on Solana. Daily spend counters update atomically. The dashboard shows it in real time.",
  },
];

export default function LandingPage() {
  return (
    <>
      <section className="mx-auto max-w-3xl px-6 pb-20 pt-24 text-center">
        <p className="text-sm font-medium tracking-wide text-accent">
          Agentic payments on Solana
        </p>
        <h1 className="mt-4 text-4xl font-bold tracking-tight sm:text-5xl">
          Pay-per-call for AI agents.
          <br />
          Policy on-chain.
        </h1>
        <p className="mx-auto mt-6 max-w-xl text-lg text-muted">
          Program-enforced spend limits, hash-linked receipts, and multi-LLM
          routing. The financial control plane for autonomous agents.
        </p>
        <div className="mt-10 flex items-center justify-center gap-4">
          <Link
            href="/docs"
            className="rounded-md bg-foreground px-5 py-2.5 text-sm font-medium text-background hover:bg-foreground/90"
          >
            Get started
          </Link>
          <Link
            href="/dashboard"
            className="rounded-md border border-border px-5 py-2.5 text-sm font-medium text-foreground hover:bg-accent/5"
          >
            View dashboard
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
            Three steps from agent request to on-chain receipt.
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
            Rust server, Next.js dashboard, Anchor program.
          </p>
          <div className="mt-8 overflow-hidden rounded-lg border border-border bg-card">
            <pre className="overflow-x-auto px-6 py-5 font-mono text-sm leading-relaxed text-muted">
{`┌──────────────┐     ┌──────────────────┐     ┌──────────────┐
│   Agent       │────▶│   payq-server    │────▶│   Solana      │
│   (any HTTP   │     │   (Rust / Axum)  │     │   (Anchor)    │
│    client)    │◀────│                  │     │               │
└──────────────┘     │  ┌────────────┐  │     │  Vault PDA    │
                     │  │  x402      │  │     │  SpendRecord  │
┌──────────────┐     │  │  routing   │  │     └──────────────┘
│   Dashboard   │────▶│  └────────────┘  │
│   (Next.js)   │ SSE │                  │     ┌──────────────┐
│               │◀────│  ┌────────────┐  │────▶│  LLM APIs    │
└──────────────┘     │  │  Solana    │  │     │  Gemini/Groq  │
                     │  │  client    │  │     │  OpenAI       │
                     │  └────────────┘  │     └──────────────┘
                     └──────────────────┘`}
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
