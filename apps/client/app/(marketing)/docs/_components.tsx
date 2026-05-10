/**
 * Docs primitives. Single-file collection so each docs page can pull
 * exactly what it needs without scattering small components across a
 * directory. None of these are routes (the underscore prefix keeps
 * Next.js from treating this folder as a route segment).
 *
 * Conventions:
 *   - Block elements use semantic tags + `docs-*` class names so
 *     they pick up the global prose styles in globals.css.
 *   - HTTP method badges use `MethodPill` for consistency.
 *   - Endpoint rows pair a method + path + one-line description so
 *     reference tables stay scannable.
 */

import type { ReactNode } from "react";

import { CopyButton } from "./_copy-button";

/* ── headings & prose ──────────────────────────────────────────── */

export function PageHead({
  eyebrow,
  title,
  description,
}: {
  eyebrow?: string;
  title: ReactNode;
  description?: ReactNode;
}) {
  return (
    <header className="border-b border-[var(--hairline)] pb-10">
      {eyebrow && <p className="lp-eyebrow">{eyebrow}</p>}
      <h1 className="docs-h1 mt-4">{title}</h1>
      {description && (
        <p className="mt-5 max-w-2xl text-[16px] leading-[1.65] text-[color-mix(in_srgb,var(--fg)_70%,transparent)]">
          {description}
        </p>
      )}
    </header>
  );
}

export function H2({ id, children }: { id: string; children: ReactNode }) {
  return (
    <h2 id={id} className="docs-h2 mt-16 scroll-mt-24">
      <a href={`#${id}`} className="group inline-flex items-baseline gap-2">
        {children}
        <span className="font-mono text-[14px] text-faint opacity-0 transition-opacity group-hover:opacity-100">
          #
        </span>
      </a>
    </h2>
  );
}

export function H3({ id, children }: { id?: string; children: ReactNode }) {
  return (
    <h3 id={id} className="docs-h3 mt-10">
      {children}
    </h3>
  );
}

export function P({ children }: { children: ReactNode }) {
  return <p className="docs-p mt-4">{children}</p>;
}

export function UL({ children }: { children: ReactNode }) {
  return <ul className="mt-4 space-y-2 pl-5 [&>li]:list-disc">{children}</ul>;
}

export function LI({ children }: { children: ReactNode }) {
  return <li className="docs-li">{children}</li>;
}

export function Strong({ children }: { children: ReactNode }) {
  return <strong className="docs-strong">{children}</strong>;
}

export function Code({ children }: { children: ReactNode }) {
  return <code className="docs-inline">{children}</code>;
}

export function A({
  href,
  children,
  external,
}: {
  href: string;
  children: ReactNode;
  external?: boolean;
}) {
  return (
    <a
      href={href}
      className="docs-a"
      {...(external
        ? { target: "_blank", rel: "noopener noreferrer" }
        : {})}
    >
      {children}
    </a>
  );
}

/* ── code block ────────────────────────────────────────────────── */

/**
 * Bordered code surface with optional title bar (filename / lang)
 * and a copy-to-clipboard button. Keeps the editorial feel of the
 * docs without depending on a syntax-highlighting bundle.
 */
export function CodeBlock({
  children,
  title,
  lang,
}: {
  children: string;
  title?: string;
  lang?: string;
}) {
  return (
    <div className="mt-5 overflow-hidden rounded-lg border border-[var(--hairline)] bg-[var(--surface)]">
      {(title || lang) && (
        <div className="flex items-center justify-between border-b border-[var(--hairline)] px-4 py-2">
          <p className="font-mono text-[11px] tracking-tight text-muted">
            {title ?? lang}
          </p>
          {lang && title && (
            <p className="font-mono text-[10px] uppercase tracking-[0.18em] text-faint">
              {lang}
            </p>
          )}
        </div>
      )}
      <div className="relative">
        <pre className="overflow-x-auto px-4 py-4 font-mono text-[12.5px] leading-relaxed text-foreground">
          <code>{children}</code>
        </pre>
        <div className="absolute right-2 top-2">
          <CopyButton value={children} />
        </div>
      </div>
    </div>
  );
}

/* ── HTTP method pill ──────────────────────────────────────────── */

const methodColor: Record<string, string> = {
  GET: "text-[var(--accent-soft)] border-[var(--accent-soft)]/30 bg-[var(--accent-soft)]/8",
  POST: "text-[var(--buy)] border-[var(--buy)]/30 bg-[var(--buy)]/8",
  PUT: "text-[var(--warn)] border-[var(--warn)]/30 bg-[var(--warn)]/8",
  DELETE: "text-[var(--sell)] border-[var(--sell)]/30 bg-[var(--sell)]/8",
  WS: "text-[var(--accent)] border-[var(--accent)]/30 bg-[var(--accent)]/8",
  SSE: "text-[var(--accent)] border-[var(--accent)]/30 bg-[var(--accent)]/8",
};

export function MethodPill({ method }: { method: string }) {
  const cls = methodColor[method] ?? methodColor.GET;
  return (
    <span
      className={`inline-flex h-5 min-w-[44px] items-center justify-center rounded border px-2 font-mono text-[10px] font-medium uppercase tracking-[0.08em] ${cls}`}
    >
      {method}
    </span>
  );
}

/* ── endpoint row (used in API ref tables) ────────────────────── */

export function Endpoint({
  method,
  path,
  description,
}: {
  method: string;
  path: string;
  description: string;
}) {
  return (
    <div className="flex items-center justify-between gap-4 border-t border-[var(--hairline)] py-3 first:border-t-0">
      <div className="flex min-w-0 items-center gap-3">
        <MethodPill method={method} />
        <code className="docs-inline truncate">{path}</code>
      </div>
      <p className="hidden truncate text-[13px] text-muted sm:block">
        {description}
      </p>
    </div>
  );
}

/* ── parameter / field table ───────────────────────────────────── */

interface FieldRow {
  name: string;
  type: string;
  description: string;
  required?: boolean;
}

export function FieldTable({ rows }: { rows: FieldRow[] }) {
  return (
    <div className="mt-5 overflow-hidden rounded-lg border border-[var(--hairline)]">
      <table className="w-full text-left">
        <thead>
          <tr className="border-b border-[var(--hairline)] bg-[var(--surface)]">
            <Th>Field</Th>
            <Th>Type</Th>
            <Th>Description</Th>
          </tr>
        </thead>
        <tbody>
          {rows.map((r) => (
            <tr
              key={r.name}
              className="border-b border-[var(--hairline)] last:border-0"
            >
              <td className="px-4 py-3 align-top">
                <code className="docs-inline">{r.name}</code>
                {r.required && (
                  <span className="ml-2 font-mono text-[10px] uppercase tracking-[0.14em] text-[var(--sell)]">
                    req
                  </span>
                )}
              </td>
              <td className="px-4 py-3 align-top font-mono text-[12px] text-muted">
                {r.type}
              </td>
              <td className="px-4 py-3 align-top text-[13px] text-[color-mix(in_srgb,var(--fg)_75%,transparent)]">
                {r.description}
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}

function Th({ children }: { children: ReactNode }) {
  return (
    <th className="px-4 py-2.5 font-mono text-[10px] font-medium uppercase tracking-[0.14em] text-muted">
      {children}
    </th>
  );
}

/* ── callout (info / warn / note) ──────────────────────────────── */

export function Callout({
  kind = "info",
  title,
  children,
}: {
  kind?: "info" | "warn" | "note";
  title?: string;
  children: ReactNode;
}) {
  const colorByKind: Record<string, string> = {
    info: "border-l-[var(--accent-soft)]",
    warn: "border-l-[var(--warn)]",
    note: "border-l-[var(--hairline-strong)]",
  };
  return (
    <div
      className={`mt-6 rounded-r border-l-2 ${colorByKind[kind]} bg-[var(--surface)] px-4 py-3`}
    >
      {title && (
        <p className="font-mono text-[10px] uppercase tracking-[0.18em] text-muted">
          {title}
        </p>
      )}
      <div className="mt-1 text-[14px] leading-[1.6] text-[color-mix(in_srgb,var(--fg)_82%,transparent)]">
        {children}
      </div>
    </div>
  );
}

/* ── prev / next pager at the foot of each page ───────────────── */

interface PagerLink {
  href: string;
  label: string;
}

export function Pager({ prev, next }: { prev?: PagerLink; next?: PagerLink }) {
  return (
    <nav className="mt-20 grid grid-cols-2 gap-4 border-t border-[var(--hairline)] pt-8">
      <div>
        {prev && (
          <a
            href={prev.href}
            className="block rounded-md border border-[var(--hairline)] px-4 py-3 transition-colors hover:bg-[var(--surface)]"
          >
            <p className="font-mono text-[10px] uppercase tracking-[0.18em] text-faint">
              ← Previous
            </p>
            <p className="mt-1 text-[14px] text-foreground">{prev.label}</p>
          </a>
        )}
      </div>
      <div>
        {next && (
          <a
            href={next.href}
            className="block rounded-md border border-[var(--hairline)] px-4 py-3 text-right transition-colors hover:bg-[var(--surface)]"
          >
            <p className="font-mono text-[10px] uppercase tracking-[0.18em] text-faint">
              Next →
            </p>
            <p className="mt-1 text-[14px] text-foreground">{next.label}</p>
          </a>
        )}
      </div>
    </nav>
  );
}
