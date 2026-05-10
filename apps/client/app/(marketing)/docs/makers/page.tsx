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
  title: "For makers · Nyxbid",
  description: "How to commit, reveal, fund, and settle a winning quote.",
};

/**
 * Makers guide. Operational, not theoretical — five sections that
 * walk a maker from "I see an open RFQ" to "my quote settled".
 * Bot-builders should read /docs/agents next.
 */
export default function MakersPage() {
  return (
    <article>
      <PageHead
        eyebrow="Trade · maker"
        title={<>Quote sealed flow.</>}
        description={
          <>
            You&rsquo;re a maker if you hold inventory and want to
            quote private RFQs on demand. Every RFQ is one taker,
            one window, and one winner — first reveal in-band wins.
          </>
        }
      />

      <H2 id="before">Before you start</H2>
      <UL>
        <LI>
          A wallet with inventory in the assets you intend to quote.
        </LI>
        <LI>
          A small SOL balance for fees and a maker bond — typically
          a few cents per intent you commit to.
        </LI>
        <LI>
          A way to receive open intents in real time. The dashboard
          works for manual quoting; bots should connect over
          WebSocket.
        </LI>
      </UL>

      <H2 id="watch">1. Watch open RFQs</H2>
      <P>
        Open <A href="/intents">the maker inbox</A> or stream{" "}
        <Code>ws://&lt;host&gt;/ws</Code>. You&rsquo;ll see every
        open intent: side, pair, size, deadline. The taker&rsquo;s
        limit price is hidden, so quote conservatively.
      </P>
      <Callout kind="note" title="What you see">
        Side, pair, size, deadline, and the running list of
        committed maker counts. Nothing else leaks until reveal.
      </Callout>

      <H2 id="commit">2. Commit a sealed quote</H2>
      <P>
        Pick a price you&rsquo;d be happy to fill at and a random
        nonce. Hash it locally:
      </P>
      <pre className="mt-5 overflow-x-auto rounded-lg border border-[var(--hairline)] bg-[var(--surface)] px-4 py-3 font-mono text-[12.5px] leading-relaxed text-foreground">
        <code>{`commitment = SHA-256( price ‖ size ‖ nonce )`}</code>
      </pre>
      <P>
        Sign one transaction posting{" "}
        <Code>{"{ intent_id, commitment, bond }"}</Code> to the
        program. Your commitment is now locked. You can&rsquo;t
        change the price; you can only choose to reveal it or to
        forfeit the bond.
      </P>

      <H2 id="reveal">3. Reveal during the window</H2>
      <P>
        Once the reveal window opens, post the original{" "}
        <Code>(price, size, nonce)</Code> on-chain. The program
        verifies the SHA-256 matches your commitment and ranks all
        revealed quotes.
      </P>
      <UL>
        <LI>
          <Strong>Best valid price wins.</Strong> Best meaning
          best-for-the-taker — lowest ask if they&rsquo;re buying,
          highest bid if they&rsquo;re selling.
        </LI>
        <LI>
          <Strong>Ties</Strong> break on commitment timestamp.
          Earlier commitments rank ahead.
        </LI>
        <LI>
          <Strong>Stale or out-of-band quotes</Strong> are dropped
          and don&rsquo;t cost you the bond — you only forfeit if
          you fail to reveal.
        </LI>
      </UL>

      <H2 id="settle">4. Fund &amp; settle</H2>
      <P>
        If you win, the program emits an award event and waits for
        you to fund the maker leg. One last signed transaction
        moves your inventory into escrow; both legs swap atomically;
        the receipt drops on-chain.
      </P>

      <Callout kind="warn" title="Don't ghost a win">
        Winning and not funding the leg before the settle deadline
        is the worst thing a maker can do — the bond goes to the
        taker, the slot is replaced by the next-best valid quote,
        and your reputation score on the venue takes a hit.
      </Callout>

      <H2 id="economics">Economics</H2>
      <UL>
        <LI>
          <Strong>Fees.</Strong> Currently zero protocol fee on
          devnet. Mainnet launch fee is a flat{" "}
          <Code>0.05%</Code> on the maker leg, paid in the quote
          asset.
        </LI>
        <LI>
          <Strong>Bonds.</Strong> A small SOL bond is locked when
          you commit and refunded on either reveal or expiry.
          Forfeits go to the counter-party that took the
          drop-off, not the protocol.
        </LI>
        <LI>
          <Strong>Inventory.</Strong> The protocol never custodies
          your inventory between trades. You hold what you hold,
          and you fund the leg only if your quote wins.
        </LI>
      </UL>

      <Pager
        prev={{ href: "/docs/takers", label: "For takers" }}
        next={{ href: "/docs/agents", label: "Agent integration" }}
      />
    </article>
  );
}
