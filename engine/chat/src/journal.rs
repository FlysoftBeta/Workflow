//! Bounded replay with revision-aligned metadata and distinct process/service epochs.
use crate::{
    error::{ChatError, ErrorKind, Result},
    model::*,
    reducer,
};
use std::{
    collections::VecDeque,
    sync::{Condvar, Mutex},
    time::Duration,
};

pub const UPDATE_BYTES: usize = 1024 * 1024;
struct Change {
    revision: i64,
    event: Option<AgentEvent>,
    bytes: usize,
    metadata: ChatMetadata,
}
struct Inner {
    epoch: String,
    revision: i64,
    state: AgentState,
    metadata: ChatMetadata,
    history: VecDeque<Change>,
    bytes: usize,
}
pub struct Journal {
    inner: Mutex<Inner>,
    changed: Condvar,
    max_events: usize,
    max_bytes: usize,
}
impl Default for Journal {
    fn default() -> Self {
        Self::new(4096, 8 * 1024 * 1024)
    }
}
impl Journal {
    pub fn new(max_events: usize, max_bytes: usize) -> Self {
        Self {
            inner: Mutex::new(Inner {
                epoch: uuid::Uuid::new_v4().to_string(),
                revision: 0,
                state: AgentState::default(),
                metadata: ChatMetadata::default(),
                history: VecDeque::new(),
                bytes: 0,
            }),
            changed: Condvar::new(),
            max_events,
            max_bytes,
        }
    }
    fn record(&self, inner: &mut Inner, event: Option<AgentEvent>) -> Result<()> {
        let revision = inner
            .revision
            .checked_add(1)
            .ok_or_else(|| ChatError::new(ErrorKind::Closed, "chat revision exhausted"))?;
        let bytes = event
            .as_ref()
            .map(serde_json::to_vec)
            .transpose()?
            .map_or(0, |v| v.len());
        inner.revision = revision;
        inner.bytes += bytes;
        inner.history.push_back(Change {
            revision,
            event,
            bytes,
            metadata: inner.metadata.clone(),
        });
        while inner.history.len() > self.max_events || inner.bytes > self.max_bytes {
            inner.bytes -= inner.history.pop_front().unwrap().bytes;
        }
        self.changed.notify_all();
        Ok(())
    }
    pub fn event(&self, event: AgentEvent) -> Result<()> {
        let mut inner = self.inner.lock().unwrap();
        reducer::reduce(&mut inner.state, &event);
        if let AgentEvent::ProcessChanged {
            backend,
            state: ProcessState::Starting {},
        } = &event
        {
            inner
                .metadata
                .process_epochs
                .insert(*backend, uuid::Uuid::new_v4().to_string());
        }
        self.record(&mut inner, Some(event))
    }
    pub fn metadata(&self, mut metadata: ChatMetadata) -> Result<()> {
        let mut inner = self.inner.lock().unwrap();
        metadata.process_epochs = inner.metadata.process_epochs.clone();
        if metadata != inner.metadata {
            inner.metadata = metadata;
            self.record(&mut inner, None)?;
        }
        Ok(())
    }
    pub fn snapshot(&self) -> ChatSnapshot {
        let i = self.inner.lock().unwrap();
        ChatSnapshot {
            epoch: i.epoch.clone(),
            revision: i.revision,
            state: i.state.clone(),
            metadata: i.metadata.clone(),
        }
    }
    pub fn process_epoch(&self, kind: BackendKind) -> Option<String> {
        self.inner
            .lock()
            .unwrap()
            .metadata
            .process_epochs
            .get(&kind)
            .cloned()
    }
    pub fn validate_response(&self, key: &RequestKey, epoch: &str) -> Result<PendingRequest> {
        let i = self.inner.lock().unwrap();
        if i.metadata
            .process_epochs
            .get(&key.backend)
            .map(String::as_str)
            != Some(epoch)
        {
            return Err(ChatError::new(
                ErrorKind::RequestExpired,
                "Approval belongs to an expired backend process",
            ));
        }
        i.state
            .requests
            .get(key)
            .filter(|r| r.status == RequestStatus::Pending)
            .cloned()
            .ok_or_else(|| ChatError::new(ErrorKind::RequestExpired, "Request is no longer open"))
    }
    /// A verified environment activation invalidates every identity from its former generation.
    pub fn environment_changed(&self) -> Result<()> {
        let mut i = self.inner.lock().unwrap();
        for backend in [BackendKind::Codex, BackendKind::Claude] {
            reducer::reduce(
                &mut i.state,
                &AgentEvent::ProcessChanged {
                    backend,
                    state: ProcessState::Exited {
                        exit_code: None,
                        stderr_tail: String::new(),
                    },
                },
            );
        }
        i.epoch = uuid::Uuid::new_v4().to_string();
        i.revision = 0;
        i.history.clear();
        i.bytes = 0;
        i.metadata.process_epochs.clear();
        self.changed.notify_all();
        Ok(())
    }
    pub fn watch(&self, epoch: &str, after: i64, wait: Duration) -> Result<ChatUpdate> {
        if after < 0 || wait > Duration::from_secs(30) {
            return Err(ChatError::new(
                ErrorKind::InvalidArgument,
                "invalid chat cursor or wait",
            ));
        }
        let i = self.inner.lock().unwrap();
        let (i, _) = self
            .changed
            .wait_timeout_while(i, wait, |i| i.epoch == epoch && i.revision == after)
            .unwrap();
        let resnapshot = || ChatUpdate {
            epoch: i.epoch.clone(),
            revision: i.revision,
            metadata: i.metadata.clone(),
            resnapshot: true,
            ..Default::default()
        };
        if i.epoch != epoch
            || after > i.revision
            || (after < i.revision && i.history.front().is_none_or(|h| after < h.revision - 1))
        {
            return Ok(resnapshot());
        }
        let mut bytes = 0;
        let mut count = 0;
        let mut events = Vec::new();
        let mut revision = after;
        let mut metadata = i.metadata.clone();
        for h in i.history.iter().filter(|h| h.revision > after) {
            if count == 512 || bytes + h.bytes > UPDATE_BYTES {
                break;
            }
            count += 1;
            bytes += h.bytes;
            revision = h.revision;
            metadata = h.metadata.clone();
            if let Some(e) = &h.event {
                events.push(e.clone());
            }
        }
        if count == 0 && after < i.revision {
            return Ok(resnapshot());
        }
        Ok(ChatUpdate {
            epoch: i.epoch.clone(),
            revision,
            events,
            metadata,
            resnapshot: false,
        })
    }
}
