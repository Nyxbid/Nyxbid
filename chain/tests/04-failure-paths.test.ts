/**
 * Failure-path coverage. Each test drives a Lifecycle scenario to a
 * specific point and asserts the next instruction errors with the
 * expected #[error_code] variant from error.rs.
 *
 * Covers every failure case in docs/09:80-97 plus the new variants
 * introduced during the review rounds (NotWinningMaker, WrongFundAmount,
 * SubmitWindowTooShort, etc.).
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
  rand32,
  commitmentHash,
  quoteNotional,
  Side,
  TOKEN_PROGRAM_ID,
} from "./helpers/setup";
import { Lifecycle, expectAnchorError } from "./helpers/lifecycle";
import { SystemProgram, SYSVAR_RENT_PUBKEY } from "@solana/web3.js";

describe("failure: commitment + price/size validation", () => {
  it("reveal with wrong nonce -> CommitmentMismatch", async () => {
    const lc = await Lifecycle.create({ side: Side.Buy });
    await lc.submitQuote();
    await lc.warpToReveal();

    // Tamper the reveal nonce.
    const badNonce = rand32();
    await expectAnchorError(
      () =>
        lc.ctx.program.methods
          .revealQuote({
            revealedPrice: lc.revealedPrice,
            revealedSize: lc.revealedSize,
            nonce: Array.from(badNonce),
          } as any)
          .accountsPartial({
            maker: lc.maker.publicKey,
            intent: lc.intent,
            quote: lc.quote,
          } as any)
          .signers([lc.maker])
          .rpc(),
      "CommitmentMismatch"
    );
  });

  it("buy quote above limit -> LimitBreached", async () => {
    // limit = 50, reveal 51 (worse than limit on a buy).
    const lc = await Lifecycle.create({
      side: Side.Buy,
      limitPrice: new BN("50000000"),
      revealedPrice: new BN("51000000"),
    });
    await lc.submitQuote();
    await lc.warpToReveal();
    await expectAnchorError(() => lc.revealQuote(), "LimitBreached");
  });

  it("sell quote below limit -> LimitBreached", async () => {
    // limit = 45, reveal 44 (worse than limit on a sell).
    const lc = await Lifecycle.create({
      side: Side.Sell,
      limitPrice: new BN("45000000"),
      revealedPrice: new BN("44000000"),
    });
    await lc.submitQuote();
    await lc.warpToReveal();
    await expectAnchorError(() => lc.revealQuote(), "LimitBreached");
  });

  it("reveal with wrong size -> SizeMismatch", async () => {
    const lc = await Lifecycle.create({ side: Side.Buy });
    await lc.submitQuote();
    await lc.warpToReveal();

    // Build a tampered commitment that matches a different size,
    // then try to reveal that smaller size against a quote whose
    // commitment was built for the full size. Wait \u2014 simpler:
    // we just reveal a different size; the program checks
    // revealed_size == intent.size, so even a matching commitment
    // would still fail SizeMismatch.
    const halfSize = lc.revealedSize.div(new BN(2));
    const halfNonce = rand32();
    const halfCommitment = commitmentHash(
      lc.revealedPrice,
      halfSize,
      halfNonce
    );

    // Submit a *new* quote with the half-size commitment so the
    // commitment check passes and we hit SizeMismatch instead.
    const halfQuoteNonce = rand16();
    const [halfQuote] = await import("./helpers/setup").then((m) =>
      m.quotePda(lc.intent, lc.maker.publicKey, halfQuoteNonce)
    );
    const [reputation] = await import("./helpers/setup").then((m) =>
      m.reputationPda(lc.maker.publicKey)
    );

    // Need to submit BEFORE warping to reveal_deadline. Re-bootstrap.
    const lc2 = await Lifecycle.create({ side: Side.Buy });
    const halfQuoteNonce2 = rand16();
    const halfRevealNonce2 = rand32();
    const halfCommitment2 = commitmentHash(
      lc2.revealedPrice,
      halfSize,
      halfRevealNonce2
    );
    const [halfQuote2] = await import("./helpers/setup").then((m) =>
      m.quotePda(lc2.intent, lc2.maker.publicKey, halfQuoteNonce2)
    );
    const [rep2] = await import("./helpers/setup").then((m) =>
      m.reputationPda(lc2.maker.publicKey)
    );

    await lc2.ctx.program.methods
      .submitQuote({
        commitment: Array.from(halfCommitment2),
        nonce: Array.from(halfQuoteNonce2),
      } as any)
      .accountsPartial({
        maker: lc2.maker.publicKey,
        intent: lc2.intent,
        quote: halfQuote2,
        reputation: rep2,
        systemProgram: SystemProgram.programId,
      } as any)
      .signers([lc2.maker])
      .rpc();

    await lc2.warpToReveal();
    await expectAnchorError(
      () =>
        lc2.ctx.program.methods
          .revealQuote({
            revealedPrice: lc2.revealedPrice,
            revealedSize: halfSize,
            nonce: Array.from(halfRevealNonce2),
          } as any)
          .accountsPartial({
            maker: lc2.maker.publicKey,
            intent: lc2.intent,
            quote: halfQuote2,
          } as any)
          .signers([lc2.maker])
          .rpc(),
      "SizeMismatch"
    );
  });
});

describe("failure: timing windows", () => {
  it("submit_quote after reveal_deadline -> RevealDeadlinePassed", async () => {
    const lc = await Lifecycle.create({ side: Side.Buy });
    await lc.warpToReveal(); // past reveal_deadline
    await expectAnchorError(() => lc.submitQuote(), "RevealDeadlinePassed");
  });

  it("reveal_quote before reveal_deadline -> RevealDeadlineNotReached", async () => {
    const lc = await Lifecycle.create({ side: Side.Buy });
    await lc.submitQuote();
    // No warp - still pre-reveal.
    await expectAnchorError(
      () => lc.revealQuote(),
      "RevealDeadlineNotReached"
    );
  });

  it("reveal_quote after resolve_deadline -> ResolveDeadlinePassed", async () => {
    const lc = await Lifecycle.create({ side: Side.Buy });
    await lc.submitQuote();
    await lc.warpToSettle(); // past resolve_deadline
    await expectAnchorError(() => lc.revealQuote(), "ResolveDeadlinePassed");
  });

  it("cancel after reveal_deadline -> RevealDeadlinePassed", async () => {
    const lc = await Lifecycle.create({ side: Side.Buy });
    await lc.warpToReveal();
    await expectAnchorError(() => lc.cancel(), "RevealDeadlinePassed");
  });

  it("settle after settle_deadline -> SettleDeadlinePassed", async () => {
    const lc = await Lifecycle.create({ side: Side.Buy });
    await lc.submitQuote();
    await lc.warpToReveal();
    await lc.revealQuote();
    await lc.warpToSettle();
    await lc.fundMakerEscrow();
    await lc.warpPastSettleDeadline();
    await expectAnchorError(() => lc.settle(), "SettleDeadlinePassed");
  });
});

describe("failure: authorization", () => {
  it("cancel by non-taker -> Unauthorized", async () => {
    const lc = await Lifecycle.create({ side: Side.Buy });
    const stranger = await fundedKeypair(lc.ctx);
    // Cancel handler signs as stranger; constraint says signer.key() ==
    // intent.taker, error code Unauthorized.
    await expectAnchorError(() => lc.cancel(stranger), "Unauthorized");
  });

  it("fund_maker_escrow by non-winning maker -> NotWinningMaker", async () => {
    const { makerVaultPda, quotePda, reputationPda } = await import(
      "./helpers/setup"
    );
    const lc = await Lifecycle.create({ side: Side.Buy });
    await lc.submitQuote();

    // Second maker submits a quote with the same commitment but a
    // different quote PDA (different nonce). Their quote will not be
    // intent.winning_quote because the primary reveals first.
    const otherMaker = await fundedKeypair(lc.ctx);
    const otherBaseAta = await createAta(lc.ctx, lc.baseMint, otherMaker);
    await mintToAta(
      lc.ctx,
      lc.baseMint,
      otherBaseAta,
      BigInt(lc.expectedMakerFund.toString())
    );

    const otherNonce = rand16();
    const [otherQuote] = quotePda(lc.intent, otherMaker.publicKey, otherNonce);
    const [otherRep] = reputationPda(otherMaker.publicKey);
    await lc.ctx.program.methods
      .submitQuote({
        commitment: Array.from(lc.commitment),
        nonce: Array.from(otherNonce),
      } as any)
      .accountsPartial({
        maker: otherMaker.publicKey,
        intent: lc.intent,
        quote: otherQuote,
        reputation: otherRep,
        systemProgram: SystemProgram.programId,
      } as any)
      .signers([otherMaker])
      .rpc();

    await lc.warpToReveal();
    await lc.revealQuote(); // primary maker becomes winner.
    await lc.warpToSettle();

    const [otherMakerVault] = makerVaultPda(lc.intent);
    await expectAnchorError(
      () =>
        lc.ctx.program.methods
          .fundMakerEscrow({ amount: lc.expectedMakerFund } as any)
          .accountsPartial({
            maker: otherMaker.publicKey,
            intent: lc.intent,
            quote: otherQuote, // their own non-winning quote
            escrow: lc.escrow,
            makerLockMint: lc.baseMint,
            makerSource: otherBaseAta,
            makerVault: otherMakerVault,
            reputation: otherRep,
            tokenProgram: TOKEN_PROGRAM_ID,
            systemProgram: SystemProgram.programId,
            rent: SYSVAR_RENT_PUBKEY,
          } as any)
          .signers([otherMaker])
          .rpc(),
      "NotWinningMaker"
    );
  });

  it("fund_maker_escrow with wrong amount -> WrongFundAmount", async () => {
    const lc = await Lifecycle.create({ side: Side.Buy });
    await lc.submitQuote();
    await lc.warpToReveal();
    await lc.revealQuote();
    await lc.warpToSettle();
    // Try to underfund.
    const wrong = lc.expectedMakerFund.sub(new BN(1));
    await expectAnchorError(
      () => lc.fundMakerEscrow(wrong),
      "WrongFundAmount"
    );
  });
});

describe("failure: lifecycle ordering", () => {
  it("settle before fund_maker_escrow -> AccountNotInitialized (maker_vault)", async () => {
    const lc = await Lifecycle.create({ side: Side.Buy });
    await lc.submitQuote();
    await lc.warpToReveal();
    await lc.revealQuote();
    // Status is still Open (flips to Resolved inside fund_maker_escrow).
    // The settle account list demands the maker_vault PDA which only
    // gets created in fund_maker_escrow, so the failure surfaces as
    // Anchor's built-in AccountNotInitialized rather than the
    // IntentNotResolved domain error. This is a defense-in-depth
    // double-check that the lifecycle ordering is enforced by
    // account presence in addition to status.
    await expectAnchorError(() => lc.settle(), "AccountNotInitialized");
  });

  it("double settle -> AccountNotInitialized (escrow closed)", async () => {
    const lc = await Lifecycle.create({ side: Side.Buy });
    await lc.submitQuote();
    await lc.warpToReveal();
    await lc.revealQuote();
    await lc.warpToSettle();
    await lc.fundMakerEscrow();
    await lc.settle(); // first one succeeds
    // After first settle, the Escrow PDA is closed (close = taker_rent_beneficiary).
    // The second settle's account constraints hit AccountNotInitialized
    // on `escrow` before any handler logic runs - which is the correct
    // protection: closed accounts cannot be re-used.
    await expectAnchorError(() => lc.settle(), "AccountNotInitialized");
  });
});

describe("failure: create_intent input validation", () => {
  it("reveal_deadline already past -> SubmitWindowTooShort", async () => {
    // Hand-build create_intent with reveal_deadline before clock+min.
    const ctx = await bootstrap();
    await warpTo(ctx, 1_700_000_000);
    const taker = await fundedKeypair(ctx);
    const baseMint = await createTestMint(ctx, 9);
    const quoteMint = await createTestMint(ctx, 6);
    const takerQuoteAta = await createAta(ctx, quoteMint, taker);
    const size = new BN("10000000000");
    const limit = new BN("50000000");
    const lock = quoteNotional(size, limit);
    await mintToAta(ctx, quoteMint, takerQuoteAta, BigInt(lock.toString()));

    const t0 = await nowTs(ctx);
    const intentNonce = rand16();
    const [intent] = intentPda(taker.publicKey, intentNonce);
    const [escrow] = escrowPda(intent);
    const [takerVault] = takerVaultPda(intent);
    const dummy = commitmentHash(new BN(0), new BN(0), Buffer.alloc(32));

    await expectAnchorError(
      () =>
        ctx.program.methods
          .createIntent({
            side: Side.Buy,
            size,
            limitPrice: limit,
            // Already past: t0 + 2 vs MIN_SUBMIT_WINDOW_SECS = 5.
            revealDeadline: new BN(t0 + 2),
            resolveDeadline: new BN(t0 + 60),
            settleDeadline: new BN(t0 + 90),
            commitmentRoot: Array.from(dummy),
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
          .rpc(),
      "SubmitWindowTooShort"
    );
  });

  it("resolve_deadline <= reveal_deadline -> BadDeadlines", async () => {
    const ctx = await bootstrap();
    await warpTo(ctx, 1_700_000_000);
    const taker = await fundedKeypair(ctx);
    const baseMint = await createTestMint(ctx, 9);
    const quoteMint = await createTestMint(ctx, 6);
    const takerQuoteAta = await createAta(ctx, quoteMint, taker);
    const size = new BN("10000000000");
    const limit = new BN("50000000");
    const lock = quoteNotional(size, limit);
    await mintToAta(ctx, quoteMint, takerQuoteAta, BigInt(lock.toString()));

    const t0 = await nowTs(ctx);
    const intentNonce = rand16();
    const [intent] = intentPda(taker.publicKey, intentNonce);
    const [escrow] = escrowPda(intent);
    const [takerVault] = takerVaultPda(intent);
    const dummy = commitmentHash(new BN(0), new BN(0), Buffer.alloc(32));

    await expectAnchorError(
      () =>
        ctx.program.methods
          .createIntent({
            side: Side.Buy,
            size,
            limitPrice: limit,
            // resolve_deadline NOT after reveal_deadline.
            revealDeadline: new BN(t0 + 30),
            resolveDeadline: new BN(t0 + 30),
            settleDeadline: new BN(t0 + 90),
            commitmentRoot: Array.from(dummy),
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
          .rpc(),
      "BadDeadlines"
    );
  });
});
