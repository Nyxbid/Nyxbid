/**
 * StatCard / StatCell — unit primitives for the dashboard's number
 * grid. The label sits on the same monospace cap-tracking as every
 * other "section tag" in the app for visual consistency.
 *
 * `StatCell` is the in-card-grid variant (no border, divides via
 * its parent's `divide-x`). `StatCard` is the standalone variant.
 */

interface Props {
  label: string;
  value: string;
  sub?: string;
}

export function StatCard({ label, value, sub }: Props) {
  return (
    <div className="card px-5 py-4">
      <p className="font-mono text-[10px] uppercase tracking-[0.18em] text-muted">
        {label}
      </p>
      <p className="mt-1.5 font-mono text-[24px] font-medium tabular-nums tracking-tight text-foreground">
        {value}
      </p>
      {sub && (
        <p className="mt-0.5 font-mono text-[11px] tabular-nums text-faint">
          {sub}
        </p>
      )}
    </div>
  );
}

export function StatCell({ label, value, sub }: Props) {
  return (
    <div className="px-5 py-5">
      <p className="font-mono text-[10px] uppercase tracking-[0.18em] text-muted">
        {label}
      </p>
      <p className="mt-1.5 font-mono text-[22px] font-medium tabular-nums tracking-tight text-foreground">
        {value}
      </p>
      {sub && (
        <p className="mt-0.5 font-mono text-[11px] tabular-nums text-faint">
          {sub}
        </p>
      )}
    </div>
  );
}
