"use client";

import { useEffect, useState } from "react";

import { timeUntil } from "@/lib/format";

/**
 * Live countdown to a deadline. Updates every second when the
 * deadline is within 5 minutes, every 30s otherwise — preserves
 * accuracy where it matters and keeps the page quiet otherwise.
 *
 * Doherty: deadlines that the user is actively waiting on tick at
 * 1Hz so urgency is visible without being noisy.
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
  const [now, setNow] = useState(() => Date.now());

  useEffect(() => {
    const target = new Date(iso).getTime();
    const update = () => setNow(Date.now());
    const remaining = target - Date.now();
    const interval = remaining < 5 * 60_000 ? 1000 : 30_000;
    const id = setInterval(update, interval);
    return () => clearInterval(id);
  }, [iso]);

  const expired = new Date(iso).getTime() <= now;

  return (
    <span
      className={`tabular-nums ${expired ? "text-muted" : ""} ${className}`}
    >
      {prefix}
      {timeUntil(iso, now)}
    </span>
  );
}
