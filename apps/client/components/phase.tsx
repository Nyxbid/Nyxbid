import type { IntentStatus } from "@/lib/data";

type Phase = "open" | "resolved" | "settled" | "cancelled" | "expired";

interface Props {
  status: IntentStatus;
  marks?: Partial<Record<Phase, string>>;
  className?: string;
}

const order: Phase[] = ["open", "resolved", "settled"];

const labels: Record<Phase, string> = {
  open: "submit",
  resolved: "reveal",
  settled: "settle",
  cancelled: "cancelled",
  expired: "expired",
};

/**
 * Lifecycle strip. One row, three columns: each column is the dot +
 * label + optional mark. Status drives which columns are reached /
 * active. Cancelled/expired collapse to a single terminal note.
 */
export function PhaseStrip({ status, marks, className = "" }: Props) {
  const isTerminalAlt = status === "cancelled" || status === "expired";
  const reachedIdx = order.indexOf(status as Phase);

  return (
    <div className={`card ${className}`}>
      <div className="grid grid-cols-3 divide-x divide-[var(--hairline)]">
        {order.map((phase, i) => {
          const reached = !isTerminalAlt && reachedIdx >= 0 && i <= reachedIdx;
          const active = !isTerminalAlt && reachedIdx >= 0 && i === reachedIdx;
          return (
            <div key={phase} className="flex flex-col gap-1.5 px-5 py-4">
              <div className="flex items-center gap-2">
                <span
                  className={`h-1.5 w-1.5 rounded-full ${
                    active
                      ? "bg-[var(--accent)] animate-[slow-pulse_2.4s_ease-in-out_infinite]"
                      : reached
                        ? "bg-[var(--buy)]"
                        : "bg-[var(--hairline-strong)]"
                  }`}
                />
                <span
                  className={`font-mono text-[10px] uppercase tracking-[0.18em] ${
                    reached || active ? "text-foreground" : "text-faint"
                  }`}
                >
                  {labels[phase]}
                </span>
              </div>
              {marks?.[phase] && (
                <span className="pl-3.5 font-mono text-[10px] tabular-nums text-muted">
                  {marks[phase]}
                </span>
              )}
            </div>
          );
        })}
      </div>
      {isTerminalAlt && (
        <p
          className={`border-t border-[var(--hairline)] px-5 py-3 font-mono text-[10px] uppercase tracking-[0.18em] ${
            status === "expired" ? "text-[var(--warn)]" : "text-[var(--sell)]"
          }`}
        >
          {status === "expired" ? "expired · refunded" : "cancelled by taker"}
        </p>
      )}
    </div>
  );
}
