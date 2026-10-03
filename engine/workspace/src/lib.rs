//! Sessions, panels, and layout. Persistence and working resources belong to other owners.
pub mod layout;
mod model;
pub use model::*;
use schemars::JsonSchema;
use serde::{Deserialize, Deserializer, Serialize};
use std::collections::HashSet;
use workflow_environment::{
    error::{Error, Result},
    json::{OpaqueJson, OpaqueObject},
    persist::now,
};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct UsageStats {
    pub uses: u64,
    pub frecency: f64,
    pub frecency_at: u64,
    #[serde(default, flatten)]
    pub extra: OpaqueObject,
}
impl Default for UsageStats {
    fn default() -> Self {
        Self {
            uses: 1,
            frecency: 1.0,
            frecency_at: 0,
            extra: Default::default(),
        }
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Session {
    pub id: String,
    #[serde(default)]
    pub name: Option<String>,
    pub created_at: u64,
    pub last_used_at: u64,
    #[serde(default)]
    pub usage: UsageStats,
    #[serde(default)]
    pub archived_at: Option<u64>,
    #[serde(default, deserialize_with = "recover_workbench")]
    pub workbench: Workbench,
    #[serde(default, flatten)]
    pub extra: OpaqueObject,
}
fn recover_workbench<'de, D: Deserializer<'de>>(d: D) -> std::result::Result<Workbench, D::Error> {
    // Buffer only at the recovery boundary: malformed layouts must not lose their session.
    // The resulting state is always a typed Workbench, never an opaque document.
    let encoded = OpaqueJson::deserialize(d)?;
    let mut workbench: Workbench = workflow_environment::json::strict_json(
        &serde_json::to_vec(&encoded).map_err(serde::de::Error::custom)?,
    )
    .unwrap_or_default();
    layout::normalize(&mut workbench);
    Ok(workbench)
}
impl Session {
    pub fn touch_at(&mut self, time: u64) {
        let old = self.last_used_at;
        if time <= old {
            return;
        }
        if time - old >= 1_800_000 {
            self.usage.frecency = self.usage.frecency
                * 0.5f64.powf(
                    time.saturating_sub(self.usage.frecency_at) as f64 / (7.0 * 86_400_000.0),
                )
                + 1.0;
            self.usage.uses = self.usage.uses.saturating_add(1);
            self.usage.frecency_at = time
        }
        self.last_used_at = time
    }
    pub fn resources(&self) -> Vec<ResourceRef> {
        layout::resources(&self.workbench)
    }
}
/// Embedded into the Server's combined document. The Server owns unknown top-level fields.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceState {
    pub sessions: Vec<Session>,
    pub active_session_id: Option<String>,
    pub pinned: Vec<String>,
}
impl WorkspaceState {
    pub fn validate(&mut self) -> Result<()> {
        let mut ids = HashSet::new();
        for s in &mut self.sessions {
            if s.id.is_empty() || !ids.insert(s.id.clone()) {
                return Err(Error::invalid("invalid session"));
            }
            layout::normalize(&mut s.workbench)
        }
        Ok(())
    }
    pub fn session(&self, id: &str) -> Option<&Session> {
        self.sessions.iter().find(|s| s.id == id)
    }
    pub fn session_mut(&mut self, id: &str) -> Option<&mut Session> {
        self.sessions.iter_mut().find(|s| s.id == id)
    }
    pub fn create_session(&mut self, name: Option<&str>, workbench: Workbench) -> Result<String> {
        let name = name.map(str::trim);
        if name == Some("") {
            return Err(Error::invalid("session name is empty"));
        }
        let time = now();
        let id = uuid::Uuid::new_v4().to_string();
        self.sessions.push(Session {
            id: id.clone(),
            name: name.map(str::to_owned),
            created_at: time,
            last_used_at: time,
            usage: UsageStats {
                frecency_at: time,
                ..Default::default()
            },
            archived_at: None,
            workbench,
            extra: Default::default(),
        });
        self.active_session_id = Some(id.clone());
        Ok(id)
    }
    pub fn activate(&mut self, id: &str) -> bool {
        if let Some(s) = self.session_mut(id) {
            s.archived_at = None;
            s.touch_at(now());
            self.active_session_id = Some(id.into());
            true
        } else {
            false
        }
    }
    pub fn enter_workbench(&mut self) -> Result<String> {
        let id = self
            .sessions
            .iter()
            .find(|s| Some(&s.id) == self.active_session_id.as_ref() && s.archived_at.is_none())
            .or_else(|| {
                self.sessions
                    .iter()
                    .filter(|s| s.archived_at.is_none())
                    .max_by_key(|s| s.last_used_at)
            })
            .map(|s| s.id.clone());
        if let Some(id) = id {
            self.activate(&id);
            Ok(id)
        } else {
            self.create_session(None, Workbench::default())
        }
    }
    pub fn rename(&mut self, id: &str, name: &str) -> Result<bool> {
        let name = name.trim();
        if name.is_empty() {
            return Err(Error::invalid("name is empty"));
        }
        Ok(if let Some(s) = self.session_mut(id) {
            s.name = Some(name.into());
            true
        } else {
            false
        })
    }
    pub fn pin(&mut self, id: &str, index: Option<i64>, pin: bool) -> bool {
        let Some(s) = self.session(id) else {
            return false;
        };
        if pin && s.name.is_none() {
            return false;
        }
        let existed = self.pinned.iter().any(|p| p == id);
        self.pinned.retain(|p| p != id);
        if pin {
            let at = index.unwrap_or(0).max(0) as usize;
            self.pinned.insert(at.min(self.pinned.len()), id.into())
        }
        pin || existed
    }
    pub fn apply_layout(
        &mut self,
        session_id: Option<&str>,
        op: &LayoutAction,
    ) -> Option<Workbench> {
        let id = session_id
            .map(str::to_owned)
            .or_else(|| self.active_session_id.clone())?;
        let s = self.session_mut(&id)?;
        if s.archived_at.is_some() {
            return None;
        }
        let w = layout::apply(&s.workbench, op);
        if w != s.workbench {
            s.workbench = w.clone();
            s.touch_at(now())
        }
        Some(w)
    }
    pub fn touch_resource(&mut self, resource: &ResourceRef) {
        if let Some(id) = self.active_session_id.clone() {
            if let Some(s) = self.session_mut(&id) {
                if s.resources().contains(resource) {
                    s.touch_at(now())
                }
            }
        }
    }
    pub fn archive(&mut self, id: &str, time: u64) -> bool {
        let Some(s) = self.session_mut(id) else {
            return false;
        };
        s.archived_at = Some(time);
        if self.active_session_id.as_deref() == Some(id) {
            self.active_session_id = None
        }
        true
    }
    pub fn maintain(&mut self, protected: &HashSet<String>, time: u64) {
        let mut active_archived = false;
        for s in &mut self.sessions {
            let retention = if s.name.is_none() {
                86_400_000
            } else {
                7 * 86_400_000
            };
            if s.archived_at.is_none()
                && !protected.contains(&s.id)
                && time.saturating_sub(s.last_used_at) >= retention
            {
                s.archived_at = Some(time);
                active_archived |= self.active_session_id.as_ref() == Some(&s.id)
            }
        }
        if active_archived {
            self.active_session_id = None
        }
    }
    pub fn rename_path(&mut self, from: &str, to: &str) {
        for s in &mut self.sessions {
            s.workbench = layout::apply(
                &s.workbench,
                &LayoutAction::RenamePath {
                    from: from.into(),
                    to: to.into(),
                },
            )
        }
    }
    pub fn after_removal(&mut self, path: &str, remaining_drafts: &HashSet<String>) {
        for s in &mut self.sessions {
            let panel_ids = s
                .workbench
                .panels
                .iter()
                .filter(|p| {
                    matches!(p.target.kind.as_str(), "file" | "image" | "diff")
                        && p.target.path.as_deref().is_some_and(|p| {
                            (p == path || p.starts_with(&format!("{path}/")))
                                && !remaining_drafts.contains(p)
                        })
                })
                .map(|p| p.id.clone())
                .collect();
            s.workbench = layout::apply(&s.workbench, &LayoutAction::Close { panel_ids })
        }
    }
    pub fn remove_conversation(&mut self, id: &str) {
        for s in &mut self.sessions {
            let panel_ids = s
                .workbench
                .panels
                .iter()
                .filter(|p| p.target.kind == "conversation" && p.target.id.as_deref() == Some(id))
                .map(|p| p.id.clone())
                .collect();
            s.workbench = layout::apply(&s.workbench, &LayoutAction::Close { panel_ids })
        }
    }
    pub fn terminal_references(&self) -> HashSet<String> {
        self.sessions
            .iter()
            .filter(|s| s.archived_at.is_none())
            .flat_map(|s| &s.workbench.panels)
            .filter(|p| p.target.kind == "terminal")
            .filter_map(|p| p.target.id.clone())
            .collect()
    }
}

#[cfg(test)]
mod tests;
