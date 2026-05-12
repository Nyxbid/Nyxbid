"use client";

import { useEffect, useState } from "react";

import { timeUntil } from "@/lib/format";

/**
 * Live countdown to a deadline. Updates every second when the
 * deadline is within 5 minutes, every 30s otherwise — preserves
 * accuracy where it matters and keeps the page quiet otherwise.
 *
 * SSR/CSR: `Date.now()` differs between the server render and the
 * first client paint by hundreds of ms, which used to crash React
 * with a hydration mismatch ("9s" vs "11s"). We render a stable
 * placeholder until the client effect runs, then swap in the live
 * tick. No content visible to the user changes, just the SSR HTML
 * is now deterministic.
 *
 * Tick cadence: the previous version computed the interval once at
 * mount, so a 10-min countdown ticked every 30s forever — even
 * after dropping under 5 min. We now reschedule via setTimeout so
 * the cadence adapts as the deadline approaches.
 */
export function Countdown({
  iso,
  prefix = "",
  className = "",
}: {
  iso: string;
  prefix?: string;
  className?: string;
}) {
  const [now, setNow] = useState<number | null>(null);

  useEffect(() => {
    let cancelled = false;
    let timer: ReturnType<typeof setTimeout> | null = null;

    const tick = () => {
      if (cancelled) return;
      const n = Date.now();
      setNow(n);
      const remaining = new Date(iso).getTime() - n;
      const delay = Math.abs(remaining) < 5 * 60_000 ? 1000 : 30_000;
      timer = setTimeout(tick, delay);
    };

    tick();
    return () => {
      cancelled = true;
      if (timer) clearTimeout(timer);
    };
  }, [iso]);

  if (now === null) {
    // Same text on server and on the first client paint → no mismatch.
    return (
      <span
        className={`tabular-nums ${className}`}
        suppressHydrationWarning
      >
        {prefix}—
      </span>
    );
  }

  const expired = new Date(iso).getTime() <= now;
  return (
    <span
      className={`tabular-nums ${expired ? "text-muted" : ""} ${className}`}
      suppressHydrationWarning
    >
      {prefix}
      {timeUntil(iso, now)}
    </span>
  );
}
