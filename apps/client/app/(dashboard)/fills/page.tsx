import { fetchJson } from "@/lib/api";
import type { Fill } from "@/lib/data";

export const dynamic = "force-dynamic";

export default async function FillsPage() {
  const fills = await fetchJson<Fill[]>("/api/fills").catch(() => []);

  return (
    <>
      <h1 className="text-xl font-semibold tracking-tight">Fills</h1>
      <p className="mt-1 text-sm text-muted">
        Settled trades with on-chain receipts.
      </p>

      <div className="mt-6 overflow-x-auto rounded-lg border border-border">
        <table className="w-full text-left text-sm">
          <thead>
            <tr className="border-b border-border bg-card text-xs uppercase tracking-wide text-muted">
              <th className="px-4 py-3 font-medium">Fill</th>
              <th className="px-4 py-3 font-medium">Intent</th>
              <th className="px-4 py-3 font-medium text-right">Size</th>
              <th className="px-4 py-3 font-medium text-right">Price</th>
              <th className="px-4 py-3 font-medium">Tx</th>
              <th className="px-4 py-3 font-medium text-right">Settled</th>
            </tr>
          </thead>
          <tbody>
            {fills.length === 0 ? (
              <tr>
                <td
                  colSpan={6}
                  className="px-4 py-8 text-center text-sm text-muted"
                >
                  No fills yet.
                </td>
              </tr>
            ) : (
              fills.map((f) => (
                <tr
                  key={f.id}
                  className="border-b border-border last:border-0"
                >
                  <td className="px-4 py-3 font-mono text-xs">{f.id}</td>
                  <td className="px-4 py-3 font-mono text-xs text-muted">
                    {f.intent_id}
                  </td>
                  <td className="px-4 py-3 text-right tabular-nums">{f.size}</td>
                  <td className="px-4 py-3 text-right tabular-nums">{f.price}</td>
                  <td className="px-4 py-3 font-mono text-xs text-muted">
                    {f.tx_signature ? (
                      <a
                        href={`https://explorer.solana.com/tx/${f.tx_signature}?cluster=devnet`}
                        target="_blank"
                        rel="noopener noreferrer"
                        className="text-accent underline-offset-2 hover:underline"
                      >
                        {f.tx_signature.slice(0, 8)}...
                      </a>
                    ) : (
                      "—"
                    )}
                  </td>
                  <td className="px-4 py-3 text-right text-xs text-muted">
                    {new Date(f.settled_at).toLocaleString()}
                  </td>
                </tr>
              ))
            )}
          </tbody>
        </table>
      </div>
    </>
  );
}
