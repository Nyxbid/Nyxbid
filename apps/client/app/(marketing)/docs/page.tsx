import {
  A,
  Code,
  H2,
  LI,
  P,
  PageHead,
  Pager,
  Strong,
  UL,
} from "./_components";

export const metadata = {
  title: "Nyxbid · Docs",
  description:
    "How to trade on Nyxbid — sealed-bid OTC RFQ on Solana.",
};

/**
 * Overview — the only page most readers will ever load. Five short
 * sections that answer the "what / why / who / how do I start"
 * questions in <2 minutes of reading. No SDK reference, no folder
 * dump. Deeper how-tos live in /docs/takers and /docs/makers.
 */
export default function DocsOverview() {
  return (
    <article>
      <PageHead
        eyebrow="Welcome"
        title={
          <>
            Trade in size, <em>without showing your hand.</em>
          </>
        }
        description={
          <>
            Nyxbid is a sealed-bid RFQ venue for OTC-size trades on
            Solana. Takers post a single sealed intent; makers commit
            blinded quotes; the best valid quote wins and both legs
            settle in one transaction.
          </>
        }
      />

      <H2 id="what">What Nyxbid is</H2>
      <P>
        A private auction for trades that are too big for an AMM
        without slippage and too noisy for a public order book.
        Instead of broadcasting your hand to a mempool full of MEV
        bots, you post one sealed RFQ and let makers compete in the
        dark.
      </P>
      <UL>
        <LI>
          <Strong>Sealed</Strong> — makers commit a hash of{" "}
          <Code>(price, size, nonce)</Code>. Nothing leaks until the
          reveal window opens.
        </LI>
        <LI>
          <Strong>Atomic</Strong> — both legs swap inside one Solana
          transaction. No half-fills, no settlement risk.
        </LI>
        <LI>
          <Strong>Agent-native</Strong> — discovery happens through
          Google&rsquo;s A2A protocol, so maker bots can find the
          venue, subscribe to events, and reply to RFQs without an
          API key.
        </LI>
      </UL>

      <H2 id="who">Who this is for</H2>
      <P>
        If you&rsquo;re moving more than a few thousand dollars at a
        time and you don&rsquo;t want a public mempool to know about
        it before you&rsquo;ve filled, Nyxbid is built for you.
      </P>
      <UL>
        <LI>
          <Strong>Takers</Strong> — funds, treasuries, OTC desks, or
          any agent that needs to fill a single block of size without
          tipping the market.
        </LI>
        <LI>
          <Strong>Makers</Strong> — market-making bots, prop desks,
          or agents that hold inventory and want to quote private
          flow on demand.
        </LI>
      </UL>

      <H2 id="get-started">Get started</H2>
      <P>
        Connect a wallet (Phantom, Solflare, or Backpack), open the
        app, and either post your first intent as a taker or watch
        the live RFQ feed as a maker. There is no signup, no API
        key, no relayer trust.
      </P>
      <UL>
        <LI>
          <A href="/docs/takers">For takers</A> — how to post a
          sealed intent and watch quotes arrive.
        </LI>
        <LI>
          <A href="/docs/makers">For makers</A> — how to commit,
          reveal, fund, and settle a winning quote.
        </LI>
        <LI>
          <A href="/docs/agents">Agent integration</A> — discover
          the venue over A2A and run a maker bot.
        </LI>
      </UL>

      <H2 id="anatomy">Anatomy of a trade</H2>
      <P>
        Every trade follows the same three movements. The whole
        cycle takes a single block to a single minute, depending on
        the deadlines you set.
      </P>
      <ol className="mt-5 space-y-3 pl-5 [&>li]:list-decimal">
        <LI>
          <Strong>Post.</Strong> Taker broadcasts a sealed RFQ for{" "}
          <Code>buy X amount of Y for at most Z</Code> with a reveal
          deadline. Their leg is escrowed on the spot.
        </LI>
        <LI>
          <Strong>Quote.</Strong> Makers stream open intents and
          submit hash commitments of{" "}
          <Code>(price, size, nonce)</Code>. The book stays private
          until the auction closes.
        </LI>
        <LI>
          <Strong>Reveal &amp; settle.</Strong> Best valid quote
          wins. The winner reveals, funds their leg, and both legs
          swap atomically.
        </LI>
      </ol>

      <H2 id="security">Security model</H2>
      <UL>
        <LI>
          <Strong>Non-custodial.</Strong> The protocol never holds
          your keys. Every state transition is a signed Solana
          transaction.
        </LI>
        <LI>
          <Strong>Hash commitments.</Strong> Quotes are SHA-256
          commitments of <Code>price · size · nonce</Code>. A maker
          who fails to reveal forfeits their bond; a taker who walks
          away forfeits their escrow.
        </LI>
        <LI>
          <Strong>Atomic escrow.</Strong> Both legs fund into a
          program-owned escrow before the swap fires. There is no
          step where one side has settled and the other has not.
        </LI>
      </UL>

      <Pager next={{ href: "/docs/takers", label: "For takers" }} />
    </article>
  );
}
