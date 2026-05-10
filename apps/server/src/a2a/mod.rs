//! A2A (Agent2Agent) protocol surface for Nyxbid.
//!
//! Implementation of the [Google A2A v1
//! specification](https://github.com/a2aproject/A2A). Surface served:
//!
//! | route                                     | role                            |
//! |-------------------------------------------|---------------------------------|
//! | `GET /.well-known/agent-card.json`        | public, JWS-signed if a key is set |
//! | `GET /.well-known/jwks.json`              | public verification key set     |
//! | `GET /agent/authenticatedExtendedCard`    | extended card (same shape today)|
//! | `POST /api/a2a/v1`                        | JSON-RPC 2.0 dispatcher         |
//!
//! JSON-RPC methods implemented:
//!
//! - `message/send`
//! - `message/stream` (SSE)
//! - `tasks/get`
//! - `tasks/cancel`
//! - `tasks/resubscribe` (SSE — replay state, then continue stream)
//! - `tasks/pushNotificationConfig/{set,get,list,delete}`
//!
//! ## Skill payload convention
//!
//! Skills piggy-back on A2A's `DataPart`. The first part of the
//! inbound `message.parts` array is expected to be:
//!
//! ```json
//! { "kind": "data", "data": { "skill": "post_intent", "input": { … } } }
//! ```
//!
//! and the response artifact contains a single `DataPart` with the
//! `PreparedTx` payload returned by the existing tx-prep layer.
//!
//! ## Auth
//!
//! `securitySchemes: {}` and `security: []` — no auth on the discovery
//! or RPC surface. On-chain authority comes from the caller signing
//! the returned transaction with their own keypair, so the venue
//! never needs to authenticate API requests. OAuth / OpenID Connect
//! schemes can be slotted in by populating those two fields without
//! changing the rest of the implementation.
//!
//! ## Card signing
//!
//! Set `A2A_SIGNING_KEY_PEM` to a PKCS#8 PEM-encoded ES256 private
//! key to enable JWS-signed cards (A2A §8.4). The public JWK is
//! derived automatically and exposed at `/.well-known/jwks.json`.
//! Without the env var the card is served unsigned and the JWKS is
//! empty.

pub mod card;
pub mod jws;
pub mod push;
pub mod rpc;
pub mod skills;
pub mod tasks;
pub mod types;

pub use card::{agent_card, extended_agent_card, jwks};
pub use push::PushNotificationStore;
pub use rpc::{rpc_handler, A2A_RPC_PATH};
pub use tasks::TaskStore;
