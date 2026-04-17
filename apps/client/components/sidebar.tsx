"use client";

import Link from "next/link";
import { usePathname } from "next/navigation";
import { WalletButton } from "@/components/wallet-button";

const nav = [
  { href: "/dashboard", label: "Dashboard" },
  { href: "/intents", label: "Intents" },
  { href: "/quotes", label: "Quotes" },
  { href: "/fills", label: "Fills" },
];

export function Sidebar() {
  const pathname = usePathname();

  return (
    <aside className="hidden w-56 shrink-0 border-r border-border bg-card md:flex md:flex-col">
      <div className="flex h-14 items-center px-5">
        <Link href="/" className="text-lg font-semibold tracking-tight">
          nyxbid
        </Link>
      </div>

      <nav className="flex flex-1 flex-col gap-0.5 px-3 pt-2">
        {nav.map(({ href, label }) => {
          const active = pathname === href;
          return (
            <Link
              key={href}
              href={href}
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

      <div className="border-t border-border px-5 py-4">
        <WalletButton />
        <p className="mt-3 text-[10px] text-muted">Solana Devnet</p>
      </div>
    </aside>
  );
}
