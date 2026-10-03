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
        || name.len() > 200
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
    let (stem, extension) = match name.rfind('.').filter(|i| *i > 0) {
        Some(i) => (&name[..i], &name[i..]),
        None => (name, ""),
    };
    for n in 0..10000 {
        let candidate = if n == 0 {
            name.to_owned()
        } else {
            format!("{stem} ({n}){extension}")
        };
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
