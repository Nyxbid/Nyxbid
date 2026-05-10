"use client";

import { useCallback, useState } from "react";
import { useConnection, useWallet } from "@solana/wallet-adapter-react";
import { Transaction } from "@solana/web3.js";

import type { PreparedTx } from "@/lib/data";
import { isWalletUserRejection } from "@/lib/tx-errors";

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
 *   3. Push the signed tx straight to the user's RPC using the wallet
 *      adapter's connection.
 *   4. Block until the cluster reaches `confirmed` for the signature.
 *
 * `run()` returns both the signature and the prepared tx so callers
 * can read PDAs (e.g. `prepared.accounts.intent`) immediately after
 * `await run(...)` — not from `state`, which may not have flushed yet.
 *
 * If the user dismisses the wallet prompt, `run` resolves to `null`
 * (no throw). State goes back to `idle` with `prepared` retained for
 * an easy retry.
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
    async (params: P): Promise<TxResult | null> => {
      if (!publicKey || !signTransaction) {
        const message = "Connect a wallet first.";
        setState({ phase: "error", error: message });
        throw new Error(message);
      }

      let prepared: PreparedTx | undefined;
      try {
        setState({ phase: "preparing" });
        prepared = await prepare(params);

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
        if (isWalletUserRejection(e)) {
          setState({
            phase: "idle",
            ...(prepared ? { prepared } : {}),
          });
          return null;
        }
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
