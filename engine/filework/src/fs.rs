use crate::FileVersion;
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File, OpenOptions},
    io::Read,
    path::{Path, PathBuf},
    time::UNIX_EPOCH,
};
use workflow_environment::error::{Error, Result};

pub fn valid_path(p: &str) -> bool {
    !p.is_empty()
        && !p.starts_with('/')
        && !p.contains('\0')
        && !p.contains('\\')
        && p.split('/').all(|s| !s.is_empty() && s != "." && s != "..")
        && p != ".workspace"
        && (!p.starts_with(".workspace/")
            || matches!(p, ".workspace/config.json" | ".workspace/env.json")
            || p.strip_prefix(".workspace/services/")
                .is_some_and(|s| s.split('/').count() >= 2 && s.split('/').all(identifier_valid)))
}
pub fn identifier_valid(id: &str) -> bool {
    !id.is_empty()
        && id != "."
        && id != ".."
        && id.len() <= 160
        && id
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, b'-' | b'_' | b'.'))
}
pub fn identifier(id: &str) -> Result<&str> {
    if identifier_valid(id) {
        Ok(id)
    } else {
        Err(Error::invalid("invalid document identifier"))
    }
}
pub fn user_path(root: &Path, raw: &str, allow_root: bool) -> Result<PathBuf> {
    if raw.is_empty() && allow_root {
        return Ok(root.to_owned());
    }
    if !valid_path(raw) || raw.starts_with(".workspace/") {
        return Err(Error::invalid("not an editable user-file path"));
    }
    let mut p = root.to_owned();
    for c in raw.split('/') {
        p.push(c);
        match fs::symlink_metadata(&p) {
            Ok(m) if m.file_type().is_symlink() => {
                return Err(Error::business(
                    "unsafe_path",
                    "symbolic links are not followed",
                ));
            }
            Ok(_) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(e.into()),
        }
    }
    Ok(p)
}
pub fn version(path: &Path) -> Result<FileVersion> {
    let m = match fs::metadata(path) {
        Ok(m) if m.is_file() => m,
        Ok(_) => return Ok(FileVersion::default()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(FileVersion::default()),
        Err(e) => return Err(e.into()),
    };
    let sha256 = if m.len() <= 64 * 1024 * 1024 {
        let mut file = File::open(path)?;
        let mut digest = Sha256::new();
        let mut chunk = [0u8; 65536];
        loop {
            let n = file.read(&mut chunk)?;
            if n == 0 {
                break;
            }
            digest.update(&chunk[..n]);
        }
        Some(
            digest
                .finalize()
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect(),
        )
    } else {
        None
    };
    Ok(FileVersion {
        exists: true,
        size: m.len(),
        modified_at: modified_at(&m),
        sha256,
        extra: Default::default(),
    })
}
pub fn modified_at(m: &fs::Metadata) -> u64 {
    m.modified()
        .unwrap_or(UNIX_EPOCH)
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}
pub use workflow_environment::persist::{atomic, create_atomic};
pub fn copy_tree(from: &Path, to: &Path) -> Result<()> {
    let m = fs::symlink_metadata(from)?;
    if m.file_type().is_symlink() {
        return Err(Error::business(
            "unsafe_path",
            "copy does not follow symbolic links",
        ));
    }
    if m.is_dir() {
        fs::create_dir(to)?;
        for e in fs::read_dir(from)? {
            let e = e?;
            copy_tree(&e.path(), &to.join(e.file_name()))?
        }
    } else if m.is_file() {
        if m.len() > 16 * 1024 * 1024 {
            return Err(Error::business("too_large", "file exceeds copy size limit"));
        }
        let mut src = File::open(from)?;
        let mut dst = OpenOptions::new().create_new(true).write(true).open(to)?;
        std::io::copy(&mut src, &mut dst)?;
        dst.sync_all()?;
    } else {
        return Err(Error::business(
            "unsupported",
            "special files cannot be copied",
        ));
    }
    Ok(())
}
pub fn within(p: &str, parent: &str) -> bool {
    p == parent || p.starts_with(&format!("{parent}/"))
}
pub fn rebase(p: &str, from: &str, to: &str) -> String {
    if p == from {
        to.into()
    } else if p.starts_with(&format!("{from}/")) {
        format!("{to}{}", &p[from.len()..])
    } else {
        p.into()
    }
}
