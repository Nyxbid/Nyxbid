"use client";

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

export function Sidebar() {
  const pathname = usePathname();

  return (
    <aside className="hidden w-56 shrink-0 border-r border-[var(--hairline)] md:flex md:flex-col">
      <div className="flex h-14 items-center border-b border-[var(--hairline)] px-5">
        <Link
          href="/"
          className="text-[19px] tracking-tight text-foreground/95 hover:text-foreground"
          style={{ fontFamily: "var(--font-serif)" }}
        >
          Nyxbid
        </Link>
      </div>

      <nav className="flex flex-1 flex-col gap-px px-2.5 pt-3">
        {nav.map(({ href, label }) => {
          const active =
            pathname === href || pathname?.startsWith(`${href}/`);
          return (
            <Link
              key={href}
              href={href}
              className={`flex h-9 items-center rounded-[var(--r-sm)] px-3 text-[13px] font-medium tracking-tight transition-colors ${
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

      <div className="border-t border-[var(--hairline)] p-4">
        <WalletButton />
        <p className="mt-3 font-mono text-[10px] uppercase tracking-[0.14em] text-faint">
          devnet
        </p>
      </div>
    </aside>
  );
}
