/**
 * Bankrun test bootstrap.
 *
 * Spins up a fresh in-process Solana bank with the compiled nyxbid
 * program loaded, exposes a typed Anchor Program<Nyxbid>, and provides
 * a deterministic clock + helpers for mints, ATAs, and PDA derivation.
 */
import { startAnchor, Clock, ProgramTestContext } from "solana-bankrun";
import { BankrunProvider } from "anchor-bankrun";
import {
  AnchorProvider,
  Program,
  setProvider,
  Wallet,
  BN,
} from "@coral-xyz/anchor";
import {
  Keypair,
  PublicKey,
  SystemProgram,
  Transaction,
} from "@solana/web3.js";
import {
  createInitializeMintInstruction,
  createAssociatedTokenAccountInstruction,
  createMintToInstruction,
  getAssociatedTokenAddressSync,
  AccountLayout,
  MintLayout,
  TOKEN_PROGRAM_ID,
  ASSOCIATED_TOKEN_PROGRAM_ID,
} from "@solana/spl-token";
import { createHash } from "crypto";
import path from "path";

import type { Nyxbid } from "../../target/types/nyxbid";
// eslint-disable-next-line @typescript-eslint/no-var-requires
const idl = require("../../target/idl/nyxbid.json");

export const PROGRAM_ID = new PublicKey(
  "E9sMPu6uUJTfe72ePWr8BNjEKejUnMqsdFV6rGtsHiX2"
);

// Mirrors `pub const PRICE_SCALE: u64 = 1_000_000;` in state.rs.
export const PRICE_SCALE = new BN(1_000_000);

// Mirrors `pub const MIN_SUBMIT_WINDOW_SECS: i64 = 5;` in state.rs.
export const MIN_SUBMIT_WINDOW_SECS = 5;

// PDA seed bytes mirroring state.rs.
export const SEED = {
  intent: Buffer.from("intent"),
  quote: Buffer.from("quote"),
  escrow: Buffer.from("escrow"),
  taker_vault: Buffer.from("taker_vault"),
  maker_vault: Buffer.from("maker_vault"),
  receipt: Buffer.from("receipt"),
  reputation: Buffer.from("reputation"),
};

export interface TestCtx {
  context: ProgramTestContext;
  provider: BankrunProvider;
  program: Program<Nyxbid>;
  banksClient: ProgramTestContext["banksClient"];
  payer: Keypair;
}

/**
 * Boot a bankrun + Anchor environment with the program loaded. Repo
 * root is auto-detected from the chain/ directory.
 */
export async function bootstrap(): Promise<TestCtx> {
  // startAnchor reads Anchor.toml at the given path and loads programs[*].
  const repoRoot = path.resolve(__dirname, "../../");
  const context = await startAnchor(repoRoot, [], []);
  const provider = new BankrunProvider(context);
  setProvider(provider);

  const program = new Program<Nyxbid>(idl as any, provider);

  return {
    context,
    provider,
    program,
    banksClient: context.banksClient,
    payer: context.payer,
  };
}

// ---------- Tx submission ----------

/**
 * Sign and submit a Transaction through bankrun. Refreshes blockhash
 * automatically. Throws if the bank rejects.
 */
export async function sendTx(
  ctx: TestCtx,
  ixs: Transaction["instructions"],
  signers: Keypair[],
  feePayer: Keypair = signers[0]
): Promise<void> {
  const tx = new Transaction();
  for (const ix of ixs) tx.add(ix);
  tx.feePayer = feePayer.publicKey;
  tx.recentBlockhash = ctx.context.lastBlockhash;
  tx.sign(...signers);
  await ctx.banksClient.processTransaction(tx);
}

// ---------- Clock helpers ----------

/** Bankrun unixTimestamp as a number. Starts at 0 on boot. */
export async function nowTs(ctx: TestCtx): Promise<number> {
  const clock = await ctx.context.banksClient.getClock();
  return Number(clock.unixTimestamp);
}

/** Move the clock to the given absolute unix timestamp (seconds). */
export async function warpTo(ctx: TestCtx, unixTs: number): Promise<void> {
  const clock = await ctx.context.banksClient.getClock();
  ctx.context.setClock(
    new Clock(
      clock.slot,
      clock.epochStartTimestamp,
      clock.epoch,
      clock.leaderScheduleEpoch,
      BigInt(unixTs)
    )
  );
}

/** Advance the clock by `deltaSecs` seconds. */
export async function warpBy(ctx: TestCtx, deltaSecs: number): Promise<void> {
  const t = await nowTs(ctx);
  await warpTo(ctx, t + deltaSecs);
}

/**
 * Advance one bankrun slot. Forces a fresh blockhash so subsequent txs
 * with the same instruction/accounts/signers don't collide on the
 * "transaction already processed" check.
 */
export async function nextSlot(ctx: TestCtx): Promise<void> {
  const clock = await ctx.context.banksClient.getClock();
  ctx.context.warpToSlot(clock.slot + 1n);
}

// ---------- Funding + mints (bankrun-native) ----------

/**
 * Create and fund a fresh keypair via system transfer from the
 * pre-funded payer. Default 100 SOL.
 */
export async function fundedKeypair(
  ctx: TestCtx,
  lamports: number = 100_000_000_000
): Promise<Keypair> {
  const kp = Keypair.generate();
  const ix = SystemProgram.transfer({
    fromPubkey: ctx.payer.publicKey,
    toPubkey: kp.publicKey,
    lamports,
  });
  await sendTx(ctx, [ix], [ctx.payer]);
  return kp;
}

/**
 * Create a fresh SPL Token mint using direct ix building (bypasses
 * @solana/spl-token's connection-based actions which don't work with
 * bankrun's connection shim).
 */
export async function createTestMint(
  ctx: TestCtx,
  decimals: number,
  authority: Keypair = ctx.payer
): Promise<PublicKey> {
  const mintKp = Keypair.generate();
  const rent = await ctx.banksClient.getRent();
  const lamports = Number(rent.minimumBalance(BigInt(MintLayout.span)));

  const createAcct = SystemProgram.createAccount({
    fromPubkey: ctx.payer.publicKey,
    newAccountPubkey: mintKp.publicKey,
    space: MintLayout.span,
    lamports,
    programId: TOKEN_PROGRAM_ID,
  });
  const initMint = createInitializeMintInstruction(
    mintKp.publicKey,
    decimals,
    authority.publicKey,
    null
  );

  await sendTx(ctx, [createAcct, initMint], [ctx.payer, mintKp]);
  return mintKp.publicKey;
}

/** Create an ATA for `owner` against `mint`. Returns the ATA pubkey. */
export async function createAta(
  ctx: TestCtx,
  mint: PublicKey,
  owner: Keypair
): Promise<PublicKey> {
  const ataPk = getAssociatedTokenAddressSync(mint, owner.publicKey);
  const ix = createAssociatedTokenAccountInstruction(
    ctx.payer.publicKey,
    ataPk,
    owner.publicKey,
    mint
  );
  await sendTx(ctx, [ix], [ctx.payer]);
  return ataPk;
}

/** Mint `amount` of `mint` to `dest`. */
export async function mintToAta(
  ctx: TestCtx,
  mint: PublicKey,
  dest: PublicKey,
  amount: bigint | number,
  authority: Keypair = ctx.payer
): Promise<void> {
  const ix = createMintToInstruction(
    mint,
    dest,
    authority.publicKey,
    BigInt(amount)
  );
  await sendTx(ctx, [ix], [authority], ctx.payer);
}

/** Read a token account amount (as bigint). */
export async function tokenBalance(
  ctx: TestCtx,
  ata: PublicKey
): Promise<bigint> {
  const acct = await ctx.banksClient.getAccount(ata);
  if (!acct) throw new Error(`token account ${ata.toBase58()} not found`);
  const decoded = AccountLayout.decode(acct.data);
  return decoded.amount;
}

/** Compute the ATA address (sync). */
export function ata(owner: PublicKey, mint: PublicKey): PublicKey {
  return getAssociatedTokenAddressSync(mint, owner);
}

// ---------- PDA derivation ----------

export function intentPda(taker: PublicKey, nonce: Buffer): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [SEED.intent, taker.toBuffer(), nonce],
    PROGRAM_ID
  );
}

export function escrowPda(intent: PublicKey): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [SEED.escrow, intent.toBuffer()],
    PROGRAM_ID
  );
}

export function takerVaultPda(intent: PublicKey): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [SEED.taker_vault, intent.toBuffer()],
    PROGRAM_ID
  );
}

export function makerVaultPda(intent: PublicKey): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [SEED.maker_vault, intent.toBuffer()],
    PROGRAM_ID
  );
}

export function quotePda(
  intent: PublicKey,
  maker: PublicKey,
  nonce: Buffer
): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [SEED.quote, intent.toBuffer(), maker.toBuffer(), nonce],
    PROGRAM_ID
  );
}

export function receiptPda(intent: PublicKey): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [SEED.receipt, intent.toBuffer()],
    PROGRAM_ID
  );
}

export function reputationPda(maker: PublicKey): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [SEED.reputation, maker.toBuffer()],
    PROGRAM_ID
  );
}

// ---------- Commitment hashing ----------

/**
 * Compute commitment = sha256(price_le || size_le || nonce32) matching
 * the program's solana_sha256_hasher::hashv input encoding.
 */
export function commitmentHash(
  price: BN,
  size: BN,
  nonce32: Buffer
): Buffer {
  if (nonce32.length !== 32) {
    throw new Error(`nonce must be 32 bytes, got ${nonce32.length}`);
  }
  const h = createHash("sha256");
  h.update(price.toArrayLike(Buffer, "le", 8));
  h.update(size.toArrayLike(Buffer, "le", 8));
  h.update(nonce32);
  return h.digest();
}

/** Generate a fresh 16-byte intent/quote nonce. */
export function rand16(): Buffer {
  return Buffer.from(
    Array.from({ length: 16 }, () => Math.floor(Math.random() * 256))
  );
}

/** Generate a fresh 32-byte reveal nonce. */
export function rand32(): Buffer {
  return Buffer.from(
    Array.from({ length: 32 }, () => Math.floor(Math.random() * 256))
  );
}

// ---------- Misc ----------

export const Side = { Buy: 0, Sell: 1 } as const;

/**
 * Compute quote_notional(size, price) matching state.rs: u128 mul
 * then div by PRICE_SCALE. Returns BN.
 */
export function quoteNotional(size: BN, price: BN): BN {
  return size.mul(price).div(PRICE_SCALE);
}

export { TOKEN_PROGRAM_ID, ASSOCIATED_TOKEN_PROGRAM_ID };
