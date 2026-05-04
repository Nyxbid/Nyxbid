/**
 * Smoke test: bankrun spins up, the program loads, basic mint/ATA setup
 * works through the bankrun connection shim.
 */
import { describe, it, expect } from "bun:test";
import { BN } from "@coral-xyz/anchor";
import {
  bootstrap,
  fundedKeypair,
  createTestMint,
  createAta,
  mintToAta,
  tokenBalance,
  PROGRAM_ID,
  quoteNotional,
  commitmentHash,
  intentPda,
  escrowPda,
  takerVaultPda,
  reputationPda,
} from "./helpers/setup";

describe("smoke", () => {
  it("loads the program", async () => {
    const ctx = await bootstrap();
    const acct = await ctx.banksClient.getAccount(PROGRAM_ID);
    expect(acct).not.toBeNull();
    expect(acct!.executable).toBe(true);
  });

  it("mints + ATAs work via bankrun helpers", async () => {
    const ctx = await bootstrap();
    const owner = await fundedKeypair(ctx);
    const mint = await createTestMint(ctx, 6);
    const aOwner = await createAta(ctx, mint, owner);
    await mintToAta(ctx, mint, aOwner, 1_000_000n);
    const bal = await tokenBalance(ctx, aOwner);
    expect(bal.toString()).toBe("1000000");
  });

  it("PDA derivations are stable + non-colliding", async () => {
    const ctx = await bootstrap();
    const taker = await fundedKeypair(ctx);
    const nonce = Buffer.from(Array.from({ length: 16 }, () => 7));
    const [intent] = intentPda(taker.publicKey, nonce);
    const [escrow] = escrowPda(intent);
    const [vault] = takerVaultPda(intent);
    const [rep] = reputationPda(taker.publicKey);
    const set = new Set([intent, escrow, vault, rep].map((p) => p.toBase58()));
    expect(set.size).toBe(4);
  });

  it("commitment hash and quote_notional helpers work", () => {
    const price = new BN("48000000"); // 48 * 1e6
    const size = new BN("10000000000"); // 10 * 1e9
    const nonce = Buffer.alloc(32, 3);
    const c = commitmentHash(price, size, nonce);
    expect(c.length).toBe(32);

    // notional(10 * 1e9, 48 * 1e6) = 480 * 1e9 with PRICE_SCALE = 1e6
    const n = quoteNotional(size, price);
    expect(n.toString()).toBe("480000000000");
  });
});
