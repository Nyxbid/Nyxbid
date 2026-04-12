import { LiveReceiptTable } from "@/components/live-receipts";
import { fetchJson } from "@/lib/api";
import type { SpendReceipt } from "@/lib/data";

export const dynamic = "force-dynamic";

export default async function ReceiptsPage() {
  const receipts = await fetchJson<SpendReceipt[]>("/api/receipts");

  return (
    <>
      <h1 className="text-xl font-semibold tracking-tight">Receipts</h1>
      <p className="mt-1 text-sm text-muted">
        On-chain and pending spend receipts from agent activity.
      </p>

      <div className="mt-6">
        <LiveReceiptTable initial={receipts} showTxHash />
      </div>
    </>
  );
}
