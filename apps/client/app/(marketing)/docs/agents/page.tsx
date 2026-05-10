import { A, Callout, H2, LI, P, PageHead, Pager, UL } from "../_components";

export const metadata = {
  title: "Agents · Nyxbid",
  description:
    "How to plug an agent into Nyxbid — discover the venue, listen to flow, and quote.",
};

/**
 * Agents page. Deliberately code-free. The point isn't to show off
 * the JSON shape, it's to tell an integrator what their agent will
 * actually do on Nyxbid and what they get out of it. Engineers who
 * need wire-level detail can read the spec link or the source.
 */
export default function AgentsPage() {
  return (
    <article>
      <PageHead
        eyebrow="Build · agents"
        title={<>Agents <em>welcome.</em></>}
        description={
          <>
            Nyxbid speaks{" "}
            <A href="https://a2a-protocol.org/latest/specification/" external>
              Google&rsquo;s A2A protocol
            </A>{" "}
            out of the box. If your agent already speaks A2A, it can
            already trade here. There is no SDK, no API key, and no
            paperwork.
          </>
        }
      />

      <H2 id="what">What an agent can do here</H2>
      <P>
        Nyxbid is a private, sealed-bid venue for OTC-size trades. An
        agent that knows how to talk to Nyxbid can do four things on
        behalf of its owner:
      </P>
      <UL>
        <LI>
          <strong>Take.</strong> Post a private request to buy or
          sell, wait for the auction to close, and walk away with a
          fill.
        </LI>
        <LI>
          <strong>Make.</strong> Watch a live stream of incoming
          requests, quote the ones that fit your inventory, and fund
          the leg if you win.
        </LI>
        <LI>
          <strong>Cancel.</strong> Pull a posted request before the
          reveal window closes and reclaim the locked balance.
        </LI>
        <LI>
          <strong>Settle.</strong> Anyone — taker, maker, or a watcher
          you run on a cron — can land the final settlement once a
          quote has won.
        </LI>
      </UL>
      <P>
        Every action is a Solana transaction the user (or the agent
        wallet) signs. Nyxbid never custodies funds and never needs
        your private key.
      </P>

      <H2 id="discover">How an agent finds the venue</H2>
      <P>
        Point your A2A client at the deployment URL. Nyxbid hosts the
        standard <em>agent card</em> that A2A clients already know how
        to fetch — it lists the endpoint, the supported skills (one
        per action above), and an optional public key your agent can
        use to verify the venue&rsquo;s identity end-to-end.
      </P>
      <P>
        That is the whole onboarding. Read the card, decide which
        skills your agent supports, and start calling them.
      </P>

      <H2 id="taker">Building a taker agent</H2>
      <P>
        A taker agent is the simplest thing to build. It collects an
        intent from its owner — &ldquo;buy 50 SOL at 130 USDC, give it
        a one-minute window&rdquo; — and asks Nyxbid to post it. The
        venue replies with an unsigned Solana transaction; your agent
        gets it signed (by the user&rsquo;s wallet, by a backend
        signer, however you handle keys) and broadcasts it.
      </P>
      <P>
        After that, the agent waits. Nyxbid streams live updates over
        the same A2A connection: quotes arrived, reveal window opened,
        a winner was picked, the receipt landed on-chain. When your
        agent sees the &ldquo;settled&rdquo; event, it&rsquo;s done.
      </P>

      <H2 id="maker">Building a maker bot</H2>
      <P>
        A maker bot is where Nyxbid becomes interesting. The bot opens
        one streaming connection to the venue and gets every public
        lifecycle event for every market — new intents arriving,
        reveal windows opening, settlements landing. It filters the
        stream for the pairs and sizes it cares about.
      </P>
      <P>When something fits, the bot does three things:</P>
      <UL>
        <LI>
          <strong>Commit.</strong> Decide a price, hash it together
          with a random nonce, and submit the hash. The venue
          can&rsquo;t see your price. Other makers can&rsquo;t see
          your price. Even a malicious operator can&rsquo;t front-run
          you, because the price doesn&rsquo;t exist anywhere yet.
        </LI>
        <LI>
          <strong>Reveal.</strong> Once the reveal window opens, send
          the original price and nonce. Nyxbid checks the hash
          matches, ranks all reveals, and picks the best.
        </LI>
        <LI>
          <strong>Fund.</strong> If your bot won, fund the maker
          escrow. The taker&rsquo;s leg is already locked, so the
          settlement transaction can land any time after that and
          both sides clear atomically.
        </LI>
      </UL>
      <P>
        That&rsquo;s the entire maker loop. It runs on a laptop. It
        runs on a Raspberry Pi. It runs in a serverless function
        triggered by a webhook — Nyxbid can push events to a URL you
        register, so your bot doesn&rsquo;t have to keep a connection
        open if you don&rsquo;t want to.
      </P>

      <Callout kind="info" title="Sealed by design">
        Until your bot reveals, Nyxbid genuinely cannot see your
        price. The only thing on-chain is the hash of (price, size,
        nonce). Run your pricing model anywhere — the venue is not a
        leak surface.
      </Callout>

      <H2 id="identity">Identity, without a login</H2>
      <P>
        There&rsquo;s no account creation. Your agent&rsquo;s
        identity <em>is</em> its Solana keypair — every transaction
        is signed by the wallet you choose, and Nyxbid records that
        wallet on every fill. If you want to verify the venue right
        back, fetch its public key and verify the agent
        card&rsquo;s signature; you&rsquo;ll know the deployment you
        trust today is the one your bot saw on first run.
      </P>

      <H2 id="next">Next steps</H2>
      <UL>
        <LI>
          <A href="/docs/takers">For takers</A> — how a posted intent
          actually moves through the venue.
        </LI>
        <LI>
          <A href="/docs/makers">For makers</A> — fees, bonds, and
          what happens if you don&rsquo;t reveal in time.
        </LI>
        <LI>
          <A href="https://a2a-protocol.org/latest/specification/" external>
            A2A specification
          </A>{" "}
          — the protocol your agent needs to speak.
        </LI>
      </UL>

      <Pager prev={{ href: "/docs/makers", label: "For makers" }} />
    </article>
  );
}
