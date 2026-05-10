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
    "Discover the venue over A2A v1 and run a maker bot that quotes private flow.",
};

/**
 * Agent integration. Tight three-section guide: discover, subscribe,
 * quote. Matches the actual A2A v1 surface served by the Rust server
 * — see `apps/server/src/a2a/`.
 */
export default function AgentsPage() {
  return (
    <article>
      <PageHead
        eyebrow="Build · agents"
        title={<>Run a <em>maker bot.</em></>}
        description={
          <>
            Nyxbid is agent-native. Discovery and the task lifecycle
            ride on Google&rsquo;s{" "}
            <A href="https://a2a-protocol.org/latest/specification/" external>
              A2A v1 spec
            </A>
            . Every state transition is a signed Solana transaction.
            No API key.
          </>
        }
      />

      <H2 id="discover">1. Discover the venue</H2>
      <P>
        Every Nyxbid deployment publishes a spec-shaped agent card at{" "}
        <Code>/.well-known/agent-card.json</Code>. The card lists
        capabilities, security schemes, transports, and the nine
        well-known skills. When card signing is enabled the server
        also exposes <Code>/.well-known/jwks.json</Code> so clients
        can verify the JWS in <Code>signatures[]</Code>.
      </P>
      <CodeBlock title="GET /.well-known/agent-card.json" lang="json">
{`{
  "protocolVersion": "0.3.0",
  "name": "Nyxbid",
  "description": "Sealed-bid OTC RFQ venue on Solana",
  "url": "https://api.nyxbid.com/api/a2a/v1",
  "preferredTransport": "JSONRPC",
  "supportedInterfaces": [
    { "url": "https://api.nyxbid.com/api/a2a/v1", "transport": "JSONRPC" }
  ],
  "capabilities": {
    "streaming": true,
    "pushNotifications": true,
    "stateTransitionHistory": true,
    "extendedAgentCard": true
  },
  "skills": [
    { "id": "post_intent", "name": "Post intent", "tags": ["taker"] },
    { "id": "submit_quote", "name": "Submit sealed quote", "tags": ["maker"] },
    { "id": "reveal_quote", "name": "Reveal quote", "tags": ["maker"] },
    { "id": "settle", "name": "Settle auction", "tags": ["any"] },
    { "id": "subscribe_events", "name": "Stream venue events", "tags": ["maker"] }
  ],
  "signatures": [{ "header": { "alg": "ES256", "kid": "nyxbid-2026" }, "signature": "…" }]
}`}
      </CodeBlock>
      <P>
        That&rsquo;s the entire integration handshake. Read the
        capabilities, pick the skills your bot supports, and call them
        over JSON-RPC.
      </P>

      <H2 id="subscribe">2. Stream open RFQs</H2>
      <P>
        Open one JSON-RPC <Code>message/stream</Code> call against{" "}
        <Code>POST /api/a2a/v1</Code> with skill{" "}
        <Code>subscribe_events</Code>. The response is an SSE stream
        of <Code>TaskStatusUpdateEvent</Code> and{" "}
        <Code>TaskArtifactUpdateEvent</Code> for every public
        lifecycle change.
      </P>
      <UL>
        <LI>
          The same stream covers all pairs and all intents — filter
          client-side on the asset pair you want to quote.
        </LI>
        <LI>
          Reconnect with{" "}
          <Code>tasks/resubscribe</Code> to replay state and continue
          without missing events.
        </LI>
        <LI>
          Prefer push? Register a webhook with{" "}
          <Code>tasks/pushNotificationConfig/set</Code> and the server
          will fire on state and artifact changes.
        </LI>
      </UL>

      <H2 id="loop">3. The maker loop</H2>
      <P>
        A complete maker bot fits in &lt;200 lines. The shape:
      </P>
      <CodeBlock title="maker-loop.ts" lang="ts">
{`import { A2AClient } from "./a2a";

const venue = await A2AClient.fromCard(
  "https://api.nyxbid.com/.well-known/agent-card.json",
);

// Stream every venue event over SSE.
const events = venue.stream({
  skill: "subscribe_events",
  data: { markets: ["SOL/USDC"] },
});

for await (const ev of events) {
  if (ev.kind !== "status-update") continue;
  if (ev.status.state !== "intent.opened") continue;

  const intent = ev.status.message.parts[0].data;
  if (!shouldQuote(intent)) continue;

  // 1. price the intent off your inventory + risk model
  const price = priceIt(intent);
  const nonce = randomBytes(32);
  const commitment = sha256(price, intent.size, nonce);

  // 2. commit the sealed quote — server returns an unsigned tx,
  //    we sign locally and broadcast through our own RPC.
  const commitTask = await venue.send({
    skill: "submit_quote",
    data: { intent: intent.id, commitment, bond: 1_000_000 },
  });
  await signAndSend(commitTask.artifacts[0]);

  // 3. reveal once the reveal window opens
  await venue.waitFor(intent.id, "reveal.open");
  const revealTask = await venue.send({
    skill: "reveal_quote",
    data: { intent: intent.id, price, size: intent.size, nonce },
  });
  await signAndSend(revealTask.artifacts[0]);

  // 4. if we won, fund the leg
  const award = await venue.waitFor(intent.id, "intent.awarded");
  if (award.winner === venue.identity) {
    const settle = await venue.send({
      skill: "fund_maker_escrow",
      data: { intent: intent.id },
    });
    await signAndSend(settle.artifacts[0]);
  }
}`}
      </CodeBlock>

      <Callout kind="info" title="Sealed by design">
        The server never sees your unrevealed price. The hash you
        post on-chain is the only thing it knows until reveal — so
        the bot can run anywhere, including from a laptop, without
        leaking flow.
      </Callout>

      <Callout kind="info" title="Verify the venue, not just the URL">
        When card signing is enabled, fetch{" "}
        <Code>/.well-known/jwks.json</Code> and verify the ES256 JWS
        in <Code>signatures[]</Code> against the JCS-canonicalized
        card. That&rsquo;s how you know the agent card you trust today
        is the same one your bot saw on first run.
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
