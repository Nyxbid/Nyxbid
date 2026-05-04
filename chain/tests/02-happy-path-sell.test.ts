/**
 * Happy path: sell intent settles atomically. No price-improvement refund
 * on the sell side - the taker locked `size` of base regardless of price,
 * so there's nothing to refund. taker_refund_destination is omitted.
 *
 * Scenario:
 *   - Taker sells 10 base at limit 45 (price floor).
 *     => Taker locks 10 base.
 *   - One maker reveals 47 (above limit, beats it).
 *   - Maker delivers quote_notional(10, 47) = 470 quote.
 *   - Settle:
 *       leg 1: taker_vault -> maker_destination = 10 base.
 *       leg 2: maker_vault -> taker_destination = 470 quote.
 *       no refund leg.
 *   - Receipt + reputation as expected.
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
  warpTo,
  nowTs,
  intentPda,
  escrowPda,
  takerVaultPda,
  makerVaultPda,
  quotePda,
  receiptPda,
  reputationPda,
  commitmentHash,
  rand16,
  rand32,
  quoteNotional,
  Side,
  TOKEN_PROGRAM_ID,
  TestCtx,
} from "./helpers/setup";
import { SystemProgram, SYSVAR_RENT_PUBKEY } from "@solana/web3.js";

const BASE_DECIMALS = 9;
const QUOTE_DECIMALS = 6;

describe("happy path: sell intent settles, no refund", () => {
  it("end-to-end sell flow", async () => {
    const ctx: TestCtx = await bootstrap();
    await warpTo(ctx, 1_700_000_000);

    const taker = await fundedKeypair(ctx);
    const maker = await fundedKeypair(ctx);

    const baseMint = await createTestMint(ctx, BASE_DECIMALS);
    const quoteMint = await createTestMint(ctx, QUOTE_DECIMALS);

    const takerBaseAta = await createAta(ctx, baseMint, taker);
    const takerQuoteAta = await createAta(ctx, quoteMint, taker);
    const makerBaseAta = await createAta(ctx, baseMint, maker);
    const makerQuoteAta = await createAta(ctx, quoteMint, maker);

    // Taker has 100 base (10 to lock + headroom).
    const takerBaseInitial = 100_000_000_000n; // 100 * 1e9
    await mintToAta(ctx, baseMint, takerBaseAta, takerBaseInitial);

    // ----- intent params -----
    const size = new BN("10000000000"); // 10 * 1e9
    const limitPrice = new BN("45000000"); // 45
    const revealedPrice = new BN("47000000"); // 47 (beats the 45 floor)
    const revealedSize = size;
    const expectedMakerLeg = quoteNotional(revealedSize, revealedPrice);
    expect(expectedMakerLeg.toString()).toBe("470000000000"); // 470 * 1e9

    // Maker needs that much quote on hand. Mint enough.
    const makerQuoteInitial = BigInt(expectedMakerLeg.toString()) + 100n; // tiny headroom
    await mintToAta(ctx, quoteMint, makerQuoteAta, makerQuoteInitial);

    const t0 = await nowTs(ctx);
    const revealDeadline = new BN(t0 + 30);
    const resolveDeadline = new BN(t0 + 60);
    const settleDeadline = new BN(t0 + 90);

    const revealNonce = rand32();
    const commitment = commitmentHash(revealedPrice, revealedSize, revealNonce);

    const intentNonce = rand16();
    const [intent] = intentPda(taker.publicKey, intentNonce);
    const [escrow] = escrowPda(intent);
    const [takerVault] = takerVaultPda(intent);

    // ----- create_intent (sell -> taker locks base) -----
    const takerBaseBeforeCreate = await tokenBalance(ctx, takerBaseAta);
    await ctx.program.methods
      .createIntent({
        side: Side.Sell,
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
        takerSource: takerBaseAta,
        intent,
        escrow,
        takerVault,
        takerLockMint: baseMint, // sell -> lock base
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
        rent: SYSVAR_RENT_PUBKEY,
      } as any)
      .signers([taker])
      .rpc();

    expect(
      (
        takerBaseBeforeCreate - (await tokenBalance(ctx, takerBaseAta))
      ).toString()
    ).toBe(size.toString());
    expect((await tokenBalance(ctx, takerVault)).toString()).toBe(
      size.toString()
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

    // ----- reveal_quote -----
    await warpTo(ctx, t0 + 31);

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

    // ----- fund_maker_escrow (sell -> maker delivers quote sized by notional) -----
    await warpTo(ctx, t0 + 61);
    const [makerVault] = makerVaultPda(intent);

    await ctx.program.methods
      .fundMakerEscrow({ amount: expectedMakerLeg } as any)
      .accountsPartial({
        maker: maker.publicKey,
        intent,
        quote,
        escrow,
        makerLockMint: quoteMint, // sell -> maker locks quote
        makerSource: makerQuoteAta,
        makerVault,
        reputation,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
        rent: SYSVAR_RENT_PUBKEY,
      } as any)
      .signers([maker])
      .rpc();

    expect((await tokenBalance(ctx, makerVault)).toString()).toBe(
      expectedMakerLeg.toString()
    );

    // ----- settle -----
    const [receipt] = receiptPda(intent);
    const takerQuoteBefore = await tokenBalance(ctx, takerQuoteAta);
    const makerBaseBefore = await tokenBalance(ctx, makerBaseAta);

    await ctx.program.methods
      .settle()
      .accountsPartial({
        payer: taker.publicKey,
        intent,
        winningQuote: quote,
        escrow,
        takerVault,
        makerVault,
        // sell side: maker receives base (the leg taker locked),
        //           taker receives quote (the leg maker locked).
        makerDestination: makerBaseAta,
        takerDestination: takerQuoteAta,
        // No refund destination on the sell side.
        takerRefundDestination: null,
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

    // Maker received the full taker_amount in base.
    expect(
      ((await tokenBalance(ctx, makerBaseAta)) - makerBaseBefore).toString()
    ).toBe(size.toString());

    // Taker received the maker leg in quote.
    expect(
      ((await tokenBalance(ctx, takerQuoteAta)) - takerQuoteBefore).toString()
    ).toBe(expectedMakerLeg.toString());

    // Vaults + escrow closed.
    expect(await ctx.banksClient.getAccount(takerVault)).toBeNull();
    expect(await ctx.banksClient.getAccount(makerVault)).toBeNull();
    expect(await ctx.banksClient.getAccount(escrow)).toBeNull();

    // Receipt fields.
    const receiptAcct = await ctx.program.account.receipt.fetch(receipt);
    expect(receiptAcct.filledPrice.toString()).toBe(revealedPrice.toString());
    expect(receiptAcct.filledSize.toString()).toBe(revealedSize.toString());

    // Final state.
    const intentFinal = await ctx.program.account.intent.fetch(intent);
    expect(intentFinal.status).toBe(2); // Settled
    const repFinal = await ctx.program.account.reputation.fetch(reputation);
    expect(repFinal.settledCount.toString()).toBe("1");
    expect(repFinal.failedReveals.toString()).toBe("0");
  });
});
