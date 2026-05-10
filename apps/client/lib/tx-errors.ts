/**
 * Map raw Solana / wallet errors into short toast copy. Simulation
 * logs are huge; we match on stable substrings and Nyxbid error names.
 */

export interface FriendlyTxError {
  title: string;
  body: string;
}

/** Normalize any thrown value to a single searchable string. */
function rawMessage(err: unknown): string {
  if (err instanceof Error) return err.message;
  if (typeof err === "string") return err;
  try {
    return JSON.stringify(err);
  } catch {
    return String(err);
  }
}

export function friendlyTxError(err: unknown): FriendlyTxError {
  const raw = rawMessage(err);
  const s = raw.toLowerCase();

  if (
    s.includes("insufficientdeposit") ||
    s.includes("insufficient escrow") ||
    s.includes("error number: 6009") ||
    s.includes("6009") ||
    s.includes("0x1779")
  ) {
    return {
      title: "Not enough tokens",
      body:
        "For a buy, fund USDC at the venue’s quote mint. For a sell, you need wrapped SOL (WSOL), not native SOL. Try a smaller size or use the devnet faucet for that exact mint.",
    };
  }

  if (
    s.includes("accountnotinitialized") ||
    s.includes("error number: 3012") ||
    s.includes("3012") ||
    s.includes("0xbc4")
  ) {
    return {
      title: "Token account missing",
      body:
        "The SPL account this trade needs doesn’t exist yet. Post again — the app creates ATAs automatically — or check you’re on the right network and mint.",
    };
  }

  if (
    s.includes("user rejected") ||
    s.includes("rejected the request") ||
    s.includes("approval denied") ||
    s.includes("wallet signing request")
  ) {
    return {
      title: "Wallet cancelled",
      body: "You closed the wallet or declined the signature.",
    };
  }

  if (
    s.includes("insufficient funds") ||
    s.includes("insufficient lamports") ||
    s.includes("transfer: insufficient lamports")
  ) {
    return {
      title: "Not enough SOL for fees",
      body: "Add devnet SOL to this wallet for rent and transaction fees.",
    };
  }

  if (s.includes("simulation failed") && s.length > 400) {
    return {
      title: "Transaction didn’t simulate",
      body:
        "The chain rejected the transaction. Check token balances and mints, then try again.",
    };
  }

  const body = raw.length > 320 ? `${raw.slice(0, 317)}…` : raw;
  return {
    title: "Something went wrong",
    body: body || "Please try again.",
  };
}
