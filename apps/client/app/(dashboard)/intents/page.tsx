import { fetchJson } from "@/lib/api";
import type { Intent } from "@/lib/data";

export const dynamic = "force-dynamic";

export default async function IntentsPage() {
  const intents = await fetchJson<Intent[]>("/api/intents").catch(() => []);

  return (
    <>
      <h1 className="text-xl font-semibold tracking-tight">Intents</h1>
      <p className="mt-1 text-sm text-muted">
        Every intent posted to the venue, open or settled.
      </p>

      <div className="mt-6 overflow-x-auto rounded-lg border border-border">
        <table className="w-full text-left text-sm">
          <thead>
            <tr className="border-b border-border bg-card text-xs uppercase tracking-wide text-muted">
              <th className="px-4 py-3 font-medium">Intent</th>
              <th className="px-4 py-3 font-medium">Side</th>
              <th className="px-4 py-3 font-medium text-right">Size</th>
              <th className="px-4 py-3 font-medium text-right">Limit</th>
              <th className="px-4 py-3 font-medium">Status</th>
              <th className="px-4 py-3 font-medium text-right">Reveal</th>
            </tr>
          </thead>
          <tbody>
            {intents.length === 0 ? (
              <tr>
                <td
                  colSpan={6}
                  className="px-4 py-8 text-center text-sm text-muted"
                >
                  No intents yet.
                </td>
              </tr>
            ) : (
              intents.map((i) => (
                <tr
                  key={i.id}
                  className="border-b border-border last:border-0"
                >
                  <td className="px-4 py-3 font-mono text-xs">{i.id}</td>
                  <td className="px-4 py-3 text-xs uppercase">{i.side}</td>
                  <td className="px-4 py-3 text-right tabular-nums">{i.size}</td>
                  <td className="px-4 py-3 text-right tabular-nums">
                    {i.limit_price}
                  </td>
                  <td className="px-4 py-3 text-xs">{i.status}</td>
                  <td className="px-4 py-3 text-right text-xs text-muted">
                    {new Date(i.reveal_deadline).toLocaleTimeString()}
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
