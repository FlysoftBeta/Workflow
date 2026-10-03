//! Serialized index mutations. Failed writes never publish uncommitted state.
use crate::{
    error::{ChatError, ErrorKind, Result},
    model::*,
    ports::ChatStore,
};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IndexDocument {
    pub format: i64,
    pub conversations: Vec<IndexEntry>,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct IndexEntry {
    pub id: String,
    pub backend: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thread: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    pub cwd: String,
    pub created_at: i64,
    pub updated_at: i64,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub archived: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub forked_from: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub forked_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub effort: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preview: Option<String>,
}
impl From<&ConversationEntry> for IndexEntry {
    fn from(v: &ConversationEntry) -> Self {
        Self {
            id: v.id.clone(),
            backend: v.backend.id().into(),
            thread: v.backend_thread_id.clone(),
            title: v.title.clone(),
            cwd: v.cwd.clone(),
            created_at: v.created_at_ms,
            updated_at: v.updated_at_ms,
            archived: v.archived,
            forked_from: v.forked_from.clone(),
            forked_at: v.forked_at.clone(),
            model: v.model.clone(),
            effort: v.effort.clone(),
            preview: v.preview.clone(),
        }
    }
}
impl IndexEntry {
    fn conversation(self) -> Option<ConversationEntry> {
        if self.id.trim().is_empty() {
            return None;
        }
        let backend = match self.backend.as_str() {
            "codex" => BackendKind::Codex,
            "claude" => BackendKind::Claude,
            _ => return None,
        };
        Some(ConversationEntry {
            id: self.id,
            backend,
            backend_thread_id: self.thread,
            title: self.title,
            cwd: self.cwd,
            created_at_ms: self.created_at,
            updated_at_ms: self.updated_at,
            archived: self.archived,
            forked_from: self.forked_from,
            forked_at: self.forked_at,
            model: self.model,
            effort: self.effort,
            preview: self.preview,
        })
    }
}
impl IndexDocument {
    pub fn decode(bytes: &[u8]) -> Result<Self> {
        #[derive(Deserialize)]
        struct Probe {
            format: i64,
            #[serde(default)]
            conversations: Vec<OpaqueJson>,
        }
        let p: Probe = workflow_environment::json::strict_json(bytes)?;
        if p.format > 1 {
            return Err(ChatError::new(
                ErrorKind::UnsupportedFormat,
                "conversation index format is newer than 1",
            ));
        }
        let mut conversations = Vec::<IndexEntry>::new();
        for raw in p.conversations {
            if let Ok(candidate) =
                workflow_environment::json::strict_json::<IndexEntry>(&serde_json::to_vec(&raw)?)
            {
                if !candidate.id.trim().is_empty()
                    && matches!(candidate.backend.as_str(), "codex" | "claude")
                    && !conversations.iter().any(|v| v.id == candidate.id)
                {
                    conversations.push(candidate);
                }
            }
        }
        Ok(Self {
            format: 1,
            conversations,
        })
    }
    pub fn from_entries(entries: &[ConversationEntry]) -> Self {
        Self {
            format: 1,
            conversations: entries.iter().map(IndexEntry::from).collect(),
        }
    }
}
pub struct ConversationIndex {
    store: Arc<dyn ChatStore>,
    entries: Mutex<Vec<ConversationEntry>>,
}
impl ConversationIndex {
    pub fn load(store: Arc<dyn ChatStore>) -> Result<Self> {
        let entries = match store.read_index() {
            Ok(document) => document
                .map(|d| {
                    d.conversations
                        .into_iter()
                        .filter_map(IndexEntry::conversation)
                        .collect()
                })
                .unwrap_or_default(),
            Err(e) if e.kind == ErrorKind::InvalidArgument => {
                store.quarantine_index()?;
                Vec::new()
            }
            Err(e) => return Err(e),
        };
        Ok(Self {
            store,
            entries: Mutex::new(entries),
        })
    }
    pub fn entries(&self) -> Vec<ConversationEntry> {
        self.entries.lock().unwrap().clone()
    }
    fn edit<T>(&self, apply: impl FnOnce(&mut Vec<ConversationEntry>) -> T) -> Result<T> {
        let mut entries = self.entries.lock().unwrap();
        let mut next = entries.clone();
        let result = apply(&mut next);
        if *entries != next {
            self.store
                .write_index(&IndexDocument::from_entries(&next))?;
            *entries = next;
        }
        Ok(result)
    }
    pub fn upsert(&self, entry: ConversationEntry) -> Result<()> {
        self.edit(|all| {
            if let Some(old) = all.iter_mut().find(|v| v.id == entry.id) {
                *old = entry;
            } else {
                all.push(entry);
            }
        })
    }
    pub fn update(
        &self,
        id: &str,
        update: impl FnOnce(&mut ConversationEntry),
    ) -> Result<Option<ConversationEntry>> {
        self.edit(|all| {
            all.iter_mut().find(|v| v.id == id).map(|v| {
                update(v);
                v.clone()
            })
        })
    }
    pub fn remove(&self, id: &str) -> Result<()> {
        self.edit(|all| all.retain(|v| v.id != id))
    }
    pub fn refresh_thread(&self, thread: &ThreadState, now: i64) -> Result<()> {
        self.edit(|all| {
            let Some(v) = all.iter_mut().find(|v| {
                v.backend == thread.key.backend
                    && v.backend_thread_id.as_ref() == Some(&thread.key.id)
            }) else {
                return;
            };
            let old = v.clone();
            if v.title.is_none() {
                v.title = thread.title.clone();
            }
            if v.preview.is_none() {
                v.preview = thread
                    .turns
                    .iter()
                    .flat_map(|t| &t.items)
                    .find_map(|i| match i {
                        Item::UserMessage(u) => Some(take_utf16(&u.text(), 200)),
                        _ => None,
                    });
            }
            let last = thread.turns.last().and_then(|v| v.settings.as_ref());
            v.model = last
                .and_then(|s| s.model.clone())
                .or_else(|| thread.settings.model.clone())
                .or_else(|| v.model.clone());
            v.effort = last
                .and_then(|s| s.effort.clone())
                .or_else(|| thread.settings.effort.clone())
                .or_else(|| v.effort.clone());
            if *v != old {
                v.updated_at_ms = now;
            }
        })
    }
}
pub fn take_utf16(text: &str, max: usize) -> String {
    let mut n = 0;
    text.chars()
        .take_while(|c| {
            n += c.len_utf16();
            n <= max
        })
        .collect()
}
