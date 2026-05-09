// Quote commitment hash matching the on-chain definition in
// `crates/nyxbid-program/src/state.rs::PRICE_SCALE` and the program's
// `solana_sha256_hasher::hashv(&[price_le, size_le, nonce])`.
//
// We use the platform-built-in Web Crypto SubtleCrypto API. It's
// available everywhere Next.js runs (browsers and Node ≥ 16), so
// the bundle stays free of a hash dependency we'd otherwise pull in.
// SHA-256 is a tiny call — async-only is the only friction, and the
// callers are already async (they wrap `useNyxbidTx` flows).

const SHA256_LEN = 32;

/** Pack a u64 as little-endian 8 bytes. */
export function u64LeBytes(n: bigint | number): Uint8Array {
  const buf = new Uint8Array(8);
  const dv = new DataView(buf.buffer);
  dv.setBigUint64(0, typeof n === "bigint" ? n : BigInt(n), true);
  return buf;
}

/** Generate `len` cryptographically random bytes. */
export function randomBytes(len: number): Uint8Array {
  const out = new Uint8Array(len);
  crypto.getRandomValues(out);
  return out;
}

async function sha256(buf: Uint8Array): Promise<Uint8Array> {
  // `crypto.subtle.digest` accepts a BufferSource; copy into a fresh
  // ArrayBuffer to satisfy strict TS lib types (Uint8Array can be
  // backed by SharedArrayBuffer in newer typings).
  const view = new Uint8Array(buf.byteLength);
  view.set(buf);
  const digest = await crypto.subtle.digest("SHA-256", view.buffer);
  return new Uint8Array(digest);
}

/**
 * sha256(price_le_u64 || size_le_u64 || nonce32) — the exact byte
 * layout the program rebuilds from the on-chain reveal arguments.
 */
export async function commitmentBytes(
  price: bigint | number,
  size: bigint | number,
  nonce32: Uint8Array,
): Promise<Uint8Array> {
  if (nonce32.length !== 32) {
    throw new Error(`commitment nonce must be 32 bytes, got ${nonce32.length}`);
  }
  const buf = new Uint8Array(8 + 8 + 32);
  buf.set(u64LeBytes(price), 0);
  buf.set(u64LeBytes(size), 8);
  buf.set(nonce32, 16);
  const out = await sha256(buf);
  if (out.length !== SHA256_LEN) {
    throw new Error("sha256 produced unexpected length");
  }
  return out;
}

/** Same as `commitmentBytes` but returned as lowercase hex (no 0x). */
export async function commitmentHex(
  price: bigint | number,
  size: bigint | number,
  nonce32: Uint8Array,
): Promise<string> {
  const bytes = await commitmentBytes(price, size, nonce32);
  return bytesToHex(bytes);
}

export function bytesToHex(bytes: Uint8Array): string {
  let hex = "";
  for (const b of bytes) hex += b.toString(16).padStart(2, "0");
  return hex;
}

export function hexToBytes(hex: string): Uint8Array {
  const clean = hex.startsWith("0x") ? hex.slice(2) : hex;
  if (clean.length % 2 !== 0) throw new Error("hex must be even length");
  const out = new Uint8Array(clean.length / 2);
  for (let i = 0; i < out.length; i++) {
    out[i] = parseInt(clean.substring(i * 2, i * 2 + 2), 16);
  }
  return out;
}
