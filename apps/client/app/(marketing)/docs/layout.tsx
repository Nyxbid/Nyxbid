import Link from "next/link";

import { Brand } from "@/components/brand";
import { GithubIcon } from "@/components/icons";

import { DocsSidebar } from "./_sidebar";

/**
 * Docs shell.
 *
 *   ┌──────────┬──────────────────────────────┐
 *   │ sidebar  │     content (centered)       │
 *   │ (sticky) │     max-w-3xl mx-auto        │
 *   └──────────┴──────────────────────────────┘
 *
 * Two columns only. The previous three-column shell left an empty
 * right rail that made the prose look phone-aligned on a desktop.
 * Now the content is centered inside the right column, so margins
 * are even on both sides.
 *
 * Brand chrome (wordmark + GitHub) floats absolutely in the page's
 * top corners — there is no <header> element, deliberately.
 */
export default function DocsLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  return (
    <div className="lp-docs">
      <DocsChrome />
      <div className="mx-auto max-w-7xl px-6 md:px-10">
        <div className="grid grid-cols-1 gap-x-12 gap-y-10 pb-32 pt-32 lg:grid-cols-[220px_minmax(0,1fr)]">
          <aside className="hidden lg:block">
            <div className="sticky top-24">
              <DocsSidebar />
            </div>
          </aside>
          <main className="mx-auto w-full min-w-0 max-w-3xl">
            {children}
          </main>
        </div>
      </div>
    </div>
  );
}

function DocsChrome() {
  return (
    <div className="pointer-events-none absolute inset-x-0 top-0 z-10 px-6 pt-7 md:px-10 md:pt-9">
      <div className="pointer-events-auto mx-auto flex max-w-7xl items-center justify-between">
        <Brand size="md" />
        <Link
          href="https://github.com/Nyxbid/Nyxbid"
          target="_blank"
          rel="noopener noreferrer"
          aria-label="Nyxbid on GitHub"
          className="inline-flex h-9 w-9 items-center justify-center rounded-full text-[color-mix(in_srgb,var(--fg)_80%,transparent)] transition-colors hover:bg-[color-mix(in_srgb,var(--fg)_8%,transparent)] hover:text-foreground"
        >
          <GithubIcon size={19} />
        </Link>
      </div>
    </div>
  );
}
