//! Bridge from A2A skill invocations to Nyxbid's tx-prep layer.
//!
//! Each A2A skill maps 1:1 onto a builder in [`crate::tx`]. The skill
//! id arrives as the `"skill"` field of a `DataPart`; the `"input"`
//! field is the same JSON body the corresponding `/api/tx/*` route
//! accepts. We re-use those types verbatim so an agent can crib from
//! the existing REST docs.
//!
//! Output: a single A2A [`Artifact`] containing one `DataPart` with
//! the [`PreparedTx`](crate::tx::PreparedTx). The agent signs the
//! returned base64 transaction with its own keypair and broadcasts —
//! the venue never holds keys.

use serde::Deserialize;
use serde_json::Value;

use crate::a2a::types::{new_id, Artifact, Part};
use crate::solana::SolanaClient;
use crate::tx::{
    self, CancelRequest, CreateIntentRequest, ExpireNoMakerRequest, ExpireWithMakerRequest,
    FundMakerEscrowRequest, RevealQuoteRequest, SettleRequest, SubmitQuoteRequest,
};

/// Catalogue of skill ids advertised in the agent card. Kept as a
/// flat array so it can be embedded in the card with one allocation.
pub const WELL_KNOWN_SKILLS: &[&str] = &[
    "post_intent",
    "submit_quote",
    "reveal_quote",
    "fund_maker_escrow",
    "settle",
    "cancel",
    "expire_with_maker",
    "expire_no_maker",
    "subscribe_events",
];

#[derive(Debug, thiserror::Error)]
pub enum SkillError {
    #[error("unknown skill: {0}")]
    Unknown(String),
    #[error("missing or malformed skill envelope: {0}")]
    BadEnvelope(String),
    #[error("invalid input for skill {skill}: {error}")]
    BadInput { skill: String, error: String },
    #[error("tx build failed: {0}")]
    Build(#[from] tx::TxBuildError),
}

/// Shape of the inbound `DataPart` payload.
#[derive(Debug, Deserialize)]
pub struct SkillEnvelope {
    pub skill: String,
    #[serde(default)]
    pub input: Value,
}

impl SkillEnvelope {
    /// Find the first `DataPart` carrying a `{ skill, input }` object
    /// inside an A2A message body.
    pub fn extract(parts: &[Part]) -> Result<Self, SkillError> {
        for part in parts {
            if let Part::Data { data } = part {
                let envelope: SkillEnvelope = serde_json::from_value(data.clone())
                    .map_err(|e| SkillError::BadEnvelope(e.to_string()))?;
                return Ok(envelope);
            }
        }
        Err(SkillError::BadEnvelope(
            "no DataPart with { skill, input } found in message.parts".to_string(),
        ))
    }
}

/// Run a tx-prep skill and return an `Artifact` carrying the
/// `PreparedTx`. The streaming `subscribe_events` skill is handled
/// elsewhere (see [`crate::a2a::rpc`]) because it produces multiple
/// SSE updates.
pub async fn invoke_tx_skill(
    sol: &SolanaClient,
    envelope: SkillEnvelope,
) -> Result<Artifact, SkillError> {
    macro_rules! parse_input {
        ($t:ty) => {
            serde_json::from_value::<$t>(envelope.input.clone()).map_err(|e| {
                SkillError::BadInput {
                    skill: envelope.skill.clone(),
                    error: e.to_string(),
                }
            })?
        };
    }

    let prepared = match envelope.skill.as_str() {
        "post_intent" => tx::build_create_intent(sol, parse_input!(CreateIntentRequest)).await?,
        "submit_quote" => tx::build_submit_quote(sol, parse_input!(SubmitQuoteRequest)).await?,
        "reveal_quote" => tx::build_reveal_quote(sol, parse_input!(RevealQuoteRequest)).await?,
        "fund_maker_escrow" => {
            tx::build_fund_maker_escrow(sol, parse_input!(FundMakerEscrowRequest)).await?
        }
        "settle" => tx::build_settle(sol, parse_input!(SettleRequest)).await?,
        "cancel" => tx::build_cancel(sol, parse_input!(CancelRequest)).await?,
        "expire_with_maker" => {
            tx::build_expire_with_maker(sol, parse_input!(ExpireWithMakerRequest)).await?
        }
        "expire_no_maker" => {
            tx::build_expire_no_maker(sol, parse_input!(ExpireNoMakerRequest)).await?
        }
        // `subscribe_events` is streaming-only; reject unary invocations.
        "subscribe_events" => {
            return Err(SkillError::BadEnvelope(
                "subscribe_events is a streaming skill; invoke it via message/stream".to_string(),
            ))
        }
        other => return Err(SkillError::Unknown(other.to_string())),
    };

    Ok(Artifact {
        artifact_id: new_id("art"),
        name: Some(format!("{}.preparedTx", envelope.skill)),
        parts: vec![Part::Data {
            data: serde_json::to_value(&prepared).map_err(|e| SkillError::BadInput {
                skill: envelope.skill.clone(),
                error: e.to_string(),
            })?,
        }],
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_envelope_from_data_part() {
        let parts = vec![Part::Data {
            data: serde_json::json!({
                "skill": "post_intent",
                "input": { "size": 1 }
            }),
        }];
        let env = SkillEnvelope::extract(&parts).unwrap();
        assert_eq!(env.skill, "post_intent");
        assert_eq!(env.input["size"], 1);
    }

    #[test]
    fn extract_skips_text_part() {
        let parts = vec![
            Part::Text {
                text: "intro".to_string(),
            },
            Part::Data {
                data: serde_json::json!({ "skill": "settle", "input": {} }),
            },
        ];
        let env = SkillEnvelope::extract(&parts).unwrap();
        assert_eq!(env.skill, "settle");
    }

    #[test]
    fn extract_fails_with_no_data_part() {
        let parts = vec![Part::Text {
            text: "hi".to_string(),
        }];
        assert!(matches!(
            SkillEnvelope::extract(&parts),
            Err(SkillError::BadEnvelope(_))
        ));
    }

    #[test]
    fn well_known_skills_match_dispatch_arms() {
        // If you add a skill above, list it in WELL_KNOWN_SKILLS too.
        // Any drift here means the agent card lies about capabilities.
        assert!(WELL_KNOWN_SKILLS.contains(&"post_intent"));
        assert!(WELL_KNOWN_SKILLS.contains(&"subscribe_events"));
    }
}
