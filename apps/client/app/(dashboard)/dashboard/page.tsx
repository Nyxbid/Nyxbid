import { StatCard } from "@/components/stat-card";
import { fetchJson } from "@/lib/api";
import type { DashboardStats } from "@/lib/data";
import { formatUsdc } from "@/lib/format";

export const dynamic = "force-dynamic";

export default async function Dashboard() {
  const stats = await fetchJson<DashboardStats>("/api/dashboard").catch(
    () => ({
      open_intents: 0,
      resolved_intents: 0,
      total_fills: 0,
      notional_24h: 0,
      avg_makers_per_intent: 0,
    }),
  );

  return (
    <>
      <h1 className="text-xl font-semibold tracking-tight">Dashboard</h1>
      <p className="mt-1 text-sm text-muted">
        Live state of the sealed-bid RFQ venue.
      </p>

      <div className="mt-6 grid gap-4 sm:grid-cols-2 lg:grid-cols-4">
        <StatCard label="Open intents" value={String(stats.open_intents)} />
        <StatCard
          label="Resolved intents"
          value={String(stats.resolved_intents)}
        />
        <StatCard label="Total fills" value={String(stats.total_fills)} />
        <StatCard
          label="24h notional"
          value={formatUsdc(stats.notional_24h)}
        />
      </div>

      <section className="mt-10">
        <h2 className="text-sm font-medium uppercase tracking-wide text-muted">
          Recent activity
        </h2>
        <div className="mt-3 rounded-lg border border-border bg-card px-5 py-8 text-center text-sm text-muted">
          No fills yet. Submit an intent via the API or an A2A agent to get started.
        </div>
      </section>
    </>
  );
}
