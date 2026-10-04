//! Bounded replay with revision-aligned shared metadata and distinct process/service epochs.
use crate::{
    error::{ChatError, ErrorKind, Result},
    model::*,
    reducer,
};
use std::{
    collections::VecDeque,
    sync::{Arc, Condvar, Mutex},
    time::Duration,
};
pub const UPDATE_BYTES: usize = 1024 * 1024;
struct Change {
    revision: i64,
    event: Option<AgentEvent>,
    event_bytes: usize,
    retained_bytes: usize,
    metadata: Arc<ChatMetadata>,
    metadata_bytes: usize,
}
struct Inner {
    epoch: String,
    revision: i64,
    state: AgentState,
    metadata: Arc<ChatMetadata>,
    metadata_bytes: usize,
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
        let metadata = ChatMetadata::default();
        let metadata_bytes = serde_json::to_vec(&metadata).unwrap().len();
        Self {
            inner: Mutex::new(Inner {
                epoch: uuid::Uuid::new_v4().to_string(),
                revision: 0,
                state: AgentState::default(),
                metadata: Arc::new(metadata),
                metadata_bytes,
                history: VecDeque::new(),
                bytes: 0,
            }),
            changed: Condvar::new(),
            max_events,
            max_bytes,
        }
    }
    fn record(&self, i: &mut Inner, event: Option<AgentEvent>) -> Result<()> {
        let revision = i
            .revision
            .checked_add(1)
            .ok_or_else(|| ChatError::new(ErrorKind::Closed, "chat revision exhausted"))?;
        let event_bytes = event
            .as_ref()
            .map(serde_json::to_vec)
            .transpose()?
            .map_or(0, |v| v.len());
        // Charging metadata for each entry is conservative when Arc shares it. It also
        // bounds a sequence of large metadata-only changes without a hidden zero-cost path.
        let retained_bytes = event_bytes
            .saturating_add(i.metadata_bytes)
            .saturating_add(64);
        i.revision = revision;
        i.bytes = i.bytes.saturating_add(retained_bytes);
        i.history.push_back(Change {
            revision,
            event,
            event_bytes,
            retained_bytes,
            metadata: i.metadata.clone(),
            metadata_bytes: i.metadata_bytes,
        });
        while i.history.len() > self.max_events || i.bytes > self.max_bytes {
            i.bytes = i
                .bytes
                .saturating_sub(i.history.pop_front().unwrap().retained_bytes);
        }
        self.changed.notify_all();
        Ok(())
    }
    pub fn event(&self, event: AgentEvent) -> Result<()> {
        let mut i = self.inner.lock().unwrap();
        reducer::reduce(&mut i.state, &event);
        if let AgentEvent::ProcessChanged {
            backend,
            state: ProcessState::Starting {},
        } = &event
        {
            Arc::make_mut(&mut i.metadata)
                .process_epochs
                .insert(*backend, uuid::Uuid::new_v4().to_string());
            i.metadata_bytes = serde_json::to_vec(&*i.metadata)?.len();
        }
        self.record(&mut i, Some(event))
    }
    pub fn metadata(&self, mut metadata: ChatMetadata) -> Result<()> {
        let mut i = self.inner.lock().unwrap();
        metadata.process_epochs = i.metadata.process_epochs.clone();
        if metadata != *i.metadata {
            i.metadata_bytes = serde_json::to_vec(&metadata)?.len();
            i.metadata = Arc::new(metadata);
            self.record(&mut i, None)?;
        }
        Ok(())
    }
    pub fn snapshot(&self) -> ChatSnapshot {
        let i = self.inner.lock().unwrap();
        ChatSnapshot {
            epoch: i.epoch.clone(),
            revision: i.revision,
            state: i.state.clone(),
            metadata: (*i.metadata).clone(),
        }
    }
    /// One thread of the reduced state, without copying the rest.
    pub fn thread(&self, key: &ThreadKey) -> Option<ThreadState> {
        self.inner.lock().unwrap().state.threads.get(key).cloned()
    }
    pub fn backend(&self, kind: BackendKind) -> BackendStatus {
        self.inner
            .lock()
            .unwrap()
            .state
            .backends
            .get(&kind)
            .cloned()
            .unwrap_or_default()
    }
    pub fn epoch(&self) -> String {
        self.inner.lock().unwrap().epoch.clone()
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
        Arc::make_mut(&mut i.metadata).process_epochs.clear();
        i.metadata_bytes = serde_json::to_vec(&*i.metadata)?.len();
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
        let resnapshot = || {
            let mut update = ChatUpdate {
                epoch: i.epoch.clone(),
                revision: i.revision,
                resnapshot: true,
                ..Default::default()
            };
            if update_size(&i.epoch, i.revision, i.metadata_bytes, 0, 0) <= UPDATE_BYTES {
                update.metadata = (*i.metadata).clone();
            }
            update
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
            let event_count = events.len() + usize::from(h.event.is_some());
            let event_bytes = bytes + h.event_bytes;
            if count == 512
                || update_size(
                    &i.epoch,
                    h.revision,
                    h.metadata_bytes,
                    event_bytes,
                    event_count,
                ) > UPDATE_BYTES
            {
                break;
            }
            count += 1;
            bytes = event_bytes;
            revision = h.revision;
            metadata = h.metadata.clone();
            if let Some(e) = &h.event {
                events.push(e.clone());
            }
        }
        if (count == 0 && after < i.revision)
            || update_size(
                &i.epoch,
                revision,
                serde_json::to_vec(&*metadata)?.len(),
                bytes,
                events.len(),
            ) > UPDATE_BYTES
        {
            return Ok(resnapshot());
        }
        Ok(ChatUpdate {
            epoch: i.epoch.clone(),
            revision,
            events,
            metadata: (*metadata).clone(),
            resnapshot: false,
        })
    }
}
fn update_size(
    epoch: &str,
    revision: i64,
    metadata_bytes: usize,
    event_bytes: usize,
    event_count: usize,
) -> usize {
    // These are exactly the emitted field names and separators. Use serde for
    // the variable JSON string and integer lengths; metadata and events are cached.
    b"{\"epoch\":,\"revision\":,\"events\":[],\"metadata\":,\"resnapshot\":false}".len()
        + serde_json::to_string(epoch).unwrap().len()
        + revision.to_string().len()
        + metadata_bytes
        + event_bytes
        + event_count.saturating_sub(1)
}
