import { fetchJson } from "@/lib/api";
import type { Fill } from "@/lib/data";
import { LiveFillsTable } from "@/components/live-fills-table";
import { PageHeader } from "@/components/page-header";

export const dynamic = "force-dynamic";

export default async function FillsPage() {
  const fills = await fetchJson<Fill[]>("/api/fills").catch(
    () => [] as Fill[],
  );

  return (
    <>
      <PageHeader title="Fills" eyebrow="Settled" />
      <div className="mt-8">
        <LiveFillsTable initial={fills} />
      </div>
    </>
  );
}
