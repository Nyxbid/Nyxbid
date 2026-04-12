import Link from "next/link";

function Section({ id, title, children }: { id: string; title: string; children: React.ReactNode }) {
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
  { id: "prerequisites", label: "Prerequisites" },
  { id: "quickstart", label: "Quick start" },
  { id: "architecture", label: "Architecture" },
  { id: "api", label: "API reference" },
  { id: "smart-contract", label: "Smart contract" },
  { id: "config", label: "Configuration" },
  { id: "docker", label: "Docker" },
];

export default function DocsPage() {
  return (
    <div className="mx-auto max-w-5xl px-6 py-16">
      <h1 className="text-3xl font-bold tracking-tight">Documentation</h1>
      <p className="mt-2 text-sm text-muted">
        Everything you need to run Payq locally, deploy to devnet, and integrate
        agents.
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
          <Section id="prerequisites" title="Prerequisites">
            <p>You need the following installed before starting.</p>
            <Code>
{`# Rust toolchain
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Bun (JS runtime)
curl -fsSL https://bun.sh/install | bash

# Solana CLI 3.x
sh -c "$(curl -sSfL https://release.anza.xyz/stable/install)"

# Anchor CLI 1.0
cargo install --git https://github.com/coral-xyz/anchor anchor-cli

# Just (task runner)
cargo install just`}
            </Code>
            <p>
              After installing Solana CLI, create a devnet keypair if you
              don&apos;t have one:
            </p>
            <Code>
{`solana-keygen new --outfile ~/.config/solana/id.json
solana config set --url devnet
solana airdrop 2`}
            </Code>
          </Section>

          <Section id="quickstart" title="Quick start">
            <p>
              Clone the repo and start both the server and client with a single
              command.
            </p>
            <Code>
{`git clone https://github.com/neurocracy/payq.git
cd payq

# Copy env template and fill in your API keys
cp .env.example .env

# Start server (port 3001) + client (port 3000)
just dev`}
            </Code>
            <p>
              Open{" "}
              <InlineCode>http://localhost:3000</InlineCode> for the landing page
              and <InlineCode>http://localhost:3000/dashboard</InlineCode> for the
              dashboard. The server API is at{" "}
              <InlineCode>http://localhost:3001</InlineCode>.
            </p>
            <p>
              To send a test proposal (make sure you have a{" "}
              <InlineCode>GEMINI_API_KEY</InlineCode> or{" "}
              <InlineCode>GROQ_API_KEY</InlineCode> in your{" "}
              <InlineCode>.env</InlineCode>):
            </p>
            <Code>
{`curl -X POST http://localhost:3001/api/proposals \\
  -H "Content-Type: application/json" \\
  -d '{
    "agent_id": "agent-alpha",
    "tool": "gemini/gemini-2.0-flash",
    "prompt": "What is Solana?"
  }'`}
            </Code>
          </Section>

          <Section id="architecture" title="Architecture">
            <p>
              Payq has three layers: a Rust API server, a Next.js dashboard, and
              an Anchor program on Solana.
            </p>
            <Code>
{`apps/
  server/        Rust / Axum API server (port 3001)
    src/
      main.rs      Entry point, state init
      routes.rs    REST + SSE endpoints
      x402.rs      Tool routing (Gemini, Groq, OpenAI)
      solana.rs    On-chain transaction builder
      mock.rs      Seed data for dev

  client/        Next.js dashboard (port 3000)
    app/
      (marketing)/ Landing page + docs
      (dashboard)/ Sidebar layout + app pages

chain/
  programs/payq/ Anchor program
    src/
      lib.rs             Program entry
      state.rs           Vault + SpendRecord accounts
      errors.rs          Custom errors
      events.rs          Anchor events
      instructions/      One file per instruction

crates/
  payq-types/    Shared Rust types (Agent, Policy, Receipt)`}
            </Code>
            <p>
              The server is the orchestrator. Agents send proposals via HTTP. The
              server checks policy, calls the tool, creates a receipt, optionally
              writes it on-chain, and broadcasts it to the dashboard via SSE.
            </p>
          </Section>

          <Section id="api" title="API reference">
            <p>All endpoints are served by the Rust server on port 3001.</p>

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
                    ["GET", "/api/dashboard", "Stats + recent receipts"],
                    ["GET", "/api/agents", "List all agents"],
                    ["GET", "/api/receipts", "List all spend receipts"],
                    ["GET", "/api/policies", "List all policies"],
                    [
                      "POST",
                      "/api/proposals",
                      "Submit a tool request from an agent",
                    ],
                    ["GET", "/api/events", "SSE stream of new receipts"],
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

            <p className="mt-2 font-medium text-foreground">
              POST /api/proposals
            </p>
            <Code>
{`// Request body
{
  "agent_id": "agent-alpha",
  "tool": "gemini/gemini-2.0-flash",
  "prompt": "Explain Solana accounts"
}

// Response
{
  "receipt": {
    "id": "rcpt-a1b2c3d4",
    "agent_id": "agent-alpha",
    "agent_name": "Alpha",
    "tool": "gemini/gemini-2.0-flash",
    "amount": 150,
    "tx_hash": "5xK9...",
    "status": "confirmed",
    "timestamp": "2026-04-12T10:00:00Z",
    "proposal_hash": "abc123..."
  },
  "tool_response": "Solana accounts are..."
}`}
            </Code>
          </Section>

          <Section id="smart-contract" title="Smart contract">
            <p>
              The Anchor program manages Vault accounts and SpendRecord PDAs.
            </p>

            <p className="font-medium text-foreground">Instructions</p>
            <div className="overflow-x-auto rounded-lg border border-border">
              <table className="w-full text-left text-sm">
                <thead>
                  <tr className="border-b border-border bg-card">
                    <th className="px-4 py-3 font-medium text-foreground">
                      Instruction
                    </th>
                    <th className="px-4 py-3 font-medium text-foreground">
                      Signer
                    </th>
                    <th className="px-4 py-3 font-medium text-foreground">
                      What it does
                    </th>
                  </tr>
                </thead>
                <tbody>
                  {[
                    [
                      "initialize_vault",
                      "authority",
                      "Creates a Vault PDA with spend limits and a delegate key",
                    ],
                    [
                      "update_vault",
                      "authority",
                      "Updates limits, delegate, or pause status",
                    ],
                    [
                      "close_vault",
                      "authority",
                      "Closes the Vault, reclaims rent",
                    ],
                    [
                      "record_spend",
                      "delegate",
                      "Creates a SpendRecord PDA, updates daily counters",
                    ],
                  ].map(([instr, signer, desc]) => (
                    <tr
                      key={instr}
                      className="border-b border-border last:border-0"
                    >
                      <td className="px-4 py-3 font-mono text-xs">{instr}</td>
                      <td className="px-4 py-3 font-mono text-xs text-accent">
                        {signer}
                      </td>
                      <td className="px-4 py-3">{desc}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>

            <p className="mt-4">
              Build and deploy to devnet:
            </p>
            <Code>
{`cd chain
anchor build
anchor program deploy --provider.cluster devnet`}
            </Code>
          </Section>

          <Section id="config" title="Configuration">
            <p>
              Copy <InlineCode>.env.example</InlineCode> to{" "}
              <InlineCode>.env</InlineCode> and fill in values.
            </p>
            <div className="overflow-x-auto rounded-lg border border-border">
              <table className="w-full text-left text-sm">
                <thead>
                  <tr className="border-b border-border bg-card">
                    <th className="px-4 py-3 font-medium text-foreground">
                      Variable
                    </th>
                    <th className="px-4 py-3 font-medium text-foreground">
                      Required
                    </th>
                    <th className="px-4 py-3 font-medium text-foreground">
                      Description
                    </th>
                  </tr>
                </thead>
                <tbody>
                  {[
                    [
                      "SOLANA_RPC_URL",
                      "No",
                      "Defaults to devnet",
                    ],
                    [
                      "SOLANA_KEYPAIR_PATH",
                      "No",
                      "Path to delegate keypair for on-chain recording",
                    ],
                    [
                      "PAYQ_PROGRAM_ID",
                      "No",
                      "Deployed program address",
                    ],
                    [
                      "PAYQ_VAULT_PUBKEY",
                      "No",
                      "Vault PDA to record spends against",
                    ],
                    [
                      "GEMINI_API_KEY",
                      "Yes*",
                      "For gemini/* tools",
                    ],
                    [
                      "GROQ_API_KEY",
                      "Yes*",
                      "For groq/* tools",
                    ],
                    [
                      "OPENAI_API_KEY",
                      "Yes*",
                      "For openai/* tools",
                    ],
                  ].map(([name, req, desc]) => (
                    <tr
                      key={name}
                      className="border-b border-border last:border-0"
                    >
                      <td className="px-4 py-3 font-mono text-xs">{name}</td>
                      <td className="px-4 py-3 text-xs">{req}</td>
                      <td className="px-4 py-3">{desc}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
            <p className="text-xs">
              *At least one LLM key is needed to use LLM tools. Simulated tools
              (coingecko/*, pyth/*, helius/*) work without keys.
            </p>
          </Section>

          <Section id="docker" title="Docker">
            <p>
              Build and run both services with Docker Compose.
            </p>
            <Code>
{`# Build images
docker compose build

# Start services (server:3001, client:3000)
docker compose up -d

# View logs
docker compose logs -f

# Stop
docker compose down`}
            </Code>
            <p>
              Or use the Justfile shortcuts:{" "}
              <InlineCode>just docker-build</InlineCode>,{" "}
              <InlineCode>just docker-up</InlineCode>,{" "}
              <InlineCode>just docker-down</InlineCode>.
            </p>
          </Section>

          <div className="rounded-lg border border-border bg-card px-6 py-5">
            <p className="text-sm text-muted">
              Need help?{" "}
              <Link
                href="https://github.com/neurocracy/payq/issues"
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
