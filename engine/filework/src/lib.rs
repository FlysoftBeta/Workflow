//! Working Resources and all editable workspace file IO. Private storage is delegated to Store.
mod fs;
pub mod imports;
mod model;
pub use fs::valid_path;
pub use model::*;
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{self as stdfs, File},
    io::{Read, Seek, SeekFrom},
    path::{Path, PathBuf},
};
use workflow_environment::{
    error::{Error, Result},
    persist::{hash, now, publish_new},
    store::{Store, UploadStage},
};

#[derive(Clone)]
pub struct FileWork {
    root: PathBuf,
    store: Store,
}
enum Location {
    User(PathBuf),
    Stored(String),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExistingPathKind {
    File,
    Directory,
}
#[derive(Default)]
pub struct Uploads {
    pending: BTreeMap<String, Upload>,
}
struct Upload {
    destination: UploadDestination,
    size: u64,
    stage: UploadStage,
}
impl FileWork {
    pub fn new(root: PathBuf, store: Store) -> Self {
        Self { root, store }
    }
    pub fn root(&self) -> &Path {
        &self.root
    }
    fn location(&self, raw: &str) -> Result<Location> {
        if !valid_path(raw) {
            return Err(Error::invalid("not an editable workspace path"));
        }
        if let Some(key) = raw.strip_prefix(".workspace/") {
            self.store.metadata(key)?;
            Ok(Location::Stored(key.into()))
        } else {
            Ok(Location::User(fs::user_path(&self.root, raw, false)?))
        }
    }
    pub fn validate_path(&self, raw: &str) -> Result<()> {
        self.location(raw).map(|_| ())
    }
    pub fn user_directory(&self, raw: &str) -> Result<PathBuf> {
        let path = fs::user_path(&self.root, raw, true)?;
        if !path.is_dir() {
            return Err(Error::invalid("directory does not exist"));
        }
        Ok(path)
    }
    /// Checked path classification for terminal links; avoids reading or hashing file contents.
    pub fn existing_path_kind(&self, raw: &str) -> Result<Option<ExistingPathKind>> {
        if raw.is_empty() {
            return Ok(Some(ExistingPathKind::Directory));
        }
        let (file, directory) = match self.location(raw)? {
            Location::User(path) => match stdfs::metadata(path) {
                Ok(meta) => (meta.is_file(), meta.is_dir()),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
                Err(e) => return Err(e.into()),
            },
            Location::Stored(key) => match self.store.metadata(&key)? {
                Some(meta) => (meta.is_file, meta.is_dir),
                None => return Ok(None),
            },
        };
        Ok(if file {
            Some(ExistingPathKind::File)
        } else if directory {
            Some(ExistingPathKind::Directory)
        } else {
            None
        })
    }
    pub fn version(&self, raw: &str) -> Result<FileVersion> {
        match self.location(raw)? {
            Location::User(p) => fs::version(&p),
            Location::Stored(key) => {
                let Some(m) = self.store.metadata(&key)? else {
                    return Ok(FileVersion::default());
                };
                if !m.is_file {
                    return Ok(FileVersion::default());
                }
                let sha256 = if m.len <= 64 * 1024 * 1024 {
                    self.store
                        .read_bytes(&key, 64 * 1024 * 1024)?
                        .map(|b| hash(&b))
                } else {
                    None
                };
                Ok(FileVersion {
                    exists: true,
                    size: m.len,
                    modified_at: m.modified_at,
                    sha256,
                    extra: Default::default(),
                })
            }
        }
    }
    pub fn observe(&self, state: &mut FileWorkState, raw: &str) -> Result<FileVersion> {
        let version = self.version(raw)?;
        state.disk.insert(raw.into(), version.clone());
        Ok(version)
    }
    fn read_bytes(&self, raw: &str, limit: usize) -> Result<Vec<u8>> {
        match self.location(raw)? {
            Location::User(p) => {
                let mut out = vec![];
                File::open(p)?
                    .take(limit as u64 + 1)
                    .read_to_end(&mut out)?;
                if out.len() > limit {
                    return Err(Error::business("too_large", "file exceeds read size limit"));
                }
                Ok(out)
            }
            Location::Stored(key) => self
                .store
                .read_bytes(&key, limit)?
                .ok_or_else(|| Error::business("io", "file does not exist")),
        }
    }
    pub fn write_text(&self, raw: &str, text: &str) -> Result<()> {
        match self.location(raw)? {
            Location::User(p) => fs::atomic(&p, text.as_bytes()),
            Location::Stored(key) => self.store.write_text(&key, text),
        }
    }
    pub fn open(&self, state: &mut FileWorkState, raw: &str) -> Result<OpenFile> {
        let disk = self.observe(state, raw)?;
        let too_large = disk.size > 16 * 1024 * 1024;
        let bytes = if disk.exists && !too_large {
            Some(self.read_bytes(raw, 16 * 1024 * 1024)?)
        } else {
            None
        };
        let text = bytes
            .as_ref()
            .and_then(|b| {
                if b.contains(&0) {
                    None
                } else {
                    std::str::from_utf8(b).ok()
                }
            })
            .map(str::to_owned);
        Ok(OpenFile {
            path: raw.into(),
            disk,
            disk_text: text.clone(),
            binary: bytes.is_some() && text.is_none(),
            too_large,
            draft: state.drafts.get(raw).cloned(),
        })
    }
    pub fn diff(&self, state: &mut FileWorkState, raw: &str) -> Result<Diff> {
        let open = self.open(state, raw)?;
        let conflicted = open
            .draft
            .as_ref()
            .is_some_and(|d| !open.disk.same_content(&d.base) && !open.disk.matches_text(&d.text));
        Ok(Diff {
            path: open.path,
            disk: open.disk,
            base: open.draft.as_ref().map(|d| d.base.clone()),
            disk_text: open.disk_text,
            draft_text: open.draft.map(|d| d.text),
            conflicted,
            binary: open.binary,
            too_large: open.too_large,
        })
    }
    pub fn edit(
        &self,
        state: &mut FileWorkState,
        path: &str,
        text: &str,
        shown: &FileVersion,
    ) -> Result<()> {
        self.validate_path(path)?;
        if text.len() > 16 * 1024 * 1024 {
            return Err(Error::invalid("editable text exceeds 16 MiB"));
        }
        let current = state.drafts.get(path);
        if current.is_some_and(|d| d.text == text) {
            return Ok(());
        }
        let base = current
            .map(|d| d.base.clone())
            .unwrap_or_else(|| shown.clone());
        if base.matches_text(text) {
            state.drafts.remove(path);
        } else {
            let revision = current
                .map_or(0, |d| d.revision)
                .checked_add(1)
                .ok_or_else(|| Error::business("overflow", "draft revision overflow"))?;
            let extra = current.map(|d| d.extra.clone()).unwrap_or_default();
            state.drafts.insert(
                path.into(),
                Draft {
                    format: 1,
                    path: path.into(),
                    text: text.into(),
                    base,
                    edited_at: now(),
                    revision,
                    extra,
                },
            );
        }
        if !state.disk.contains_key(path) {
            self.observe(state, path)?;
        }
        Ok(())
    }
    pub fn save<F: Fn(&str, &str) -> Result<()>>(
        &self,
        state: &mut FileWorkState,
        path: &str,
        text: Option<&str>,
        validate: F,
    ) -> Result<SaveOutcome> {
        if let Some(text) = text {
            let base = if let Some(draft) = state.drafts.get(path) {
                draft.base.clone()
            } else if let Some(disk) = state.disk.get(path) {
                disk.clone()
            } else {
                self.observe(state, path)?
            };
            self.edit(state, path, text, &base)?
        }
        let disk = self.observe(state, path)?;
        let Some(draft) = state.drafts.get(path).cloned() else {
            return Ok(SaveOutcome::Unchanged { version: disk });
        };
        if !disk.same_content(&draft.base) && !disk.matches_text(&draft.text) {
            return Ok(SaveOutcome::Conflict { disk });
        }
        if let Err(e) = validate(path, &draft.text) {
            return Ok(SaveOutcome::Invalid { message: e.message });
        }
        if let Err(e) = self.write_text(path, &draft.text) {
            return Ok(SaveOutcome::Failed { message: e.message });
        }
        let version = self.observe(state, path)?;
        state.drafts.remove(path);
        Ok(SaveOutcome::Saved {
            version,
            text: draft.text,
        })
    }
    pub fn resolve_conflict(
        &self,
        state: &mut FileWorkState,
        path: &str,
        resolution: ConflictResolution,
    ) -> Result<()> {
        let version = self.observe(state, path)?;
        match resolution {
            ConflictResolution::UseDisk => {
                state.drafts.remove(path);
            }
            ConflictResolution::KeepMine => {
                if let Some(d) = state.drafts.get_mut(path) {
                    d.revision = d
                        .revision
                        .checked_add(1)
                        .ok_or_else(|| Error::business("overflow", "draft revision overflow"))?;
                    d.base = version;
                    d.edited_at = now()
                }
            }
        }
        Ok(())
    }
    pub fn discard(&self, state: &mut FileWorkState, path: &str) -> Result<()> {
        self.validate_path(path)?;
        state.drafts.remove(path);
        Ok(())
    }
    pub fn edit_composer(
        &self,
        state: &mut FileWorkState,
        mut draft: ComposerDraft,
        expected_revision: u64,
    ) -> Result<ComposerDraft> {
        if draft.conversation_id.trim().is_empty() {
            return Err(Error::invalid("empty conversation id"));
        }
        for a in &draft.attachments {
            self.validate_path(&a.path)?
        }
        draft.format = 1;
        let current = state.composer(&draft.conversation_id);
        if draft == current {
            return Ok(current);
        }
        if expected_revision != current.revision || draft.revision <= current.revision {
            return Err(Error::business("conflict", "composer revision changed"));
        }
        state
            .composers
            .insert(draft.conversation_id.clone(), draft.clone());
        Ok(draft)
    }
    pub fn refresh(
        &self,
        state: &mut FileWorkState,
        live_paths: impl IntoIterator<Item = String>,
    ) -> Result<()> {
        let mut paths: BTreeSet<_> = state.drafts.keys().cloned().collect();
        paths.extend(live_paths);
        paths.insert(".workspace/config.json".into());
        for p in paths {
            let same_metadata = match self.location(&p) {
                Ok(Location::User(path)) => stdfs::metadata(path).ok().is_some_and(|m| {
                    m.is_file()
                        && state.disk.get(&p).is_some_and(|v| {
                            v.exists && v.size == m.len() && v.modified_at == fs::modified_at(&m)
                        })
                }),
                Ok(Location::Stored(key)) => {
                    self.store.metadata(&key).ok().flatten().is_some_and(|m| {
                        m.is_file
                            && state.disk.get(&p).is_some_and(|v| {
                                v.exists && v.size == m.len && v.modified_at == m.modified_at
                            })
                    })
                }
                Err(_) => false,
            };
            if same_metadata {
                continue;
            }
            let Ok(disk) = self.observe(state, &p) else {
                continue;
            };
            if state
                .drafts
                .get(&p)
                .is_some_and(|d| disk.matches_text(&d.text))
            {
                state.drafts.remove(&p);
            }
        }
        Ok(())
    }
    pub fn archive<F: Fn(&str, &str) -> Result<()>>(
        &self,
        state: &mut FileWorkState,
        resources: &[ResourceRef],
        decision: Option<ArchiveDecision>,
        validate: F,
    ) -> Result<ArchiveOutcome> {
        let dirty: Vec<_> = state
            .dirty()
            .into_iter()
            .filter(|r| resources.contains(r))
            .collect();
        if decision.is_none() && !dirty.is_empty() {
            return Ok(ArchiveOutcome::NeedsDecision { resources: dirty });
        }
        let mut files: Vec<_> = dirty
            .iter()
            .filter_map(|r| match r {
                ResourceRef::File { path } => Some(path.clone()),
                _ => None,
            })
            .collect();
        files.sort();
        let mut saved_paths = vec![];
        match decision {
            Some(ArchiveDecision::SaveAll) => {
                let mut conflicts = vec![];
                for p in &files {
                    let d = state.drafts.get(p).unwrap().clone();
                    let disk = self.observe(state, p)?;
                    if !disk.same_content(&d.base) && !disk.matches_text(&d.text) {
                        conflicts.push(p.clone())
                    }
                }
                if !conflicts.is_empty() {
                    return Ok(ArchiveOutcome::SaveConflict { paths: conflicts });
                }
                for p in &files {
                    if let Err(e) = validate(p, &state.drafts.get(p).unwrap().text) {
                        return Ok(ArchiveOutcome::Invalid {
                            path: p.clone(),
                            message: e.message,
                        });
                    }
                }
                for p in &files {
                    match self.save(state, p, None, &validate)? {
                        SaveOutcome::Saved { .. } | SaveOutcome::Unchanged { .. } => {
                            saved_paths.push(p.clone())
                        }
                        SaveOutcome::Conflict { .. } => {
                            return Ok(ArchiveOutcome::SaveConflict {
                                paths: vec![p.clone()],
                            });
                        }
                        SaveOutcome::Invalid { message } => {
                            return Ok(ArchiveOutcome::Invalid {
                                path: p.clone(),
                                message,
                            });
                        }
                        SaveOutcome::Failed { message } => {
                            return Ok(ArchiveOutcome::Failed { message });
                        }
                    }
                }
            }
            Some(ArchiveDecision::Discard) => {
                for p in &files {
                    state.drafts.remove(p);
                }
                for r in &dirty {
                    if let ResourceRef::Conversation { id } = r {
                        state.clear_composer(id)?;
                    }
                }
            }
            _ => {}
        }
        Ok(ArchiveOutcome::Archived { saved_paths })
    }
    pub fn list_directory(&self, raw: &str, show_hidden: bool) -> Result<Vec<DirectoryEntry>> {
        let mut entries = vec![];
        if raw.starts_with(".workspace/") {
            let Location::Stored(key) = self.location(raw)? else {
                unreachable!()
            };
            for (key, m) in self.store.list_entries(&key)? {
                let name = key.rsplit('/').next().unwrap();
                if name == ".workspace" || (!show_hidden && name.starts_with('.')) {
                    continue;
                }
                entries.push(DirectoryEntry {
                    path: format!("{raw}/{name}"),
                    is_directory: m.is_dir,
                    size: m.len,
                    modified_at: m.modified_at,
                });
            }
        } else {
            let path = fs::user_path(&self.root, raw, true)?;
            for e in stdfs::read_dir(path)? {
                let e = e?;
                let name = e.file_name().to_string_lossy().into_owned();
                if name == ".workspace"
                    || (!show_hidden && name.starts_with('.'))
                    || e.file_type()?.is_symlink()
                {
                    continue;
                }
                let m = e.metadata()?;
                entries.push(DirectoryEntry {
                    path: if raw.is_empty() {
                        name
                    } else {
                        format!("{raw}/{name}")
                    },
                    is_directory: m.is_dir(),
                    size: m.len(),
                    modified_at: fs::modified_at(&m),
                });
            }
        }
        entries.sort_by(|a, b| {
            b.is_directory
                .cmp(&a.is_directory)
                .then(a.path.to_lowercase().cmp(&b.path.to_lowercase()))
        });
        Ok(entries)
    }
    pub fn file_operation(
        &self,
        state: &mut FileWorkState,
        operation: &FileOperation,
    ) -> FileOutcome {
        self.perform_file_operation(state, operation)
            .unwrap_or_else(|e| FileOutcome::Failed { message: e.message })
    }
    fn perform_file_operation(
        &self,
        state: &mut FileWorkState,
        operation: &FileOperation,
    ) -> Result<FileOutcome> {
        match operation {
            FileOperation::CreateFile { path, data } => {
                if data.len() > 65536 {
                    return Err(Error::invalid("blob exceeds 65536 bytes"));
                }
                match self.location(path)? {
                    Location::User(p) => fs::create_atomic(&p, data)?,
                    Location::Stored(key) => {
                        let mut stage = self.store.begin_upload()?;
                        stage.append(data)?;
                        stage.sync()?;
                        self.store.publish_upload(&mut stage, &key)?;
                    }
                }
            }
            FileOperation::CreateDirectory { path } => match self.location(path)? {
                Location::User(path) => stdfs::create_dir(path)?,
                Location::Stored(key) => self.store.create_directory(&key)?,
            },
            FileOperation::Delete { path } | FileOperation::Trash { path } => {
                if path.starts_with(".workspace/") {
                    return Err(Error::invalid("configuration cannot be deleted"));
                }
                let p = fs::user_path(&self.root, path, false)?;
                if matches!(operation, FileOperation::Trash { .. }) {
                    let id = uuid::Uuid::new_v4().to_string();
                    let entry = TrashEntry {
                        id: id.clone(),
                        path: path.clone(),
                        trashed_at: now(),
                        is_directory: p.is_dir(),
                        extra: Default::default(),
                    };
                    self.store
                        .write(&format!("trash/{id}/entry.json"), &entry)?;
                    self.store.move_in(&p, &format!("trash/{id}/content"))?;
                    return Ok(FileOutcome::Trashed { entry });
                }
                if p.is_dir() {
                    stdfs::remove_dir_all(p)?
                } else {
                    stdfs::remove_file(p)?
                }
            }
            FileOperation::Move { from, to } | FileOperation::Copy { from, to } => {
                if from.starts_with(".workspace/") || to.starts_with(".workspace/") {
                    return Err(Error::invalid("configuration cannot be moved or copied"));
                }
                let src = fs::user_path(&self.root, from, false)?;
                let dst = fs::user_path(&self.root, to, false)?;
                if dst.exists() {
                    return Err(Error::business("exists", "destination exists"));
                }
                if fs::within(to, from) {
                    return Err(Error::invalid("cannot move or copy into itself"));
                }
                if matches!(operation, FileOperation::Copy { .. }) {
                    fs::copy_tree(&src, &dst)?
                } else {
                    for path in state.drafts.keys() {
                        if fs::within(path, to) && !fs::within(path, from) {
                            return Err(Error::business(
                                "draft_conflict",
                                "destination has an unsaved draft",
                            ));
                        }
                        if fs::within(path, from) {
                            let target = fs::rebase(path, from, to);
                            if state.drafts.contains_key(&target) && !fs::within(&target, from) {
                                return Err(Error::business(
                                    "draft_conflict",
                                    "destination has an unsaved draft",
                                ));
                            }
                        }
                    }
                    publish_new(&src, &dst)?;
                    state.drafts = std::mem::take(&mut state.drafts)
                        .into_iter()
                        .map(|(p, mut d)| {
                            let new = fs::rebase(&p, from, to);
                            d.path = new.clone();
                            (new, d)
                        })
                        .collect();
                    state.disk = std::mem::take(&mut state.disk)
                        .into_iter()
                        .map(|(p, v)| (fs::rebase(&p, from, to), v))
                        .collect();
                    for d in state.composers.values_mut() {
                        for a in &mut d.attachments {
                            a.path = fs::rebase(&a.path, from, to)
                        }
                    }
                }
            }
            FileOperation::Restore { id } => {
                let id = fs::identifier(id)?;
                let entry: TrashEntry = self
                    .store
                    .read(&format!("trash/{id}/entry.json"))?
                    .ok_or_else(|| Error::business("not_found", "trash entry not found"))?;
                let original = &entry.path;
                let mut raw = original.clone();
                let mut path = fs::user_path(&self.root, &raw, false)?;
                let p = Path::new(original);
                let name = p.file_stem().and_then(|p| p.to_str()).unwrap_or("restored");
                let ext = p
                    .extension()
                    .and_then(|p| p.to_str())
                    .map(|e| format!(".{e}"))
                    .unwrap_or_default();
                let parent = p.parent().and_then(|p| p.to_str()).unwrap_or("");
                let mut n = 1;
                while path.exists() {
                    let filename = format!("{name} ({n}){ext}");
                    raw = if parent.is_empty() {
                        filename
                    } else {
                        format!("{parent}/{filename}")
                    };
                    path = fs::user_path(&self.root, &raw, false)?;
                    n += 1
                }
                stdfs::create_dir_all(path.parent().unwrap())?;
                self.store.move_out(&format!("trash/{id}/content"), &path)?;
                self.store.remove_tree(&format!("trash/{id}"))?;
                return Ok(FileOutcome::Restored { path: raw });
            }
        }
        Ok(FileOutcome::Done)
    }
    pub fn purge_trash(&self) -> Result<usize> {
        let mut count = 0;
        for key in self.store.list("trash")? {
            if let Ok(Some(entry)) = self.store.read::<TrashEntry>(&format!("{key}/entry.json")) {
                if now().saturating_sub(entry.trashed_at) >= 7 * 86_400_000 {
                    self.store.remove_tree(&key)?;
                    count += 1
                }
            }
        }
        Ok(count)
    }
    pub fn read_chunk(&self, path: &str, offset: u64, length: usize) -> Result<FileChunk> {
        if length > 65536 {
            return Err(Error::invalid("length exceeds 65536"));
        }
        let (data, size) = match self.location(path)? {
            Location::User(p) => {
                let mut f = File::open(p)?;
                let size = f.metadata()?.len();
                if offset > size {
                    return Err(Error::invalid("offset exceeds file size"));
                }
                f.seek(SeekFrom::Start(offset))?;
                let mut data = vec![];
                f.take(length as u64).read_to_end(&mut data)?;
                (data, size)
            }
            Location::Stored(key) => self.store.read_range(&key, offset, length)?,
        };
        let next_offset = offset + data.len() as u64;
        Ok(FileChunk {
            data,
            next_offset,
            eof: next_offset == size,
            size,
        })
    }
    pub fn begin_upload(
        &self,
        uploads: &mut Uploads,
        destination: UploadDestination,
        size: u64,
    ) -> Result<UploadStarted> {
        match &destination {
            UploadDestination::Allocate { directory, name } => {
                imports::validate(&self.root, directory, name)?
            }
            UploadDestination::Exact { path } => {
                if path.starts_with(".workspace/")
                    && !path.starts_with(".workspace/services/")
                    && !path.starts_with(".workspace/proxy/")
                {
                    return Err(Error::invalid(
                        "upload is only for user files and explicit service assets",
                    ));
                }
                let exists = match self.location(path)? {
                    Location::User(p) => p.exists(),
                    Location::Stored(key) => self.store.exists(&key)?,
                };
                if exists {
                    return Err(Error::business("exists", "destination already exists"));
                }
            }
        }
        if size > 8 * 1024 * 1024 * 1024 {
            return Err(Error::invalid("upload exceeds 8 GiB"));
        }
        if uploads.pending.len() >= 16 {
            return Err(Error::business("limit", "too many uploads"));
        }
        let upload_id = uuid::Uuid::new_v4().to_string();
        uploads.pending.insert(
            upload_id.clone(),
            Upload {
                destination,
                size,
                stage: self.store.begin_upload()?,
            },
        );
        Ok(UploadStarted { upload_id })
    }
    pub fn upload_chunk(
        &self,
        uploads: &mut Uploads,
        id: &str,
        offset: u64,
        data: &[u8],
    ) -> Result<UploadProgress> {
        let upload = uploads
            .pending
            .get_mut(id)
            .ok_or_else(|| Error::business("not_found", "upload not found"))?;
        if offset != upload.stage.len() {
            return Err(Error::business(
                "invalid_offset",
                "upload offset is not nextOffset",
            ));
        }
        if data.len() > 65536 {
            return Err(Error::invalid("blob exceeds 65536 bytes"));
        }
        if offset + data.len() as u64 > upload.size {
            return Err(Error::invalid("upload exceeds declared size"));
        }
        upload.stage.append(data)?;
        Ok(UploadProgress {
            next_offset: upload.stage.len(),
        })
    }
    pub fn commit_upload(&self, uploads: &mut Uploads, id: &str) -> Result<UploadOutcome> {
        let upload = uploads
            .pending
            .get_mut(id)
            .ok_or_else(|| Error::business("not_found", "upload not found"))?;
        if upload.stage.len() != upload.size {
            return Err(Error::business(
                "incomplete",
                "upload has not reached declared size",
            ));
        }
        upload.stage.sync()?;
        let path = match &upload.destination {
            UploadDestination::Allocate { directory, name } => {
                imports::publish(&self.root, &mut upload.stage, directory, name)?
            }
            UploadDestination::Exact { path } => {
                let result = match self.location(path)? {
                    Location::User(p) => {
                        stdfs::create_dir_all(p.parent().unwrap())?;
                        upload.stage.publish_new(&p).and_then(|()| {
                            File::open(p.parent().unwrap())?.sync_all()?;
                            Ok(())
                        })
                    }
                    Location::Stored(key) => self.store.publish_upload(&mut upload.stage, &key),
                };
                if let Err(e) = result {
                    return Ok(UploadOutcome::Failed { message: e.message });
                }
                path.clone()
            }
        };
        uploads.pending.remove(id);
        Ok(UploadOutcome::Done { path })
    }
    pub fn cancel_upload(&self, uploads: &mut Uploads, id: &str) {
        uploads.pending.remove(id);
    }
}
#[cfg(test)]
mod tests;
