//! Wire types for the A2A v1 protocol.
//!
//! Field names follow the A2A v1 spec verbatim (camelCase via serde),
//! so a typed A2A client built against the upstream JSON schema can
//! decode our responses without a remap layer.

use serde::{Deserialize, Serialize};

// ---- AgentCard ---------------------------------------------------------

/// Top-level agent card document served at
/// `/.well-known/agent-card.json`.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentCard {
    pub name: &'static str,
    pub description: &'static str,
    pub supported_interfaces: Vec<AgentInterface>,
    pub provider: AgentProvider,
    pub icon_url: Option<&'static str>,
    pub version: &'static str,
    pub documentation_url: Option<String>,
    pub capabilities: AgentCapabilities,
    pub security_schemes: serde_json::Value,
    pub security: Vec<serde_json::Value>,
    pub default_input_modes: Vec<&'static str>,
    pub default_output_modes: Vec<&'static str>,
    pub skills: Vec<AgentSkill>,
    /// Nyxbid-specific extension. Kept under a vendor URI so it can't
    /// collide with future official fields.
    #[serde(rename = "x-nyxbid")]
    pub nyxbid: NyxbidExtension,
}

/// One transport binding for the agent. The current build advertises
/// only `JSONRPC` over HTTP+JSON; gRPC support could be slotted in by
/// adding another entry here.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentInterface {
    pub url: String,
    pub protocol_binding: &'static str,
    pub protocol_version: &'static str,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentProvider {
    pub organization: &'static str,
    pub url: &'static str,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentCapabilities {
    pub streaming: bool,
    pub push_notifications: bool,
    pub state_transition_history: bool,
    pub extended_agent_card: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentSkill {
    pub id: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    pub tags: Vec<&'static str>,
    pub examples: Vec<&'static str>,
    pub input_modes: Vec<&'static str>,
    pub output_modes: Vec<&'static str>,
}

/// Vendor extension. Populated from runtime config so an A2A client
/// learns the program ID and (credential-stripped) cluster RPC origin
/// without an extra `/health` round-trip.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NyxbidExtension {
    pub program_id: Option<String>,
    pub cluster_rpc_url: Option<String>,
    pub well_known_skills: &'static [&'static str],
}

// ---- Messages + Parts --------------------------------------------------

/// Either user input (request) or agent output (response in history).
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Message {
    /// `"user"` or `"agent"`. Free string in the spec; we mirror the
    /// canonical two-value enumeration.
    pub role: String,
    pub parts: Vec<Part>,
    pub message_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub task_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context_id: Option<String>,
}

/// One body fragment inside a [`Message`]. We use only `text` and `data`
/// in this implementation; `file` is declared for forward compatibility
/// with future skills that ship binary payloads.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum Part {
    Text { text: String },
    Data { data: serde_json::Value },
    File { file: serde_json::Value },
}

// ---- Task lifecycle ----------------------------------------------------

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum TaskState {
    Submitted,
    Working,
    InputRequired,
    Completed,
    Canceled,
    Failed,
    Rejected,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskStatus {
    pub state: TaskState,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<Message>,
    /// RFC 3339 / ISO 8601 timestamp. We format with chrono.
    pub timestamp: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Artifact {
    pub artifact_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    pub parts: Vec<Part>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Task {
    pub id: String,
    pub context_id: String,
    pub status: TaskStatus,
    pub history: Vec<Message>,
    pub artifacts: Vec<Artifact>,
    /// Discriminator the spec uses to differentiate `Task` from
    /// `Message` payloads when both are valid responses.
    pub kind: &'static str,
}

impl Task {
    pub fn new(id: String, context_id: String, initial: Message) -> Self {
        Self {
            id,
            context_id,
            status: TaskStatus {
                state: TaskState::Submitted,
                message: None,
                timestamp: now_iso(),
            },
            history: vec![initial],
            artifacts: Vec::new(),
            kind: "task",
        }
    }

    pub fn set_state(&mut self, state: TaskState) {
        self.status = TaskStatus {
            state,
            message: None,
            timestamp: now_iso(),
        };
    }

    pub fn fail(&mut self, message: &str) {
        self.status = TaskStatus {
            state: TaskState::Failed,
            message: Some(Message {
                role: "agent".to_string(),
                parts: vec![Part::Text {
                    text: message.to_string(),
                }],
                message_id: new_id("msg"),
                task_id: Some(self.id.clone()),
                context_id: Some(self.context_id.clone()),
            }),
            timestamp: now_iso(),
        };
    }
}

// ---- Streaming events --------------------------------------------------

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskStatusUpdateEvent {
    pub task_id: String,
    pub context_id: String,
    pub kind: &'static str, // always "status-update"
    pub status: TaskStatus,
    pub r#final: bool,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskArtifactUpdateEvent {
    pub task_id: String,
    pub context_id: String,
    pub kind: &'static str, // always "artifact-update"
    pub artifact: Artifact,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub append: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_chunk: Option<bool>,
}

// ---- helpers -----------------------------------------------------------

pub fn now_iso() -> String {
    chrono::Utc::now().to_rfc3339()
}

/// Generate an opaque ID. The spec requires it to be a string; UUIDs
/// are convenient and short enough.
pub fn new_id(prefix: &str) -> String {
    format!("{prefix}_{}", uuid::Uuid::new_v4())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Field names must serialize as camelCase to satisfy the v1 spec.
    #[test]
    fn agent_card_serializes_camel_case() {
        let card = AgentCard {
            name: "Nyxbid",
            description: "test",
            supported_interfaces: vec![],
            provider: AgentProvider {
                organization: "Nyxbid",
                url: "https://example.com",
            },
            icon_url: None,
            version: "0.1.0",
            documentation_url: None,
            capabilities: AgentCapabilities {
                streaming: true,
                push_notifications: false,
                state_transition_history: false,
                extended_agent_card: false,
            },
            security_schemes: serde_json::json!({}),
            security: vec![],
            default_input_modes: vec!["application/json"],
            default_output_modes: vec!["application/json"],
            skills: vec![],
            nyxbid: NyxbidExtension {
                program_id: None,
                cluster_rpc_url: None,
                well_known_skills: &[],
            },
        };
        let json = serde_json::to_value(&card).unwrap();
        assert!(json.get("supportedInterfaces").is_some());
        assert!(json.get("defaultInputModes").is_some());
        assert!(json.get("securitySchemes").is_some());
        assert!(json.get("supported_interfaces").is_none());
    }

    /// Part variants tag with `kind` and lowercase the discriminator.
    #[test]
    fn part_serializes_with_kind_tag() {
        let p = Part::Data {
            data: serde_json::json!({ "skill": "ping" }),
        };
        let v = serde_json::to_value(&p).unwrap();
        assert_eq!(v["kind"], "data");
        assert_eq!(v["data"]["skill"], "ping");
    }

    #[test]
    fn task_state_uses_kebab_case() {
        let s = serde_json::to_value(&TaskState::InputRequired).unwrap();
        assert_eq!(s.as_str(), Some("input-required"));
    }
}
