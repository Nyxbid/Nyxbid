import { fetchJson } from "@/lib/api";
import type { Market } from "@/lib/data";
import { TradeForm } from "@/components/trade-form";
import { PageHeader } from "@/components/page-header";

export const dynamic = "force-dynamic";

export default async function TradePage() {
  const markets = await fetchJson<Market[]>("/api/markets").catch(
    () => [] as Market[],
  );

  return (
    <>
      <PageHeader title="Trade" eyebrow="Sealed-bid RFQ" />
      <div className="mt-8">
        <TradeForm markets={markets} />
      </div>
    </>
  );
}
