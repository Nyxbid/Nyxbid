import Link from "next/link";

import { fetchJson } from "@/lib/api";
import type { Intent } from "@/lib/data";
import { LiveIntentsTable } from "@/components/live-intents-table";
import { PageHeader } from "@/components/page-header";

export const dynamic = "force-dynamic";

export default async function IntentsPage() {
  const intents = await fetchJson<Intent[]>("/api/intents").catch(
    () => [] as Intent[],
  );

  return (
    <>
      <PageHeader
        title="Intents"
        eyebrow="Order book"
        actions={
          <Link
            href="/trade"
            className="inline-flex h-9 items-center rounded-[var(--r-sm)] bg-[var(--accent)] px-3.5 text-[12px] font-medium tracking-tight text-[var(--accent-fg)] transition-colors hover:bg-[var(--accent-soft)]"
          >
            New intent
          </Link>
        }
      />
      <div className="mt-8">
        <LiveIntentsTable initial={intents} />
      </div>
    </>
  );
}
