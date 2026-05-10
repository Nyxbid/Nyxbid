/**
 * Map raw Solana / wallet errors into short toast copy. Simulation
 * logs are huge; we match on stable substrings and Nyxbid error names.
 */

export interface FriendlyTxError {
  title: string;
  body: string;
}

/** User closed the wallet or tapped Reject — not an application error. */
export const WALLET_SIGN_CANCELLED: FriendlyTxError = {
  title: "Signing cancelled",
  body: "You closed the wallet or declined this transaction. Nothing was submitted.",
};

/**
 * True when the connected wallet refused to sign (no chain submission).
 * Phantom / Backpack / Solflare all surface slightly different messages —
 * match names, substrings, and EIP-1193 4001.
 */
export function isWalletUserRejection(err: unknown): boolean {
  if (!err || typeof err !== "object") return false;
  if ("code" in err) {
    const code = (err as { code?: number }).code;
    if (code === 4001) return true;
  }
  const name =
    err instanceof Error && typeof err.name === "string" ? err.name : "";
  const raw = rawMessage(err);
  const s = raw.toLowerCase();
  const looksRejected =
    s.includes("user rejected") ||
    s.includes("rejected the request") ||
    s.includes("approval denied") ||
    s.includes("request rejected") ||
    s.includes("user denied") ||
    s.includes("cancelled") ||
    s.includes("canceled");
  if (looksRejected) return true;
  // Wallet adapter uses these class names almost exclusively for "user
  // closed the prompt" — but we still require a soft signal in the text
  // so we do not swallow unrelated signing failures that reuse the name.
  if (
    name === "WalletSignTransactionError" ||
    name === "WalletSendTransactionError"
  ) {
    return (
      s.includes("reject") ||
      s.includes("denied") ||
      s.includes("cancel")
    );
  }
  return false;
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
      title: "Not enough balance",
      body:
        "Buys lock USDC and sells lock SOL. Add more of the asset you’re posting (or shrink the size) and try again.",
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
    s.includes("wallet signing request") ||
    s.includes("request rejected") ||
    s.includes("user denied")
  ) {
    return WALLET_SIGN_CANCELLED;
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
