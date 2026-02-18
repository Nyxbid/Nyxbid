"use client";

import { useEffect, useRef } from "react";
import type { StreamEvent } from "@/lib/data";

const API_URL = process.env.NEXT_PUBLIC_API_URL ?? "http://localhost:8080";

export function useEventStream(onEvent: (ev: StreamEvent) => void) {
  const callbackRef = useRef(onEvent);
  callbackRef.current = onEvent;

  useEffect(() => {
    const es = new EventSource(`${API_URL}/api/events`);

    es.onmessage = (e) => {
      try {
        const ev: StreamEvent = JSON.parse(e.data);
        callbackRef.current(ev);
      } catch {
        // ignore parse errors
      }
    };

    es.onerror = () => {
      // EventSource auto-reconnects on error
    };

    return () => es.close();
  }, []);
}
