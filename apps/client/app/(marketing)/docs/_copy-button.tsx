"use client";

import { useState } from "react";

import { CheckIcon, CopyIcon } from "@/components/icons";

/**
 * In-place "copy to clipboard" button for code blocks. Pure client
 * component split out so the docs primitives file can stay server.
 *
 * Behaviour: click copies, swaps icon to a check for ~1.5s, reverts.
 * If the clipboard API is missing or rejects (HTTP without TLS,
 * iframe sandbox, etc.) we silently keep the copy icon.
 */
export function CopyButton({ value }: { value: string }) {
  const [copied, setCopied] = useState(false);

  return (
    <button
      type="button"
      onClick={async () => {
        try {
          await navigator.clipboard.writeText(value);
          setCopied(true);
          setTimeout(() => setCopied(false), 1500);
        } catch {
          /* swallow — best effort */
        }
      }}
      aria-label="Copy code"
      className="flex h-7 w-7 items-center justify-center rounded border border-transparent text-muted transition-colors hover:border-[var(--hairline-strong)] hover:bg-[var(--surface-2)] hover:text-foreground"
    >
      {copied ? <CheckIcon size={13} /> : <CopyIcon size={13} />}
    </button>
  );
}
