import type { ReactNode } from "react";

/**
 * Single-source page header.
 *
 * Layout: mono eyebrow → serif title → optional right-rail actions.
 * Replaces the old "title + marketing-copy subtitle" pattern; product
 * pages don't need to re-explain the product on every visit.
 *
 * Why serif for the title: the marketing surface uses serif for the
 * brand voice; matching that on dashboard pages keeps the wordmark,
 * landing hero, and product page heads visually one family. The body
 * copy stays sans because data tables read better that way.
 */
export function PageHeader({
  title,
  eyebrow,
  actions,
}: {
  title: string;
  eyebrow?: string;
  actions?: ReactNode;
}) {
  return (
    <div className="flex items-end justify-between gap-4">
      <div>
        {eyebrow && (
          <p className="font-mono text-[10px] uppercase tracking-[0.18em] text-muted">
            {eyebrow}
          </p>
        )}
        <h1
          className="mt-2 text-[32px] leading-none tracking-tight text-foreground"
          style={{ fontFamily: "var(--font-serif)" }}
        >
          {title}
        </h1>
      </div>
      {actions && <div className="flex items-center gap-2">{actions}</div>}
    </div>
  );
}
