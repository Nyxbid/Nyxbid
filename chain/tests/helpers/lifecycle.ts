/**
 * High-level lifecycle helper. Sets up a buy or sell scenario with one
 * maker and exposes typed methods for each instruction. Failure-path
 * tests use this to drive a scenario to a specific point and then
 * assert that the next instruction errors as expected.
 *
 * Usage:
 *   const lc = await Lifecycle.create({ side: Side.Buy });
 *   await lc.submitQuote();
 *   await lc.warpToReveal();
 *   await lc.revealQuote();
 *   await lc.warpToSettle();
 *   await lc.fundMakerEscrow();
 *   await lc.settle();
 */
import { BN } from "@coral-xyz/anchor";
import {
  Keypair,
  PublicKey,
  SystemProgram,
  SYSVAR_RENT_PUBKEY,
} from "@solana/web3.js";

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
} from "./setup";

const BASE_DECIMALS = 9;
const QUOTE_DECIMALS = 6;

export interface LifecycleOpts {
  side: typeof Side.Buy | typeof Side.Sell;
  size?: BN;
  limitPrice?: BN;
  revealedPrice?: BN;
  /** Window lengths from t0; defaults match the happy path. */
  revealAfter?: number;
  resolveAfter?: number;
  settleAfter?: number;
}

export class Lifecycle {
  ctx!: TestCtx;
  side!: typeof Side.Buy | typeof Side.Sell;

  taker!: Keypair;
  maker!: Keypair;

  baseMint!: PublicKey;
  quoteMint!: PublicKey;

  takerBaseAta!: PublicKey;
  takerQuoteAta!: PublicKey;
  makerBaseAta!: PublicKey;
  makerQuoteAta!: PublicKey;

  size!: BN;
  limitPrice!: BN;
  revealedPrice!: BN;
  revealedSize!: BN;
  expectedTakerLock!: BN;
  expectedMakerFund!: BN;

  intent!: PublicKey;
  escrow!: PublicKey;
  takerVault!: PublicKey;
  makerVault!: PublicKey;
  quote!: PublicKey;
  receipt!: PublicKey;
  reputation!: PublicKey;

  intentNonce!: Buffer;
  quoteNonce16!: Buffer;
  revealNonce!: Buffer;
  commitment!: Buffer;

  t0!: number;
  revealDeadline!: BN;
  resolveDeadline!: BN;
  settleDeadline!: BN;

  /** Build a fresh scenario and run create_intent. */
  static async create(opts: LifecycleOpts): Promise<Lifecycle> {
    const lc = new Lifecycle();
    await lc.init(opts);
    await lc.createIntent();
    return lc;
  }

  private async init(opts: LifecycleOpts): Promise<void> {
    this.ctx = await bootstrap();
    await warpTo(this.ctx, 1_700_000_000);

    this.side = opts.side;
    this.size = opts.size ?? new BN("10000000000"); // 10 base
    this.limitPrice = opts.limitPrice ?? new BN("50000000"); // 50

    // Default revealed price beats the limit by 2.
    if (opts.revealedPrice) {
      this.revealedPrice = opts.revealedPrice;
    } else {
      this.revealedPrice =
        this.side === Side.Buy
          ? this.limitPrice.sub(new BN("2000000")) // 48 for buy
          : new BN("47000000"); // 47 for sell (limit 45 default in tests)
    }
    this.revealedSize = this.size;

    this.taker = await fundedKeypair(this.ctx);
    this.maker = await fundedKeypair(this.ctx);
    this.baseMint = await createTestMint(this.ctx, BASE_DECIMALS);
    this.quoteMint = await createTestMint(this.ctx, QUOTE_DECIMALS);

    this.takerBaseAta = await createAta(this.ctx, this.baseMint, this.taker);
    this.takerQuoteAta = await createAta(this.ctx, this.quoteMint, this.taker);
    this.makerBaseAta = await createAta(this.ctx, this.baseMint, this.maker);
    this.makerQuoteAta = await createAta(this.ctx, this.quoteMint, this.maker);

    // Fund the side that locks.
    this.expectedTakerLock =
      this.side === Side.Buy
        ? quoteNotional(this.size, this.limitPrice)
        : this.size;
    this.expectedMakerFund =
      this.side === Side.Buy
        ? this.revealedSize
        : quoteNotional(this.revealedSize, this.revealedPrice);

    if (this.side === Side.Buy) {
      await mintToAta(
        this.ctx,
        this.quoteMint,
        this.takerQuoteAta,
        BigInt(this.expectedTakerLock.toString())
      );
      await mintToAta(
        this.ctx,
        this.baseMint,
        this.makerBaseAta,
        BigInt(this.expectedMakerFund.toString())
      );
    } else {
      await mintToAta(
        this.ctx,
        this.baseMint,
        this.takerBaseAta,
        BigInt(this.expectedTakerLock.toString())
      );
      await mintToAta(
        this.ctx,
        this.quoteMint,
        this.makerQuoteAta,
        BigInt(this.expectedMakerFund.toString())
      );
    }

    this.t0 = await nowTs(this.ctx);
    this.revealDeadline = new BN(this.t0 + (opts.revealAfter ?? 30));
    this.resolveDeadline = new BN(this.t0 + (opts.resolveAfter ?? 60));
    this.settleDeadline = new BN(this.t0 + (opts.settleAfter ?? 90));

    this.intentNonce = rand16();
    [this.intent] = intentPda(this.taker.publicKey, this.intentNonce);
    [this.escrow] = escrowPda(this.intent);
    [this.takerVault] = takerVaultPda(this.intent);
    [this.makerVault] = makerVaultPda(this.intent);

    this.quoteNonce16 = rand16();
    this.revealNonce = rand32();
    this.commitment = commitmentHash(
      this.revealedPrice,
      this.revealedSize,
      this.revealNonce
    );
    [this.quote] = quotePda(this.intent, this.maker.publicKey, this.quoteNonce16);
    [this.receipt] = receiptPda(this.intent);
    [this.reputation] = reputationPda(this.maker.publicKey);
  }

  // ---------- instructions ----------

  async createIntent(): Promise<void> {
    const takerLockMint =
      this.side === Side.Buy ? this.quoteMint : this.baseMint;
    const takerSource =
      this.side === Side.Buy ? this.takerQuoteAta : this.takerBaseAta;

    await this.ctx.program.methods
      .createIntent({
        side: this.side,
        size: this.size,
        limitPrice: this.limitPrice,
        revealDeadline: this.revealDeadline,
        resolveDeadline: this.resolveDeadline,
        settleDeadline: this.settleDeadline,
        commitmentRoot: Array.from(this.commitment),
        nonce: Array.from(this.intentNonce),
      } as any)
      .accountsPartial({
        taker: this.taker.publicKey,
        baseMint: this.baseMint,
        quoteMint: this.quoteMint,
        takerSource,
        intent: this.intent,
        escrow: this.escrow,
        takerVault: this.takerVault,
        takerLockMint,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
        rent: SYSVAR_RENT_PUBKEY,
      } as any)
      .signers([this.taker])
      .rpc();
  }

  async submitQuote(maker: Keypair = this.maker): Promise<void> {
    const [reputation] = reputationPda(maker.publicKey);
    let quoteAcct = this.quote;
    let quoteNonce = this.quoteNonce16;
    if (!maker.publicKey.equals(this.maker.publicKey)) {
      quoteNonce = rand16();
      [quoteAcct] = quotePda(this.intent, maker.publicKey, quoteNonce);
    }

    await this.ctx.program.methods
      .submitQuote({
        commitment: Array.from(this.commitment),
        nonce: Array.from(quoteNonce),
      } as any)
      .accountsPartial({
        maker: maker.publicKey,
        intent: this.intent,
        quote: quoteAcct,
        reputation,
        systemProgram: SystemProgram.programId,
      } as any)
      .signers([maker])
      .rpc();
  }

  async warpToReveal(deltaPastDeadline = 1): Promise<void> {
    await warpTo(this.ctx, this.t0 + 30 + deltaPastDeadline);
  }

  async warpToSettle(deltaPastDeadline = 1): Promise<void> {
    await warpTo(this.ctx, this.t0 + 60 + deltaPastDeadline);
  }

  async warpPastSettleDeadline(deltaPastDeadline = 1): Promise<void> {
    await warpTo(this.ctx, this.t0 + 90 + deltaPastDeadline);
  }

  async revealQuote(): Promise<void> {
    await this.ctx.program.methods
      .revealQuote({
        revealedPrice: this.revealedPrice,
        revealedSize: this.revealedSize,
        nonce: Array.from(this.revealNonce),
      } as any)
      .accountsPartial({
        maker: this.maker.publicKey,
        intent: this.intent,
        quote: this.quote,
      } as any)
      .signers([this.maker])
      .rpc();
  }

  async fundMakerEscrow(amount: BN = this.expectedMakerFund): Promise<void> {
    const makerLockMint =
      this.side === Side.Buy ? this.baseMint : this.quoteMint;
    const makerSource =
      this.side === Side.Buy ? this.makerBaseAta : this.makerQuoteAta;

    await this.ctx.program.methods
      .fundMakerEscrow({ amount } as any)
      .accountsPartial({
        maker: this.maker.publicKey,
        intent: this.intent,
        quote: this.quote,
        escrow: this.escrow,
        makerLockMint,
        makerSource,
        makerVault: this.makerVault,
        reputation: this.reputation,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
        rent: SYSVAR_RENT_PUBKEY,
      } as any)
      .signers([this.maker])
      .rpc();
  }

  async settle(): Promise<void> {
    // Buy => maker_destination is taker's quote leg gives maker quote;
    // taker_destination receives base; refund_destination is taker's
    // quote ATA when buy + price improvement.
    const makerDestination =
      this.side === Side.Buy ? this.makerQuoteAta : this.makerBaseAta;
    const takerDestination =
      this.side === Side.Buy ? this.takerBaseAta : this.takerQuoteAta;
    const takerRefundDestination =
      this.side === Side.Buy ? this.takerQuoteAta : null;

    await this.ctx.program.methods
      .settle()
      .accountsPartial({
        payer: this.taker.publicKey,
        intent: this.intent,
        winningQuote: this.quote,
        escrow: this.escrow,
        takerVault: this.takerVault,
        makerVault: this.makerVault,
        makerDestination,
        takerDestination,
        takerRefundDestination,
        takerRentBeneficiary: this.taker.publicKey,
        makerRentBeneficiary: this.maker.publicKey,
        receipt: this.receipt,
        reputation: this.reputation,
        baseMint: this.baseMint,
        quoteMint: this.quoteMint,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      } as any)
      .signers([this.taker])
      .rpc();
  }

  async cancel(signer: Keypair = this.taker): Promise<void> {
    const takerDestination =
      this.side === Side.Buy ? this.takerQuoteAta : this.takerBaseAta;

    await this.ctx.program.methods
      .cancel()
      .accountsPartial({
        taker: signer.publicKey,
        intent: this.intent,
        escrow: this.escrow,
        takerVault: this.takerVault,
        takerDestination,
        tokenProgram: TOKEN_PROGRAM_ID,
      } as any)
      .signers([signer])
      .rpc();
  }

  async expireNoMaker(opts: {
    withWinner?: boolean;
  } = {}): Promise<void> {
    const takerDestination =
      this.side === Side.Buy ? this.takerQuoteAta : this.takerBaseAta;

    await this.ctx.program.methods
      .expireNoMaker()
      .accountsPartial({
        payer: this.taker.publicKey,
        intent: this.intent,
        escrow: this.escrow,
        takerVault: this.takerVault,
        takerDestination,
        takerRentBeneficiary: this.taker.publicKey,
        winningQuote: opts.withWinner ? this.quote : null,
        winningMakerReputation: opts.withWinner ? this.reputation : null,
        tokenProgram: TOKEN_PROGRAM_ID,
      } as any)
      .signers([this.taker])
      .rpc();
  }

  async expireWithMaker(): Promise<void> {
    const takerDestination =
      this.side === Side.Buy ? this.takerQuoteAta : this.takerBaseAta;
    const makerDestination =
      this.side === Side.Buy ? this.makerBaseAta : this.makerQuoteAta;

    await this.ctx.program.methods
      .expireWithMaker()
      .accountsPartial({
        payer: this.taker.publicKey,
        intent: this.intent,
        escrow: this.escrow,
        takerVault: this.takerVault,
        takerDestination,
        takerRentBeneficiary: this.taker.publicKey,
        makerVault: this.makerVault,
        makerDestination,
        makerRentBeneficiary: this.maker.publicKey,
        reputation: this.reputation,
        tokenProgram: TOKEN_PROGRAM_ID,
      } as any)
      .signers([this.taker])
      .rpc();
  }
}

/**
 * Assert that an Anchor program call rejects with a specific error code
 * name (matching `#[error_code]` in error.rs). Returns the error so
 * callers can inspect more if needed.
 */
export async function expectAnchorError(
  fn: () => Promise<unknown>,
  errorName: string
): Promise<void> {
  try {
    await fn();
  } catch (e: any) {
    const msg: string = e?.toString?.() ?? "";
    const err = e?.error?.errorCode?.code ?? "";
    if (err === errorName || msg.includes(errorName)) return;
    throw new Error(
      `expected error ${errorName}, got: ${msg}\n${JSON.stringify(
        e?.error ?? {},
        null,
        2
      )}`
    );
  }
  throw new Error(`expected error ${errorName}, but call succeeded`);
}
