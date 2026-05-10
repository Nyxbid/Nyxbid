import { fetchJson } from "@/lib/api";
import type { Intent } from "@/lib/data";
import { MakerInbox } from "@/components/maker-inbox";
import { PageHeader } from "@/components/page-header";

export const dynamic = "force-dynamic";

export default async function QuotesPage() {
  const intents = await fetchJson<Intent[]>("/api/intents").catch(
    () => [] as Intent[],
  );

  return (
    <>
      <PageHeader title="Maker" eyebrow="Sealed quotes" />
      <div className="mt-8">
        <MakerInbox initial={intents} />
      </div>
    </>
  );
}
