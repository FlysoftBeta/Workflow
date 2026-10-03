//! Atomic destination allocation for external imports, independent of client enumeration.
use crate::{
    protocol::{Error, Result},
    storage,
};
use std::{fs, path::Path};
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
    storage::path(root, directory, true)?;
    Ok(())
}
pub fn publish(root: &Path, temp: &Path, directory: &str, name: &str) -> Result<String> {
    validate(root, directory, name)?;
    let parent = storage::path(root, directory, true)?;
    fs::create_dir_all(&parent)?;
    for n in 0..10000 {
        let candidate = variant(name, n);
        let relative = if directory.is_empty() {
            candidate
        } else {
            format!("{directory}/{candidate}")
        };
        let destination = storage::path(root, &relative, false)?;
        match storage::publish_new(temp, &destination) {
            Ok(()) => {
                fs::File::open(parent)?.sync_all()?;
                return Ok(relative);
            }
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(e.into()),
        }
    }
    Err(Error::business("name_limit", "no free import destination"))
}
fn prefix_bytes(text: &str, limit: usize) -> &str {
    let mut end = text.len().min(limit);
    while !text.is_char_boundary(end) {
        end -= 1;
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
    fn concurrent_imports_allocate_without_overwriting() {
        let root = tempfile::tempdir().unwrap();
        let temp = root.path().join("upload");
        fs::write(root.path().join("photo.jpg"), b"original").unwrap();
        fs::write(&temp, b"new").unwrap();
        assert_eq!(
            publish(root.path(), &temp, "", "photo.jpg").unwrap(),
            "photo (1).jpg"
        );
        assert_eq!(
            fs::read(root.path().join("photo.jpg")).unwrap(),
            b"original"
        );
        fs::write(&temp, b"next").unwrap();
        assert_eq!(
            publish(root.path(), &temp, "", "photo.jpg").unwrap(),
            "photo (2).jpg"
        );
    }
    #[test]
    fn variants_preserve_compound_extensions_and_utf8_limits() {
        assert_eq!(variant("bundle.tar.gz", 1), "bundle (1).tar.gz");
        let name = format!("{}.txt", "界".repeat(83));
        let name = variant(&name, 1);
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
            assert!(validate(d.path(), dir, name).is_err());
        }
    }
}
