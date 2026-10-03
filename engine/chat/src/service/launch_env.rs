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
pub struct Secret(pub String);
impl std::fmt::Debug for Secret {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.write_str("<redacted>")
    }
}
