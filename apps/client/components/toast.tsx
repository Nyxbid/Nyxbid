"use client";

import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useRef,
  useState,
  type ReactNode,
} from "react";

type ToastKind = "info" | "success" | "error";

interface Toast {
  id: number;
  kind: ToastKind;
  title: string;
  body?: string;
  href?: string;
  hrefLabel?: string;
}

interface Ctx {
  push: (t: Omit<Toast, "id">) => number;
  dismiss: (id: number) => void;
  update: (id: number, patch: Partial<Omit<Toast, "id">>) => void;
}

const ToastCtx = createContext<Ctx | null>(null);

/**
 * Lightweight toast system tuned for tx feedback.
 *
 * Why hand-rolled? Two reasons:
 *  - we never need queue management or theming, so a 60-line provider
 *    beats pulling in `react-hot-toast` + its CSS;
 *  - tx flows often need to *update* a toast in place (preparing →
 *    signing → confirmed), and `update(id, patch)` is the exact
 *    primitive we need.
 *
 * Aesthetic-Usability + Doherty: positioned in the bottom-right with
 * a 200ms slide-in so the user gets <400ms feedback after every
 * action without the screen ever rearranging.
 */
export function ToastProvider({ children }: { children: ReactNode }) {
  const [toasts, setToasts] = useState<Toast[]>([]);
  const idRef = useRef(0);

  const dismiss = useCallback((id: number) => {
    setToasts((cur) => cur.filter((t) => t.id !== id));
  }, []);

  const push = useCallback<Ctx["push"]>((t) => {
    const id = ++idRef.current;
    setToasts((cur) => [...cur, { id, ...t }]);
    return id;
  }, []);

  const update = useCallback<Ctx["update"]>((id, patch) => {
    setToasts((cur) =>
      cur.map((t) => (t.id === id ? { ...t, ...patch } : t)),
    );
  }, []);

  return (
    <ToastCtx.Provider value={{ push, dismiss, update }}>
      {children}
      <div className="pointer-events-none fixed inset-x-0 bottom-4 z-50 flex flex-col items-center gap-2 px-4 sm:bottom-6 sm:right-6 sm:items-end sm:px-0">
        {toasts.map((t) => (
          <ToastItem key={t.id} toast={t} onDismiss={() => dismiss(t.id)} />
        ))}
      </div>
    </ToastCtx.Provider>
  );
}

function ToastItem({
  toast,
  onDismiss,
}: {
  toast: Toast;
  onDismiss: () => void;
}) {
  // Auto-dismiss successes and errors; keep info open until updated.
  useEffect(() => {
    if (toast.kind === "info") return;
    const t = setTimeout(onDismiss, toast.kind === "error" ? 6000 : 4500);
    return () => clearTimeout(t);
  }, [toast.kind, onDismiss]);

  const accent =
    toast.kind === "success"
      ? "border-emerald-500/50 bg-emerald-500/[0.14] shadow-emerald-900/10"
      : toast.kind === "error"
        ? "border-rose-500/50 bg-rose-500/[0.14] shadow-rose-900/10"
        : "border-[color-mix(in_srgb,var(--accent)_35%,var(--hairline))] bg-[color-mix(in_srgb,var(--card)_96%,var(--accent)_8%)] ring-1 ring-[color-mix(in_srgb,var(--accent)_18%,transparent)]";

  return (
    <div
      role="status"
      className={`pointer-events-auto w-full max-w-sm animate-toast-in rounded-lg border ${accent} px-4 py-3.5 shadow-xl shadow-black/20 backdrop-blur-md`}
    >
      <div className="flex items-start gap-3">
        <span
          className={`mt-1 inline-block h-2 w-2 shrink-0 rounded-full ${
            toast.kind === "success"
              ? "bg-emerald-500"
              : toast.kind === "error"
                ? "bg-rose-500"
                : "animate-pulse bg-accent"
          }`}
        />
        <div className="min-w-0 flex-1">
          <p className="text-[13px] font-semibold leading-snug text-foreground">
            {toast.title}
          </p>
          {toast.body && (
            <p className="mt-1 break-words text-[12px] leading-relaxed text-foreground/85">
              {toast.body}
            </p>
          )}
          {toast.href && (
            <a
              href={toast.href}
              target="_blank"
              rel="noopener noreferrer"
              className="mt-1 inline-block text-xs font-medium text-accent hover:underline"
            >
              {toast.hrefLabel ?? "View"} →
            </a>
          )}
        </div>
        <button
          onClick={onDismiss}
          className="text-muted hover:text-foreground"
          aria-label="Dismiss"
        >
          <svg width="14" height="14" viewBox="0 0 14 14" fill="none">
            <path
              d="M3 3l8 8M11 3L3 11"
              stroke="currentColor"
              strokeWidth="1.5"
              strokeLinecap="round"
            />
          </svg>
        </button>
      </div>
    </div>
  );
}

export function useToast(): Ctx {
  const ctx = useContext(ToastCtx);
  if (!ctx) throw new Error("useToast must be used inside <ToastProvider>");
  return ctx;
}
