import Link from "next/link";

function Section({
  id,
  title,
  children,
}: {
  id: string;
  title: string;
  children: React.ReactNode;
}) {
  return (
    <section id={id} className="scroll-mt-20">
      <h2 className="text-xl font-bold tracking-tight">{title}</h2>
      <div className="mt-4 space-y-4 text-sm leading-relaxed text-muted">
        {children}
      </div>
    </section>
  );
}

function Code({ children }: { children: string }) {
  return (
    <pre className="overflow-x-auto rounded-lg border border-border bg-card px-5 py-4 font-mono text-sm leading-relaxed">
      {children}
    </pre>
  );
}

function InlineCode({ children }: { children: React.ReactNode }) {
  return (
    <code className="rounded bg-card px-1.5 py-0.5 font-mono text-xs text-foreground">
      {children}
    </code>
  );
}

const tocItems = [
  { id: "overview", label: "Overview" },
  { id: "prerequisites", label: "Prerequisites" },
  { id: "quickstart", label: "Quick start" },
  { id: "flow", label: "Intent flow" },
  { id: "api", label: "API reference" },
  { id: "mcp", label: "MCP tools" },
  { id: "program", label: "Anchor program" },
  { id: "config", label: "Configuration" },
];

export default function DocsPage() {
  return (
    <div className="mx-auto max-w-5xl px-6 py-16">
      <h1 className="text-3xl font-bold tracking-tight">Documentation</h1>
      <p className="mt-2 text-sm text-muted">
        Run Nyxbid locally, deploy to devnet, and wire up agents via MCP.
      </p>

      <div className="mt-12 flex gap-16">
        <nav className="hidden w-44 shrink-0 lg:block">
          <div className="sticky top-8 space-y-2">
            <p className="text-xs font-medium uppercase tracking-wide text-muted">
              On this page
            </p>
            {tocItems.map((item) => (
              <a
                key={item.id}
                href={`#${item.id}`}
                className="block text-sm text-muted hover:text-foreground"
              >
                {item.label}
              </a>
            ))}
          </div>
        </nav>

        <div className="min-w-0 flex-1 space-y-16">
          <Section id="overview" title="Overview">
            <p>
              Nyxbid is a sealed-bid RFQ venue for OTC-size trades on Solana.
              Takers post intents, makers submit sealed price commitments, and
              the winning quote settles atomically via HTLC-style escrow on
              chain. All tools are exposed over the Model Context Protocol so
              agents can trade natively.
            </p>
          </Section>

          <Section id="prerequisites" title="Prerequisites">
            <Code>
{`curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
curl -fsSL https://bun.sh/install | bash
sh -c "$(curl -sSfL https://release.anza.xyz/stable/install)"
cargo install --git https://github.com/coral-xyz/anchor anchor-cli
cargo install just`}
            </Code>
          </Section>

          <Section id="quickstart" title="Quick start">
            <Code>
{`git clone https://github.com/Nyxbid/nyxbid.git
cd nyxbid
cp .env.example .env
just dev`}
            </Code>
            <p>
              App at <InlineCode>http://localhost:3000</InlineCode>, API at{" "}
              <InlineCode>http://localhost:8080</InlineCode>.
            </p>
          </Section>

          <Section id="flow" title="Intent flow">
            <Code>
{`1. Taker calls POST /api/intents
   { side, base_mint, quote_mint, size, limit_price }
   Server stores Intent, broadcasts IntentCreated over SSE.

2. Each maker computes commitment = H(price || size || nonce)
   and calls the Anchor program submit_quote instruction.

3. After reveal_deadline, makers reveal (price, size, nonce).
   Program verifies commitment, picks winner that clears limit.

4. Settle in a single tx:
   - taker's quote-mint -> maker
   - maker's base-mint  -> taker
   - receipt PDA is written with (price, size, timestamp).`}
            </Code>
          </Section>

          <Section id="api" title="API reference">
            <div className="overflow-x-auto rounded-lg border border-border">
              <table className="w-full text-left text-sm">
                <thead>
                  <tr className="border-b border-border bg-card">
                    <th className="px-4 py-3 font-medium text-foreground">
                      Method
                    </th>
                    <th className="px-4 py-3 font-medium text-foreground">
                      Path
                    </th>
                    <th className="px-4 py-3 font-medium text-foreground">
                      Description
                    </th>
                  </tr>
                </thead>
                <tbody>
                  {[
                    ["GET", "/health", "Server version and status"],
                    ["GET", "/api/dashboard", "Aggregate stats"],
                    ["GET", "/api/markets", "List supported markets"],
                    ["GET", "/api/intents", "List intents"],
                    ["POST", "/api/intents", "Create a new intent"],
                    ["GET", "/api/intents/:id", "Get a single intent"],
                    [
                      "GET",
                      "/api/intents/:id/quotes",
                      "Quotes attached to an intent",
                    ],
                    ["GET", "/api/fills", "List settled fills"],
                    ["GET", "/api/events", "SSE stream of lifecycle events"],
                  ].map(([method, path, desc]) => (
                    <tr
                      key={path}
                      className="border-b border-border last:border-0"
                    >
                      <td className="px-4 py-3 font-mono text-xs text-accent">
                        {method}
                      </td>
                      <td className="px-4 py-3 font-mono text-xs">{path}</td>
                      <td className="px-4 py-3">{desc}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          </Section>

          <Section id="mcp" title="MCP tools">
            <p>
              The server exposes the trading lifecycle as Model Context Protocol
              tools for AI agents.
            </p>
            <Code>
{`nyxbid.list_markets      () -> Market[]
nyxbid.create_intent     (symbol, side, size, limit) -> Intent
nyxbid.list_quotes       (intent_id) -> Quote[]
nyxbid.get_receipt       (intent_id) -> Fill?
nyxbid.cancel_intent     (intent_id) -> Intent`}
            </Code>
          </Section>

          <Section id="program" title="Anchor program">
            <Code>
{`create_intent     taker signs   -> Intent PDA
submit_quote      maker signs   -> Quote PDA (commitment)
resolve_auction   any signer    -> reveal, pick winner
settle            any signer    -> atomic swap + Receipt PDA
cancel            taker signs   -> close open intent`}
            </Code>
          </Section>

          <Section id="config" title="Configuration">
            <Code>
{`SOLANA_RPC_URL=https://api.devnet.solana.com
SOLANA_KEYPAIR_PATH=~/.config/solana/id.json
NYXBID_PROGRAM_ID=NYXBiDzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz
NYXBID_USDC_MINT=4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU
RUST_LOG=info`}
            </Code>
          </Section>

          <div className="rounded-lg border border-border bg-card px-6 py-5">
            <p className="text-sm text-muted">
              Need help?{" "}
              <Link
                href="https://github.com/Nyxbid/nyxbid/issues"
                className="text-accent hover:underline"
                target="_blank"
                rel="noopener noreferrer"
              >
                Open an issue
              </Link>{" "}
              on GitHub.
            </p>
          </div>
        </div>
      </div>
    </div>
  );
}
