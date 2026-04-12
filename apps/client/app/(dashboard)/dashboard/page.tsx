import { StatCard } from "@/components/stat-card";
import { LiveReceiptTable } from "@/components/live-receipts";
import { fetchJson } from "@/lib/api";
import type { DashboardResponse } from "@/lib/data";
import { formatUsdc } from "@/lib/format";

export const dynamic = "force-dynamic";

export default async function Dashboard() {
  const { stats, recent_receipts } = await fetchJson<DashboardResponse>(
    "/api/dashboard",
  );

  return (
    <>
      <h1 className="text-xl font-semibold tracking-tight">Dashboard</h1>
      <p className="mt-1 text-sm text-muted">
        Overview of agent spend activity and policy status.
      </p>

      <div className="mt-6 grid gap-4 sm:grid-cols-2 lg:grid-cols-4">
        <StatCard
          label="Spent today"
          value={formatUsdc(stats.total_spent_today)}
        />
        <StatCard label="Active agents" value={String(stats.active_agents)} />
        <StatCard
          label="Receipts today"
          value={String(stats.receipts_today)}
        />
        <StatCard
          label="Active policies"
          value={String(stats.active_policies)}
        />
      </div>

      <section className="mt-10">
        <h2 className="text-sm font-medium uppercase tracking-wide text-muted">
          Recent activity
        </h2>
        <div className="mt-3">
          <LiveReceiptTable initial={recent_receipts} />
        </div>
      </section>
    </>
  );
}
