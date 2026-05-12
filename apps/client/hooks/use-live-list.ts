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
 * Refresh schedule on a matched event:
 *   1. immediate (the server now only publishes WS events *after* its
 *      state-apply task has reconciled the store, so this almost
 *      always returns post-event data);
 *   2. retry at 800ms in case a slow RPC pushed the apply past the WS
 *      hop;
 *   3. retry at 2500ms as a last-chance for confirmed-vs-finalized
 *      lag on devnet.
 *
 * Beyond chain events, we also refresh:
 *   - **on mount**, so the SSR seed (which may be stale by the time
 *     the user navigates back) gets reconciled.
 *   - **on tab focus / visibility change**, so coming back from
 *     another tab shows current state without a hard reload.
 *   - **on WS reconnect** (`onConnected`), so a network blip doesn't
 *     leave us looking at a frozen book.
 *   - **every 8s as a poll fallback**, so even if the WS silently
 *     dies (proxy timeout, mobile background tab, etc.) the page
 *     still catches up within a few seconds.
 *
 * Together these kill the "I had to refresh the page to see my
 * intent" feedback loop.
 *
 * Doherty: live -> visible in <1 slot under normal conditions.
 */
const RETRY_DELAYS_MS = [0, 800, 2500] as const;
const POLL_INTERVAL_MS = 8000;

export function useLiveResource<T>(
  path: string,
  seed: T,
  shouldRefresh: (env: ChainEnvelope) => boolean = () => true,
): { data: T; refresh: () => Promise<void> } {
  const [data, setData] = useState<T>(seed);
  const timersRef = useRef<Set<ReturnType<typeof setTimeout>>>(new Set());
  const predicateRef = useRef(shouldRefresh);
  // Sync the latest predicate via layout-effect — `useChainStream`
  // reads `predicateRef.current` from inside the WS handler, so the
  // sync has to happen before paint, not as a deferred effect.
  useLayoutEffect(() => {
    predicateRef.current = shouldRefresh;
  });

  const clearTimers = useCallback(() => {
    for (const t of timersRef.current) clearTimeout(t);
    timersRef.current.clear();
  }, []);

  const refresh = useCallback(async () => {
    try {
      const next = await fetchJson<T>(path);
      setData(next);
    } catch {
      // Keep last good value; next event or poll tick will retry.
    }
  }, [path]);

  /** Schedule a burst of refreshes that cover the indexer ↔ store
   *  race window without spamming the server. */
  const scheduleBurst = useCallback(() => {
    clearTimers();
    for (const delay of RETRY_DELAYS_MS) {
      const t = setTimeout(() => {
        timersRef.current.delete(t);
        void refresh();
      }, delay);
      timersRef.current.add(t);
    }
  }, [clearTimers, refresh]);

  const onEnvelope = useCallback(
    (env: ChainEnvelope) => {
      if (!predicateRef.current(env)) return;
      scheduleBurst();
    },
    [scheduleBurst],
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

  // Low-frequency poll as a fallback if the WS silently dies. Cheap
  // (~8s) and pauses when the tab isn't visible to avoid hammering
  // the server with background work.
  useEffect(() => {
    const id = setInterval(() => {
      if (typeof document === "undefined") return;
      if (document.visibilityState !== "visible") return;
      void refresh();
    }, POLL_INTERVAL_MS);
    return () => clearInterval(id);
  }, [refresh]);

  useEffect(() => clearTimers, [clearTimers]);

  return { data, refresh };
}
