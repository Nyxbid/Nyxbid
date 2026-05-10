//! Agent2Agent (A2A) discovery surface for Nyxbid.
//!
//! Per the A2A spec (a2a-protocol.org), an agent advertises itself
//! through a JSON document served at `/.well-known/agent-card.json`.
//! Other agents fetch this card to learn:
//!
//! - the agent's identity and human-friendly name,
//! - which transports + URLs it accepts ("preferred" + "additional"),
//! - the skills it exposes (here: post a sealed intent, submit a
//!   sealed quote, drive a settlement), and
//! - the security schemes a peer can use.
//!
//! Nyxbid runs as a *headless* RFQ venue — the heavy lifting (key
//! custody, signing) lives with the wallet/agent; the venue's role is
//! to publish the catalog and route prepared transactions. The agent
//! card therefore advertises **transaction-prep skills** rather than
//! "do the trade for me" skills, which keeps custody and policy with
//! the caller's agent.
//!
//! The card is generated dynamically so the public URL and program ID
//! follow the live deployment (`PUBLIC_BASE_URL`, `NYXBID_PROGRAM_ID`).

use axum::{extract::State, http::header, response::IntoResponse, Json};
use serde::Serialize;

use crate::state::SharedState;

/// The on-the-wire shape of `/.well-known/agent-card.json`. Field
/// names follow the A2A `AgentCard` schema verbatim so consumers that
/// load the card with a typed client decode without a remap layer.
#[derive(Debug, Serialize)]
pub struct AgentCard {
    pub protocol_version: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    pub url: String,
    pub preferred_transport: &'static str,
    pub additional_interfaces: Vec<TransportInterface>,
    pub provider: Provider,
    pub version: &'static str,
    pub default_input_modes: Vec<&'static str>,
    pub default_output_modes: Vec<&'static str>,
    pub capabilities: Capabilities,
    pub skills: Vec<Skill>,
    pub security_schemes: SecuritySchemes,
    pub supports_authenticated_extended_card: bool,
    /// Nyxbid-specific extension fields. Not part of the canonical A2A
    /// spec, but lets agents discover the on-chain anchor (program ID,
    /// cluster) without an extra `/health` round trip.
    pub extensions: NyxbidExtensions,
}

#[derive(Debug, Serialize)]
pub struct TransportInterface {
    pub transport: &'static str,
    pub url: String,
}

#[derive(Debug, Serialize)]
pub struct Provider {
    pub organization: &'static str,
    pub url: &'static str,
}

#[derive(Debug, Serialize)]
pub struct Capabilities {
    pub streaming: bool,
    pub push_notifications: bool,
}

#[derive(Debug, Serialize)]
pub struct Skill {
    pub id: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    pub tags: Vec<&'static str>,
    pub examples: Vec<&'static str>,
    pub input_modes: Vec<&'static str>,
    pub output_modes: Vec<&'static str>,
}

#[derive(Debug, Serialize)]
pub struct SecuritySchemes {
    pub none: SecurityNone,
}

/// We don't gate the catalog or tx-prep behind any auth right now;
/// callers authenticate themselves *to the chain* by signing the
/// returned transaction with their own wallet/keypair. This is the
/// correct shape for an A2A agent that hands work back to the caller.
#[derive(Debug, Serialize)]
pub struct SecurityNone {
    #[serde(rename = "type")]
    pub kind: &'static str,
    pub description: &'static str,
}

#[derive(Debug, Serialize)]
pub struct NyxbidExtensions {
    pub solana_program_id: Option<String>,
    /// Public origin of the operator's Solana RPC endpoint —
    /// **without credentials**. Some providers (OrbitFlare, Helius,
    /// Triton) embed the API key in a `?api_key=...` query string on
    /// the URL the operator drops into `SOLANA_RPC_URL`. The agent
    /// card is served unauthenticated and cached publicly, so we
    /// strip everything from `?` onward before it leaves the
    /// process. Agents that want to talk to the chain themselves
    /// should bring their own RPC URL anyway; this field is
    /// informational, not a relay credential.
    pub cluster_rpc_url: Option<String>,
}

/// `GET /.well-known/agent-card.json`. Cache for 60s — the card
/// rarely changes but agents may discover us aggressively.
pub async fn agent_card(State(state): State<SharedState>) -> impl IntoResponse {
    let s = state.read().await;
    let base_url =
        std::env::var("PUBLIC_BASE_URL").unwrap_or_else(|_| "http://localhost:8080".to_string());
    let card = AgentCard {
        protocol_version: "0.3.0",
        name: "Nyxbid",
        description: "Sealed-bid RFQ venue for OTC-size trades on Solana. \
                      Atomic settlement on chain. Agent-native discovery via A2A.",
        url: format!("{base_url}/api"),
        preferred_transport: "JSONRPC",
        additional_interfaces: vec![
            TransportInterface {
                transport: "REST",
                url: format!("{base_url}/api"),
            },
            TransportInterface {
                transport: "WebSocket",
                url: format!("{base_url}/ws"),
            },
        ],
        provider: Provider {
            organization: "Nyxbid",
            url: "https://github.com/Nyxbid/Nyxbid",
        },
        version: env!("CARGO_PKG_VERSION"),
        default_input_modes: vec!["application/json"],
        default_output_modes: vec!["application/json"],
        capabilities: Capabilities {
            streaming: true,
            push_notifications: false,
        },
        skills: vec![
            Skill {
                id: "post_intent",
                name: "Post sealed intent",
                description:
                    "Build an unsigned `create_intent` transaction. Caller signs with \
                     their own wallet and broadcasts. Returns the deterministic Intent \
                     PDA so the caller can subscribe to its lifecycle before the tx lands.",
                tags: vec!["solana", "rfq", "otc", "intent"],
                examples: vec![
                    "Post a buy intent for 50 SOL with limit 195 USDC valid for 60s.",
                    "Open an RFQ to sell 100k USDC of SOL with reveal in 30s.",
                ],
                input_modes: vec!["application/json"],
                output_modes: vec!["application/json"],
            },
            Skill {
                id: "submit_quote",
                name: "Submit sealed quote",
                description:
                    "Build an unsigned `submit_quote` transaction with a (price, size, \
                     nonce) commitment. Maker keeps the secret until reveal. The card \
                     consumer must already know the target Intent PDA.",
                tags: vec!["solana", "maker", "rfq", "commitment"],
                examples: vec!["Quote 50 SOL @ 193.40 USDC against intent 8aF...."],
                input_modes: vec!["application/json"],
                output_modes: vec!["application/json"],
            },
            Skill {
                id: "reveal_quote",
                name: "Reveal sealed quote",
                description:
                    "Build an unsigned `reveal_quote` transaction. Reveals (price, size, \
                     nonce) within the resolve window. The chain replaces the running \
                     winner if this reveal improves the price.",
                tags: vec!["solana", "maker", "reveal"],
                examples: vec![],
                input_modes: vec!["application/json"],
                output_modes: vec!["application/json"],
            },
            Skill {
                id: "fund_maker_escrow",
                name: "Lock maker leg",
                description:
                    "Build an unsigned `fund_maker_escrow` transaction. The winning \
                     maker locks their leg into the per-intent vault. After this, \
                     anyone may settle.",
                tags: vec!["solana", "maker", "escrow"],
                examples: vec![],
                input_modes: vec!["application/json"],
                output_modes: vec!["application/json"],
            },
            Skill {
                id: "settle",
                name: "Settle atomically",
                description:
                    "Build an unsigned `settle` transaction that performs the dual-leg \
                     SPL transfer, mints a Receipt PDA, and (for buy intents) refunds \
                     any taker overpay. Permissionless — any payer can drive it.",
                tags: vec!["solana", "settlement", "atomic"],
                examples: vec![],
                input_modes: vec!["application/json"],
                output_modes: vec!["application/json"],
            },
            Skill {
                id: "stream_events",
                name: "Stream chain events",
                description:
                    "Subscribe to the WebSocket interface for real-time chain events \
                     (IntentCreated, QuoteSubmitted, QuoteRevealed, AuctionResolved, \
                     Settled, Cancelled). Useful for maker bots that watch every new \
                     intent and react with a quote.",
                tags: vec!["solana", "stream", "events"],
                examples: vec![],
                input_modes: vec!["application/json"],
                output_modes: vec!["application/json"],
            },
        ],
        security_schemes: SecuritySchemes {
            none: SecurityNone {
                kind: "noAuth",
                description:
                    "Catalog and tx-prep are public; authority is asserted by signing \
                     the returned transaction with the caller's own keypair.",
            },
        },
        supports_authenticated_extended_card: false,
        extensions: NyxbidExtensions {
            solana_program_id: s.solana.as_ref().map(|x| x.program_id.to_string()),
            cluster_rpc_url: s
                .solana
                .as_ref()
                .map(|x| public_origin(&x.rpc_url)),
        },
    };

    (
        [
            (header::CONTENT_TYPE, "application/json"),
            (header::CACHE_CONTROL, "public, max-age=60"),
        ],
        Json(card),
    )
}

/// Strip everything from `?` onward — i.e. the query-string portion
/// that some RPC providers use to carry an API key. Used before the
/// rpc URL is published in the public agent card.
fn public_origin(url: &str) -> String {
    match url.split_once('?') {
        Some((origin, _)) => origin.to_string(),
        None => url.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::public_origin;

    #[test]
    fn public_origin_strips_api_key_query() {
        assert_eq!(
            public_origin("https://ams.rpc.orbitflare.com?api_key=ORBIT-XXX"),
            "https://ams.rpc.orbitflare.com",
        );
    }

    #[test]
    fn public_origin_keeps_url_when_no_query() {
        assert_eq!(
            public_origin("https://api.devnet.solana.com"),
            "https://api.devnet.solana.com",
        );
    }

    #[test]
    fn public_origin_strips_only_first_question_mark() {
        // pathological case — keep the host portion, drop the rest.
        assert_eq!(
            public_origin("https://rpc.example.com/foo?a=1?b=2"),
            "https://rpc.example.com/foo",
        );
    }
}
