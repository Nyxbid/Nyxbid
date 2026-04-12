"use client";

import { useLiveReceipts } from "@/hooks/use-sse";
import { StatusDot } from "@/components/status-dot";
import type { SpendReceipt } from "@/lib/data";
import { formatUsdc, timeAgo } from "@/lib/format";

export function LiveReceiptTable({
  initial,
  showTxHash = false,
}: {
  initial: SpendReceipt[];
  showTxHash?: boolean;
}) {
  const receipts = useLiveReceipts(initial);

  return (
    <div className="overflow-x-auto rounded-lg border border-border">
      <table className="w-full text-left text-sm">
        <thead>
          <tr className="border-b border-border bg-card text-xs uppercase tracking-wide text-muted">
            <th className="px-4 py-3 font-medium">Agent</th>
            <th className="px-4 py-3 font-medium">Tool</th>
            <th className="px-4 py-3 font-medium text-right">Amount</th>
            {showTxHash && (
              <th className="px-4 py-3 font-medium">Tx Hash</th>
            )}
            <th className="px-4 py-3 font-medium">Status</th>
            <th className="px-4 py-3 font-medium text-right">Time</th>
          </tr>
        </thead>
        <tbody>
          {receipts.map((r) => (
            <tr
              key={r.id}
              className="border-b border-border last:border-0 transition-colors hover:bg-accent/5 animate-in fade-in duration-300"
            >
              <td className="px-4 py-3 font-medium">{r.agent_name}</td>
              <td className="px-4 py-3 font-mono text-xs text-muted">
                {r.tool}
              </td>
              <td className="px-4 py-3 text-right tabular-nums">
                {formatUsdc(r.amount)}
              </td>
              {showTxHash && (
                <td className="px-4 py-3 font-mono text-xs text-muted">
                  {r.tx_hash ? (
                    <a
                      href={`https://explorer.solana.com/tx/${r.tx_hash}?cluster=devnet`}
                      target="_blank"
                      rel="noopener noreferrer"
                      className="text-accent underline-offset-2 hover:underline"
                    >
                      {r.tx_hash.slice(0, 8)}...
                    </a>
                  ) : (
                    "—"
                  )}
                </td>
              )}
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
  );
}
