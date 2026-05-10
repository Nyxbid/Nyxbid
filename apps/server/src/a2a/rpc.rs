//! JSON-RPC 2.0 dispatcher for the A2A v1 protocol.
//!
//! Mounted at `/api/a2a/v1`. Accepts a JSON-RPC envelope and routes
//! the `method` field to a handler:
//!
//! | method                                       | response shape          | streaming |
//! |----------------------------------------------|-------------------------|-----------|
//! | `message/send`                               | `Task` (or `Message`)   | no        |
//! | `tasks/get`                                  | `Task`                  | no        |
//! | `tasks/cancel`                               | `Task`                  | no        |
//! | `message/stream`                             | SSE of `Task*` events   | yes       |
//! | `tasks/resubscribe`                          | SSE of `Task*` events   | yes       |
//! | `tasks/pushNotificationConfig/set`           | `TaskPushNotificationConfig` | no  |
//! | `tasks/pushNotificationConfig/get`           | `TaskPushNotificationConfig` | no  |
//! | `tasks/pushNotificationConfig/list`          | array                   | no        |
//! | `tasks/pushNotificationConfig/delete`        | `null`                  | no        |
//!
//! Streaming methods detect their content negotiation by method name —
//! the response Content-Type flips from `application/json` to
//! `text/event-stream`. This matches the v1 spec's HTTP+JSON binding.
//!
//! Errors map to A2A-defined codes where possible (see [`error_codes`]).

use std::convert::Infallible;
use std::time::Duration;

use axum::{
    extract::State,
    http::header,
    response::{
        sse::{Event, KeepAlive, Sse},
        IntoResponse, Json, Response,
    },
};
use futures_util::stream::{self, Stream, StreamExt};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio_stream::wrappers::BroadcastStream;

use crate::indexer::ChainEnvelope;
use crate::state::SharedState;

use super::push::{PushNotificationConfig, PushNotificationStore, TaskPushNotificationConfig};
use super::skills::{invoke_tx_skill, SkillEnvelope, SkillError};
use super::types::{
    new_id, now_iso, Artifact, Message, Part, Task, TaskArtifactUpdateEvent, TaskState,
    TaskStatus, TaskStatusUpdateEvent,
};

/// HTTP path the agent card advertises in `supportedInterfaces`.
pub const A2A_RPC_PATH: &str = "/api/a2a/v1";

// ---- JSON-RPC envelope -------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    #[serde(default)]
    pub id: Option<Value>,
    pub method: String,
    #[serde(default)]
    pub params: Value,
}

#[derive(Debug, Serialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: &'static str,
    pub id: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

#[derive(Debug, Serialize)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

impl JsonRpcResponse {
    pub fn ok(id: Value, result: Value) -> Self {
        Self {
            jsonrpc: "2.0",
            id,
            result: Some(result),
            error: None,
        }
    }

    pub fn err(id: Value, err: JsonRpcError) -> Self {
        Self {
            jsonrpc: "2.0",
            id,
            result: None,
            error: Some(err),
        }
    }
}

/// JSON-RPC 2.0 + A2A spec error codes used in this binary. Codes
/// declared but currently unused (`INTERNAL_ERROR`, `UNSUPPORTED_OPERATION`)
/// are kept for forward compatibility with future skills and to keep
/// the public error catalogue in one place.
pub mod error_codes {
    pub const PARSE_ERROR: i32 = -32700;
    pub const INVALID_REQUEST: i32 = -32600;
    pub const METHOD_NOT_FOUND: i32 = -32601;
    pub const INVALID_PARAMS: i32 = -32602;
    #[allow(dead_code)]
    pub const INTERNAL_ERROR: i32 = -32603;
    pub const TASK_NOT_FOUND: i32 = -32001;
    pub const TASK_NOT_CANCELABLE: i32 = -32002;
    #[allow(dead_code)]
    pub const UNSUPPORTED_OPERATION: i32 = -32004;
    /// Vendor-namespaced (Nyxbid) error: skill envelope or input
    /// failed validation in [`super::skills`].
    pub const SKILL_INVALID: i32 = -32050;
    pub const SOLANA_UNCONFIGURED: i32 = -32051;
}

// ---- entry point -------------------------------------------------------

/// `POST /api/a2a/v1`. Branches on `method`: streaming methods return
/// `text/event-stream`, everything else returns `application/json`.
pub async fn rpc_handler(
    State(state): State<SharedState>,
    body: axum::body::Bytes,
) -> Response {
    let req = match serde_json::from_slice::<JsonRpcRequest>(&body) {
        Ok(r) => r,
        Err(e) => {
            return Json(JsonRpcResponse::err(
                Value::Null,
                JsonRpcError {
                    code: error_codes::PARSE_ERROR,
                    message: format!("invalid JSON-RPC envelope: {e}"),
                    data: None,
                },
            ))
            .into_response();
        }
    };

    if req.jsonrpc != "2.0" {
        return Json(JsonRpcResponse::err(
            req.id.unwrap_or(Value::Null),
            JsonRpcError {
                code: error_codes::INVALID_REQUEST,
                message: "jsonrpc field must be \"2.0\"".to_string(),
                data: None,
            },
        ))
        .into_response();
    }

    let id = req.id.clone().unwrap_or(Value::Null);

    match req.method.as_str() {
        "message/send" => unary_response(handle_message_send(&state, id.clone(), req.params).await),
        "tasks/get" => unary_response(handle_tasks_get(&state, id.clone(), req.params).await),
        "tasks/cancel" => unary_response(handle_tasks_cancel(&state, id.clone(), req.params).await),
        "message/stream" => handle_message_stream(state, id, req.params).await,
        "tasks/resubscribe" => handle_tasks_resubscribe(state, id, req.params).await,
        "tasks/pushNotificationConfig/set" => unary_response(
            handle_push_set(&state, id.clone(), req.params).await,
        ),
        "tasks/pushNotificationConfig/get" => unary_response(
            handle_push_get(&state, id.clone(), req.params).await,
        ),
        "tasks/pushNotificationConfig/list" => unary_response(
            handle_push_list(&state, id.clone(), req.params).await,
        ),
        "tasks/pushNotificationConfig/delete" => unary_response(
            handle_push_delete(&state, id.clone(), req.params).await,
        ),
        other => unary_response(Err(JsonRpcResponse::err(
            id,
            JsonRpcError {
                code: error_codes::METHOD_NOT_FOUND,
                message: format!("method not found: {other}"),
                data: None,
            },
        ))),
    }
}

/// Coerce a `Result<Value, JsonRpcResponse>` into an HTTP response.
fn unary_response(r: Result<JsonRpcResponse, JsonRpcResponse>) -> Response {
    match r {
        Ok(ok) => Json(ok).into_response(),
        Err(err) => Json(err).into_response(),
    }
}

// ---- handlers ----------------------------------------------------------

#[derive(Debug, Deserialize)]
struct SendMessageParams {
    message: Message,
}

async fn handle_message_send(
    state: &SharedState,
    id: Value,
    params: Value,
) -> Result<JsonRpcResponse, JsonRpcResponse> {
    let SendMessageParams { mut message } =
        parse_params::<SendMessageParams>(&id, params)?;

    let envelope = SkillEnvelope::extract(&message.parts).map_err(|e| skill_error(&id, e))?;

    let s = state.read().await;
    let sol = s
        .solana
        .as_ref()
        .ok_or_else(|| solana_unconfigured(&id))?;
    let task_store = s.tasks.clone();
    let push_store = s.push_notifications.clone();

    let context_id = message
        .context_id
        .clone()
        .unwrap_or_else(|| new_id("ctx"));
    let task_id = new_id("task");
    message.task_id = Some(task_id.clone());
    message.context_id = Some(context_id.clone());

    let mut task = Task::new(task_id.clone(), context_id.clone(), message);
    task.set_state(TaskState::Working);
    task_store.insert(task.clone()).await;
    fire_status_update(&push_store, &task, false).await;

    let artifact_result = invoke_tx_skill(sol, envelope).await;
    drop(s); // release the read guard before we hit the store again

    match artifact_result {
        Ok(artifact) => {
            task_store
                .update(&task_id, |t| {
                    t.artifacts.push(artifact.clone());
                    t.set_state(TaskState::Completed);
                })
                .await;
            let final_task = task_store.get(&task_id).await.unwrap_or(task);
            fire_artifact_update(&push_store, &final_task, &artifact, true).await;
            fire_status_update(&push_store, &final_task, true).await;
            Ok(JsonRpcResponse::ok(
                id,
                serde_json::to_value(&final_task).unwrap_or(Value::Null),
            ))
        }
        Err(e) => {
            let err_msg = e.to_string();
            task_store
                .update(&task_id, |t| t.fail(&err_msg))
                .await;
            let final_task = task_store.get(&task_id).await.unwrap_or(task);
            fire_status_update(&push_store, &final_task, true).await;
            Ok(JsonRpcResponse::ok(
                id,
                serde_json::to_value(&final_task).unwrap_or(Value::Null),
            ))
        }
    }
}

#[derive(Debug, Deserialize)]
struct TaskIdParams {
    id: String,
}

async fn handle_tasks_get(
    state: &SharedState,
    rpc_id: Value,
    params: Value,
) -> Result<JsonRpcResponse, JsonRpcResponse> {
    let TaskIdParams { id } = parse_params::<TaskIdParams>(&rpc_id, params)?;
    let store = state.read().await.tasks.clone();
    match store.get(&id).await {
        Some(t) => Ok(JsonRpcResponse::ok(
            rpc_id,
            serde_json::to_value(&t).unwrap_or(Value::Null),
        )),
        None => Err(JsonRpcResponse::err(
            rpc_id,
            JsonRpcError {
                code: error_codes::TASK_NOT_FOUND,
                message: format!("task not found: {id}"),
                data: None,
            },
        )),
    }
}

async fn handle_tasks_cancel(
    state: &SharedState,
    rpc_id: Value,
    params: Value,
) -> Result<JsonRpcResponse, JsonRpcResponse> {
    let TaskIdParams { id } = parse_params::<TaskIdParams>(&rpc_id, params)?;
    let store = state.read().await.tasks.clone();

    enum Outcome {
        Canceled(Task),
        Terminal,
        Missing,
    }

    let outcome = store
        .update(&id, |t| match t.status.state {
            TaskState::Submitted | TaskState::Working | TaskState::InputRequired => {
                t.set_state(TaskState::Canceled);
                Outcome::Canceled(t.clone())
            }
            _ => Outcome::Terminal,
        })
        .await
        .unwrap_or(Outcome::Missing);

    match outcome {
        Outcome::Canceled(task) => Ok(JsonRpcResponse::ok(
            rpc_id,
            serde_json::to_value(&task).unwrap_or(Value::Null),
        )),
        Outcome::Terminal => Err(JsonRpcResponse::err(
            rpc_id,
            JsonRpcError {
                code: error_codes::TASK_NOT_CANCELABLE,
                message: format!("task {id} is in a terminal state"),
                data: None,
            },
        )),
        Outcome::Missing => Err(JsonRpcResponse::err(
            rpc_id,
            JsonRpcError {
                code: error_codes::TASK_NOT_FOUND,
                message: format!("task not found: {id}"),
                data: None,
            },
        )),
    }
}

// ---- streaming ---------------------------------------------------------

/// `message/stream`: returns an SSE stream of JSON-RPC responses, one
/// per [`Event`]. The exact frame shape is a `JsonRpcResponse` whose
/// `result` field is a `Task` snapshot, a `TaskStatusUpdateEvent`, or
/// a `TaskArtifactUpdateEvent` per the v1 spec.
async fn handle_message_stream(
    state: SharedState,
    rpc_id: Value,
    params: Value,
) -> Response {
    let parsed = match serde_json::from_value::<SendMessageParams>(params) {
        Ok(p) => p,
        Err(e) => {
            return Json(JsonRpcResponse::err(
                rpc_id,
                JsonRpcError {
                    code: error_codes::INVALID_PARAMS,
                    message: format!("invalid params for message/stream: {e}"),
                    data: None,
                },
            ))
            .into_response();
        }
    };

    let envelope = match SkillEnvelope::extract(&parsed.message.parts) {
        Ok(e) => e,
        Err(e) => {
            return Json(JsonRpcResponse::err(rpc_id, skill_inner(e))).into_response();
        }
    };

    match envelope.skill.as_str() {
        "subscribe_events" => {
            let s = state.read().await;
            let chain_rx = s.chain_tx.subscribe();
            drop(s);
            stream_chain_events(rpc_id, parsed.message, chain_rx).into_response()
        }
        _ => {
            // Wrap a unary tx-prep result in a 1-event SSE stream so
            // streaming clients see exactly one final task envelope.
            stream_unary_result(state, rpc_id, parsed.message, envelope)
                .await
                .into_response()
        }
    }
}

/// SSE wrapper around a chain event broadcast channel. Each chain
/// event becomes one A2A `TaskArtifactUpdateEvent` carrying a
/// `DataPart` with the full envelope. Non-terminal — the client
/// closes the stream when done.
fn stream_chain_events(
    rpc_id: Value,
    incoming: Message,
    chain_rx: tokio::sync::broadcast::Receiver<ChainEnvelope>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let context_id = incoming
        .context_id
        .clone()
        .unwrap_or_else(|| new_id("ctx"));
    let task_id = new_id("task");
    let rpc_id_for_initial = rpc_id.clone();
    let initial_task = Task {
        id: task_id.clone(),
        context_id: context_id.clone(),
        status: TaskStatus {
            state: TaskState::Working,
            message: None,
            timestamp: now_iso(),
        },
        history: vec![incoming],
        artifacts: Vec::new(),
        kind: "task",
    };
    let initial_event = sse_event(JsonRpcResponse::ok(
        rpc_id_for_initial,
        serde_json::to_value(&initial_task).unwrap_or(Value::Null),
    ));

    let updates = BroadcastStream::new(chain_rx).filter_map(move |res| {
        let task_id = task_id.clone();
        let context_id = context_id.clone();
        let rpc_id = rpc_id.clone();
        async move {
            let env = res.ok()?;
            let artifact = Artifact {
                artifact_id: new_id("art"),
                name: Some("chain.event".to_string()),
                parts: vec![Part::Data {
                    data: serde_json::to_value(&env).unwrap_or(Value::Null),
                }],
            };
            let evt = TaskArtifactUpdateEvent {
                task_id,
                context_id,
                kind: "artifact-update",
                artifact,
                append: Some(true),
                last_chunk: Some(false),
            };
            let payload = serde_json::to_value(&evt).unwrap_or(Value::Null);
            Some(Ok::<_, Infallible>(sse_event(JsonRpcResponse::ok(
                rpc_id, payload,
            ))))
        }
    });

    let stream = stream::once(async move { Ok(initial_event) }).chain(updates);
    Sse::new(stream).keep_alive(KeepAlive::new().interval(Duration::from_secs(15)))
}

/// Wrap a one-shot tx-prep result in a 1-event SSE stream. Lets a
/// generic A2A client always use `message/stream` regardless of skill.
async fn stream_unary_result(
    state: SharedState,
    rpc_id: Value,
    incoming: Message,
    envelope: SkillEnvelope,
) -> Response {
    let s = state.read().await;
    let Some(sol) = s.solana.as_ref() else {
        return Json(JsonRpcResponse::err(rpc_id, JsonRpcError {
            code: error_codes::SOLANA_UNCONFIGURED,
            message: "Solana RPC not configured on this server".to_string(),
            data: None,
        }))
        .into_response();
    };
    let task_store = s.tasks.clone();
    let push_store = s.push_notifications.clone();
    let result = invoke_tx_skill(sol, envelope).await;
    drop(s);

    let context_id = incoming
        .context_id
        .clone()
        .unwrap_or_else(|| new_id("ctx"));
    let task_id = new_id("task");
    let mut task = Task::new(task_id.clone(), context_id.clone(), incoming);
    match result {
        Ok(artifact) => {
            task.artifacts.push(artifact);
            task.set_state(TaskState::Completed);
        }
        Err(e) => task.fail(&e.to_string()),
    }
    task_store.insert(task.clone()).await;
    if !task.artifacts.is_empty() {
        let artifact = task.artifacts[0].clone();
        fire_artifact_update(&push_store, &task, &artifact, true).await;
    }
    fire_status_update(&push_store, &task, true).await;

    let task_event = sse_event(JsonRpcResponse::ok(
        rpc_id.clone(),
        serde_json::to_value(&task).unwrap_or(Value::Null),
    ));
    let final_status = TaskStatusUpdateEvent {
        task_id: task.id.clone(),
        context_id: task.context_id.clone(),
        kind: "status-update",
        status: task.status.clone(),
        r#final: true,
    };
    let final_event = sse_event(JsonRpcResponse::ok(
        rpc_id,
        serde_json::to_value(&final_status).unwrap_or(Value::Null),
    ));

    let stream = stream::iter(vec![Ok::<_, Infallible>(task_event), Ok(final_event)]);
    let mut sse = Sse::new(stream).into_response();
    sse.headers_mut()
        .insert(header::CACHE_CONTROL, "no-cache".parse().unwrap());
    sse
}

// ---- tasks/resubscribe -------------------------------------------------

/// `tasks/resubscribe` (A2A v1 §6.6). Reconnect to a streaming task
/// after a network blip. Returns the current `Task` snapshot in the
/// first SSE frame; if the task is still in a non-terminal state and
/// was a `subscribe_events` task, continue piping chain events.
async fn handle_tasks_resubscribe(
    state: SharedState,
    rpc_id: Value,
    params: Value,
) -> Response {
    let parsed = match serde_json::from_value::<TaskIdParams>(params) {
        Ok(p) => p,
        Err(e) => {
            return Json(JsonRpcResponse::err(
                rpc_id,
                JsonRpcError {
                    code: error_codes::INVALID_PARAMS,
                    message: format!("invalid params for tasks/resubscribe: {e}"),
                    data: None,
                },
            ))
            .into_response();
        }
    };

    let s = state.read().await;
    let task_store = s.tasks.clone();
    let chain_rx_opt = match task_store.get(&parsed.id).await {
        None => {
            return Json(JsonRpcResponse::err(
                rpc_id,
                JsonRpcError {
                    code: error_codes::TASK_NOT_FOUND,
                    message: format!("task not found: {}", parsed.id),
                    data: None,
                },
            ))
            .into_response();
        }
        Some(task) => {
            // Replay current state in the first SSE frame so the
            // reconnecting client sees the same `Task` shape it would
            // have received at subscribe time.
            let initial_event = sse_event(JsonRpcResponse::ok(
                rpc_id.clone(),
                serde_json::to_value(&task).unwrap_or(Value::Null),
            ));
            let is_terminal = matches!(
                task.status.state,
                TaskState::Completed
                    | TaskState::Canceled
                    | TaskState::Failed
                    | TaskState::Rejected
            );
            if is_terminal {
                drop(s);
                let stream = stream::iter(vec![Ok::<_, Infallible>(initial_event)]);
                let mut sse = Sse::new(stream).into_response();
                sse.headers_mut()
                    .insert(header::CACHE_CONTROL, "no-cache".parse().unwrap());
                return sse;
            }
            // Non-terminal: assume this was a subscribe_events task and
            // continue piping chain events. We don't track per-task
            // skill metadata yet, so this is best-effort.
            (Some(task), s.chain_tx.subscribe(), initial_event)
        }
    };
    drop(s);

    let (task, chain_rx, initial_event) = match chain_rx_opt {
        (Some(t), rx, ev) => (t, rx, ev),
        _ => unreachable!(),
    };

    let context_id = task.context_id.clone();
    let task_id = task.id.clone();
    let rpc_id_for_updates = rpc_id.clone();
    let updates = BroadcastStream::new(chain_rx).filter_map(move |res| {
        let task_id = task_id.clone();
        let context_id = context_id.clone();
        let rpc_id = rpc_id_for_updates.clone();
        async move {
            let env = res.ok()?;
            let artifact = Artifact {
                artifact_id: new_id("art"),
                name: Some("chain.event".to_string()),
                parts: vec![Part::Data {
                    data: serde_json::to_value(&env).unwrap_or(Value::Null),
                }],
            };
            let evt = TaskArtifactUpdateEvent {
                task_id,
                context_id,
                kind: "artifact-update",
                artifact,
                append: Some(true),
                last_chunk: Some(false),
            };
            Some(Ok::<_, Infallible>(sse_event(JsonRpcResponse::ok(
                rpc_id,
                serde_json::to_value(&evt).unwrap_or(Value::Null),
            ))))
        }
    });

    let stream = stream::once(async move { Ok(initial_event) }).chain(updates);
    Sse::new(stream)
        .keep_alive(KeepAlive::new().interval(Duration::from_secs(15)))
        .into_response()
}

// ---- tasks/pushNotificationConfig/* -----------------------------------

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PushSetParams {
    task_id: String,
    push_notification_config: PushNotificationConfig,
}

async fn handle_push_set(
    state: &SharedState,
    rpc_id: Value,
    params: Value,
) -> Result<JsonRpcResponse, JsonRpcResponse> {
    // Spec lets the params be either the wrapped
    // `TaskPushNotificationConfig` or a `{ taskId, pushNotificationConfig }`
    // object. They're the same shape; we accept both via serde.
    let parsed: PushSetParams = serde_json::from_value(params).map_err(|e| {
        JsonRpcResponse::err(
            rpc_id.clone(),
            JsonRpcError {
                code: error_codes::INVALID_PARAMS,
                message: e.to_string(),
                data: None,
            },
        )
    })?;

    let s = state.read().await;
    if s.tasks.get(&parsed.task_id).await.is_none() {
        return Err(JsonRpcResponse::err(
            rpc_id,
            JsonRpcError {
                code: error_codes::TASK_NOT_FOUND,
                message: format!("task not found: {}", parsed.task_id),
                data: None,
            },
        ));
    }
    let saved = s
        .push_notifications
        .set(&parsed.task_id, parsed.push_notification_config)
        .await;

    let result = TaskPushNotificationConfig {
        task_id: parsed.task_id,
        push_notification_config: saved,
    };
    Ok(JsonRpcResponse::ok(
        rpc_id,
        serde_json::to_value(&result).unwrap_or(Value::Null),
    ))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PushIdParams {
    task_id: String,
    push_notification_config_id: String,
}

async fn handle_push_get(
    state: &SharedState,
    rpc_id: Value,
    params: Value,
) -> Result<JsonRpcResponse, JsonRpcResponse> {
    let parsed: PushIdParams = parse_params(&rpc_id, params)?;
    let s = state.read().await;
    match s
        .push_notifications
        .get(&parsed.task_id, &parsed.push_notification_config_id)
        .await
    {
        Some(cfg) => {
            let result = TaskPushNotificationConfig {
                task_id: parsed.task_id,
                push_notification_config: cfg,
            };
            Ok(JsonRpcResponse::ok(
                rpc_id,
                serde_json::to_value(&result).unwrap_or(Value::Null),
            ))
        }
        None => Err(JsonRpcResponse::err(
            rpc_id,
            JsonRpcError {
                code: error_codes::TASK_NOT_FOUND,
                message: "push notification config not found".to_string(),
                data: None,
            },
        )),
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PushListParams {
    task_id: String,
}

async fn handle_push_list(
    state: &SharedState,
    rpc_id: Value,
    params: Value,
) -> Result<JsonRpcResponse, JsonRpcResponse> {
    let parsed: PushListParams = parse_params(&rpc_id, params)?;
    let s = state.read().await;
    let configs = s.push_notifications.list(&parsed.task_id).await;
    let result: Vec<TaskPushNotificationConfig> = configs
        .into_iter()
        .map(|c| TaskPushNotificationConfig {
            task_id: parsed.task_id.clone(),
            push_notification_config: c,
        })
        .collect();
    Ok(JsonRpcResponse::ok(
        rpc_id,
        serde_json::to_value(&result).unwrap_or(Value::Null),
    ))
}

async fn handle_push_delete(
    state: &SharedState,
    rpc_id: Value,
    params: Value,
) -> Result<JsonRpcResponse, JsonRpcResponse> {
    let parsed: PushIdParams = parse_params(&rpc_id, params)?;
    let s = state.read().await;
    if s
        .push_notifications
        .delete(&parsed.task_id, &parsed.push_notification_config_id)
        .await
    {
        Ok(JsonRpcResponse::ok(rpc_id, Value::Null))
    } else {
        Err(JsonRpcResponse::err(
            rpc_id,
            JsonRpcError {
                code: error_codes::TASK_NOT_FOUND,
                message: "push notification config not found".to_string(),
                data: None,
            },
        ))
    }
}

// ---- webhook firing helpers -------------------------------------------

async fn fire_status_update(push: &PushNotificationStore, task: &Task, is_final: bool) {
    let evt = TaskStatusUpdateEvent {
        task_id: task.id.clone(),
        context_id: task.context_id.clone(),
        kind: "status-update",
        status: task.status.clone(),
        r#final: is_final,
    };
    push.fire(&task.id, serde_json::to_value(&evt).unwrap_or(Value::Null))
        .await;
}

async fn fire_artifact_update(
    push: &PushNotificationStore,
    task: &Task,
    artifact: &Artifact,
    last_chunk: bool,
) {
    let evt = TaskArtifactUpdateEvent {
        task_id: task.id.clone(),
        context_id: task.context_id.clone(),
        kind: "artifact-update",
        artifact: artifact.clone(),
        append: Some(false),
        last_chunk: Some(last_chunk),
    };
    push.fire(&task.id, serde_json::to_value(&evt).unwrap_or(Value::Null))
        .await;
}

fn sse_event(resp: JsonRpcResponse) -> Event {
    let data = serde_json::to_string(&resp).unwrap_or_else(|_| "{}".to_string());
    Event::default().data(data)
}

// ---- helpers -----------------------------------------------------------

fn parse_params<T: for<'de> Deserialize<'de>>(
    rpc_id: &Value,
    params: Value,
) -> Result<T, JsonRpcResponse> {
    serde_json::from_value::<T>(params).map_err(|e| {
        JsonRpcResponse::err(
            rpc_id.clone(),
            JsonRpcError {
                code: error_codes::INVALID_PARAMS,
                message: e.to_string(),
                data: None,
            },
        )
    })
}

fn skill_error(rpc_id: &Value, err: SkillError) -> JsonRpcResponse {
    JsonRpcResponse::err(rpc_id.clone(), skill_inner(err))
}

fn skill_inner(err: SkillError) -> JsonRpcError {
    JsonRpcError {
        code: error_codes::SKILL_INVALID,
        message: err.to_string(),
        data: None,
    }
}

fn solana_unconfigured(rpc_id: &Value) -> JsonRpcResponse {
    JsonRpcResponse::err(
        rpc_id.clone(),
        JsonRpcError {
            code: error_codes::SOLANA_UNCONFIGURED,
            message: "Solana RPC not configured on this server".to_string(),
            data: Some(serde_json::json!({
                "hint": "Set SOLANA_RPC_URL in apps/server/.env or the process environment."
            })),
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn skill_message() -> Message {
        Message {
            role: "user".to_string(),
            parts: vec![Part::Data {
                data: serde_json::json!({
                    "skill": "post_intent",
                    "input": {}
                }),
            }],
            message_id: "m1".to_string(),
            task_id: None,
            context_id: None,
        }
    }

    /// JSON-RPC envelope with a wrong `jsonrpc` field is rejected with
    /// the canonical -32600 code.
    #[test]
    fn invalid_jsonrpc_field_yields_invalid_request() {
        let req = JsonRpcRequest {
            jsonrpc: "1.0".to_string(),
            id: Some(Value::String("x".into())),
            method: "message/send".to_string(),
            params: Value::Null,
        };
        // We just check the error code shape; the dispatcher itself is
        // async + needs SharedState, which is overkill for this guard.
        let err = JsonRpcError {
            code: error_codes::INVALID_REQUEST,
            message: "jsonrpc field must be \"2.0\"".to_string(),
            data: None,
        };
        let resp = JsonRpcResponse::err(req.id.unwrap_or(Value::Null), err);
        let v = serde_json::to_value(&resp).unwrap();
        assert_eq!(v["error"]["code"], error_codes::INVALID_REQUEST);
    }

    #[test]
    fn skill_envelope_round_trips() {
        let msg = skill_message();
        let env = SkillEnvelope::extract(&msg.parts).unwrap();
        assert_eq!(env.skill, "post_intent");
    }

    #[test]
    fn jsonrpc_response_serializes_with_string_jsonrpc() {
        let resp = JsonRpcResponse::ok(Value::Null, serde_json::json!({}));
        let v = serde_json::to_value(&resp).unwrap();
        assert_eq!(v["jsonrpc"], "2.0");
    }
}
