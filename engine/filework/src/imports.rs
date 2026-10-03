//! Engine-owned, race-safe destination allocation for imported user files.
use crate::fs::user_path;
use std::path::Path;
use workflow_environment::{
    error::{Error, Result},
    store::UploadStage,
};

pub fn validate(root: &Path, directory: &str, name: &str) -> Result<()> {
    if name.trim().is_empty()
        || name == "."
        || name == ".."
        || name.len() > 255
        || name
            .chars()
            .any(|c| c == '/' || c == '\\' || c.is_control())
        || directory == ".workspace"
        || directory.starts_with(".workspace/")
    {
        return Err(Error::invalid("invalid import destination"));
    }
    user_path(root, directory, true)?;
    Ok(())
}
pub fn publish(
    root: &Path,
    stage: &mut UploadStage,
    directory: &str,
    name: &str,
) -> Result<String> {
    validate(root, directory, name)?;
    let parent = user_path(root, directory, true)?;
    std::fs::create_dir_all(&parent)?;
    for n in 0..10000 {
        let candidate = variant(name, n);
        let relative = if directory.is_empty() {
            candidate
        } else {
            format!("{directory}/{candidate}")
        };
        let destination = user_path(root, &relative, false)?;
        match stage.publish_new(&destination) {
            Ok(()) => {
                std::fs::File::open(&parent)?.sync_all()?;
                return Ok(relative);
            }
            Err(e) if destination.exists() => {
                let _ = e;
                continue;
            }
            Err(e) => return Err(e),
        }
    }
    Err(Error::business("name_limit", "no free import destination"))
}
fn prefix_bytes(text: &str, limit: usize) -> &str {
    let mut end = text.len().min(limit);
    while !text.is_char_boundary(end) {
        end -= 1
    }
    &text[..end]
}
fn variant(name: &str, n: usize) -> String {
    if n == 0 {
        return name.into();
    }
    let lower = name.to_lowercase();
    let split = [".tar.gz", ".tar.xz", ".tar.bz2", ".tar.zst", ".d.ts"]
        .iter()
        .find(|suffix| lower.ends_with(**suffix) && name.len() > suffix.len())
        .map(|suffix| name.len() - suffix.len())
        .or_else(|| name.rfind('.').filter(|i| *i > 0 && *i < name.len() - 1));
    let (stem, extension) = split
        .map(|i| (&name[..i], &name[i..]))
        .unwrap_or((name, ""));
    let suffix = format!(" ({n})");
    let extension = prefix_bytes(extension, 255 - suffix.len() - 1);
    let stem = prefix_bytes(stem, 255 - suffix.len() - extension.len());
    format!("{stem}{suffix}{extension}")
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn variants_preserve_compound_extensions_and_utf8_limits() {
        assert_eq!(variant("bundle.tar.gz", 1), "bundle (1).tar.gz");
        let name = variant(&format!("{}.txt", "界".repeat(83)), 1);
        assert!(name.len() <= 255);
        assert!(name.ends_with(" (1).txt"));
    }
    #[test]
    fn private_state_and_path_components_cannot_be_allocated() {
        let d = tempfile::tempdir().unwrap();
        for (dir, name) in [
            (".workspace/services/proxy", "x"),
            ("", "../x"),
            ("", ""),
            ("../escape", "x"),
        ] {
            assert!(validate(d.path(), dir, name).is_err())
        }
    }
}
