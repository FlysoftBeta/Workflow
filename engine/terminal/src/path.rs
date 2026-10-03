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
