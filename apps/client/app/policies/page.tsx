import { policies } from "@/lib/data";
import { formatUsdc } from "@/lib/format";

export default function PoliciesPage() {
  return (
    <>
      <h1 className="text-xl font-semibold tracking-tight">Policies</h1>
      <p className="mt-1 text-sm text-muted">
        Spend policies governing agent budgets and tool access.
      </p>

      <div className="mt-6 grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
        {policies.map((p) => (
          <div
            key={p.id}
            className={`rounded-lg border bg-card px-5 py-4 ${
              p.active ? "border-border" : "border-border opacity-50"
            }`}
          >
            <div className="flex items-center justify-between">
              <p className="font-medium">{p.name}</p>
              <span
                className={`rounded-full px-2 py-0.5 text-[10px] font-medium uppercase tracking-wide ${
                  p.active
                    ? "bg-emerald-500/10 text-emerald-600 dark:text-emerald-400"
                    : "bg-zinc-500/10 text-zinc-500"
                }`}
              >
                {p.active ? "active" : "inactive"}
              </span>
            </div>

            <dl className="mt-4 space-y-2 text-sm">
              <div className="flex justify-between">
                <dt className="text-muted">Daily limit</dt>
                <dd className="tabular-nums">{formatUsdc(p.daily_limit)}</dd>
              </div>
              <div className="flex justify-between">
                <dt className="text-muted">Per-tx limit</dt>
                <dd className="tabular-nums">{formatUsdc(p.per_tx_limit)}</dd>
              </div>
            </dl>

            <div className="mt-4 flex flex-wrap gap-1.5">
              {p.allowed_tools.map((tool) => (
                <span
                  key={tool}
                  className="rounded bg-accent/10 px-2 py-0.5 font-mono text-[11px] text-accent"
                >
                  {tool}
                </span>
              ))}
            </div>
          </div>
        ))}
      </div>
    </>
  );
}
