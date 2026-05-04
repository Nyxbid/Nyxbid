/**
 * Regression tests for the Lifecycle helper itself.
 *
 * Pin behavior that contributors are likely to break:
 *   - warp helpers must derive offsets from the stored deadline BNs,
 *     not from hard-coded constants. A test passing custom revealAfter
 *     / resolveAfter / settleAfter values to Lifecycle.create() must
 *     land in the right window.
 */
import { describe, it, expect } from "bun:test";
import { Side } from "./helpers/setup";
import { Lifecycle } from "./helpers/lifecycle";
import { nowTs } from "./helpers/setup";

describe("Lifecycle helper regression", () => {
  it("warp helpers honor custom revealAfter/resolveAfter/settleAfter", async () => {
    // Compressed windows: 5s submit, 5s reveal, 5s settle.
    // Default helper would have used 30/60/90, which would land WAY
    // past every deadline and fail every subsequent constraint.
    const lc = await Lifecycle.create({
      side: Side.Buy,
      revealAfter: 5,
      resolveAfter: 10,
      settleAfter: 15,
    });

    // Stored deadlines should reflect the custom offsets.
    expect(lc.revealDeadline.toNumber()).toBe(lc.t0 + 5);
    expect(lc.resolveDeadline.toNumber()).toBe(lc.t0 + 10);
    expect(lc.settleDeadline.toNumber()).toBe(lc.t0 + 15);

    // submit_quote works pre-reveal (clock at t0).
    await lc.submitQuote();

    // warpToReveal should land at revealDeadline + 1 = t0 + 6.
    await lc.warpToReveal();
    expect(await nowTs(lc.ctx)).toBe(lc.t0 + 6);

    // reveal_quote needs clock in [reveal_deadline, resolve_deadline)
    // = [t0+5, t0+10). We're at t0+6, so this should succeed.
    await lc.revealQuote();

    // warpToSettle should land at resolveDeadline + 1 = t0 + 11.
    await lc.warpToSettle();
    expect(await nowTs(lc.ctx)).toBe(lc.t0 + 11);

    // fund_maker_escrow needs clock in [resolve_deadline, settle_deadline)
    // = [t0+10, t0+15). We're at t0+11, so this should succeed.
    await lc.fundMakerEscrow();

    // settle inside settle window.
    await lc.settle();

    // Final state.
    const intent = await lc.ctx.program.account.intent.fetch(lc.intent);
    expect(intent.status).toBe(2); // Settled

    // Sanity: warpPastSettleDeadline lands at settleDeadline + 1.
    // (Test isn't strictly meaningful post-settle but proves the helper
    // arithmetic is correct.)
    const lc2 = await Lifecycle.create({
      side: Side.Buy,
      revealAfter: 5,
      resolveAfter: 10,
      settleAfter: 15,
    });
    await lc2.warpPastSettleDeadline();
    expect(await nowTs(lc2.ctx)).toBe(lc2.t0 + 16);
  });

  it("default timings still produce 30/60/90 offsets", async () => {
    // Regression test for the original behavior: when no custom timing
    // is passed, defaults match the happy-path tests (which all use
    // these implicit values).
    const lc = await Lifecycle.create({ side: Side.Buy });
    expect(lc.revealDeadline.toNumber()).toBe(lc.t0 + 30);
    expect(lc.resolveDeadline.toNumber()).toBe(lc.t0 + 60);
    expect(lc.settleDeadline.toNumber()).toBe(lc.t0 + 90);
  });
});
