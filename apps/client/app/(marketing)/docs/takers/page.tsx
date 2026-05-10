import {
  A,
  Callout,
  Code,
  H2,
  LI,
  P,
  PageHead,
  Pager,
  Strong,
  UL,
} from "../_components";

export const metadata = {
  title: "For takers · Nyxbid",
  description: "How to post a sealed-bid intent and watch quotes arrive.",
};

/**
 * Takers guide. Tight, action-oriented. Five sections — pre-flight,
 * post, watch, settle, troubleshoot. No protocol-internals dump.
 */
export default function TakersPage() {
  return (
    <article>
      <PageHead
        eyebrow="Trade · taker"
        title={<>Post a sealed intent.</>}
        description={
          <>
            You&rsquo;re a taker if you want to fill size and you
            don&rsquo;t care which maker fills it — only that the
            best valid price wins. The whole flow is four steps and
            two clicks.
          </>
        }
      />

      <H2 id="before">Before you start</H2>
      <UL>
        <LI>
          A wallet with the asset you want to sell (or USDC, if
          you&rsquo;re buying).
        </LI>
        <LI>
          A small SOL balance for transaction fees — usually under{" "}
          <Code>0.001 SOL</Code> per trade.
        </LI>
        <LI>
          A rough idea of your <Strong>max slippage</Strong> — the
          worst price you&rsquo;d still accept.
        </LI>
      </UL>

      <H2 id="post">1. Post the intent</H2>
      <P>
        Open <A href="/trade">the trade form</A> and fill in:
      </P>
      <UL>
        <LI>
          <Strong>Side</Strong> — buy or sell the base asset.
        </LI>
        <LI>
          <Strong>Pair</Strong> — currently <Code>SOL/USDC</Code>{" "}
          and SPL tokens whitelisted by the venue.
        </LI>
        <LI>
          <Strong>Size</Strong> — the amount of base asset.
        </LI>
        <LI>
          <Strong>Limit price</Strong> — the worst price
          you&rsquo;ll accept. Quotes outside this band are
          rejected automatically.
        </LI>
        <LI>
          <Strong>Deadline</Strong> — how long makers have to quote
          and reveal. 60–120 seconds is typical for liquid pairs.
        </LI>
      </UL>
      <P>
        Sign one transaction. Your taker leg is escrowed on the spot
        and the intent goes live for makers.
      </P>

      <Callout kind="info" title="What gets revealed">
        Only your <Strong>side, pair, size, and deadline</Strong>{" "}
        are public. Your limit price is private until reveal — makers
        quote blind.
      </Callout>

      <H2 id="watch">2. Watch quotes arrive</H2>
      <P>
        Makers stream the open intent over WebSocket and submit hash
        commitments. The intent detail page shows commitments as
        they land — a count, the average commitment age, and the
        current quoter set, but never the prices themselves.
      </P>
      <UL>
        <LI>
          <Strong>Commit phase</Strong> — anyone can commit, no
          price visible.
        </LI>
        <LI>
          <Strong>Reveal phase</Strong> — committed makers reveal{" "}
          <Code>(price, size, nonce)</Code> on-chain. The program
          verifies the hash matches.
        </LI>
        <LI>
          <Strong>Award</Strong> — best valid revealed price wins
          automatically. There&rsquo;s no manual selection.
        </LI>
      </UL>

      <H2 id="settle">3. Settle</H2>
      <P>
        Once a winner is awarded, both legs swap inside a single
        transaction. You&rsquo;ll see your fill receipt on the same
        intent page within a couple of seconds of the award.
      </P>
      <P>
        If no maker reveals before the deadline, your escrow is
        refunded automatically — no manual cancel, no stuck funds.
      </P>

      <H2 id="risks">Risks you should know</H2>
      <UL>
        <LI>
          <Strong>No-fill risk.</Strong> If no maker quotes inside
          your limit, your intent expires and you get your escrow
          back. You don&rsquo;t pay for a failed RFQ.
        </LI>
        <LI>
          <Strong>Maker drop-off.</Strong> A maker who commits but
          fails to reveal forfeits a small bond to you. Drop-off is
          rare on liquid pairs but priced into your slippage.
        </LI>
        <LI>
          <Strong>Block-time tail.</Strong> Settlement still has to
          land on Solana. Set deadlines a few seconds longer than
          your block-time tolerance to avoid expiring on chain.
        </LI>
      </UL>

      <Pager
        prev={{ href: "/docs", label: "Overview" }}
        next={{ href: "/docs/makers", label: "For makers" }}
      />
    </article>
  );
}
