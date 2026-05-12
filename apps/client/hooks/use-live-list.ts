"use client";

import {
  useCallback,
  useEffect,
  useLayoutEffect,
  useRef,
  useState,
} from "react";

import { fetchJson } from "@/lib/api";
import type { ChainEnvelope } from "@/lib/data";
import { useChainStream } from "@/hooks/use-ws";

/**
 * Combine an initial server-side seed with a live WebSocket reducer.
 *
 * - `seed`: SSR fetch result. Used as the first render's value so the
 *   page paints instantly with real data, never flashes empty.
 * - `path`: REST endpoint to refetch when an event lands. Refetching
 *   beats event-only reducers because it reflects the chain's actual
 *   `Intent.status`, including transitions the event payload alone
 *   doesn't carry.
 * - `shouldRefresh`: predicate over the chain envelope; only refetch
 *   when the event is relevant.
 *
 * Refetches are debounced at 250ms so a burst of events (e.g. five
 * quotes within one slot) collapses into a single read.
 *
 * Beyond chain events, we also refresh:
 *   - **on mount**, so the SSR seed (which may be seconds-to-minutes
 *     stale by the time the user navigates back) gets reconciled.
 *   - **on tab focus / visibility change**, so coming back from
 *     another tab shows current state without a hard reload.
 *   - **on WS reconnect** (`onConnected`), so a network blip doesn't
 *     leave us looking at a frozen book.
 *
 * Together these kill the "I had to refresh the page to see my
 * intent" feedback loop.
 *
 * Doherty: live -> visible in <1 slot under normal conditions.
 */
export function useLiveResource<T>(
  path: string,
  seed: T,
  shouldRefresh: (env: ChainEnvelope) => boolean = () => true,
): { data: T; refresh: () => Promise<void> } {
  const [data, setData] = useState<T>(seed);
  const debounceRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const predicateRef = useRef(shouldRefresh);
  // Sync the latest predicate via layout-effect — `useChainStream`
  // reads `predicateRef.current` from inside the WS handler, so the
  // sync has to happen before paint, not as a deferred effect.
  useLayoutEffect(() => {
    predicateRef.current = shouldRefresh;
  });

  const refresh = useCallback(async () => {
    try {
      const next = await fetchJson<T>(path);
      setData(next);
    } catch {
      // Keep last good value; next event will retry.
    }
  }, [path]);

  const onEnvelope = useCallback(
    (env: ChainEnvelope) => {
      if (!predicateRef.current(env)) return;
      if (debounceRef.current) clearTimeout(debounceRef.current);
      debounceRef.current = setTimeout(() => {
        debounceRef.current = null;
        void refresh();
      }, 250);
    },
    [refresh],
  );

  useChainStream(onEnvelope, { onConnected: refresh });

  // Reconcile the SSR seed once after mount. Without this, navigating
  // back to a dashboard page shows stale data until a chain event
  // wakes it up.
  useEffect(() => {
    // eslint-disable-next-line react-hooks/set-state-in-effect -- intentional: the SSR seed needs a one-shot revalidate on the client to stay fresh after back/forward navigation
    void refresh();
  }, [refresh]);

  // Refresh whenever the tab regains focus / visibility. Browsers
  // throttle background tabs and may pause WS frames, so the snapshot
  // is the easiest way to catch back up on return.
  useEffect(() => {
    const onVisible = () => {
      if (document.visibilityState === "visible") void refresh();
    };
    window.addEventListener("focus", onVisible);
    document.addEventListener("visibilitychange", onVisible);
    return () => {
      window.removeEventListener("focus", onVisible);
      document.removeEventListener("visibilitychange", onVisible);
    };
  }, [refresh]);

  useEffect(() => {
    return () => {
      if (debounceRef.current) clearTimeout(debounceRef.current);
    };
  }, []);

  return { data, refresh };
}
