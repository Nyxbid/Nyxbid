"use client";

import { useState } from "react";
import Link from "next/link";
import { usePathname } from "next/navigation";
import { WalletButton } from "@/components/wallet-button";

const nav = [
  { href: "/dashboard", label: "Dashboard" },
  { href: "/agents", label: "Agents" },
  { href: "/receipts", label: "Receipts" },
  { href: "/policies", label: "Policies" },
];

export function MobileNav() {
  const [open, setOpen] = useState(false);
  const pathname = usePathname();

  return (
    <div className="md:hidden">
      <div className="flex h-14 items-center justify-between border-b border-border bg-card px-4">
        <Link href="/" className="text-lg font-semibold tracking-tight">
          payq
        </Link>
        <button
          onClick={() => setOpen(!open)}
          className="flex h-9 w-9 items-center justify-center rounded-md text-muted hover:bg-accent/5"
          aria-label="Toggle menu"
        >
          {open ? (
            <svg width="20" height="20" viewBox="0 0 20 20" fill="none" stroke="currentColor" strokeWidth="1.5">
              <path d="M5 5l10 10M15 5L5 15" />
            </svg>
          ) : (
            <svg width="20" height="20" viewBox="0 0 20 20" fill="none" stroke="currentColor" strokeWidth="1.5">
              <path d="M3 5h14M3 10h14M3 15h14" />
            </svg>
          )}
        </button>
      </div>

      {open && (
        <div className="border-b border-border bg-card px-4 pb-4 pt-2">
          <nav className="flex flex-col gap-0.5">
            {nav.map(({ href, label }) => {
              const active = pathname === href;
              return (
                <Link
                  key={href}
                  href={href}
                  onClick={() => setOpen(false)}
                  className={`rounded-md px-3 py-2 text-sm font-medium ${
                    active
                      ? "bg-accent/10 text-accent"
                      : "text-muted hover:bg-accent/5 hover:text-foreground"
                  }`}
                >
                  {label}
                </Link>
              );
            })}
          </nav>
          <div className="mt-4 border-t border-border pt-4">
            <WalletButton />
          </div>
        </div>
      )}
    </div>
  );
}
