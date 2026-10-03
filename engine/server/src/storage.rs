use crate::{
    layout,
    protocol::{Error, Result},
};
use serde_json::{Value as V, json};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};
pub fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}
pub fn hash(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
pub fn atomic(path: &Path, bytes: &[u8]) -> Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| Error::invalid("missing parent"))?;
    fs::create_dir_all(parent)?;
    let temp = parent.join(format!(".write-{}", uuid::Uuid::new_v4()));
    let result = (|| {
        let mut f = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&temp)?;
        if let Ok(metadata) = fs::metadata(path) {
            f.set_permissions(metadata.permissions())?;
        }
        f.write_all(bytes)?;
        f.sync_all()?;
        fs::rename(&temp, path)?;
        File::open(parent)?.sync_all()?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(temp);
    }
    result
}
pub fn create_atomic(path: &Path, bytes: &[u8]) -> Result<()> {
    let parent = path.parent().unwrap();
    fs::create_dir_all(parent)?;
    let temp = parent.join(format!(".write-{}", uuid::Uuid::new_v4()));
    let result = (|| {
        let mut f = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&temp)?;
        f.write_all(bytes)?;
        f.sync_all()?;
        publish_new(&temp, path)?;
        File::open(parent)?.sync_all()?;
        Ok(())
    })();
    let _ = fs::remove_file(temp);
    result
}
/// Publish a completed file without replacing a concurrently created destination. Android app
/// sandboxes prohibit hard links; renameat2 provides the same atomic no-overwrite guarantee.
pub fn publish_new(from: &Path, to: &Path) -> std::io::Result<()> {
    use std::{ffi::CString, os::unix::ffi::OsStrExt};
    let from = CString::new(from.as_os_str().as_bytes())?;
    let to = CString::new(to.as_os_str().as_bytes())?;
    let result = unsafe {
        libc::syscall(libc::SYS_renameat2, libc::AT_FDCWD, from.as_ptr(),
            libc::AT_FDCWD, to.as_ptr(), libc::RENAME_NOREPLACE)
    };
    if result == 0 { Ok(()) } else { Err(std::io::Error::last_os_error()) }
}
/// Walk every existing component and refuse symlinks, including symlinks into the root.
/// This is an app-owned workspace, not a security boundary against processes with the same UID.
pub fn path(root: &Path, raw: &str, allow_root: bool) -> Result<PathBuf> {
    if raw.is_empty() && allow_root {
        return Ok(root.to_owned());
    }
    if !layout::valid_path(raw) {
        return Err(Error::invalid("not an editable workspace path"));
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
pub fn version(path: &Path) -> Result<V> {
    let m = match fs::metadata(path) {
        Ok(m) if m.is_file() => m,
        Ok(_) => return Ok(missing()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(missing()),
        Err(e) => return Err(e.into()),
    };
    let sha = if m.len() <= 64 * 1024 * 1024 {
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
                .collect::<String>(),
        )
    } else {
        None
    };
    Ok(
        json!({"exists":true,"size":m.len(),"modifiedAt":m.modified().unwrap_or(UNIX_EPOCH).duration_since(UNIX_EPOCH).unwrap_or_default().as_millis()as u64,"sha256":sha}),
    )
}
pub fn missing() -> V {
    json!({"exists":false,"size":0,"modifiedAt":0,"sha256":null})
}
pub fn same(a: &V, b: &V) -> bool {
    if a["exists"] == false && b["exists"] == false {
        return true;
    }
    if a["exists"] != b["exists"] {
        return false;
    }
    if a["sha256"].is_string() && b["sha256"].is_string() {
        a["sha256"] == b["sha256"]
    } else {
        a["size"] == b["size"] && a["modifiedAt"] == b["modifiedAt"]
    }
}
pub fn read_json(path: &Path) -> Result<V> {
    let f = File::open(path)?;
    let mut bytes = vec![];
    f.take(32 * 1024 * 1024 + 1).read_to_end(&mut bytes)?;
    if bytes.len() > 32 * 1024 * 1024 {
        return Err(Error::business("too_large", "document exceeds limit"));
    }
    crate::protocol::strict_json(&bytes).map_err(|e| Error::business("corrupt", &e.to_string()))
}
pub fn write_json(path: &Path, v: &V) -> Result<()> {
    atomic(
        path,
        &serde_json::to_vec_pretty(v).map_err(|_| Error::invalid("invalid document"))?,
    )
}
pub fn identifier(id: &str) -> Result<&str> {
    if id.is_empty()
        || id.len() > 160
        || id == "."
        || id == ".."
        || !id
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, b'-' | b'_' | b'.'))
    {
        return Err(Error::invalid("invalid document identifier"));
    }
    Ok(id)
}
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
            copy_tree(&e.path(), &to.join(e.file_name()))?;
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
pub fn default_config() -> V {
    json!({"version":2,"appearance":{"theme":"system","density":"compact","fontScale":1.0,"monoFontSize":13.0},"agent":{"backend":"codex","backends":{}},"overlay":{"enabled":false,"extraApps":[]},"launcher":["workbench","proxy","settings"],"terminal":{"extraKeysPinned":false}})
}
pub fn config(v: &V) -> Result<V> {
    if !v.is_object() || v["version"] != 2 {
        return Err(Error::invalid("config.version must be 2"));
    }
    let mut out = default_config();
    merge(&mut out, v);
    for obj in ["appearance", "agent", "overlay", "terminal"] {
        if !out[obj].is_object() {
            return Err(Error::invalid(&format!("{obj}: expected object")));
        }
    }
    for (key, values) in [
        ("theme", vec!["system", "light", "dark"]),
        ("density", vec!["compact", "standard"]),
    ] {
        if !values.contains(&layout::s(&out["appearance"][key])) {
            return Err(Error::invalid(&format!("appearance.{key}: invalid value")));
        }
    }
    for (key, min, max) in [("fontScale", 0.8, 1.3), ("monoFontSize", 10.0, 20.0)] {
        if !out["appearance"][key]
            .as_f64()
            .is_some_and(|n| n >= min && n <= max)
        {
            return Err(Error::invalid(&format!("appearance.{key}: out of range")));
        }
    }
    if layout::s(&out["agent"]["backend"]).trim().is_empty() {
        return Err(Error::invalid("agent.backend: expected nonempty string"));
    }
    if !out["agent"]["backends"].is_object() {
        return Err(Error::invalid("agent.backends: expected object"));
    }
    for defaults in out["agent"]["backends"].as_object().unwrap().values() {
        if !defaults.is_object() {
            return Err(Error::invalid("backend defaults: expected object"));
        }
        for key in ["model", "effort"] {
            if !defaults[key].is_null() && !defaults[key].is_string() {
                return Err(Error::invalid("backend defaults: expected string"));
            }
        }
    }
    if !out["agent"]["permissions"].is_null() && !out["agent"]["permissions"].is_string() {
        return Err(Error::invalid("agent.permissions: expected string"));
    }
    for (a, b) in [("overlay", "enabled"), ("terminal", "extraKeysPinned")] {
        if !out[a][b].is_boolean() {
            return Err(Error::invalid("expected boolean"));
        }
    }
    for (a, b) in [("overlay", "extraApps")] {
        if !out[a][b]
            .as_array()
            .is_some_and(|a| a.iter().all(|v| v.as_str().is_some_and(app_ref)))
        {
            return Err(Error::invalid("overlay.extraApps: invalid application"));
        }
    }
    let Some(launcher) = out["launcher"].as_array() else {
        return Err(Error::invalid("launcher: expected array"));
    };
    let mut entries = vec![];
    for entry in launcher {
        let Some(id) = entry.as_str() else {
            return Err(Error::invalid("launcher: expected string"));
        };
        if !matches!(id, "workbench" | "proxy" | "settings") && !app_ref(id) {
            return Err(Error::invalid("launcher: invalid application"));
        }
        if !entries.contains(entry) {
            entries.push(entry.clone());
        }
    }
    if !entries.contains(&json!("workbench")) {
        entries.insert(0, json!("workbench"));
    }
    for required in ["proxy", "settings"] {
        if !entries.contains(&json!(required)) {
            entries.push(json!(required));
        }
    }
    out["launcher"] = json!(entries);
    Ok(out)
}
fn app_ref(id: &str) -> bool {
    let mut parts = id.split('/');
    let pkg = parts.next().unwrap_or("");
    let activity = parts.next();
    parts.next().is_none()
        && pkg.contains('.')
        && pkg
            .split('.')
            .all(|s| !s.is_empty() && s.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_'))
        && activity.is_none_or(|s| {
            !s.is_empty()
                && s.bytes()
                    .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'.' | b'$'))
        })
}
pub fn merge(to: &mut V, from: &V) {
    if let (Some(a), Some(b)) = (to.as_object_mut(), from.as_object()) {
        for (k, v) in b {
            if let Some(current) = a.get_mut(k) {
                merge(current, v);
            } else {
                a.insert(k.clone(), v.clone());
            }
        }
    } else {
        *to = from.clone();
    }
}
