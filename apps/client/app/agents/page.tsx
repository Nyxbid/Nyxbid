import { StatusDot } from "@/components/status-dot";
import { agents } from "@/lib/data";
import { formatUsdc } from "@/lib/format";

export default function AgentsPage() {
  return (
    <>
      <h1 className="text-xl font-semibold tracking-tight">Agents</h1>
      <p className="mt-1 text-sm text-muted">
        Configured agent nodes and their current budget usage.
      </p>

      <div className="mt-6 grid gap-4 sm:grid-cols-2">
        {agents.map((a) => {
          const pct =
            a.daily_budget > 0
              ? Math.round((a.spent_today / a.daily_budget) * 100)
              : 0;

          return (
            <div
              key={a.id}
              className="rounded-lg border border-border bg-card px-5 py-4"
            >
              <div className="flex items-center justify-between">
                <div>
                  <p className="font-medium">{a.name}</p>
                  <p className="mt-0.5 text-xs capitalize text-muted">
                    {a.role}
                  </p>
                </div>
                <StatusDot status={a.status} />
              </div>

              {/* Budget bar */}
              <div className="mt-4">
                <div className="flex items-baseline justify-between text-xs text-muted">
                  <span>
                    {formatUsdc(a.spent_today)} / {formatUsdc(a.daily_budget)}
                  </span>
                  <span>{pct}%</span>
                </div>
                <div className="mt-1.5 h-1.5 w-full rounded-full bg-border">
                  <div
                    className="h-1.5 rounded-full bg-accent transition-all"
                    style={{ width: `${Math.min(pct, 100)}%` }}
                  />
                </div>
              </div>
            </div>
          );
        })}
      </div>
    </>
  );
}
