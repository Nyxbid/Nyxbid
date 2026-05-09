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

  useChainStream(onEnvelope);

  useEffect(() => {
    return () => {
      if (debounceRef.current) clearTimeout(debounceRef.current);
    };
  }, []);

  return { data, refresh };
}
