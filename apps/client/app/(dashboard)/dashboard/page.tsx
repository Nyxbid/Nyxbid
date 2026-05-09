import { fetchJson } from "@/lib/api";
import type { DashboardStats, Fill, Intent } from "@/lib/data";
import { LiveDashboard } from "@/components/live-dashboard";

export const dynamic = "force-dynamic";

const ZERO: DashboardStats = {
  open_intents: 0,
  resolved_intents: 0,
  total_fills: 0,
  notional_24h: 0,
  avg_makers_per_intent: 0,
};

export default async function Dashboard() {
  const [stats, fills, intents] = await Promise.all([
    fetchJson<DashboardStats>("/api/dashboard").catch(() => ZERO),
    fetchJson<Fill[]>("/api/fills").catch(() => [] as Fill[]),
    fetchJson<Intent[]>("/api/intents").catch(() => [] as Intent[]),
  ]);

  return (
    <LiveDashboard
      initialStats={stats}
      initialFills={fills}
      initialIntents={intents}
    />
  );
}
