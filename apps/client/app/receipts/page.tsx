import { StatusDot } from "@/components/status-dot";
import { receipts } from "@/lib/data";
import { formatUsdc, timeAgo } from "@/lib/format";

export default function ReceiptsPage() {
  return (
    <>
      <h1 className="text-xl font-semibold tracking-tight">Receipts</h1>
      <p className="mt-1 text-sm text-muted">
        On-chain and pending spend receipts from agent activity.
      </p>

      <div className="mt-6 overflow-x-auto rounded-lg border border-border">
        <table className="w-full text-left text-sm">
          <thead>
            <tr className="border-b border-border bg-card text-xs uppercase tracking-wide text-muted">
              <th className="px-4 py-3 font-medium">Agent</th>
              <th className="px-4 py-3 font-medium">Tool</th>
              <th className="px-4 py-3 font-medium text-right">Amount</th>
              <th className="px-4 py-3 font-medium">Tx Hash</th>
              <th className="px-4 py-3 font-medium">Status</th>
              <th className="px-4 py-3 font-medium text-right">Time</th>
            </tr>
          </thead>
          <tbody>
            {receipts.map((r) => (
              <tr
                key={r.id}
                className="border-b border-border last:border-0 transition-colors hover:bg-accent/5"
              >
                <td className="px-4 py-3 font-medium">{r.agent_name}</td>
                <td className="px-4 py-3 font-mono text-xs text-muted">
                  {r.tool}
                </td>
                <td className="px-4 py-3 text-right tabular-nums">
                  {formatUsdc(r.amount)}
                </td>
                <td className="px-4 py-3 font-mono text-xs text-muted">
                  {r.tx_hash ?? "—"}
                </td>
                <td className="px-4 py-3">
                  <StatusDot status={r.status} />
                </td>
                <td className="px-4 py-3 text-right text-xs text-muted">
                  {timeAgo(r.timestamp)}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </>
  );
}
