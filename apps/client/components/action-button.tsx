"use client";

import { type ButtonHTMLAttributes, type ReactNode } from "react";

import type { TxPhase } from "@/hooks/use-nyxbid-tx";

interface Props extends ButtonHTMLAttributes<HTMLButtonElement> {
  phase?: TxPhase;
  variant?: "primary" | "secondary" | "danger";
  loading?: boolean;
  children: ReactNode;
}

const phaseLabel: Partial<Record<TxPhase, string>> = {
  preparing: "Preparing…",
  signing: "Sign in wallet…",
  sending: "Sending…",
  confirming: "Confirming…",
  confirmed: "Confirmed",
};

const variantClass: Record<NonNullable<Props["variant"]>, string> = {
  primary:
    "bg-[var(--accent)] text-[var(--accent-fg)] hover:bg-[var(--accent-soft)] disabled:bg-[var(--accent)]/40 disabled:text-[var(--accent-fg)]/60",
  secondary:
    "border border-[var(--hairline-strong)] text-foreground bg-transparent hover:bg-[var(--surface-2)] disabled:text-muted",
  danger:
    "border border-[var(--sell)]/40 text-[var(--sell)] bg-transparent hover:bg-[var(--sell)]/10 disabled:opacity-50",
};

/**
 * Tx-aware button. Phase-driven label, no hover scale/translate
 * (those are out of style for trading UIs). Single 120ms color
 * transition only.
 */
export function ActionButton({
  phase,
  variant = "primary",
  loading,
  disabled,
  children,
  className = "",
  ...rest
}: Props) {
  const inFlight =
    phase &&
    phase !== "idle" &&
    phase !== "confirmed" &&
    phase !== "error";
  const label = inFlight ? (phaseLabel[phase] ?? children) : children;

  return (
    <button
      {...rest}
      disabled={disabled || loading || !!inFlight}
      className={`inline-flex h-10 min-w-[120px] items-center justify-center gap-2 rounded-[var(--r-sm)] px-4 text-[13px] font-medium tracking-tight transition-colors duration-[120ms] disabled:cursor-not-allowed ${variantClass[variant]} ${className}`}
    >
      {(loading || inFlight) && <Spinner />}
      {label}
    </button>
  );
}

function Spinner() {
  return (
    <svg
      width="13"
      height="13"
      viewBox="0 0 13 13"
      fill="none"
      className="animate-spin opacity-90"
      aria-hidden="true"
    >
      <circle
        cx="6.5"
        cy="6.5"
        r="5"
        stroke="currentColor"
        strokeOpacity="0.25"
        strokeWidth="1.5"
      />
      <path
        d="M11.5 6.5a5 5 0 00-5-5"
        stroke="currentColor"
        strokeWidth="1.5"
        strokeLinecap="round"
      />
    </svg>
  );
}
