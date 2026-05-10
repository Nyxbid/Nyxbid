"use client";

import { useCallback, useState } from "react";
import { useConnection, useWallet } from "@solana/wallet-adapter-react";
import { Transaction } from "@solana/web3.js";

import type { PreparedTx } from "@/lib/data";

export type TxPhase =
  | "idle"
  | "preparing" // talking to /api/tx/*
  | "signing" // wallet open
  | "sending" // RPC submit
  | "confirming" // awaiting cluster confirmation
  | "confirmed"
  | "error";

export interface TxState {
  phase: TxPhase;
  signature?: string;
  prepared?: PreparedTx;
  error?: string;
}

const initial: TxState = { phase: "idle" };

/**
 * Drive a Nyxbid action end-to-end:
 *
 *   1. POST to `/api/tx/<action>` to get a `PreparedTx` (server never
 *      signs; it just borsh-encodes the instruction and bakes a
 *      blockhash).
 *   2. Hand the bincode-serialised tx to the connected wallet so the
 *      user signs it.
 *   3. Push the signed tx straight to the user's RPC (faster than
 *      relay-via-server) using the wallet adapter's connection.
 *   4. Block until the cluster reaches `confirmed` for the signature,
 *      using the `blockhash` + `last_valid_block_height` the server
 *      already pinned.
 *
 * The hook surfaces a flat phase enum so a button can show
 * "Preparing… → Sign in wallet → Confirming…" without juggling
 * booleans. On error we keep the partial state so the UI can render
 * a `Retry` action against the same prepared tx.
 *
 * Jakob's Law: the flow matches the wallet UX every Solana app uses,
 * so users never have to learn a new pattern.
 */
/**
 * `run()` returns BOTH the signature and the prepared tx so callers
 * can read PDAs (e.g. `prepared.accounts.intent`) immediately after
 * `await run(...)`. Reading them off `state.prepared` after the
 * await would be wrong: React batches setState calls inside async
 * fns, so the outer caller's render closure still holds the
 * pre-run `state`. The return value is the only correct path.
 */
export interface TxResult {
  signature: string;
  prepared: PreparedTx;
}

export function useNyxbidTx<P>(prepare: (params: P) => Promise<PreparedTx>) {
  const { publicKey, signTransaction } = useWallet();
  const { connection } = useConnection();
  const [state, setState] = useState<TxState>(initial);

  const reset = useCallback(() => setState(initial), []);

  const run = useCallback(
    async (params: P): Promise<TxResult> => {
      if (!publicKey || !signTransaction) {
        const message = "Connect a wallet first.";
        setState({ phase: "error", error: message });
        throw new Error(message);
      }

      try {
        setState({ phase: "preparing" });
        const prepared = await prepare(params);

        setState({ phase: "signing", prepared });
        const txBytes = Uint8Array.from(atob(prepared.tx_base64), (c) =>
          c.charCodeAt(0),
        );
        const tx = Transaction.from(txBytes);
        const signed = await signTransaction(tx);

        setState({ phase: "sending", prepared });
        const signature = await connection.sendRawTransaction(
          signed.serialize(),
          { skipPreflight: false, maxRetries: 3 },
        );

        setState({ phase: "confirming", prepared, signature });
        const confirmation = await connection.confirmTransaction(
          {
            signature,
            blockhash: prepared.blockhash,
            lastValidBlockHeight: prepared.last_valid_block_height,
          },
          "confirmed",
        );
        if (confirmation.value.err) {
          throw new Error(
            `tx failed: ${JSON.stringify(confirmation.value.err)}`,
          );
        }

        setState({ phase: "confirmed", prepared, signature });
        return { signature, prepared };
      } catch (e) {
        const message =
          e instanceof Error ? e.message : "transaction failed";
        setState((prev) => ({ ...prev, phase: "error", error: message }));
        throw e;
      }
    },
    [connection, prepare, publicKey, signTransaction],
  );

  return { state, run, reset };
}
