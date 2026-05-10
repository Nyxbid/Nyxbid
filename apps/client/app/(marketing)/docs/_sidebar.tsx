"use client";

import Link from "next/link";
import { usePathname } from "next/navigation";

/**
 * Trader-focused docs sidebar.
 *
 * No "REST API" page, no "Anchor program" page, no folder-structure
 * dump — Nyxbid doesn't ship an SDK, so the docs are written for the
 * two human roles that actually use the venue (takers and makers)
 * plus the agent integration that makes Nyxbid distinct, plus a
 * one-page mainnet deploy guide for operators.
 */
const groups: { title: string; items: { href: string; label: string }[] }[] = [
  {
    title: "Get started",
    items: [
      { href: "/docs", label: "Overview" },
    ],
  },
  {
    title: "Trade",
    items: [
      { href: "/docs/takers", label: "For takers" },
      { href: "/docs/makers", label: "For makers" },
    ],
  },
  {
    title: "Build",
    items: [
      { href: "/docs/agents", label: "Agent integration" },
    ],
  },
];

export function DocsSidebar() {
  const pathname = usePathname();
  return (
    <nav className="flex flex-col gap-7">
      {groups.map((group) => (
        <div key={group.title}>
          <p className="docs-nav-group px-2.5">{group.title}</p>
          <div className="mt-2 flex flex-col gap-px">
            {group.items.map((item) => {
              const isActive = pathname === item.href;
              return (
                <Link
                  key={item.href}
                  href={item.href}
                  className={`docs-nav-item ${isActive ? "is-active" : ""}`}
                >
                  {item.label}
                </Link>
              );
            })}
          </div>
        </div>
      ))}
    </nav>
  );
}
