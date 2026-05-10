// Display helpers. Keep all unit math here so pages stay declarative.

export const PRICE_SCALE = 1_000_000;

/**
 * Mint → decimals lookup. Until /api/markets exposes per-mint
 * decimals (and the indexer hydrates them from the SPL Mint account),
 * the wallet/UI layer needs *some* table to scale `size` and
 * `price` correctly. SOL/USDC is the only live market so the table
 * is one row. Add new entries here when adding markets.
 */
const MINT_DECIMALS: Record<string, number> = {
  // WSOL
  So11111111111111111111111111111111111111112: 9,
  // Devnet USDC faucet mint
  "4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU": 6,
  // Mainnet USDC
  EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v: 6,
};

export function decimalsFor(mint: string): number {
  return MINT_DECIMALS[mint] ?? 6;
}

/** USDC has 6 decimals on devnet/mainnet. SOL has 9. Default to 6. */
export function formatTokenAmount(
  minor: number | bigint,
  decimals: number = 6,
  fractionDigits: number = 4,
): string {
  const n = typeof minor === "bigint" ? Number(minor) : minor;
  const v = n / Math.pow(10, decimals);
  return v.toLocaleString(undefined, {
    minimumFractionDigits: 0,
    maximumFractionDigits: fractionDigits,
  });
}

/** Format USDC minor-units (6 decimals) as a $ string. */
export function formatUsdc(minor: number | bigint): string {
  const n = typeof minor === "bigint" ? Number(minor) : minor;
  const dollars = n / 1_000_000;
  return dollars.toLocaleString("en-US", {
    style: "currency",
    currency: "USD",
    minimumFractionDigits: 2,
    maximumFractionDigits: 2,
  });
}

/**
 * Convert PRICE_SCALE-encoded price to a human "quote per base" number.
 *
 * On-chain `limit_price` and `revealed_price` are stored as
 * "quote-minor per base-minor × PRICE_SCALE", so to get the human
 * "quote per base" we have to undo *both* the PRICE_SCALE and the
 * decimals shift between base and quote.
 *
 * Example: SOL/USDC, base=9, quote=6, scaled=100_000 → 100 USDC/SOL.
 */
export function formatPrice(
  scaled: number | bigint,
  baseMint: string,
  quoteMint: string,
  fractionDigits = 4,
): string {
  const n = typeof scaled === "bigint" ? Number(scaled) : scaled;
  const shift = decimalsFor(quoteMint) - decimalsFor(baseMint);
  const human = n / PRICE_SCALE / Math.pow(10, shift);
  return human.toLocaleString(undefined, {
    minimumFractionDigits: 0,
    maximumFractionDigits: fractionDigits,
  });
}

/**
 * Inverse: human price (e.g. "100" USDC per SOL) → on-chain
 * `limit_price` integer.
 *
 * The on-chain contract is:
 *   quote_amount_minor = size_minor × price_scaled / PRICE_SCALE
 * which means `price_scaled` is "quote-minor per base-minor", times
 * PRICE_SCALE. So we have to fold the decimals shift in here, or the
 * lock amount blows up by 10^(base-quote) and the program rejects
 * with InsufficientDeposit.
 *
 * Example: 100 USDC per 1 SOL, base=9, quote=6:
 *   price_scaled = 100 × 10^6 × 10^(6-9) = 100 × 1000 = 100_000
 *   lock = 1e9 × 100_000 / 1e6 = 1e8 = 100 USDC minor ✓
 */
export function priceToScaled(
  human: number,
  baseMintOrDecimals: string | number,
  quoteMintOrDecimals: string | number,
): number {
  const base =
    typeof baseMintOrDecimals === "string"
      ? decimalsFor(baseMintOrDecimals)
      : baseMintOrDecimals;
  const quote =
    typeof quoteMintOrDecimals === "string"
      ? decimalsFor(quoteMintOrDecimals)
      : quoteMintOrDecimals;
  const shift = quote - base;
  return Math.round(human * PRICE_SCALE * Math.pow(10, shift));
}

/** "8aF...kK7" rendering for a long base58 pubkey. */
export function shortPk(pk: string, head = 4, tail = 4): string {
  if (pk.length <= head + tail + 3) return pk;
  return `${pk.slice(0, head)}…${pk.slice(-tail)}`;
}

/** "3 min ago" relative time label. */
export function timeAgo(iso: string): string {
  const diff = Date.now() - new Date(iso).getTime();
  const mins = Math.floor(diff / 60_000);
  if (mins < 1) return "just now";
  if (mins < 60) return `${mins}m ago`;
  const hrs = Math.floor(mins / 60);
  if (hrs < 24) return `${hrs}h ago`;
  return `${Math.floor(hrs / 24)}d ago`;
}

/** "in 42s" / "12m" style countdown to an ISO future deadline. */
export function timeUntil(iso: string, nowMs: number = Date.now()): string {
  const diff = new Date(iso).getTime() - nowMs;
  if (diff <= 0) return "expired";
  const secs = Math.floor(diff / 1000);
  if (secs < 60) return `${secs}s`;
  const mins = Math.floor(secs / 60);
  if (mins < 60) return `${mins}m ${secs % 60}s`;
  const hrs = Math.floor(mins / 60);
  return `${hrs}h ${mins % 60}m`;
}

/**
 * Cluster the explorer should point at. Resolved once at module
 * load from `NEXT_PUBLIC_SOLANA_CLUSTER` so both server-side and
 * client-side renders agree, and so a deploy that flips to mainnet
 * doesn't have to update every call site.
 *
 * Valid values per the Solana Explorer URL contract: `devnet`,
 * `testnet`, `mainnet-beta`, `custom`. Default `devnet` because the
 * dev stack runs there and we'd rather link to a working explorer
 * than a dead mainnet page when the env var is absent.
 */
const EXPLORER_CLUSTER =
  process.env.NEXT_PUBLIC_SOLANA_CLUSTER ?? "devnet";

export function explorerTxUrl(sig: string, cluster = EXPLORER_CLUSTER): string {
  return `https://explorer.solana.com/tx/${sig}?cluster=${cluster}`;
}

export function explorerAccountUrl(
  pk: string,
  cluster = EXPLORER_CLUSTER,
): string {
  return `https://explorer.solana.com/address/${pk}?cluster=${cluster}`;
}
