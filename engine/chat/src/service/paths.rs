//! Workspace-relative paths and the agents' guest view, exactly as the client's Kotlin `AgentPaths`
//! resolves them. Agent homes are visible workspace configuration below `.workspace/agents/<id>`,
//! but agents see them only at their guest homes; FileWork's allowlist still decides access.
use crate::error::{ChatError, Result};
use workflow_environment::access::{AGENT_HOMES, DIRECTORY, GUEST_WORKSPACE};

/// The sole production agent root inside the container.
pub const ROOT: &str = GUEST_WORKSPACE;

/// Kotlin `WorkspacePaths.normalize`: relative, no `..`, empty and `.` segments dropped.
pub fn normalize(raw: &str) -> Result<String> {
    if raw.starts_with('/') {
        return Err(ChatError::invalid("Workspace paths must be relative"));
    }
    if raw.contains('\0') {
        return Err(ChatError::invalid("Invalid path"));
    }
    let parts: Vec<&str> = raw
        .split('/')
        .filter(|p| !p.is_empty() && *p != ".")
        .collect();
    if parts.contains(&"..") {
        return Err(ChatError::invalid("Path is outside the workspace"));
    }
    Ok(parts.join("/"))
}
fn within(path: &str, directory: &str) -> bool {
    directory.is_empty() || path == directory || path.starts_with(&format!("{directory}/"))
}
fn homes() -> impl Iterator<Item = (&'static str, String)> {
    AGENT_HOMES
        .iter()
        .map(|h| (h.guest, format!("{DIRECTORY}/{}", h.key())))
}

/// The agent-visible path of a workspace-relative path (`""` is the root).
pub fn to_agent(relative: &str) -> Result<String> {
    let clean = normalize(relative)?;
    for (guest, visible) in homes() {
        if within(&clean, &visible) {
            return Ok(format!("{guest}{}", &clean[visible.len()..]));
        }
    }
    Ok(if clean.is_empty() {
        ROOT.into()
    } else {
        format!("{ROOT}/{clean}")
    })
}

/// The workspace-relative path of an agent path, or `None` when it is outside the workspace or
/// names app-internal state. Relative inputs are relative to the workspace root.
pub fn to_workspace(agent_path: &str) -> Option<String> {
    let mut path = agent_path.trim();
    if let Some(rest) = path.strip_prefix("file://") {
        path = rest;
    }
    if path.starts_with('/') {
        for (guest, visible) in homes() {
            if path == guest || path.starts_with(&format!("{guest}/")) {
                let rest = normalize(path[guest.len()..].trim_start_matches('/')).ok()?;
                return Some(if rest.is_empty() {
                    visible
                } else {
                    format!("{visible}/{rest}")
                });
            }
        }
    }
    let relative = if path.starts_with('/') {
        if path != ROOT && !path.starts_with(&format!("{ROOT}/")) {
            return None;
        }
        path[ROOT.len()..].trim_start_matches('/')
    } else {
        path.strip_prefix("./").unwrap_or(path)
    };
    let clean = normalize(relative).ok()?;
    if within(&clean, DIRECTORY) {
        return None;
    }
    Some(clean)
}

/// The workspace-relative key below `.workspace/` of a file in a guest agent home, refusing
/// credentials. Used for vendor-owned files such as Claude transcripts.
pub fn agent_home_key(guest_path: &str) -> Option<String> {
    for home in AGENT_HOMES {
        let Some(rest) = guest_path.strip_prefix(&format!("{}/", home.guest)) else {
            continue;
        };
        let rest = normalize(rest).ok().filter(|r| !r.is_empty())?;
        let first = rest.split('/').next().unwrap_or_default();
        if home.credentials.contains(&first) {
            return None;
        }
        return Some(format!("{}/{rest}", home.key()));
    }
    None
}

pub fn guess_mime(path: &str) -> Option<&'static str> {
    let ext = path.rsplit_once('.').map(|(_, e)| e.to_ascii_lowercase());
    match ext.as_deref() {
        Some("png") => Some("image/png"),
        Some("jpg" | "jpeg") => Some("image/jpeg"),
        Some("gif") => Some("image/gif"),
        Some("webp") => Some("image/webp"),
        Some("bmp") => Some("image/bmp"),
        Some("pdf") => Some("application/pdf"),
        Some("md") => Some("text/markdown"),
        Some("txt") => Some("text/plain"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn agent_paths_match_the_retained_resolution() {
        assert_eq!(to_agent("").unwrap(), "/workspace");
        assert_eq!(to_agent("src/a.rs").unwrap(), "/workspace/src/a.rs");
        assert_eq!(
            to_agent(".workspace/agents/codex/config.toml").unwrap(),
            "/home/work/.codex/config.toml"
        );
        assert_eq!(
            to_agent(".workspace/agents/claude").unwrap(),
            "/home/work/.claude"
        );
        assert!(to_agent("../x").is_err());
        assert!(to_agent("/etc/passwd").is_err());
        assert_eq!(
            to_workspace("/workspace/src/a.rs").as_deref(),
            Some("src/a.rs")
        );
        assert_eq!(to_workspace("file:///workspace/a").as_deref(), Some("a"));
        assert_eq!(to_workspace("./b").as_deref(), Some("b"));
        assert_eq!(
            to_workspace("/home/work/.claude/CLAUDE.md").as_deref(),
            Some(".workspace/agents/claude/CLAUDE.md")
        );
        assert_eq!(to_workspace("/workspace/.workspace/state"), None);
        assert_eq!(to_workspace("/etc/passwd"), None);
        assert_eq!(to_workspace("/workspace/../etc"), None);
        assert_eq!(
            agent_home_key("/home/work/.claude/projects/-workspace/s.jsonl").as_deref(),
            Some("agents/claude/projects/-workspace/s.jsonl")
        );
        assert_eq!(agent_home_key("/home/work/.claude/.credentials.json"), None);
        assert_eq!(agent_home_key("/home/work/.codex/auth.json"), None);
        assert_eq!(agent_home_key("/home/work/other"), None);
    }
}
