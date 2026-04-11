export default function Home() {
  return (
    <div className="flex min-h-full flex-col items-center justify-center bg-zinc-50 px-6 py-24 font-sans dark:bg-black">
      <main className="flex max-w-lg flex-col gap-6 text-center sm:text-left">
        <p className="text-sm font-medium uppercase tracking-widest text-zinc-500">
          Payq
        </p>
        <h1 className="text-3xl font-semibold leading-tight tracking-tight text-zinc-900 dark:text-zinc-50">
          Agentic payments, settled on Solana
        </h1>
        <p className="text-lg leading-relaxed text-zinc-600 dark:text-zinc-400">
          Orchestration API and on-chain receipts—LLMs off-chain, policy and
          USDC on-chain.
        </p>
        <p className="text-sm text-zinc-500">
          Run <code className="rounded bg-zinc-200 px-1.5 py-0.5 dark:bg-zinc-800">cargo run -p payq-server</code>{" "}
          from the repo root for the Axum backend.
        </p>
      </main>
    </div>
  );
}
