"use client";

import { useEffect, useLayoutEffect, useRef } from "react";

import {
  isChainEnvelope,
  type ChainEnvelope,
  type WsMessage,
} from "@/lib/data";
import { websocketUrl } from "@/lib/api";

/**
 * Subscribe to the server's `/ws` chain-event stream.
 *
 * The handler is held in a ref so callers don't have to wrap it in
 * `useCallback`; the WebSocket is created exactly once per mount,
 * reconnects with exponential backoff on close/error, and the cleanup
 * tears down both the socket and the reconnect timer so the page
 * never leaks connections.
 *
 * `onConnected` fires every time the socket transitions to OPEN —
 * including after a reconnect — so callers can refetch their REST
 * snapshot and pick up any events the page missed while offline.
 *
 * Doherty Threshold: a chain event reaches the UI within one slot
 * (~400ms) of landing on chain.
 */
export function useChainStream(
  onEnvelope: (env: ChainEnvelope) => void,
  options: { enabled?: boolean; onConnected?: () => void } = {},
): void {
  const enabled = options.enabled ?? true;
  const handlerRef = useRef(onEnvelope);
  const connectedRef = useRef(options.onConnected);
  // Keep the latest handler in a ref via a layout-effect so the
  // WebSocket message dispatcher always calls the freshest closure
  // without us having to re-create the socket on every render.
  useLayoutEffect(() => {
    handlerRef.current = onEnvelope;
    connectedRef.current = options.onConnected;
  });

  useEffect(() => {
    if (!enabled) return;

    let socket: WebSocket | null = null;
    let reconnectTimer: ReturnType<typeof setTimeout> | null = null;
    let attempt = 0;
    let cancelled = false;

    const connect = () => {
      if (cancelled) return;
      socket = new WebSocket(websocketUrl());

      socket.onopen = () => {
        attempt = 0;
        connectedRef.current?.();
      };

      socket.onmessage = (e) => {
        try {
          const msg = JSON.parse(e.data) as WsMessage;
          if (isChainEnvelope(msg)) {
            handlerRef.current(msg);
          }
        } catch {
          // ignore malformed frames; the WS is JSON-only
        }
      };

      const scheduleReconnect = () => {
        if (cancelled) return;
        attempt = Math.min(attempt + 1, 6);
        const delay = Math.min(500 * 2 ** attempt, 15_000);
        reconnectTimer = setTimeout(connect, delay);
      };

      socket.onclose = scheduleReconnect;
      socket.onerror = () => {
        // onclose fires immediately after onerror, so let scheduleReconnect
        // run from there. Closing here just speeds up the cycle.
        socket?.close();
      };
    };

    connect();

    return () => {
      cancelled = true;
      if (reconnectTimer) clearTimeout(reconnectTimer);
      socket?.close();
    };
  }, [enabled]);
}
