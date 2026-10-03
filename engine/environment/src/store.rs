//! All private workspace paths and file operations are constructed here.
use crate::{Error, Result, persist};
use serde::{Serialize, de::DeserializeOwned};
use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    net::IpAddr,
    os::{fd::AsRawFd, unix::fs::PermissionsExt},
    path::{Path, PathBuf},
    time::UNIX_EPOCH,
};

#[derive(Clone, Debug)]
pub struct Store {
    directory: PathBuf,
}
#[derive(Clone, Debug)]
pub struct StoredMetadata {
    pub is_file: bool,
    pub is_dir: bool,
    pub len: u64,
    pub modified_at: u64,
}
/// Keeps the single Engine owner lease for this workspace until dropped.
pub struct StoreLock(File);
impl Drop for StoreLock {
    fn drop(&mut self) {
        unsafe {
            libc::flock(self.0.as_raw_fd(), libc::LOCK_UN);
        }
    }
}
impl Store {
    pub fn open(root: &Path) -> Result<Self> {
        let directory = root.join(".workspace");
        if fs::symlink_metadata(&directory).is_ok_and(|m| m.file_type().is_symlink()) {
            return Err(Error::business(
                "unsafe_path",
                "workspace state directory is a symbolic link",
            ));
        }
        fs::create_dir_all(&directory)?;
        Ok(Self { directory })
    }
    pub(crate) fn path(&self, key: &str) -> Result<PathBuf> {
        if key.is_empty()
            || key.contains('\\')
            || key.contains('\0')
            || key
                .split('/')
                .any(|v| v.is_empty() || v == "." || v == "..")
        {
            return Err(Error::invalid("invalid store-relative key"));
        }
        let mut path = self.directory.clone();
        for segment in key.split('/') {
            path.push(segment);
            match fs::symlink_metadata(&path) {
                Ok(m) if m.file_type().is_symlink() => {
                    return Err(Error::business(
                        "unsafe_path",
                        "symbolic links are not followed",
                    ));
                }
                Ok(_) => (),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => (),
                Err(e) => return Err(e.into()),
            }
        }
        Ok(path)
    }
    pub fn read<T: DeserializeOwned>(&self, key: &str) -> Result<Option<T>> {
        let Some(bytes) = self.read_bytes(key, 32 * 1024 * 1024)? else {
            return Ok(None);
        };
        crate::json::strict_json(&bytes)
            .map(Some)
            .map_err(|e| Error::business("corrupt", &e.to_string()))
    }
    pub fn write<T: Serialize + ?Sized>(&self, key: &str, value: &T) -> Result<()> {
        persist::write_json(&self.path(key)?, value)
    }
    pub fn read_bytes(&self, key: &str, max: usize) -> Result<Option<Vec<u8>>> {
        let file = match File::open(self.path(key)?) {
            Ok(f) => f,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(e.into()),
        };
        let mut bytes = Vec::new();
        file.take(max as u64 + 1).read_to_end(&mut bytes)?;
        if bytes.len() > max {
            return Err(Error::business("too_large", "document exceeds limit"));
        }
        Ok(Some(bytes))
    }
    pub fn read_text(&self, key: &str) -> Result<Option<String>> {
        self.read_bytes(key, 16 * 1024 * 1024)?
            .map(|b| {
                String::from_utf8(b)
                    .map_err(|_| Error::business("invalid_text", "document is not UTF-8"))
            })
            .transpose()
    }
    pub fn write_text(&self, key: &str, text: &str) -> Result<()> {
        if text.len() > 16 * 1024 * 1024 {
            return Err(Error::business("too_large", "document exceeds limit"));
        }
        self.write_bytes(key, text.as_bytes())
    }
    pub fn write_bytes(&self, key: &str, bytes: &[u8]) -> Result<()> {
        persist::atomic(&self.path(key)?, bytes)
    }
    pub fn create_bytes(&self, key: &str, bytes: &[u8]) -> Result<()> {
        persist::create_atomic(&self.path(key)?, bytes)
    }
    pub fn metadata(&self, key: &str) -> Result<Option<StoredMetadata>> {
        match fs::metadata(self.path(key)?) {
            Ok(m) => Ok(Some(StoredMetadata {
                is_file: m.is_file(),
                is_dir: m.is_dir(),
                len: m.len(),
                modified_at: m
                    .modified()
                    .unwrap_or(UNIX_EPOCH)
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis() as u64,
            })),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e.into()),
        }
    }
    pub fn exists(&self, key: &str) -> Result<bool> {
        Ok(self.metadata(key)?.is_some())
    }
    pub fn remove(&self, key: &str) -> Result<()> {
        match fs::remove_file(self.path(key)?) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(e.into()),
        }
    }
    pub fn remove_tree(&self, key: &str) -> Result<()> {
        let path = self.path(key)?;
        match fs::symlink_metadata(&path) {
            Ok(m) if m.is_dir() => Ok(fs::remove_dir_all(path)?),
            Ok(_) => self.remove(key),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(e.into()),
        }
    }
    pub fn list(&self, prefix: &str) -> Result<Vec<String>> {
        let entries = match fs::read_dir(self.path(prefix)?) {
            Ok(v) => v,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(vec![]),
            Err(e) => return Err(e.into()),
        };
        let mut keys = vec![];
        for entry in entries {
            let name = entry?
                .file_name()
                .into_string()
                .map_err(|_| Error::business("invalid_text", "state filename is not UTF-8"))?;
            let key = format!("{prefix}/{name}");
            self.path(&key)?;
            keys.push(key);
        }
        keys.sort();
        Ok(keys)
    }
    pub fn backup(&self, key: &str) -> Result<()> {
        if let Some(bytes) = self.read_bytes(key, 32 * 1024 * 1024)? {
            self.write_bytes(&format!("{key}.bak"), &bytes)?;
        }
        Ok(())
    }
    pub fn quarantine(&self, key: &str) -> Result<Option<String>> {
        if !self.exists(key)? {
            return Ok(None);
        }
        let target = format!(
            "corrupt/{}-{}",
            uuid::Uuid::new_v4(),
            key.rsplit('/').next().unwrap()
        );
        let path = self.path(&target)?;
        fs::create_dir_all(path.parent().unwrap())?;
        fs::rename(self.path(key)?, path)?;
        Ok(Some(target))
    }
    pub fn move_in(&self, source: &Path, key: &str) -> Result<()> {
        let destination = self.path(key)?;
        fs::create_dir_all(destination.parent().unwrap())?;
        persist::publish_new(source, &destination)?;
        Ok(())
    }
    pub fn move_out(&self, key: &str, destination: &Path) -> Result<()> {
        persist::publish_new(&self.path(key)?, destination)?;
        Ok(())
    }
    pub fn lock(&self) -> Result<StoreLock> {
        let file = OpenOptions::new()
            .create(true)
            .truncate(false)
            .write(true)
            .open(self.path("engine.lock")?)?;
        if unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } != 0 {
            return Err(Error::business(
                "busy",
                "another engine owns this workspace",
            ));
        }
        Ok(StoreLock(file))
    }
    pub fn write_resolver(&self, servers: &[IpAddr]) -> Result<()> {
        if servers.len() > 16 {
            return Err(Error::invalid("too many DNS servers"));
        }
        let text: String = servers
            .iter()
            .map(|ip| format!("nameserver {ip}\n"))
            .collect();
        let path = self.path("environment/network/resolv.conf")?;
        persist::atomic(&path, text.as_bytes())?;
        fs::set_permissions(path, fs::Permissions::from_mode(0o644))?;
        Ok(())
    }
    pub fn begin_upload(&self) -> Result<UploadStage> {
        let path = self.path(&format!("uploads/{}", uuid::Uuid::new_v4()))?;
        fs::create_dir_all(path.parent().unwrap())?;
        let file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&path)?;
        Ok(UploadStage {
            path,
            file,
            len: 0,
            published: false,
        })
    }
}
pub struct UploadStage {
    path: PathBuf,
    file: File,
    len: u64,
    published: bool,
}
impl UploadStage {
    pub fn append(&mut self, bytes: &[u8]) -> Result<()> {
        self.file.write_all(bytes)?;
        self.len += bytes.len() as u64;
        Ok(())
    }
    pub fn sync(&self) -> Result<()> {
        self.file.sync_all()?;
        Ok(())
    }
    pub fn path(&self) -> &Path {
        &self.path
    }
    pub fn len(&self) -> u64 {
        self.len
    }
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
    pub fn publish_new(&mut self, destination: &Path) -> Result<()> {
        self.sync()?;
        persist::publish_new(&self.path, destination)?;
        self.published = true;
        if let Some(parent) = destination.parent() {
            File::open(parent)?.sync_all()?;
        }
        Ok(())
    }
}
impl Drop for UploadStage {
    fn drop(&mut self) {
        if !self.published {
            let _ = fs::remove_file(&self.path);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};
    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    struct Document {
        revision: u64,
    }
    #[test]
    fn typed_recovery_preserves_original_and_last_backup() {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::open(dir.path()).unwrap();
        store
            .write("state/test.json", &Document { revision: 7 })
            .unwrap();
        store.backup("state/test.json").unwrap();
        store
            .write_bytes("state/test.json", br#"{"revision":1,"revision":2}"#)
            .unwrap();
        assert_eq!(
            store.read::<Document>("state/test.json").unwrap_err().kind,
            "corrupt"
        );
        let damaged = store.quarantine("state/test.json").unwrap().unwrap();
        assert!(
            store
                .read_text(&damaged)
                .unwrap()
                .unwrap()
                .contains("revision")
        );
        assert_eq!(
            store.read::<Document>("state/test.json.bak").unwrap(),
            Some(Document { revision: 7 })
        );
        assert_eq!(store.read::<Document>("state/test.json").unwrap(), None);
    }
    #[test]
    fn validates_keys_and_rejects_symlink_components() {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::open(dir.path()).unwrap();
        for key in [
            "",
            "/tmp/x",
            "../x",
            "state//x",
            "state/./x",
            "state/../x",
            "a\\b",
        ] {
            assert!(store.write_text(key, "bad").is_err(), "{key}");
        }
        std::os::unix::fs::symlink(dir.path(), store.directory.join("link")).unwrap();
        assert!(store.write_text("link/x", "bad").is_err());
    }
    #[test]
    fn upload_no_clobber_cleanup_and_owner_lock() {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::open(dir.path()).unwrap();
        let lease = store.lock().unwrap();
        assert!(store.lock().is_err());
        drop(lease);
        assert!(store.lock().is_ok());
        let mut stage = store.begin_upload().unwrap();
        stage.append(b"new").unwrap();
        let path = stage.path().to_owned();
        let target = dir.path().join("file");
        fs::write(&target, "old").unwrap();
        assert!(stage.publish_new(&target).is_err());
        assert_eq!(fs::read_to_string(&target).unwrap(), "old");
        drop(stage);
        assert!(!path.exists());
    }
}

/// The private layout is centralized here; callers pass domain identifiers.
pub mod keys {
    use crate::{Result, persist::identifier};
    pub const WORKSPACE_STATE: &str = "state/workspace.json";
    pub const TERMINAL_STATE: &str = "state/terminals.json";
    pub const CONFIG: &str = "config.json";
    pub const DECLARATION: &str = "env.json";
    pub const SERVICE_REPORTS: &str = "state/services";
    pub fn document(namespace: &str, key: &str) -> Result<String> {
        Ok(format!(
            "documents/{}/{}.json",
            identifier(namespace)?,
            identifier(key)?
        ))
    }
    pub fn service(service: &str, key: &str) -> Result<String> {
        let service = identifier(service)?;
        let key = identifier(key)?;
        Ok(if service == "proxy" { format!("proxy/{key}") } else { format!("services/{service}/{key}") })
    }
    pub fn service_sidecar(service: &str, key: &str) -> Result<String> {
        document(&format!("services.{}", identifier(service)?), key)
    }
    pub fn service_report(id: &str) -> Result<String> {
        Ok(format!("{SERVICE_REPORTS}/{}.json", identifier(id)?))
    }
    pub fn service_control(id: &str) -> Result<String> {
        Ok(format!("state/local-services/{}.json", identifier(id)?))
    }
}
impl Store {
    pub fn read_range(&self, key: &str, offset: u64, length: usize) -> Result<(Vec<u8>, u64)> {
        use std::io::{Seek, SeekFrom};
        let mut file = File::open(self.path(key)?)?;
        let size = file.metadata()?.len();
        if offset > size {
            return Err(Error::invalid("offset exceeds file size"));
        }
        file.seek(SeekFrom::Start(offset))?;
        let mut bytes = Vec::new();
        file.take(length as u64).read_to_end(&mut bytes)?;
        Ok((bytes, size))
    }
    pub fn publish_upload(&self, stage: &mut UploadStage, key: &str) -> Result<()> {
        let path = self.path(key)?;
        fs::create_dir_all(path.parent().unwrap())?;
        stage.publish_new(&path)
    }
}

impl Store {
    /// Explicit editable service directories use create_dir semantics.
    pub fn create_directory(&self, key: &str) -> Result<()> {
        fs::create_dir(self.path(key)?)?;
        Ok(())
    }
    /// Explorer listings skip symbolic-link children, matching ordinary user files. An empty
    /// prefix lists `.workspace` itself; callers filter the result through [`crate::access`].
    pub fn list_entries(&self, prefix: &str) -> Result<Vec<(String, StoredMetadata)>> {
        let mut entries = Vec::new();
        let directory = if prefix.is_empty() {
            self.directory.clone()
        } else {
            self.path(prefix)?
        };
        for entry in fs::read_dir(directory)? {
            let entry = entry?;
            if entry.file_type()?.is_symlink() {
                continue;
            }
            let Ok(name) = entry.file_name().into_string() else {
                // Non-UTF-8 names cannot be classified and are never visible configuration.
                continue;
            };
            let key = if prefix.is_empty() {
                name
            } else {
                format!("{prefix}/{name}")
            };
            if let Some(metadata) = self.metadata(&key)? {
                entries.push((key, metadata));
            }
        }
        entries.sort_by(|a, b| a.0.cmp(&b.0));
        Ok(entries)
    }
}
