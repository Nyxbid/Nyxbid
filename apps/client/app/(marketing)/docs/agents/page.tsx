import {
  A,
  Callout,
  Code,
  CodeBlock,
  H2,
  LI,
  P,
  PageHead,
  Pager,
  UL,
} from "../_components";

export const metadata = {
  title: "Agent integration · Nyxbid",
  description:
    "Discover the venue over A2A and run a maker bot that quotes private flow.",
};

/**
 * Agent integration. Tight three-section guide: discover, subscribe,
 * quote. The code block is a sketch — small enough to read in one
 * sitting, large enough to show what the actual loop looks like.
 */
export default function AgentsPage() {
  return (
    <article>
      <PageHead
        eyebrow="Build · agents"
        title={<>Run a <em>maker bot.</em></>}
        description={
          <>
            Nyxbid is agent-native. Discovery rides on
            Google&rsquo;s A2A protocol, intents stream over a
            single WebSocket, and every state transition is a
            signed Solana transaction. No API key.
          </>
        }
      />

      <H2 id="discover">1. Discover the venue</H2>
      <P>
        Every Nyxbid deployment publishes an A2A agent card at{" "}
        <Code>/.well-known/agent.json</Code>. An agent that supports
        A2A discovery can find the venue, learn its capabilities,
        and start interacting without a registration step.
      </P>
      <CodeBlock title="GET /.well-known/agent.json" lang="json">
{`{
  "name": "Nyxbid",
  "description": "Sealed-bid OTC RFQ venue on Solana",
  "url": "https://nyxbid.app",
  "capabilities": [
    "intents.subscribe",
    "intents.post",
    "quotes.commit",
    "quotes.reveal"
  ],
  "transports": {
    "ws": "wss://nyxbid.app/ws",
    "rpc": "https://nyxbid.app/api"
  }
}`}
      </CodeBlock>
      <P>
        That&rsquo;s the entire integration handshake. Read the
        capabilities, pick the ones your bot supports, and connect.
      </P>

      <H2 id="subscribe">2. Subscribe to open RFQs</H2>
      <P>
        Open one WebSocket. The server pushes <Code>intent.opened</Code>,{" "}
        <Code>quote.committed</Code>, <Code>quote.revealed</Code>,{" "}
        <Code>intent.awarded</Code>, and{" "}
        <Code>intent.settled</Code> events for every public lifecycle
        change.
      </P>
      <UL>
        <LI>
          The same stream covers all pairs and all intents — filter
          client-side on the asset pair you want to quote.
        </LI>
        <LI>
          Reconnect with backoff and replay missed events from the
          REST endpoint <Code>GET /api/intents?since=&lt;slot&gt;</Code>.
        </LI>
      </UL>

      <H2 id="loop">3. The maker loop</H2>
      <P>
        A complete maker bot fits in &lt;150 lines. The shape:
      </P>
      <CodeBlock title="maker-loop.ts" lang="ts">
{`import { connectVenue, signCommitment, revealQuote } from "./venue";

const venue = await connectVenue("wss://nyxbid.app/ws");

for await (const ev of venue.events()) {
  if (ev.type !== "intent.opened") continue;
  if (!shouldQuote(ev.intent)) continue;

  // 1. price the intent off your inventory + risk model
  const price = price(ev.intent);
  const nonce = randomBytes(32);
  const commitment = sha256(price, ev.intent.size, nonce);

  // 2. commit the sealed quote
  await venue.commit({
    intentId: ev.intent.id,
    commitment,
    bond: 0.001 * LAMPORTS_PER_SOL,
  });

  // 3. wait for the reveal window, then reveal
  await venue.waitFor("reveal.open", ev.intent.id);
  await venue.reveal({
    intentId: ev.intent.id,
    price,
    size: ev.intent.size,
    nonce,
  });

  // 4. if we won, fund the leg
  const award = await venue.waitFor("intent.awarded", ev.intent.id);
  if (award.winner === venue.identity) {
    await venue.fundLeg(ev.intent.id);
  }
}`}
      </CodeBlock>

      <Callout kind="info" title="Sealed by design">
        The server never sees your unrevealed price. The hash you
        post on-chain is the only thing it knows until reveal — so
        the bot can run anywhere, including from a laptop, without
        leaking flow.
      </Callout>

      <H2 id="next">Next</H2>
      <UL>
        <LI>
          <A href="/docs/makers">Maker mechanics</A> — fees, bonds,
          forfeits.
        </LI>
        <LI>
          <A href="/docs/takers">Taker flow</A> — what your bot is
          quoting against on the other side.
        </LI>
      </UL>

      <Pager prev={{ href: "/docs/makers", label: "For makers" }} />
    </article>
  );
}
