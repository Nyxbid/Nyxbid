import type { IntentStatus } from "@/lib/data";

const styles: Record<IntentStatus, string> = {
  open: "bg-[var(--accent)]/10 text-[var(--accent)] border-[var(--accent)]/25",
  resolved: "bg-[var(--warn)]/10 text-[var(--warn)] border-[var(--warn)]/25",
  settled: "bg-[var(--buy)]/10 text-[var(--buy)] border-[var(--buy)]/25",
  cancelled: "bg-[var(--surface-2)] text-muted border-[var(--hairline-strong)]",
  expired: "bg-[var(--sell)]/10 text-[var(--sell)] border-[var(--sell)]/25",
};

export function StatusPill({ status }: { status: IntentStatus }) {
  return (
    <span
      className={`inline-flex h-5 items-center rounded-full border px-2 font-mono text-[10px] uppercase tracking-[0.14em] ${styles[status]}`}
    >
      {status}
    </span>
  );
}
