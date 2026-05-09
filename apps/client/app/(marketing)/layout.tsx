import Link from "next/link";

/**
 * Marketing shell.
 *
 * No <header> element at all — the wordmark and GitHub icon are
 * rendered inside the hero itself (see `(marketing)/page.tsx`).
 * That's deliberate: the user kept perceiving a "solid navbar"
 * because *any* persistent element in the top viewport region
 * reads as a bar. The fix is to not render one.
 *
 * Footer stays — it's at the bottom of the document flow, not the
 * top, so it can't be confused for a navbar.
 */
export default function MarketingLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  return (
    <div className="lp flex min-h-screen flex-col">
      <main className="flex-1">{children}</main>

      <footer className="lp-cosmos">
        <div className="mx-auto flex max-w-7xl flex-col gap-6 px-6 py-10 md:flex-row md:items-center md:justify-between md:px-10">
          <p className="font-mono text-[10px] uppercase tracking-[0.22em] text-[color-mix(in_srgb,var(--fg)_55%,transparent)]">
            © 2026 Nyxbid · built on solana
          </p>
          <div className="flex items-center gap-7">
            {[
              { href: "/docs", label: "Docs" },
              {
                href: "https://github.com/Nyxbid/Nyxbid",
                label: "GitHub",
                external: true,
              },
              { href: "https://x.com/", label: "X", external: true },
            ].map((l) => (
              <Link
                key={l.label}
                href={l.href}
                className="font-mono text-[10px] uppercase tracking-[0.22em] text-[color-mix(in_srgb,var(--fg)_70%,transparent)] hover:text-foreground"
                {...(l.external
                  ? { target: "_blank", rel: "noopener noreferrer" }
                  : {})}
              >
                {l.label} ↗
              </Link>
            ))}
          </div>
        </div>
      </footer>
    </div>
  );
}
