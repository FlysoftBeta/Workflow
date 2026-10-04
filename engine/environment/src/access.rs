//! The typed visibility policy of `<workspace-root>/.workspace/`.
//!
//! The explorer shows `.workspace` as a protected folder. This module is its only allowlist:
//! FileWork enforces [`classify`] for every client file API, and the guest mask returned by
//! [`guest_masks`] is derived from the same tables. Private Engine state and the disposable
//! `cache/` are therefore hidden from file APIs and masked in the guest, while explicitly editable
//! configuration stays reachable in both places. Agent homes appear in the guest only at their own
//! mount points, so a walk of `/workspace` never reaches agent credentials through a second path.

/// The directory name below the workspace root.
pub const DIRECTORY: &str = ".workspace";
/// Guest mount point of the workspace root for processes that see user files.
pub const GUEST_WORKSPACE: &str = "/workspace";

/// What client file APIs may do with a visible `.workspace/` entry.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Access {
    /// A fixed folder: listed, never created, renamed, moved, deleted or written.
    Folder,
    /// Explicitly editable configuration.
    Editable,
}
impl Access {
    pub fn writable(self) -> bool {
        self == Access::Editable
    }
}

/// Classes of the top-level `.workspace/` entries. Names absent from [`TOP_LEVEL`] are private.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Top {
    /// One editable configuration file.
    File,
    /// A fixed folder whose descendants are editable.
    EditableTree,
    /// A fixed folder of fixed per-service folders whose descendants are editable.
    Services,
    /// Agent homes, mounted at their own guest paths.
    Agents,
    /// Engine-private state and the disposable cache, masked in the guest.
    Private,
}
const TOP_LEVEL: &[(&str, Top)] = &[
    ("config.json", Top::File),
    ("proxy", Top::EditableTree),
    ("services", Top::Services),
    ("agents", Top::Agents),
    ("state", Top::Private),
    ("environment", Top::Private),
    ("documents", Top::Private),
    ("corrupt", Top::Private),
    ("trash", Top::Private),
    ("engine.lock", Top::Private),
    ("cache", Top::Private),
];

/// A coding agent's home below `.workspace/agents/<id>/`, bound at [`AgentHome::guest`].
///
/// Only the named configuration entries are visible. Credentials, sessions, history, logs and
/// caches written by the agent stay private.
#[derive(Debug)]
pub struct AgentHome {
    pub id: &'static str,
    /// Guest path of the home for chat and terminal processes.
    pub guest: &'static str,
    /// The vendor variable naming that home.
    pub variable: &'static str,
    files: &'static [&'static str],
    trees: &'static [&'static str],
    /// Never listed, read or written through file APIs and never logged. In the guest they exist
    /// only below [`AgentHome::guest`].
    pub credentials: &'static [&'static str],
}
pub const AGENT_HOMES: &[AgentHome] = &[
    AgentHome {
        id: "codex",
        guest: "/home/work/.codex",
        variable: "CODEX_HOME",
        files: &["config.toml", "AGENTS.md"],
        trees: &["prompts", "rules", "skills"],
        credentials: &["auth.json"],
    },
    AgentHome {
        id: "claude",
        guest: "/home/work/.claude",
        variable: "CLAUDE_CONFIG_DIR",
        files: &["settings.json", "CLAUDE.md"],
        trees: &["agents", "commands", "skills"],
        credentials: &[".credentials.json", ".claude.json"],
    },
];
impl AgentHome {
    /// The home relative to `.workspace/`.
    pub fn key(&self) -> String {
        format!("agents/{}", self.id)
    }
}

fn segment(name: &str) -> bool {
    crate::persist::identifier(name).is_ok()
}

/// Classify a `.workspace/`-relative key; `""` is `.workspace` itself. `None` means private:
/// absent from listings and refused by every file API.
pub fn classify(key: &str) -> Option<Access> {
    if key.is_empty() {
        return Some(Access::Folder);
    }
    let parts: Vec<&str> = key.split('/').collect();
    if !parts.iter().all(|p| segment(p)) {
        return None;
    }
    let (top, rest) = parts.split_first()?;
    match TOP_LEVEL.iter().find(|(name, _)| name == top)?.1 {
        Top::File => rest.is_empty().then_some(Access::Editable),
        Top::EditableTree => Some(if rest.is_empty() {
            Access::Folder
        } else {
            Access::Editable
        }),
        Top::Services => Some(if rest.len() < 2 {
            Access::Folder
        } else {
            Access::Editable
        }),
        Top::Agents => agents(rest),
        Top::Private => None,
    }
}
fn agents(rest: &[&str]) -> Option<Access> {
    let Some((first, tail)) = rest.split_first() else {
        return Some(Access::Folder);
    };
    let home = AGENT_HOMES.iter().find(|home| home.id == *first)?;
    let Some((entry, below)) = tail.split_first() else {
        return Some(Access::Folder);
    };
    if home.credentials.contains(entry) {
        return None;
    }
    if home.files.contains(entry) {
        return below.is_empty().then_some(Access::Editable);
    }
    home.trees.contains(entry).then_some(Access::Editable)
}

/// Whether a workspace-relative path names `.workspace` or something below it.
pub fn is_internal(path: &str) -> bool {
    path == DIRECTORY || path.starts_with(".workspace/")
}

/// The `.workspace/`-relative key of an internal workspace-relative path.
pub fn key(path: &str) -> Option<&str> {
    if path == DIRECTORY {
        Some("")
    } else {
        path.strip_prefix(".workspace/")
    }
}

/// Guest paths hidden below `/workspace/.workspace` for processes that mount the workspace:
/// private top-level state, the cache and the agents tree, whose homes are mounted at their own
/// guest paths.
///
/// The runtime matches hides against resolved guest paths and maps a directory back to the guest
/// through its longest host prefix. Masking a single credential below `/workspace` would therefore
/// still let a recursive walk reach it through the home mount; masking the whole alias does not.
pub fn guest_masks() -> Vec<String> {
    let base = format!("{GUEST_WORKSPACE}/{DIRECTORY}");
    TOP_LEVEL
        .iter()
        .filter(|(_, class)| matches!(class, Top::Private | Top::Agents))
        .map(|(name, _)| format!("{base}/{name}"))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn explicit_allowlist_exposes_configuration_only() {
        for key in [
            "",
            "proxy",
            "services",
            "services/example",
            "agents",
            "agents/codex",
            "agents/claude",
        ] {
            assert_eq!(classify(key), Some(Access::Folder), "{key}");
        }
        for key in [
            "config.json",
            "proxy/config.yaml",
            "proxy/providers/list.yaml",
            "services/example/providers/list.yaml",
            "agents/codex/config.toml",
            "agents/codex/AGENTS.md",
            "agents/codex/prompts",
            "agents/codex/prompts/review.md",
            "agents/codex/skills/lint/SKILL.md",
            "agents/claude/settings.json",
            "agents/claude/CLAUDE.md",
            "agents/claude/commands/fix.md",
            "agents/claude/agents/reviewer.md",
        ] {
            assert_eq!(classify(key), Some(Access::Editable), "{key}");
        }
    }

    #[test]
    fn private_state_credentials_and_unknown_entries_are_hidden() {
        for key in [
            "state",
            "state/workspace.json",
            "environment",
            "environment/stores/home/work",
            "documents/chat/conversations.json",
            "cache",
            "cache/uploads/x",
            "cache/tools/payload/abc/codex/bin/codex",
            "cache/generations/job/rootfs",
            "env.json",
            "uploads/x",
            "agents/tools",
            "agents/tools/payload/abc/codex/bin/codex",
            "corrupt",
            "trash/1/entry.json",
            "engine.lock",
            "config.json.bak",
            "config.json/inner",
            "unknown",
            "services/../state",
            "proxy/./x",
            "proxy/a b",
            "agents/other",
            "agents/codex/auth.json",
            "agents/codex/sessions/2026/10/rollout.jsonl",
            "agents/codex/history.jsonl",
            "agents/codex/log/codex-tui.log",
            "agents/codex/config.toml/x",
            "agents/claude/.credentials.json",
            "agents/claude/.claude.json",
            "agents/claude/projects/-workspace/session.jsonl",
            "agents/claude/settings.local.json",
        ] {
            assert_eq!(classify(key), None, "{key}");
        }
        for home in AGENT_HOMES {
            for credential in home.credentials {
                assert_eq!(classify(&format!("{}/{credential}", home.key())), None);
            }
        }
    }

    #[test]
    fn guest_mask_matches_the_allowlist_and_runtime_capacity() {
        let masks = guest_masks();
        // The runtime accepts 16 hides and reserves one for its own metadata directory.
        assert!(masks.len() < 16, "{masks:?}");
        let prefix = "/workspace/.workspace/";
        for (name, class) in TOP_LEVEL {
            let masked = masks.contains(&format!("{prefix}{name}"));
            assert_eq!(
                masked,
                matches!(class, Top::Private | Top::Agents),
                "{name}"
            );
            assert_eq!(classify(name).is_none(), *class == Top::Private, "{name}");
            // Editable configuration outside the agents tree stays reachable below /workspace.
            if classify(name) == Some(Access::Editable) {
                assert!(!masked, "{name}");
            }
        }
        assert!(masks.contains(&format!("{prefix}agents")));
        assert!(masks.contains(&format!("{prefix}cache")));
        for home in AGENT_HOMES {
            assert!(home.guest.starts_with("/home/work/"));
        }
    }

    #[test]
    fn internal_paths_map_to_store_keys() {
        assert!(is_internal(".workspace"));
        assert!(is_internal(".workspace/config.json"));
        assert!(!is_internal(".workspaces"));
        assert!(!is_internal("docs/.workspace"));
        assert_eq!(key(".workspace"), Some(""));
        assert_eq!(key(".workspace/agents/codex"), Some("agents/codex"));
        assert_eq!(key("src/main.rs"), None);
    }
}
