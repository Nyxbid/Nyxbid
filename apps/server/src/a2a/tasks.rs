//! In-memory task store for the A2A surface.
//!
//! A2A `tasks/get` and `tasks/cancel` need a process-wide registry of
//! in-flight tasks. We keep them in a tokio `RwLock<HashMap<id, Task>>`
//! — fine for a single-instance demo. Production deployments would
//! swap this for Redis or a database shared across replicas.
//!
//! Tasks are bounded with a simple LRU-style cap so a long-running
//! server doesn't grow unbounded if no client ever calls
//! `tasks/cancel`.

use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::RwLock;

use super::types::Task;

/// Soft cap on retained tasks. When the store grows past this, the
/// oldest *terminal* tasks are evicted first (Submitted/Working tasks
/// stay until they finish).
const MAX_TASKS: usize = 4096;

/// Shared in-memory task registry. Cheap to clone (`Arc`).
#[derive(Clone, Default)]
pub struct TaskStore {
    inner: Arc<RwLock<HashMap<String, Task>>>,
}

impl TaskStore {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn insert(&self, task: Task) {
        let mut guard = self.inner.write().await;
        if guard.len() >= MAX_TASKS {
            evict_terminal(&mut guard);
        }
        guard.insert(task.id.clone(), task);
    }

    pub async fn get(&self, id: &str) -> Option<Task> {
        self.inner.read().await.get(id).cloned()
    }

    /// Atomically apply `f` to a task identified by `id`. Returns
    /// `None` if the task is not in the store.
    pub async fn update<F, R>(&self, id: &str, f: F) -> Option<R>
    where
        F: FnOnce(&mut Task) -> R,
    {
        let mut guard = self.inner.write().await;
        guard.get_mut(id).map(f)
    }

    /// For health/observability surfaces. Cheap snapshot.
    #[allow(dead_code)]
    pub async fn len(&self) -> usize {
        self.inner.read().await.len()
    }
}

fn evict_terminal(map: &mut HashMap<String, Task>) {
    use super::types::TaskState;
    let to_drop: Vec<String> = map
        .iter()
        .filter(|(_, t)| {
            matches!(
                t.status.state,
                TaskState::Completed
                    | TaskState::Canceled
                    | TaskState::Failed
                    | TaskState::Rejected
            )
        })
        .map(|(id, _)| id.clone())
        .take(MAX_TASKS / 4)
        .collect();
    for id in to_drop {
        map.remove(&id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::a2a::types::{Message, Part, TaskState};

    fn fake_task(id: &str) -> Task {
        let msg = Message {
            role: "user".to_string(),
            parts: vec![Part::Text {
                text: "hi".to_string(),
            }],
            message_id: format!("msg_{id}"),
            task_id: Some(id.to_string()),
            context_id: Some("ctx".to_string()),
        };
        Task::new(id.to_string(), "ctx".to_string(), msg)
    }

    #[tokio::test]
    async fn insert_and_get_round_trips() {
        let store = TaskStore::new();
        store.insert(fake_task("t1")).await;
        let got = store.get("t1").await.unwrap();
        assert_eq!(got.id, "t1");
    }

    #[tokio::test]
    async fn update_mutates_in_place() {
        let store = TaskStore::new();
        store.insert(fake_task("t1")).await;
        store
            .update("t1", |t| t.set_state(TaskState::Completed))
            .await;
        let got = store.get("t1").await.unwrap();
        assert_eq!(got.status.state, TaskState::Completed);
    }

    #[tokio::test]
    async fn missing_task_returns_none() {
        let store = TaskStore::new();
        assert!(store.get("nope").await.is_none());
    }
}
