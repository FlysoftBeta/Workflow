//! Transactional post-script home: scripts write a private copy; activation merges
//! only their delta, detects intervening user edits, and exchanges directories.
use crate::{
    protocol::{Error, Result},
    storage,
};
use serde_json::{Value as V, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    os::unix::{
        ffi::OsStrExt,
        fs::{DirBuilderExt, PermissionsExt, symlink},
    },
    path::{Path, PathBuf},
};
fn scan(root: &Path) -> Result<BTreeMap<String, V>> {
    fn visit(root: &Path, dir: &Path, out: &mut BTreeMap<String, V>) -> Result<()> {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            let relative = path
                .strip_prefix(root)
                .unwrap()
                .to_str()
                .ok_or_else(|| Error::business("home_stage", "home contains a non-UTF-8 filename"))?
                .to_owned();
            let meta = fs::symlink_metadata(&path)?;
            let kind = if meta.is_dir() {
                "directory"
            } else if meta.is_file() {
                "file"
            } else if meta.file_type().is_symlink() {
                "symlink"
            } else {
                continue;
            };
            let mut value = json!({"kind":kind,"mode":meta.permissions().mode()&0o7777});
            if kind == "file" {
                use sha2::{Digest, Sha256};
                use std::io::Read;
                let mut file = fs::File::open(&path)?;
                let mut digest = Sha256::new();
                let mut bytes = [0u8; 65536];
                loop {
                    let n = file.read(&mut bytes)?;
                    if n == 0 {
                        break;
                    }
                    digest.update(&bytes[..n]);
                }
                value["sha256"] = json!(
                    digest
                        .finalize()
                        .iter()
                        .map(|b| format!("{b:02x}"))
                        .collect::<String>()
                );
            } else if kind == "symlink" {
                value["target"] =
                    json!(
                        fs::read_link(&path)?
                            .to_str()
                            .ok_or_else(|| Error::business(
                                "home_stage",
                                "home symlink target is not UTF-8"
                            ))?
                    );
            }
            out.insert(relative, value);
            if kind == "directory" {
                visit(root, &path, out)?;
            }
        }
        Ok(())
    }
    let mut out = BTreeMap::new();
    if root.exists() {
        visit(root, root, &mut out)?;
    }
    Ok(out)
}
fn copy_tree(from: &Path, to: &Path) -> Result<()> {
    let meta = fs::symlink_metadata(from)?;
    if meta.file_type().is_symlink() {
        symlink(fs::read_link(from)?, to)?;
    } else if meta.is_dir() {
        let existed = to.exists();
        fs::create_dir_all(to)?;
        for entry in fs::read_dir(from)? {
            let entry = entry?;
            copy_tree(&entry.path(), &to.join(entry.file_name()))?;
        }
        if !existed {
            fs::set_permissions(to, meta.permissions())?;
        }
    } else if meta.is_file() {
        fs::copy(from, to)?;
        fs::set_permissions(to, meta.permissions())?;
    }
    // Unix sockets/FIFOs are process artifacts; never copy or expose their bytes.
    Ok(())
}
fn remove(path: &Path) -> Result<()> {
    match fs::symlink_metadata(path) {
        Ok(m) => {
            if m.is_dir() {
                fs::remove_dir_all(path)?;
            } else {
                fs::remove_file(path)?;
            }
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => return Err(e.into()),
    }
    Ok(())
}
fn live_home(root: &Path) -> PathBuf {
    root.join(".workspace/environment/stores/home/work")
}
pub fn prepare(root: &Path, generation: &Path) -> Result<()> {
    let live = live_home(root);
    fs::create_dir_all(&live)?;
    if !fs::symlink_metadata(&live)?.is_dir() {
        return Err(Error::business(
            "home_stage",
            "persistent home root is not a directory",
        ));
    }
    let before = scan(&live)?;
    let stage = generation.join("post-home");
    fs::DirBuilder::new().mode(0o700).create(&stage)?;
    copy_tree(&live, &stage)?;
    fs::set_permissions(&stage, fs::Permissions::from_mode(0o700))?;
    if scan(&live)? != before || scan(&stage)? != before {
        return Err(Error::business(
            "home_changed",
            "home changed while preparing post scripts; retry",
        ));
    }
    storage::write_json(&generation.join("home-baseline.json"), &json!(before))
}
fn baseline(generation: &Path) -> Result<BTreeMap<String, V>> {
    let v = storage::read_json(&generation.join("home-baseline.json"))?;
    serde_json::from_value(v).map_err(|_| Error::business("home_stage", "invalid home baseline"))
}
/// Called only with the environment lock and after all managed processes stop.
pub fn activate(root: &Path, generation: &Path) -> Result<()> {
    let stage = generation.join("post-home");
    if !stage.exists() || generation.join("home-applied").exists() {
        return Ok(());
    }
    if !fs::symlink_metadata(&stage)?.is_dir() {
        return Err(Error::business(
            "home_stage",
            "post scripts replaced the home root with a non-directory",
        ));
    }
    let before = baseline(generation)?;
    let after = scan(&stage)?;
    let live = live_home(root);
    fs::create_dir_all(&live)?;
    let current = scan(&live)?;
    let keys: BTreeSet<String> = before.keys().chain(after.keys()).cloned().collect();
    let changed: Vec<String> = keys
        .into_iter()
        .filter(|k| before.get(k) != after.get(k))
        .collect();
    for path in &changed {
        if current.get(path) != before.get(path) {
            return Err(Error::business(
                "home_conflict",
                "home changed at a post-script path; old environment retained, rebuild to retry",
            ));
        }
        if before.get(path).is_some_and(|v| v["kind"] == "directory")
            && after.get(path).is_none_or(|v| v["kind"] != "directory")
        {
            let prefix = format!("{path}/");
            if current
                .iter()
                .filter(|(k, _)| k.starts_with(&prefix))
                .any(|(k, v)| before.get(k) != Some(v))
            {
                return Err(Error::business(
                    "home_conflict",
                    "new home content conflicts with a post-script deletion",
                ));
            }
        }
    }
    if changed.is_empty() {
        storage::atomic(&generation.join("home-applied"), b"1\n")?;
        return Ok(());
    }
    let candidate = generation.join("home-activation");
    remove(&candidate)?;
    fs::DirBuilder::new().mode(0o700).create(&candidate)?;
    copy_tree(&live, &candidate)?;
    fs::set_permissions(&candidate, fs::Permissions::from_mode(0o700))?;
    // Parents precede children in BTree order. Remove old type first, then materialize
    // the staged entry; unchanged descendants come from the current home copy.
    for path in &changed {
        if Path::new(path)
            .ancestors()
            .skip(1)
            .filter_map(|p| p.to_str())
            .filter(|p| !p.is_empty())
            .any(|parent| after.get(parent).is_none_or(|v| v["kind"] != "directory"))
        {
            continue;
        }
        let target = candidate.join(path);
        match after.get(path) {
            None => remove(&target)?,
            Some(value) if value["kind"] == "directory" => {
                if fs::symlink_metadata(&target).is_ok_and(|m| !m.is_dir()) {
                    remove(&target)?;
                }
                fs::create_dir_all(&target)?;
                fs::set_permissions(
                    &target,
                    fs::Permissions::from_mode(value["mode"].as_u64().unwrap_or(0o755) as u32),
                )?;
            }
            Some(_) => {
                remove(&target)?;
                fs::create_dir_all(target.parent().unwrap())?;
                copy_tree(&stage.join(path), &target)?;
            }
        }
    }
    if scan(&live)? != current {
        remove(&candidate)?;
        return Err(Error::business(
            "home_changed",
            "home changed during activation; retry restart",
        ));
    }
    // Linux/Android renameat2 exchange makes the complete home switch atomic.
    let a = std::ffi::CString::new(candidate.as_os_str().as_bytes()).unwrap();
    let b = std::ffi::CString::new(live.as_os_str().as_bytes()).unwrap();
    let rc = unsafe {
        libc::syscall(
            libc::SYS_renameat2,
            libc::AT_FDCWD,
            a.as_ptr(),
            libc::AT_FDCWD,
            b.as_ptr(),
            libc::RENAME_EXCHANGE,
        )
    };
    if rc != 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    // Candidate now contains the prior home; do not retain a secret-bearing backup.
    let _ = remove(&candidate);
    let _ = storage::atomic(&generation.join("home-applied"), b"1\n");
    let _ = remove(&stage);
    Ok(())
}
pub fn cleanup(generation: &Path) {
    let _ = remove(&generation.join("post-home"));
    let _ = remove(&generation.join("home-activation"));
    let _ = fs::remove_file(generation.join("home-baseline.json"));
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn merge_preserves_unrelated_live_edits_and_publishes_script_changes() {
        let t = tempfile::tempdir().unwrap();
        let root = t.path();
        let live = live_home(root);
        fs::create_dir_all(&live).unwrap();
        fs::write(live.join("existing"), "old").unwrap();
        let generation = root.join("gen");
        fs::create_dir(&generation).unwrap();
        prepare(root, &generation).unwrap();
        fs::write(generation.join("post-home/script"), "created by script").unwrap();
        fs::write(live.join("existing"), "edited while building").unwrap();
        activate(root, &generation).unwrap();
        assert_eq!(
            fs::read_to_string(live.join("existing")).unwrap(),
            "edited while building"
        );
        assert_eq!(
            fs::read_to_string(live.join("script")).unwrap(),
            "created by script"
        );
        assert!(!generation.join("post-home").exists());
        assert!(!generation.join("home-activation").exists());
    }
    #[test]
    fn conflict_keeps_old_home_and_failed_scripts_never_publish() {
        let t = tempfile::tempdir().unwrap();
        let root = t.path();
        let live = live_home(root);
        fs::create_dir_all(&live).unwrap();
        fs::write(live.join("same"), "base").unwrap();
        let generation = root.join("gen");
        fs::create_dir(&generation).unwrap();
        prepare(root, &generation).unwrap();
        fs::write(generation.join("post-home/same"), "script").unwrap();
        assert_eq!(fs::read_to_string(live.join("same")).unwrap(), "base");
        fs::write(live.join("same"), "active user").unwrap();
        assert_eq!(
            activate(root, &generation).unwrap_err().kind,
            "home_conflict"
        );
        assert_eq!(
            fs::read_to_string(live.join("same")).unwrap(),
            "active user"
        );
    }
}

#[cfg(test)]
mod directory_replacement_tests {
    use super::*;
    #[test]
    fn replacing_directory_with_symlink_never_touches_target_children() {
        let t = tempfile::tempdir().unwrap();
        let root = t.path();
        let live = live_home(root);
        fs::create_dir_all(live.join("dir")).unwrap();
        fs::write(live.join("dir/old"), "base").unwrap();
        let outside = tempfile::tempdir().unwrap();
        fs::write(outside.path().join("old"), "must remain").unwrap();
        let generation = root.join("gen");
        fs::create_dir(&generation).unwrap();
        prepare(root, &generation).unwrap();
        fs::remove_dir_all(generation.join("post-home/dir")).unwrap();
        symlink(outside.path(), generation.join("post-home/dir")).unwrap();
        activate(root, &generation).unwrap();
        assert!(
            fs::symlink_metadata(live.join("dir"))
                .unwrap()
                .file_type()
                .is_symlink()
        );
        assert_eq!(
            fs::read_to_string(outside.path().join("old")).unwrap(),
            "must remain"
        );
    }
}
