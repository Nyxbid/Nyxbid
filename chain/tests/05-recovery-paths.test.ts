/**
 * Recovery-path coverage. Validates the fixes from review rounds 1-3:
 *
 *   - expire_no_maker closes the P0 lockup (no maker => taker refund).
 *   - expire_no_maker with optional winner accounts applies the
 *     failed_reveals penalty (P1 from round 3) for the
 *     reveal-but-don't-fund grief.
 *   - expire_no_maker rejects passing accounts when no winner exists.
 *   - expire_no_maker rejects omitting accounts when a winner exists.
 *   - expire_with_maker (winner funded but never settled) refunds both
 *     legs and bumps failed_reveals.
 */
import { describe, it, expect } from "bun:test";
import { BN } from "@anchor-lang/core";
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
  rand16,
  commitmentHash,
  quoteNotional,
  tokenBalance,
  Side,
  TOKEN_PROGRAM_ID,
} from "./helpers/setup";
import { Lifecycle, expectAnchorError } from "./helpers/lifecycle";
import { SystemProgram, SYSVAR_RENT_PUBKEY } from "@solana/web3.js";

describe("recovery: expire_no_maker", () => {
  it("empty market (no winner) -> taker refund, no reputation change", async () => {
    const lc = await Lifecycle.create({ side: Side.Buy });
    // No quotes submitted, no reveals. Just wait out settle_deadline.
    await lc.warpPastSettleDeadline();

    const takerQuoteBefore = await tokenBalance(lc.ctx, lc.takerQuoteAta);
    await lc.expireNoMaker(); // optional accounts default to null

    // Taker got their lock back.
    expect(
      ((await tokenBalance(lc.ctx, lc.takerQuoteAta)) - takerQuoteBefore).toString()
    ).toBe(lc.expectedTakerLock.toString());

    // Vaults + escrow closed.
    expect(await lc.ctx.banksClient.getAccount(lc.takerVault)).toBeNull();
    expect(await lc.ctx.banksClient.getAccount(lc.escrow)).toBeNull();

    // Status flipped to Expired.
    const intentFinal = await lc.ctx.program.account.intent.fetch(lc.intent);
    expect(intentFinal.status).toBe(4); // Expired
  });

  it("revealed-but-not-funded winner -> taker refund + failed_reveals++", async () => {
    const lc = await Lifecycle.create({ side: Side.Buy });
    await lc.submitQuote();
    await lc.warpToReveal();
    await lc.revealQuote(); // primary maker becomes winning_quote

    // Skip fund_maker_escrow. Wait out settle_deadline.
    await lc.warpPastSettleDeadline();

    const takerQuoteBefore = await tokenBalance(lc.ctx, lc.takerQuoteAta);
    await lc.expireNoMaker({ withWinner: true });

    // Taker got the full lock back; maker was never funded so they
    // don't get any leg.
    expect(
      ((await tokenBalance(lc.ctx, lc.takerQuoteAta)) - takerQuoteBefore).toString()
    ).toBe(lc.expectedTakerLock.toString());

    // Reputation got the failed_reveal penalty.
    const rep = await lc.ctx.program.account.reputation.fetch(lc.reputation);
    expect(rep.failedReveals.toString()).toBe("1");
    expect(rep.quotesSubmitted.toString()).toBe("1");
    expect(rep.quotesWon.toString()).toBe("0"); // never funded => never won
    expect(rep.settledCount.toString()).toBe("0");

    // Status Expired.
    const intentFinal = await lc.ctx.program.account.intent.fetch(lc.intent);
    expect(intentFinal.status).toBe(4);
  });

  it("winner exists but optional accounts omitted -> MissingWinnerAccounts", async () => {
    const lc = await Lifecycle.create({ side: Side.Buy });
    await lc.submitQuote();
    await lc.warpToReveal();
    await lc.revealQuote(); // sets intent.winning_quote
    await lc.warpPastSettleDeadline();

    // Default `expireNoMaker()` passes null for the optional accounts.
    await expectAnchorError(
      () => lc.expireNoMaker(),
      "MissingWinnerAccounts"
    );
  });

  it("no winner but optional accounts passed -> UnexpectedWinnerAccounts", async () => {
    const lc = await Lifecycle.create({ side: Side.Buy });
    // A maker submits a quote but never reveals - so a real Quote PDA
    // exists but intent.winning_quote stays default. This isolates the
    // UnexpectedWinnerAccounts guard from Anchor's account-existence
    // checks.
    await lc.submitQuote();
    await lc.warpPastSettleDeadline();

    // Pass the existing (un-revealed) quote + its reputation. The
    // handler should reject with UnexpectedWinnerAccounts because
    // intent.winning_quote == default.
    await expectAnchorError(
      () => lc.expireNoMaker({ withWinner: true }),
      "UnexpectedWinnerAccounts"
    );
  });
});

describe("recovery: expire_with_maker", () => {
  it("winner funded but never settled -> both legs refund + failed_reveals++", async () => {
    const lc = await Lifecycle.create({ side: Side.Buy });
    await lc.submitQuote();
    await lc.warpToReveal();
    await lc.revealQuote();
    await lc.warpToSettle();
    await lc.fundMakerEscrow(); // status -> Resolved, maker locked
    // Skip settle. Wait out settle_deadline.
    await lc.warpPastSettleDeadline();

    const takerQuoteBefore = await tokenBalance(lc.ctx, lc.takerQuoteAta);
    const makerBaseBefore = await tokenBalance(lc.ctx, lc.makerBaseAta);

    await lc.expireWithMaker();

    // Taker got the full lock back.
    expect(
      ((await tokenBalance(lc.ctx, lc.takerQuoteAta)) - takerQuoteBefore).toString()
    ).toBe(lc.expectedTakerLock.toString());
    // Maker got their fund back (they had locked revealedSize of base).
    expect(
      ((await tokenBalance(lc.ctx, lc.makerBaseAta)) - makerBaseBefore).toString()
    ).toBe(lc.expectedMakerFund.toString());

    // Vaults + escrow all closed.
    expect(await lc.ctx.banksClient.getAccount(lc.takerVault)).toBeNull();
    expect(await lc.ctx.banksClient.getAccount(lc.makerVault)).toBeNull();
    expect(await lc.ctx.banksClient.getAccount(lc.escrow)).toBeNull();

    // Reputation: quotes_won was bumped at fund time, but failed_reveals
    // also fires here for never-settling.
    const rep = await lc.ctx.program.account.reputation.fetch(lc.reputation);
    expect(rep.quotesWon.toString()).toBe("1");
    expect(rep.failedReveals.toString()).toBe("1");
    expect(rep.settledCount.toString()).toBe("0");

    // Status Expired.
    const intentFinal = await lc.ctx.program.account.intent.fetch(lc.intent);
    expect(intentFinal.status).toBe(4);
  });
});
