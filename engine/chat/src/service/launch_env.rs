//! Inherited host credentials and dynamic loader injection never enter vendor launches.
use std::collections::BTreeMap;
pub fn filtered(base: &BTreeMap<String, String>) -> BTreeMap<String, String> {
    const DENIED: [&str; 7] = [
        "OPENAI_",
        "CODEX_",
        "ANTHROPIC_",
        "CLAUDE_",
        "CLAUDECODE",
        "LD_PRELOAD",
        "LD_LIBRARY_PATH",
    ];
    base.iter()
        .filter(|(key, _)| !DENIED.iter().any(|p| key.starts_with(p)))
        .map(|(key, value)| (key.clone(), value.clone()))
        .collect()
}
/// A credential held in memory only. Its `Debug` form is redacted and it is never persisted.
#[derive(Clone, PartialEq, Eq)]
pub struct Secret(pub String);

