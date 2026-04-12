import { StatCard } from "@/components/stat-card";
import { StatusDot } from "@/components/status-dot";
import { dashboardStats, receipts } from "@/lib/data";
import { formatUsdc, timeAgo } from "@/lib/format";

export default function Dashboard() {
  return (
    <>
      <h1 className="text-xl font-semibold tracking-tight">Dashboard</h1>
      <p className="mt-1 text-sm text-muted">
        Overview of agent spend activity and policy status.
      </p>

      {/* Stats */}
      <div className="mt-6 grid gap-4 sm:grid-cols-2 lg:grid-cols-4">
        <StatCard
          label="Spent today"
          value={formatUsdc(dashboardStats.total_spent_today)}
        />
        <StatCard
          label="Active agents"
          value={String(dashboardStats.active_agents)}
        />
        <StatCard
          label="Receipts today"
          value={String(dashboardStats.receipts_today)}
        />
        <StatCard
          label="Active policies"
          value={String(dashboardStats.active_policies)}
        />
      </div>

      {/* Recent activity */}
      <section className="mt-10">
        <h2 className="text-sm font-medium uppercase tracking-wide text-muted">
          Recent activity
        </h2>

        <div className="mt-3 overflow-x-auto rounded-lg border border-border">
          <table className="w-full text-left text-sm">
            <thead>
              <tr className="border-b border-border bg-card text-xs uppercase tracking-wide text-muted">
                <th className="px-4 py-3 font-medium">Agent</th>
                <th className="px-4 py-3 font-medium">Tool</th>
                <th className="px-4 py-3 font-medium text-right">Amount</th>
                <th className="px-4 py-3 font-medium">Status</th>
                <th className="px-4 py-3 font-medium text-right">Time</th>
              </tr>
            </thead>
            <tbody>
              {receipts.map((r) => (
                <tr
                  key={r.id}
                  className="border-b border-border last:border-0 transition-colors hover:bg-accent/5"
                >
                  <td className="px-4 py-3 font-medium">{r.agent_name}</td>
                  <td className="px-4 py-3 font-mono text-xs text-muted">
                    {r.tool}
                  </td>
                  <td className="px-4 py-3 text-right tabular-nums">
                    {formatUsdc(r.amount)}
                  </td>
                  <td className="px-4 py-3">
                    <StatusDot status={r.status} />
                  </td>
                  <td className="px-4 py-3 text-right text-xs text-muted">
                    {timeAgo(r.timestamp)}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </section>
    </>
  );
}
