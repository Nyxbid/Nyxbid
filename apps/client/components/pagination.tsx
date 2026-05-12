"use client";

import { useEffect, useMemo, useState } from "react";

/**
 * Lightweight in-memory paginator for list views.
 *
 * The caller passes the full array; this hook hands back a stable
 * window plus the controls to render. It guards against three real
 * footguns the previous "just let the div scroll" approach had:
 *
 *   1. The page only ever shows a bounded number of rows, so the
 *      surrounding layout never grows beyond viewport. No more
 *      mystery scrollbars on a 200-intent book.
 *
 *   2. When the underlying list shrinks (e.g. fills get pruned, or
 *      the WS reorders), the current page is clamped so the user
 *      never lands on an empty page.
 *
 *   3. Page index is local state, not URL state — the user's view
 *      doesn't reset on every live WebSocket refresh.
 *
 * `pageSize` defaults to 20, which fits a standard dashboard card
 * without forcing the body to scroll on a 1080p screen.
 */
export interface PageState<T> {
  /** Sliced data for the current page. */
  rows: T[];
  /** Zero-based index of the current page. */
  page: number;
  /** Total number of pages (>= 1, even when empty). */
  pageCount: number;
  /** Inclusive 1-based row indices currently visible: `[from, to]`. */
  from: number;
  to: number;
  /** Total rows across all pages. */
  total: number;
  setPage: (p: number) => void;
  next: () => void;
  prev: () => void;
  canNext: boolean;
  canPrev: boolean;
}

export function usePagination<T>(rows: T[], pageSize = 20): PageState<T> {
  const [page, setPage] = useState(0);

  // Clamp the current page when the dataset shrinks. Without this,
  // a live WS update that drops trailing rows would leave the user
  // stranded on a now-empty page index.
  const pageCount = Math.max(1, Math.ceil(rows.length / pageSize));
  useEffect(() => {
    if (page > pageCount - 1) setPage(pageCount - 1);
  }, [page, pageCount]);

  const safePage = Math.min(page, pageCount - 1);

  const sliced = useMemo(() => {
    const start = safePage * pageSize;
    return rows.slice(start, start + pageSize);
  }, [rows, safePage, pageSize]);

  const total = rows.length;
  const from = total === 0 ? 0 : safePage * pageSize + 1;
  const to = Math.min((safePage + 1) * pageSize, total);

  return {
    rows: sliced,
    page: safePage,
    pageCount,
    from,
    to,
    total,
    setPage,
    next: () => setPage((p) => Math.min(p + 1, pageCount - 1)),
    prev: () => setPage((p) => Math.max(p - 1, 0)),
    canNext: safePage < pageCount - 1,
    canPrev: safePage > 0,
  };
}

interface PaginatorProps {
  page: number;
  pageCount: number;
  from: number;
  to: number;
  total: number;
  onPrev: () => void;
  onNext: () => void;
  canPrev: boolean;
  canNext: boolean;
  /** Optional label for what the rows represent, e.g. "intents". */
  noun?: string;
}

/**
 * Default footer for paginated tables. Renders only when there's
 * more than one page so single-page lists stay clean.
 */
export function Paginator({
  page,
  pageCount,
  from,
  to,
  total,
  onPrev,
  onNext,
  canPrev,
  canNext,
  noun = "rows",
}: PaginatorProps) {
  if (pageCount <= 1) return null;
  return (
    <div className="flex items-center justify-between border-t border-[var(--hairline)] px-5 py-3 text-[11px] tracking-tight text-muted">
      <p className="font-mono">
        {from}–{to} of {total} {noun}
      </p>
      <div className="flex items-center gap-2">
        <PagerButton onClick={onPrev} disabled={!canPrev}>
          ‹ Prev
        </PagerButton>
        <span className="font-mono text-faint">
          {page + 1} / {pageCount}
        </span>
        <PagerButton onClick={onNext} disabled={!canNext}>
          Next ›
        </PagerButton>
      </div>
    </div>
  );
}

function PagerButton({
  children,
  disabled,
  onClick,
}: {
  children: React.ReactNode;
  disabled: boolean;
  onClick: () => void;
}) {
  return (
    <button
      type="button"
      onClick={onClick}
      disabled={disabled}
      className="inline-flex h-7 items-center rounded-[var(--r-xs)] border border-[var(--hairline-strong)] bg-[var(--surface)] px-2.5 font-mono text-[11px] tracking-tight text-foreground transition-colors hover:bg-[var(--surface-2)] disabled:cursor-not-allowed disabled:opacity-40 disabled:hover:bg-[var(--surface)]"
    >
      {children}
    </button>
  );
}
