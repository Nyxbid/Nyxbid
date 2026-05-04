/**
 * Happy path: buy intent settles atomically with buy-side price-improvement
 * refund flowing to the taker.
 *
 * Scenario:
 *   - Taker buys 10 base at limit 50 (PRICE_SCALE-encoded).
 *     => Taker locks quote_notional(10, 50) = 500 quote.
 *   - One maker submits a sealed commitment for price 48.
 *   - After reveal_deadline, maker reveals 48 -> becomes winner.
 *   - After resolve_deadline, maker funds the opposite leg
 *     (10 base, since buy means maker delivers base).
 *   - Settle inside the settle window:
 *       leg 1: taker_vault -> maker_destination = 480 quote
 *              (executed price, not the 500 limit).
 *       leg 2: maker_vault -> taker_destination = 10 base.
 *       leg 3: taker_vault -> taker_refund_destination = 20 quote
 *              (the 500 - 480 price improvement).
 *   - Receipt is written, reputation counters bump correctly.
 */
import { describe, it, expect } from "bun:test";
import { BN } from "@anchor-lang/core";
import {
  bootstrap,
  fundedKeypair,
  createTestMint,
  createAta,
  mintToAta,
  tokenBalance,
  warpTo,
  nowTs,
  intentPda,
  escrowPda,
  takerVaultPda,
  makerVaultPda,
  quotePda,
  receiptPda,
  reputationPda,
  ata,
  commitmentHash,
  rand16,
  rand32,
  quoteNotional,
  Side,
  PRICE_SCALE,
  TOKEN_PROGRAM_ID,
  TestCtx,
} from "./helpers/setup";
import { SystemProgram, SYSVAR_RENT_PUBKEY } from "@solana/web3.js";

const BASE_DECIMALS = 9;
const QUOTE_DECIMALS = 6;

describe("happy path: buy intent settles + refunds price improvement", () => {
  it("end-to-end buy flow", async () => {
    const ctx: TestCtx = await bootstrap();

    // Anchor / bankrun's clock starts at 0; ratchet it to a sane value
    // so deadlines arithmetic doesn't underflow.
    await warpTo(ctx, 1_700_000_000);

    // ----- actors -----
    const taker = await fundedKeypair(ctx);
    const maker = await fundedKeypair(ctx);

    // ----- mints + funded ATAs -----
    const baseMint = await createTestMint(ctx, BASE_DECIMALS);
    const quoteMint = await createTestMint(ctx, QUOTE_DECIMALS);

    const takerQuoteAta = await createAta(ctx, quoteMint, taker);
    const takerBaseAta = await createAta(ctx, baseMint, taker);
    const makerBaseAta = await createAta(ctx, baseMint, maker);
    const makerQuoteAta = await createAta(ctx, quoteMint, maker);

    // Taker has 1000 quote (well above the 500 they need to lock).
    const takerQuoteInitial = 1_000_000_000n; // 1000 * 1e6
    await mintToAta(ctx, quoteMint, takerQuoteAta, takerQuoteInitial);
    // Maker has 100 base (10 they need plus headroom).
    const makerBaseInitial = 100_000_000_000n; // 100 * 1e9
    await mintToAta(ctx, baseMint, makerBaseAta, makerBaseInitial);

    // ----- intent params -----
    // Buy 10 base at limit 50. PRICE_SCALE = 1_000_000.
    // size in base lamports (9 decimals): 10 * 1e9 = 10_000_000_000
    // limit in PRICE_SCALE units: 50 * 1e6 = 50_000_000
    const size = new BN("10000000000");
    const limitPrice = new BN("50000000");
    const expectedLock = quoteNotional(size, limitPrice);
    expect(expectedLock.toString()).toBe("500000000000"); // 500 * 1e9 = 5e11
    // ^ Note: this is the *raw* notional with no decimal cancellation;
    // we don't pretend the math is decimal-balanced, just verify it.

    // Top up taker's quote ATA to cover the lock (more than the
    // 1e9 we minted above).
    if (BigInt(expectedLock.toString()) > takerQuoteInitial) {
      const extra = BigInt(expectedLock.toString()) - takerQuoteInitial;
      await mintToAta(ctx, quoteMint, takerQuoteAta, extra);
    }
    const takerQuoteBefore = await tokenBalance(ctx, takerQuoteAta);

    // Deadlines: live in absolute unix-seconds because that's what the
    // program reads from Clock::get().
    const t0 = await nowTs(ctx);
    const revealDeadline = new BN(t0 + 30);
    const resolveDeadline = new BN(t0 + 60);
    const settleDeadline = new BN(t0 + 90);

    // ----- commitment -----
    const revealedPrice = new BN("48000000"); // 48
    const revealedSize = size; // full size
    const revealNonce = rand32();
    const commitment = commitmentHash(revealedPrice, revealedSize, revealNonce);

    // ----- create_intent -----
    const intentNonce = rand16();
    const [intent] = intentPda(taker.publicKey, intentNonce);
    const [escrow] = escrowPda(intent);
    const [takerVault] = takerVaultPda(intent);

    await ctx.program.methods
      .createIntent({
        side: Side.Buy,
        size,
        limitPrice,
        revealDeadline,
        resolveDeadline,
        settleDeadline,
        commitmentRoot: Array.from(commitment),
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

    // Taker's quote ATA should have lost exactly expectedLock.
    const takerQuoteAfterCreate = await tokenBalance(ctx, takerQuoteAta);
    expect(
      (takerQuoteBefore - takerQuoteAfterCreate).toString()
    ).toBe(expectedLock.toString());
    expect((await tokenBalance(ctx, takerVault)).toString()).toBe(
      expectedLock.toString()
    );

    // ----- submit_quote -----
    const quoteNonce16 = rand16();
    const [quote] = quotePda(intent, maker.publicKey, quoteNonce16);
    const [reputation] = reputationPda(maker.publicKey);

    await ctx.program.methods
      .submitQuote({
        commitment: Array.from(commitment),
        nonce: Array.from(quoteNonce16),
      } as any)
      .accountsPartial({
        maker: maker.publicKey,
        intent,
        quote,
        reputation,
        systemProgram: SystemProgram.programId,
      } as any)
      .signers([maker])
      .rpc();

    const repAfterSubmit = await ctx.program.account.reputation.fetch(
      reputation
    );
    expect(repAfterSubmit.quotesSubmitted.toString()).toBe("1");

    // ----- warp into the reveal window -----
    await warpTo(ctx, t0 + 31); // past reveal_deadline (30), before resolve (60)

    // ----- reveal_quote -----
    await ctx.program.methods
      .revealQuote({
        revealedPrice,
        revealedSize,
        nonce: Array.from(revealNonce),
      } as any)
      .accountsPartial({
        maker: maker.publicKey,
        intent,
        quote,
      } as any)
      .signers([maker])
      .rpc();

    const intentAfterReveal = await ctx.program.account.intent.fetch(intent);
    expect(intentAfterReveal.winningQuote.toBase58()).toBe(quote.toBase58());
    expect(intentAfterReveal.winningPrice.toString()).toBe(
      revealedPrice.toString()
    );

    // ----- warp past resolve_deadline into the settle window -----
    await warpTo(ctx, t0 + 61); // past resolve_deadline (60), before settle (90)

    // ----- fund_maker_escrow -----
    // Buy: maker delivers base_mint, sized by revealed_size (10 * 1e9).
    const [makerVault] = makerVaultPda(intent);
    const makerFundAmount = revealedSize;

    await ctx.program.methods
      .fundMakerEscrow({ amount: makerFundAmount } as any)
      .accountsPartial({
        maker: maker.publicKey,
        intent,
        quote,
        escrow,
        makerLockMint: baseMint,
        makerSource: makerBaseAta,
        makerVault,
        reputation,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
        rent: SYSVAR_RENT_PUBKEY,
      } as any)
      .signers([maker])
      .rpc();

    expect((await tokenBalance(ctx, makerBaseAta)).toString()).toBe(
      (makerBaseInitial - BigInt(makerFundAmount.toString())).toString()
    );
    expect((await tokenBalance(ctx, makerVault)).toString()).toBe(
      makerFundAmount.toString()
    );

    const intentAfterFund = await ctx.program.account.intent.fetch(intent);
    // Resolved status enum index = 1.
    expect(intentAfterFund.status).toBe(1);
    const repAfterFund = await ctx.program.account.reputation.fetch(reputation);
    expect(repAfterFund.quotesWon.toString()).toBe("1");

    // ----- settle -----
    // Buy at fill 48 vs limit 50: the executed cost is 480 quote;
    // the 20 quote excess flows back to the taker via taker_refund_destination.
    const [receipt] = receiptPda(intent);
    const expectedTakerPaid = quoteNotional(revealedSize, revealedPrice);
    const expectedRefund = expectedLock.sub(expectedTakerPaid);
    expect(expectedTakerPaid.toString()).toBe("480000000000");
    expect(expectedRefund.toString()).toBe("20000000000");

    const takerBaseBefore = await tokenBalance(ctx, takerBaseAta);
    const makerQuoteBefore = await tokenBalance(ctx, makerQuoteAta);
    const takerQuoteRefundBefore = await tokenBalance(ctx, takerQuoteAta);

    await ctx.program.methods
      .settle()
      .accountsPartial({
        payer: taker.publicKey,
        intent,
        winningQuote: quote,
        escrow,
        takerVault,
        makerVault,
        makerDestination: makerQuoteAta, // maker receives quote (the leg taker locked)
        takerDestination: takerBaseAta, // taker receives base (the leg maker locked)
        takerRefundDestination: takerQuoteAta, // refund target == taker's quote ATA
        takerRentBeneficiary: taker.publicKey,
        makerRentBeneficiary: maker.publicKey,
        receipt,
        reputation,
        baseMint,
        quoteMint,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      } as any)
      .signers([taker])
      .rpc();

    // ----- assertions -----
    // Maker received exactly expectedTakerPaid (480 quote), not the locked 500.
    expect(
      ((await tokenBalance(ctx, makerQuoteAta)) - makerQuoteBefore).toString()
    ).toBe(expectedTakerPaid.toString());

    // Taker received the full revealed_size of base.
    expect(
      ((await tokenBalance(ctx, takerBaseAta)) - takerBaseBefore).toString()
    ).toBe(revealedSize.toString());

    // Taker got the price-improvement refund.
    expect(
      (
        (await tokenBalance(ctx, takerQuoteAta)) - takerQuoteRefundBefore
      ).toString()
    ).toBe(expectedRefund.toString());

    // Vaults are closed (account no longer exists).
    expect(await ctx.banksClient.getAccount(takerVault)).toBeNull();
    expect(await ctx.banksClient.getAccount(makerVault)).toBeNull();
    // Escrow PDA is also closed (rent reclaim from review round 3).
    expect(await ctx.banksClient.getAccount(escrow)).toBeNull();

    // Receipt is permanent and matches the executed values.
    const receiptAcct = await ctx.program.account.receipt.fetch(receipt);
    expect(receiptAcct.filledPrice.toString()).toBe(revealedPrice.toString());
    expect(receiptAcct.filledSize.toString()).toBe(revealedSize.toString());
    expect(receiptAcct.taker.toBase58()).toBe(taker.publicKey.toBase58());
    expect(receiptAcct.maker.toBase58()).toBe(maker.publicKey.toBase58());

    // Final intent + reputation state.
    const intentFinal = await ctx.program.account.intent.fetch(intent);
    // Settled status enum index = 2.
    expect(intentFinal.status).toBe(2);

    const repFinal = await ctx.program.account.reputation.fetch(reputation);
    expect(repFinal.settledCount.toString()).toBe("1");
    expect(repFinal.failedReveals.toString()).toBe("0");
  });
});
