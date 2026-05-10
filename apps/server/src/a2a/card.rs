//! Discovery surface — `/.well-known/agent-card.json`,
//! `/.well-known/jwks.json`, and `/agent/authenticatedExtendedCard`.
//!
//! The card is generated dynamically so the public URL and program ID
//! follow the live deployment (`PUBLIC_BASE_URL`, `NYXBID_PROGRAM_ID`).
//! When `A2A_SIGNING_KEY_PEM` is configured the card is signed with
//! ES256 + JCS-canonicalised payload (A2A §8.4); see [`super::jws`].

use axum::{
    extract::State,
    http::header::{CACHE_CONTROL, CONTENT_TYPE},
    response::IntoResponse,
    Json,
};

use crate::state::SharedState;
use crate::url_privacy::public_origin;

use super::jws::maybe_sign;
use super::skills::WELL_KNOWN_SKILLS;
use super::types::{
    AgentCapabilities, AgentCard, AgentInterface, AgentProvider, AgentSkill, NyxbidExtension,
};

/// `GET /.well-known/agent-card.json` — public card. Cached briefly so
/// crawlers and discovery indexers don't hammer the route.
pub async fn agent_card(State(state): State<SharedState>) -> impl IntoResponse {
    let value = build_card(&state, false).await;
    (
        [
            (CONTENT_TYPE, "application/json"),
            (CACHE_CONTROL, "public, max-age=60"),
        ],
        Json(value),
    )
}

/// `GET /agent/authenticatedExtendedCard` — A2A v1 §6.10. In a strict
/// deployment this would gate behind a `securitySchemes` entry; the
/// current build serves the same content as the public card with
/// `extendedAgentCard: true` so spec-strict clients can still call
/// the route.
pub async fn extended_agent_card(State(state): State<SharedState>) -> impl IntoResponse {
    let value = build_card(&state, true).await;
    (
        [
            (CONTENT_TYPE, "application/json"),
            // Extended cards may carry sensitive data — don't cache.
            (CACHE_CONTROL, "no-store"),
        ],
        Json(value),
    )
}

/// `GET /.well-known/jwks.json` — public verification keys for the
/// JWS-signed agent card. Empty key set when card signing is disabled.
pub async fn jwks() -> impl IntoResponse {
    (
        [
            (CONTENT_TYPE, "application/jwk-set+json"),
            (CACHE_CONTROL, "public, max-age=300"),
        ],
        Json(super::jws::derive_jwks()),
    )
}

async fn build_card(state: &SharedState, extended: bool) -> serde_json::Value {
    let s = state.read().await;
    let base_url =
        std::env::var("PUBLIC_BASE_URL").unwrap_or_else(|_| "http://localhost:8080".to_string());

    let card = AgentCard {
        name: "Nyxbid",
        description: "Sealed-bid RFQ venue for OTC-size trades on Solana. \
                      Atomic settlement on chain. Agent-native via A2A v1.",
        supported_interfaces: vec![AgentInterface {
            url: format!("{base_url}{}", super::A2A_RPC_PATH),
            protocol_binding: "JSONRPC",
            protocol_version: "1.0",
        }],
        provider: AgentProvider {
            organization: "Nyxbid",
            url: "https://github.com/Nyxbid/Nyxbid",
        },
        icon_url: None,
        version: env!("CARGO_PKG_VERSION"),
        documentation_url: Some(format!("{base_url}/docs/agents")),
        capabilities: AgentCapabilities {
            streaming: true,
            push_notifications: true,
            state_transition_history: true,
            // Always advertise the extended card; the route is wired
            // up regardless of which build_card call we're inside.
            extended_agent_card: true,
        },
        // No auth on either card today. To require auth on the
        // extended card, populate `security_schemes` and `security`
        // here (e.g. with a Bearer scheme) and gate the
        // `extended_agent_card` handler accordingly.
        security_schemes: serde_json::json!({}),
        security: vec![],
        default_input_modes: vec!["application/json"],
        default_output_modes: vec!["application/json"],
        skills: nyxbid_skills(),
        nyxbid: NyxbidExtension {
            program_id: s.solana.as_ref().map(|x| x.program_id.to_string()),
            cluster_rpc_url: s.solana.as_ref().map(|x| public_origin(&x.rpc_url)),
            well_known_skills: WELL_KNOWN_SKILLS,
        },
    };

    let mut value = serde_json::to_value(&card).unwrap_or(serde_json::Value::Null);

    // Mark extended cards distinctly so spec-strict clients can tell
    // them apart even though our content is identical today.
    if extended {
        if let Some(obj) = value.as_object_mut() {
            obj.insert("isExtended".to_string(), serde_json::Value::Bool(true));
        }
    }

    maybe_sign(value)
}

fn nyxbid_skills() -> Vec<AgentSkill> {
    vec![
        AgentSkill {
            id: "post_intent",
            name: "Post sealed intent",
            description: "Build an unsigned create_intent transaction. Caller signs with \
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
        AgentSkill {
            id: "submit_quote",
            name: "Submit sealed quote",
            description: "Build an unsigned submit_quote transaction with a (price, size, \
                          nonce) commitment. Maker keeps the secret until reveal.",
            tags: vec!["solana", "maker", "rfq", "commitment"],
            examples: vec!["Quote 50 SOL @ 193.40 USDC against intent 8aF...."],
            input_modes: vec!["application/json"],
            output_modes: vec!["application/json"],
        },
        AgentSkill {
            id: "reveal_quote",
            name: "Reveal sealed quote",
            description: "Build an unsigned reveal_quote transaction. Reveals (price, size, \
                          nonce) within the resolve window.",
            tags: vec!["solana", "maker", "reveal"],
            examples: vec![],
            input_modes: vec!["application/json"],
            output_modes: vec!["application/json"],
        },
        AgentSkill {
            id: "fund_maker_escrow",
            name: "Lock maker leg",
            description: "Build an unsigned fund_maker_escrow transaction. The winning \
                          maker locks their leg into the per-intent vault.",
            tags: vec!["solana", "maker", "escrow"],
            examples: vec![],
            input_modes: vec!["application/json"],
            output_modes: vec!["application/json"],
        },
        AgentSkill {
            id: "settle",
            name: "Settle atomically",
            description: "Build an unsigned settle transaction that performs the dual-leg \
                          SPL transfer, mints a Receipt PDA, and refunds taker overpay.",
            tags: vec!["solana", "settlement", "atomic"],
            examples: vec![],
            input_modes: vec!["application/json"],
            output_modes: vec!["application/json"],
        },
        AgentSkill {
            id: "cancel",
            name: "Cancel intent",
            description: "Build an unsigned cancel transaction. Taker reclaims their leg \
                          before the reveal deadline.",
            tags: vec!["solana", "taker", "cancel"],
            examples: vec![],
            input_modes: vec!["application/json"],
            output_modes: vec!["application/json"],
        },
        AgentSkill {
            id: "expire_with_maker",
            name: "Expire (maker funded)",
            description: "Permissionless expiry after the settle deadline when a maker \
                          funded but never settled. Refunds both legs.",
            tags: vec!["solana", "expiry"],
            examples: vec![],
            input_modes: vec!["application/json"],
            output_modes: vec!["application/json"],
        },
        AgentSkill {
            id: "expire_no_maker",
            name: "Expire (no maker)",
            description: "Permissionless expiry after the settle deadline when no maker \
                          funded the escrow. Refunds the taker.",
            tags: vec!["solana", "expiry"],
            examples: vec![],
            input_modes: vec!["application/json"],
            output_modes: vec!["application/json"],
        },
        AgentSkill {
            id: "subscribe_events",
            name: "Stream chain events",
            description: "Subscribe to real-time chain events (IntentCreated, \
                          QuoteSubmitted, QuoteRevealed, AuctionResolved, Settled, \
                          Cancelled). Streaming-only — invoke via message/stream.",
            tags: vec!["solana", "stream", "events"],
            examples: vec![],
            input_modes: vec!["application/json"],
            output_modes: vec!["text/event-stream", "application/json"],
        },
    ]
}
