"use client";

import { useEffect, useCallback, useRef, useState } from "react";
import type { SpendReceipt } from "@/lib/data";

const API_URL = process.env.NEXT_PUBLIC_API_URL ?? "http://localhost:8080";

export function useReceiptStream(onReceipt: (receipt: SpendReceipt) => void) {
  const callbackRef = useRef(onReceipt);
  callbackRef.current = onReceipt;

  useEffect(() => {
    const es = new EventSource(`${API_URL}/api/events`);

    es.addEventListener("receipt", (e) => {
      try {
        const receipt: SpendReceipt = JSON.parse(e.data);
        callbackRef.current(receipt);
      } catch {
        // ignore parse errors
      }
    });

    es.onerror = () => {
      // EventSource will auto-reconnect
    };

    return () => es.close();
  }, []);
}

export function useLiveReceipts(initial: SpendReceipt[]) {
  const [receipts, setReceipts] = useState(initial);

  const handleNew = useCallback((receipt: SpendReceipt) => {
    setReceipts((prev) => [receipt, ...prev]);
  }, []);

  useReceiptStream(handleNew);

  return receipts;
}
