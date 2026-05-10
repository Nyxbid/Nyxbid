"use client";

import { useState } from "react";
import Link from "next/link";
import { usePathname } from "next/navigation";

import { WalletButton } from "@/components/wallet-button";

const nav = [
  { href: "/trade", label: "Trade" },
  { href: "/dashboard", label: "Dashboard" },
  { href: "/intents", label: "Intents" },
  { href: "/quotes", label: "Quotes" },
  { href: "/fills", label: "Fills" },
];

export function MobileNav() {
  const [open, setOpen] = useState(false);
  const pathname = usePathname();

  return (
    <div className="md:hidden">
      <div className="flex h-14 items-center justify-between border-b border-[var(--hairline)] px-4">
        <Link
          href="/"
          className="text-[19px] tracking-tight text-foreground/95 hover:text-foreground"
          style={{ fontFamily: "var(--font-serif)" }}
        >
          Nyxbid
        </Link>
        <button
          onClick={() => setOpen(!open)}
          className="flex h-9 w-9 items-center justify-center rounded-[var(--r-sm)] text-muted hover:bg-[var(--surface)] hover:text-foreground"
          aria-label="Toggle menu"
        >
          {open ? (
            <svg width="18" height="18" viewBox="0 0 18 18" fill="none" stroke="currentColor" strokeWidth="1.5">
              <path d="M4 4l10 10M14 4L4 14" />
            </svg>
          ) : (
            <svg width="18" height="18" viewBox="0 0 18 18" fill="none" stroke="currentColor" strokeWidth="1.5">
              <path d="M3 5h12M3 9h12M3 13h12" />
            </svg>
          )}
        </button>
      </div>

      {open && (
        <div className="border-b border-[var(--hairline)] px-4 pb-4 pt-2">
          <nav className="flex flex-col gap-px">
            {nav.map(({ href, label }) => {
              const active =
                pathname === href || pathname?.startsWith(`${href}/`);
              return (
                <Link
                  key={href}
                  href={href}
                  onClick={() => setOpen(false)}
                  className={`flex h-9 items-center rounded-[var(--r-sm)] px-3 text-[13px] font-medium ${
                    active
                      ? "bg-[var(--surface-2)] text-foreground"
                      : "text-muted hover:bg-[var(--surface)] hover:text-foreground"
                  }`}
                >
                  {label}
                </Link>
              );
            })}
          </nav>
          <div className="mt-4 border-t border-[var(--hairline)] pt-4">
            <WalletButton />
          </div>
        </div>
      )}
    </div>
  );
}
