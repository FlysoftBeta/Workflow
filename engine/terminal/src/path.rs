use workflow_environment::{Error, Result};

/// Convert an explicitly selected workspace-relative directory to its guest path.
pub fn working_directory(relative: &str) -> Result<String> {
    if relative.starts_with('/')
        || relative.split('/').any(|p| p == "..")
        || relative.contains('\0')
    {
        return Err(Error::invalid("invalid terminal directory"));
    }
    Ok(if relative.is_empty() {
        "/workspace".into()
    } else {
        format!("/workspace/{relative}")
    })
}

/// Resolve terminal output against an observed guest cwd without touching files.
/// The returned workspace-relative candidate still requires FileWork's access checks.
pub fn resolve_workspace_path(cwd: &str, path: &str) -> Result<String> {
    if path.is_empty() || path.contains('\0') || cwd.contains('\0') {
        return Err(Error::invalid("invalid terminal path"));
    }
    let absolute = if path.starts_with('/') {
        path.to_owned()
    } else {
        format!("{cwd}/{path}")
    };
    let mut segments = Vec::new();
    for segment in absolute.split('/') {
        match segment {
            "" | "." => {}
            ".." => {
                if segments.len() <= 1 {
                    return Err(Error::invalid("terminal path leaves workspace"));
                }
                segments.pop();
            }
            part => segments.push(part),
        }
    }
    if !absolute.starts_with('/') || segments.first() != Some(&"workspace") {
        return Err(Error::invalid("terminal path is outside workspace"));
    }
    Ok(segments[1..].join("/"))
}

pub(crate) fn decode_cwd(path: &str) -> Option<String> {
    let input = path.as_bytes();
    let mut bytes = Vec::with_capacity(input.len());
    let mut i = 0;
    while i < input.len() {
        if input[i] == b'%' && i + 2 < input.len() {
            if let (Some(a), Some(b)) = (
                (input[i + 1] as char).to_digit(16),
                (input[i + 2] as char).to_digit(16),
            ) {
                bytes.push((a * 16 + b) as u8);
                i += 3;
                continue;
            }
        }
        bytes.push(input[i]);
        i += 1;
    }
    let value = String::from_utf8(bytes).ok()?;
    if value.len() > 4096 || value.contains('\0') || value.split('/').any(|p| p == "..") {
        return None;
    }
    Some(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn osc_working_directories_decode_unicode_and_reject_parent_traversal() {
        assert_eq!(
            decode_cwd("/workspace/a%20b/%E4%B8%AD").unwrap(),
            "/workspace/a b/中"
        );
        assert!(decode_cwd("/workspace/%2e%2e/elsewhere").is_none());
        assert!(decode_cwd("/workspace/%00").is_none());
        assert!(decode_cwd("/workspace/%ff").is_none());
    }
    #[test]
    fn directories_and_link_candidates_stay_within_the_workspace() {
        assert_eq!(working_directory("").unwrap(), "/workspace");
        assert_eq!(working_directory("src").unwrap(), "/workspace/src");
        assert!(working_directory("../elsewhere").is_err());
        assert!(working_directory("/tmp").is_err());
        assert_eq!(
            resolve_workspace_path("/workspace/src", "../a b.rs").unwrap(),
            "a b.rs"
        );
        assert_eq!(
            resolve_workspace_path("/tmp", "/workspace/src/main.rs").unwrap(),
            "src/main.rs"
        );
        assert!(resolve_workspace_path("/tmp", "main.rs").is_err());
        assert!(resolve_workspace_path("/workspace", "../secret").is_err());
        assert!(resolve_workspace_path("/workspace", "/workspace-other/a").is_err());
    }
}

#[derive(
    Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "lowercase")]
pub enum PathKind {
    File,
    Directory,
}
#[derive(
    Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "camelCase")]
#[schemars(rename = "TerminalResolvedPath")]
pub struct ResolvedPath {
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<PathKind>,
    /// Zero-based source position, converted from a positive terminal suffix.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub column: Option<u32>,
}
#[derive(
    Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "camelCase")]
#[schemars(rename = "TerminalResolvedPaths")]
pub struct ResolvedPaths {
    pub terminal_id: String,
    pub generation: u64,
    pub cwd: String,
    pub paths: Vec<ResolvedPath>,
}
impl ResolvedPath {
    pub fn rejected(text: &str) -> Self {
        Self {
            text: text.into(),
            path: None,
            kind: None,
            line: None,
            column: None,
        }
    }
}
/// Candidate names only. Existence and symlink checks are FileWork's authority.
/// Try the exact filename before interpreting a source-position suffix.
pub fn candidates(cwd: &str, text: &str) -> Vec<(String, Option<u32>, Option<u32>)> {
    let mut out = Vec::new();
    if text.is_empty() || text.len() > 4096 || text.chars().any(char::is_control) {
        return out;
    }
    let input = text.trim().trim_matches(|c| c == '\'' || c == '"');
    let mut fragment = None;
    let path = if let Some(uri) = input.strip_prefix("file://") {
        let Some((authority, rest)) = uri.split_once('/') else {
            return out;
        };
        if !authority.is_empty() && !authority.eq_ignore_ascii_case("localhost") {
            return out;
        }
        let (uri_path, suffix) = rest
            .split_once('#')
            .map_or((rest, None), |(p, f)| (p, Some(f)));
        if uri_path.contains('?') {
            return out;
        }
        fragment = suffix.and_then(hash_position);
        let Some(decoded) = percent_decode(uri_path) else {
            return out;
        };
        format!("/{decoded}")
    } else {
        // A non-file URI belongs to the URL handler, never to a workspace filename.
        if input.contains("://") {
            return out;
        }
        input.to_owned()
    };
    if let Ok(candidate) = resolve_workspace_path(cwd, &path) {
        let (line, column) = fragment.map_or((None, None), |(l, c)| (Some(l), c));
        out.push((candidate, line, column));
    }
    if fragment.is_none() {
        if let Some((file, line, column)) = source_suffix(&path) {
            if let Ok(candidate) = resolve_workspace_path(cwd, file) {
                out.push((candidate, Some(line), column));
            }
        }
    }
    out
}
fn coordinate(text: &str) -> Option<u32> {
    text.trim()
        .parse::<u32>()
        .ok()
        .filter(|n| *n > 0)
        .map(|n| n - 1)
}
fn hash_position(text: &str) -> Option<(u32, Option<u32>)> {
    let text = text.strip_prefix('L')?;
    let (line, column) = text
        .split_once('C')
        .map_or((text, None), |(l, c)| (l, Some(c)));
    let column = match column {
        Some(value) => Some(coordinate(value)?),
        None => None,
    };
    Some((coordinate(line)?, column))
}
fn source_suffix(path: &str) -> Option<(&str, u32, Option<u32>)> {
    if let Some((file, suffix)) = path.rsplit_once('#') {
        if let Some((line, column)) = hash_position(suffix) {
            return Some((file, line, column));
        }
    }
    if let Some(prefix) = path.strip_suffix(')') {
        if let Some((file, position)) = prefix.rsplit_once('(') {
            if let Some((line, column)) = position.split_once(',') {
                return Some((file, coordinate(line)?, Some(coordinate(column)?)));
            }
            return Some((file, coordinate(position)?, None));
        }
    }
    let (head, last) = path.rsplit_once(':')?;
    let last = coordinate(last)?;
    if let Some((file, line)) = head
        .rsplit_once(':')
        .and_then(|(f, n)| coordinate(n).map(|n| (f, n)))
    {
        Some((file, line, Some(last)))
    } else {
        Some((head, last, None))
    }
}
fn percent_decode(text: &str) -> Option<String> {
    let input = text.as_bytes();
    let mut bytes = Vec::with_capacity(input.len());
    let mut i = 0;
    while i < input.len() {
        if input[i] == b'%' {
            let a = (*input.get(i + 1)? as char).to_digit(16)?;
            let b = (*input.get(i + 2)? as char).to_digit(16)?;
            let decoded = (a * 16 + b) as u8;
            // Escaped separators must not change path-component or authority boundaries.
            if matches!(decoded, b'/' | b'\\' | 0) {
                return None;
            }
            bytes.push(decoded);
            i += 3;
        } else {
            bytes.push(input[i]);
            i += 1;
        }
    }
    let text = String::from_utf8(bytes).ok()?;
    if text.chars().any(char::is_control) {
        None
    } else {
        Some(text)
    }
}

#[cfg(test)]
mod resolution_tests {
    use super::*;
    #[test]
    fn source_positions_are_zero_based_and_literal_names_are_tried_first() {
        assert_eq!(
            candidates("/workspace/src", "main.rs:12:3"),
            vec![
                ("src/main.rs:12:3".into(), None, None),
                ("src/main.rs".into(), Some(11), Some(2))
            ]
        );
        assert_eq!(
            candidates("/workspace", "'space name.rs:1'"),
            vec![
                ("space name.rs:1".into(), None, None),
                ("space name.rs".into(), Some(0), None)
            ]
        );
        assert_eq!(candidates("/workspace", "x:0").len(), 1);
        assert!(candidates("/workspace", "../../outside:5").is_empty());
        assert!(candidates("/workspace", "bad\u{1b}[0m").is_empty());
    }
}

#[cfg(test)]
mod uri_tests {
    use super::*;
    #[test]
    fn local_file_uris_decode_unicode_spaces_and_keep_source_positions() {
        assert_eq!(
            candidates(
                "/workspace/elsewhere",
                "file://localhost/workspace/src/a%20b.rs:3:2"
            )
            .last()
            .unwrap(),
            &("src/a b.rs".into(), Some(2), Some(1))
        );
        assert_eq!(
            candidates("/workspace", "file:///workspace/%E4%B8%AD.rs#L3C2"),
            vec![("中.rs".into(), Some(2), Some(1))]
        );
        assert_eq!(
            candidates("/workspace", "file://LOCALHOST/workspace/a%2Bb.rs")[0].0,
            "a+b.rs"
        );
        assert_eq!(
            candidates("/workspace/src", "file:///workspace/main.rs")[0].0,
            "main.rs"
        );
        assert_eq!(
            candidates("/workspace/src", "foo.rs(3, 2)").last().unwrap(),
            &("src/foo.rs".into(), Some(2), Some(1))
        );
        assert_eq!(
            candidates("/workspace/src", "foo.rs(3)").last().unwrap(),
            &("src/foo.rs".into(), Some(2), None)
        );
    }
    #[test]
    fn uri_authority_utf8_and_encoded_boundaries_fail_closed() {
        for path in [
            "file://remote/workspace/foo",
            "file://user@localhost/workspace/foo",
            "file://localhost:80/workspace/foo",
            "file://localhost",
            "file:///workspace/%",
            "file:///workspace/%zz",
            "file:///workspace/%ff",
            "file:///workspace/%00",
            "file:///workspace/%2fsecret",
            "file:///workspace/%5csecret",
            "file:///workspace/%2e%2e/outside",
            "file://%2flocalhost/workspace/foo",
            "https://example.test/workspace/foo",
        ] {
            assert!(candidates("/workspace", path).is_empty(), "{path}");
        }
    }
}
