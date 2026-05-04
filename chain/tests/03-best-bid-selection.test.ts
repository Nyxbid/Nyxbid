/**
 * Best-bid replacement test.
 *
 * Three makers each submit a sealed commitment, then reveal in a
 * deterministic order. The program should retain only the best valid
 * revealed quote in intent.winning_quote, with strict-improvement
 * semantics (ties keep the earlier winner so a later maker can't grief
 * by reposting the same price).
 *
 * Buy side:  lower price wins.
 * Sell side: higher price wins.
 *
 * Two scenarios in this file.
 */
import { describe, it, expect } from "bun:test";
import { BN } from "@coral-xyz/anchor";
import {
  bootstrap,
  fundedKeypair,
  createTestMint,
  createAta,
  mintToAta,
  warpTo,
  nowTs,
  intentPda,
  escrowPda,
  takerVaultPda,
  quotePda,
  reputationPda,
  commitmentHash,
  rand16,
  rand32,
  quoteNotional,
  Side,
  TOKEN_PROGRAM_ID,
  TestCtx,
} from "./helpers/setup";
import {
  PublicKey,
  SystemProgram,
  SYSVAR_RENT_PUBKEY,
  Keypair,
} from "@solana/web3.js";

const BASE_DECIMALS = 9;
const QUOTE_DECIMALS = 6;

interface Maker {
  kp: Keypair;
  quotePk: PublicKey;
  quoteNonce16: Buffer;
  revealNonce: Buffer;
  price: BN;
  commitment: Buffer;
}

async function setupMaker(
  ctx: TestCtx,
  intent: PublicKey,
  size: BN,
  price: BN
): Promise<Maker> {
  const kp = await fundedKeypair(ctx);
  const quoteNonce16 = rand16();
  const revealNonce = rand32();
  const commitment = commitmentHash(price, size, revealNonce);
  const [quotePk] = quotePda(intent, kp.publicKey, quoteNonce16);
  return { kp, quotePk, quoteNonce16, revealNonce, price, commitment };
}

async function submit(ctx: TestCtx, intent: PublicKey, m: Maker) {
  const [reputation] = reputationPda(m.kp.publicKey);
  await ctx.program.methods
    .submitQuote({
      commitment: Array.from(m.commitment),
      nonce: Array.from(m.quoteNonce16),
    } as any)
    .accountsPartial({
      maker: m.kp.publicKey,
      intent,
      quote: m.quotePk,
      reputation,
      systemProgram: SystemProgram.programId,
    } as any)
    .signers([m.kp])
    .rpc();
}

async function reveal(ctx: TestCtx, intent: PublicKey, m: Maker, size: BN) {
  await ctx.program.methods
    .revealQuote({
      revealedPrice: m.price,
      revealedSize: size,
      nonce: Array.from(m.revealNonce),
    } as any)
    .accountsPartial({
      maker: m.kp.publicKey,
      intent,
      quote: m.quotePk,
    } as any)
    .signers([m.kp])
    .rpc();
}

describe("best-bid replacement", () => {
  it("buy: lower price replaces higher; ties keep the earlier winner", async () => {
    const ctx: TestCtx = await bootstrap();
    await warpTo(ctx, 1_700_000_000);

    const taker = await fundedKeypair(ctx);
    const baseMint = await createTestMint(ctx, BASE_DECIMALS);
    const quoteMint = await createTestMint(ctx, QUOTE_DECIMALS);
    const takerQuoteAta = await createAta(ctx, quoteMint, taker);

    const size = new BN("10000000000");
    const limitPrice = new BN("50000000"); // 50
    const lockAmount = quoteNotional(size, limitPrice);
    await mintToAta(
      ctx,
      quoteMint,
      takerQuoteAta,
      BigInt(lockAmount.toString())
    );

    const t0 = await nowTs(ctx);
    const intentNonce = rand16();
    const [intent] = intentPda(taker.publicKey, intentNonce);
    const [escrow] = escrowPda(intent);
    const [takerVault] = takerVaultPda(intent);

    // Use a single dummy commitment_root on the intent itself - it's not
    // checked against quotes anyway. The per-quote commitments are what
    // matter.
    const dummyRoot = commitmentHash(new BN(0), new BN(0), Buffer.alloc(32, 0));

    await ctx.program.methods
      .createIntent({
        side: Side.Buy,
        size,
        limitPrice,
        revealDeadline: new BN(t0 + 30),
        resolveDeadline: new BN(t0 + 60),
        settleDeadline: new BN(t0 + 90),
        commitmentRoot: Array.from(dummyRoot),
        nonce: Array.from(intentNonce),
      } as any)
      .accountsPartial({
        taker: taker.publicKey,
        baseMint,
        quoteMint,
        takerSource: takerQuoteAta,
        intent,
        escrow,
        takerVault,
        takerLockMint: quoteMint,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
        rent: SYSVAR_RENT_PUBKEY,
      } as any)
      .signers([taker])
      .rpc();

    // Three makers, prices 49, 47, 47. Order of reveal: 49 first
    // (becomes winner), then 47 from maker B (replaces), then 47 from
    // maker C (tie - should NOT replace).
    const mA = await setupMaker(ctx, intent, size, new BN("49000000"));
    const mB = await setupMaker(ctx, intent, size, new BN("47000000"));
    const mC = await setupMaker(ctx, intent, size, new BN("47000000"));

    await submit(ctx, intent, mA);
    await submit(ctx, intent, mB);
    await submit(ctx, intent, mC);

    await warpTo(ctx, t0 + 31);

    // Reveal A first => winner.
    await reveal(ctx, intent, mA, size);
    let st = await ctx.program.account.intent.fetch(intent);
    expect(st.winningQuote.toBase58()).toBe(mA.quotePk.toBase58());
    expect(st.winningPrice.toString()).toBe("49000000");

    // Reveal B (47 < 49) => replaces.
    await reveal(ctx, intent, mB, size);
    st = await ctx.program.account.intent.fetch(intent);
    expect(st.winningQuote.toBase58()).toBe(mB.quotePk.toBase58());
    expect(st.winningPrice.toString()).toBe("47000000");

    // Reveal C (47 == 47) => tie, no replacement, B keeps the win.
    await reveal(ctx, intent, mC, size);
    st = await ctx.program.account.intent.fetch(intent);
    expect(st.winningQuote.toBase58()).toBe(mB.quotePk.toBase58());
    expect(st.winningPrice.toString()).toBe("47000000");

    // C's quote is still recorded as revealed even though it didn't win.
    const cQuote = await ctx.program.account.quote.fetch(mC.quotePk);
    expect(cQuote.revealed).toBe(true);
    expect(cQuote.revealedPrice.toString()).toBe("47000000");
  });

  it("sell: higher price replaces lower; ties keep the earlier winner", async () => {
    const ctx: TestCtx = await bootstrap();
    await warpTo(ctx, 1_700_000_000);

    const taker = await fundedKeypair(ctx);
    const baseMint = await createTestMint(ctx, BASE_DECIMALS);
    const quoteMint = await createTestMint(ctx, QUOTE_DECIMALS);
    const takerBaseAta = await createAta(ctx, baseMint, taker);

    const size = new BN("10000000000");
    const limitPrice = new BN("45000000"); // 45 floor
    await mintToAta(ctx, baseMint, takerBaseAta, BigInt(size.toString()));

    const t0 = await nowTs(ctx);
    const intentNonce = rand16();
    const [intent] = intentPda(taker.publicKey, intentNonce);
    const [escrow] = escrowPda(intent);
    const [takerVault] = takerVaultPda(intent);

    const dummyRoot = commitmentHash(new BN(0), new BN(0), Buffer.alloc(32, 0));

    await ctx.program.methods
      .createIntent({
        side: Side.Sell,
        size,
        limitPrice,
        revealDeadline: new BN(t0 + 30),
        resolveDeadline: new BN(t0 + 60),
        settleDeadline: new BN(t0 + 90),
        commitmentRoot: Array.from(dummyRoot),
        nonce: Array.from(intentNonce),
      } as any)
      .accountsPartial({
        taker: taker.publicKey,
        baseMint,
        quoteMint,
        takerSource: takerBaseAta,
        intent,
        escrow,
        takerVault,
        takerLockMint: baseMint,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
        rent: SYSVAR_RENT_PUBKEY,
      } as any)
      .signers([taker])
      .rpc();

    // Prices 46 (worst valid), 48 (better), 48 (tie).
    const mA = await setupMaker(ctx, intent, size, new BN("46000000"));
    const mB = await setupMaker(ctx, intent, size, new BN("48000000"));
    const mC = await setupMaker(ctx, intent, size, new BN("48000000"));

    await submit(ctx, intent, mA);
    await submit(ctx, intent, mB);
    await submit(ctx, intent, mC);

    await warpTo(ctx, t0 + 31);

    await reveal(ctx, intent, mA, size);
    let st = await ctx.program.account.intent.fetch(intent);
    expect(st.winningQuote.toBase58()).toBe(mA.quotePk.toBase58());
    expect(st.winningPrice.toString()).toBe("46000000");

    await reveal(ctx, intent, mB, size); // 48 > 46 => replaces
    st = await ctx.program.account.intent.fetch(intent);
    expect(st.winningQuote.toBase58()).toBe(mB.quotePk.toBase58());
    expect(st.winningPrice.toString()).toBe("48000000");

    await reveal(ctx, intent, mC, size); // 48 == 48 => no replacement
    st = await ctx.program.account.intent.fetch(intent);
    expect(st.winningQuote.toBase58()).toBe(mB.quotePk.toBase58());
    expect(st.winningPrice.toString()).toBe("48000000");
  });
});
