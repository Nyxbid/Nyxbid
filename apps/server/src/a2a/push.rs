//! A2A v1 push-notification config + webhook delivery.
//!
//! Clients that don't want to keep an SSE connection open register a
//! webhook URL via `tasks/pushNotificationConfig/set`. When the task
//! transitions state or accumulates a new artifact, we POST a
//! `TaskStatusUpdateEvent` or `TaskArtifactUpdateEvent` to that URL.
//!
//! Multiple configs per task are allowed; each gets its own opaque
//! `id` so individual configs can be deleted without taking down the
//! rest. Authentication is a simple bearer token attached to outgoing
//! POSTs — the spec also defines OAuth2/OpenID variants which are
//! out of scope for this build.
//!
//! Delivery is fire-and-forget: a single failed POST does not retry,
//! but `tracing::warn!` records the failure so operators can see it.

use std::collections::HashMap;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::RwLock;

/// Optional auth attached to webhook deliveries. The spec calls this
/// `pushNotificationAuthenticationInfo`; we implement only the
/// `bearer` shape.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PushNotificationAuth {
    /// Free-form auth scheme name. Only `"bearer"` is recognised.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scheme: Option<String>,
    /// Bearer credential transmitted as `Authorization: Bearer <token>`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub credentials: Option<String>,
}

/// One webhook configuration as exchanged on the wire.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PushNotificationConfig {
    /// Server-assigned identifier. The set-handler fills this in if
    /// the client omits it; subsequent get/list/delete refer back to it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// Public URL the server will POST updates to.
    pub url: String,
    /// Optional opaque token included in outgoing payload `metadata`.
    /// Spec §10.5: lets the receiver correlate webhook events with
    /// the original request.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub token: Option<String>,
    /// Optional bearer auth applied to webhook POSTs.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub authentication: Option<PushNotificationAuth>,
}

/// Wire shape for `tasks/pushNotificationConfig/set` and friends. The
/// spec uses this triplet for every push-notification method; we mirror
/// it verbatim.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskPushNotificationConfig {
    pub task_id: String,
    pub push_notification_config: PushNotificationConfig,
}

/// In-memory map: task id -> registered configs.
#[derive(Clone, Default)]
pub struct PushNotificationStore {
    inner: Arc<RwLock<HashMap<String, Vec<PushNotificationConfig>>>>,
}

impl PushNotificationStore {
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert or replace a config. The returned config is the same
    /// payload with `id` populated.
    pub async fn set(
        &self,
        task_id: &str,
        mut config: PushNotificationConfig,
    ) -> PushNotificationConfig {
        let id = config
            .id
            .clone()
            .unwrap_or_else(|| format!("pn_{}", uuid::Uuid::new_v4()));
        config.id = Some(id.clone());

        let mut guard = self.inner.write().await;
        let entry = guard.entry(task_id.to_string()).or_default();
        if let Some(existing) = entry.iter_mut().find(|c| c.id.as_deref() == Some(&id)) {
            *existing = config.clone();
        } else {
            entry.push(config.clone());
        }
        config
    }

    pub async fn list(&self, task_id: &str) -> Vec<PushNotificationConfig> {
        self.inner
            .read()
            .await
            .get(task_id)
            .cloned()
            .unwrap_or_default()
    }

    pub async fn get(
        &self,
        task_id: &str,
        config_id: &str,
    ) -> Option<PushNotificationConfig> {
        self.inner
            .read()
            .await
            .get(task_id)
            .and_then(|v| v.iter().find(|c| c.id.as_deref() == Some(config_id)).cloned())
    }

    /// Returns true if a matching entry was removed.
    pub async fn delete(&self, task_id: &str, config_id: &str) -> bool {
        let mut guard = self.inner.write().await;
        let Some(v) = guard.get_mut(task_id) else {
            return false;
        };
        let before = v.len();
        v.retain(|c| c.id.as_deref() != Some(config_id));
        let changed = v.len() != before;
        if v.is_empty() {
            guard.remove(task_id);
        }
        changed
    }

    /// Fire-and-forget every webhook configured for `task_id`.
    /// `payload` is the JSON body — typically a serialised
    /// `TaskStatusUpdateEvent` or `TaskArtifactUpdateEvent`.
    pub async fn fire(&self, task_id: &str, payload: Value) {
        let configs = self.list(task_id).await;
        if configs.is_empty() {
            return;
        }
        for cfg in configs {
            let body = wrap_payload(&cfg, payload.clone());
            tokio::spawn(async move {
                deliver(cfg, body).await;
            });
        }
    }
}

fn wrap_payload(cfg: &PushNotificationConfig, payload: Value) -> Value {
    // Mirror the spec's recommendation: include the client's correlation
    // token under `metadata.token` so the receiver can route safely.
    let mut object = serde_json::json!({ "event": payload });
    if let Some(token) = &cfg.token {
        if let Some(o) = object.as_object_mut() {
            o.insert(
                "metadata".to_string(),
                serde_json::json!({ "token": token }),
            );
        }
    }
    object
}

async fn deliver(cfg: PushNotificationConfig, body: Value) {
    let client = match reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            tracing::warn!(error = %e, url = %cfg.url, "push: client build failed");
            return;
        }
    };
    let mut req = client.post(&cfg.url).json(&body);
    if let Some(auth) = &cfg.authentication {
        if matches!(auth.scheme.as_deref(), Some("bearer")) {
            if let Some(token) = &auth.credentials {
                req = req.bearer_auth(token);
            }
        }
    }
    match req.send().await {
        Ok(resp) if resp.status().is_success() => {
            tracing::debug!(url = %cfg.url, status = %resp.status(), "push: delivered");
        }
        Ok(resp) => {
            tracing::warn!(url = %cfg.url, status = %resp.status(), "push: non-2xx response");
        }
        Err(e) => {
            tracing::warn!(error = %e, url = %cfg.url, "push: delivery failed");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg(url: &str) -> PushNotificationConfig {
        PushNotificationConfig {
            id: None,
            url: url.to_string(),
            token: Some("tok123".to_string()),
            authentication: None,
        }
    }

    #[tokio::test]
    async fn set_assigns_id_when_missing() {
        let store = PushNotificationStore::new();
        let saved = store.set("task-1", cfg("https://example.com")).await;
        assert!(saved.id.is_some());
    }

    #[tokio::test]
    async fn list_round_trips_configs() {
        let store = PushNotificationStore::new();
        store.set("task-1", cfg("https://a")).await;
        store.set("task-1", cfg("https://b")).await;
        assert_eq!(store.list("task-1").await.len(), 2);
    }

    #[tokio::test]
    async fn delete_removes_config_and_returns_true() {
        let store = PushNotificationStore::new();
        let saved = store.set("task-1", cfg("https://a")).await;
        let id = saved.id.unwrap();
        assert!(store.delete("task-1", &id).await);
        assert!(store.list("task-1").await.is_empty());
    }

    #[tokio::test]
    async fn delete_unknown_returns_false() {
        let store = PushNotificationStore::new();
        assert!(!store.delete("task-1", "missing").await);
    }

    #[test]
    fn wrap_payload_includes_token_metadata() {
        let body = wrap_payload(&cfg("https://x"), serde_json::json!({"k":1}));
        assert_eq!(body["metadata"]["token"], "tok123");
        assert_eq!(body["event"]["k"], 1);
    }
}
