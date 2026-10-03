use crate::{
    layout::{self, s},
    protocol::{Error, Result},
    storage::{self, now},
};
use serde_json::{Value as V, json};
use std::{
    collections::{BTreeSet, HashSet},
    fs,
    path::{Path, PathBuf},
};
#[derive(Clone)]
pub struct Workspace {
    pub root: PathBuf,
    pub revision: u64,
    pub state: V,
    pub writable: bool,
}
impl Workspace {
    pub fn load(root: PathBuf) -> Result<Self> {
        fs::create_dir_all(root.join(".workspace/state"))?;
        for dir in [
            "documents",
            "uploads",
            "corrupt",
            "trash",
            "services",
            "state/services",
        ] {
            fs::create_dir_all(root.join(".workspace").join(dir))?;
        }
        for e in fs::read_dir(root.join(".workspace/uploads"))? {
            let e = e?;
            if e.file_type()?.is_file() {
                let _ = fs::remove_file(e.path());
            }
        }
        let mut out = Self {
            root,
            revision: 0,
            state: json!({"status":"ready","failure":null,"sessions":[],"activeSessionId":null,"pinned":[],"drafts":{},"composers":{},"disk":{},"config":storage::default_config(),"configProblem":null,"notices":[],"writeError":null}),
            writable: true,
        };
        let path = out.state_path();
        if path.exists() {
            match storage::read_json(&path) {
                Ok(v) if v["format"] != 1 => {
                    out.writable = false;
                    out.state["status"] = json!("failed");
                    out.state["failure"] =
                        json!("Unsupported workspace state format; source preserved read-only");
                    out.notice(
                        "newer_format",
                        "Unsupported workspace state format; source preserved",
                    );
                    return Ok(out);
                }
                Ok(v) => match validate_state(&v) {
                    Ok(()) => {
                        out.revision = v["revision"].as_u64().unwrap();
                        out.state = v["state"].clone();
                        for session in out.state["sessions"].as_array_mut().unwrap() {
                            layout::normalize(&mut session["workbench"]);
                        }
                        out.state["status"] = json!("ready");
                        out.state["failure"] = V::Null;
                    }
                    Err(e) if e.kind == "unsupported_format" => {
                        out.writable = false;
                        out.state["status"] = json!("failed");
                        out.state["failure"] = json!(e.message);
                        return Ok(out);
                    }
                    Err(_) => out.recover(&path)?,
                },
                Err(_) => out.recover(&path)?,
            }
        }
        let cp = out.root.join(".workspace/config.json");
        if !cp.exists() {
            storage::write_json(&cp, &out.state["config"])?;
        }
        out.reload_config();
        let ep = out.root.join(".workspace/env.json");
        if !ep.exists() {
            storage::write_json(
                &ep,
                &json!({"version":1,"packages":[],"env":{},"post_scripts":[]}),
            )?;
        }
        out.maintenance()?;
        out.persist()?;
        Ok(out)
    }
    fn state_path(&self) -> PathBuf {
        self.root.join(".workspace/state/workspace.json")
    }
    fn notice(&mut self, kind: &str, message: &str) {
        self.state["notices"]
            .as_array_mut()
            .unwrap()
            .push(json!({"id":uuid::Uuid::new_v4().to_string(),"kind":kind,"message":message}));
    }
    fn recover(&mut self, path: &Path) -> Result<()> {
        fs::rename(
            path,
            self.root
                .join(".workspace/corrupt")
                .join(format!("{}-workspace.json", uuid::Uuid::new_v4())),
        )?;
        let backup = self.root.join(".workspace/state/workspace.json.bak");
        if let Ok(v) = storage::read_json(&backup) {
            if validate_state(&v).is_ok() {
                self.revision = v["revision"].as_u64().unwrap();
                self.state = v["state"].clone();
            }
        }
        self.notice(
            "recovered",
            "Damaged workspace state preserved; latest valid backup restored when available",
        );
        Ok(())
    }
    pub fn snapshot(&self) -> V {
        json!({"revision":self.revision,"state":self.state})
    }
    pub fn persist(&self) -> Result<()> {
        if !self.writable {
            return Err(Error::business(
                "read_only",
                "workspace format is unsupported",
            ));
        }
        let path = self.state_path();
        if path.exists() {
            let bytes = fs::read(&path)?;
            storage::atomic(&path.with_extension("json.bak"), &bytes)?;
        }
        {
            let document = json!({"format":1,"revision":self.revision,"state":self.state});
            if serde_json::to_vec(&document).unwrap().len() > 24 * 1024 * 1024 {
                return Err(Error::business(
                    "state_limit",
                    "workspace state exceeds 24 MiB; save or discard drafts",
                ));
            }
            storage::write_json(&path, &document)
        }
    }
    pub fn commit(&mut self, mut candidate: Self) -> Result<()> {
        candidate.revision = self
            .revision
            .checked_add(1)
            .ok_or_else(|| Error::business("overflow", "revision overflow"))?;
        candidate.state["writeError"] = V::Null;
        if let Err(e) = candidate.persist() {
            self.state["writeError"] = json!(e.message);
            return Err(e);
        }
        *self = candidate;
        Ok(())
    }
    pub fn command(&mut self, name: &str, a: &V) -> Result<V> {
        if !self.writable {
            return Err(Error::business("read_only", "workspace is read only"));
        }
        let mut next = self.clone();
        let value = next.execute(name, a)?;
        if next.state != self.state || name == "flush" {
            self.commit(next)?;
        }
        Ok(json!({"revision":self.revision,"state":self.state,"value":value}))
    }
    fn sessions(&self) -> &Vec<V> {
        self.state["sessions"].as_array().unwrap()
    }
    fn session_index(&self, id: &str) -> Option<usize> {
        self.sessions().iter().position(|v| s(&v["id"]) == id)
    }
    fn create_session(&mut self, name: Option<&str>, w: V) -> Result<V> {
        let name = name.map(str::trim);
        if name == Some("") {
            return Err(Error::invalid("session name is empty"));
        }
        let time = now();
        let id = uuid::Uuid::new_v4().to_string();
        self.state["sessions"].as_array_mut().unwrap().push(json!({"id":id,"name":name,"createdAt":time,"lastUsedAt":time,"usage":{"uses":1,"frecency":1.0,"frecencyAt":time},"archivedAt":null,"workbench":w}));
        self.state["activeSessionId"] = json!(id);
        Ok(json!(id))
    }
    fn activate(&mut self, id: &str) -> V {
        if let Some(i) = self.session_index(id) {
            self.state["sessions"][i]["archivedAt"] = V::Null;
            touch(&mut self.state["sessions"][i]);
            self.state["activeSessionId"] = json!(id);
            json!(true)
        } else {
            json!(false)
        }
    }
    fn observe(&mut self, path: &str) -> Result<V> {
        let version = storage::version(&storage::path(&self.root, path, false)?)?;
        self.state["disk"][path] = version.clone();
        Ok(version)
    }
    fn edit(&mut self, path: &str, text: &str, shown: &V) -> Result<()> {
        storage::path(&self.root, path, false)?;
        if text.len() > 16 * 1024 * 1024 {
            return Err(Error::invalid("editable text exceeds 16 MiB"));
        }
        let current = self.state["drafts"][path].clone();
        if current["text"] == text {
            return Ok(());
        }
        let base = if current.is_object() {
            current["base"].clone()
        } else {
            validate_version(shown)?;
            shown.clone()
        };
        if base["exists"] == true && base["sha256"] == storage::hash(text.as_bytes()) {
            self.state["drafts"].as_object_mut().unwrap().remove(path);
        } else {
            let revision = current["revision"]
                .as_u64()
                .unwrap_or(0)
                .checked_add(1)
                .ok_or_else(|| Error::business("overflow", "draft revision overflow"))?;
            self.state["drafts"][path] = json!({"format":1,"path":path,"text":text,"base":base,"editedAt":now(),"revision":revision});
        }
        if self.state["disk"].get(path).is_none() {
            self.observe(path)?;
        }
        self.touch_resource(&json!({"kind":"file","path":path}));
        Ok(())
    }
    fn touch_resource(&mut self, r: &V) {
        if let Some(i) = self.session_index(s(&self.state["activeSessionId"])) {
            if layout::resources(&self.state["sessions"][i]["workbench"]).contains(r) {
                touch(&mut self.state["sessions"][i]);
            }
        }
    }
    fn validate_text(&self, path: &str, text: &str) -> Result<()> {
        if path == ".workspace/config.json" {
            let v =
                crate::protocol::strict_json(text.as_bytes()).map_err(|e| Error::invalid(&e.to_string()))?;
            storage::config(&v)?;
        }
        if path == ".workspace/env.json" {
            let v =
                crate::protocol::strict_json(text.as_bytes()).map_err(|e| Error::invalid(&e.to_string()))?;
            crate::environment::validate(&v)?;
        }
        Ok(())
    }
    fn save(&mut self, path: &str) -> Result<V> {
        let disk = self.observe(path)?;
        let draft = self.state["drafts"][path].clone();
        if draft.is_null() {
            return Ok(json!({"kind":"unchanged","version":disk}));
        }
        let text = s(&draft["text"]);
        if !storage::same(&disk, &draft["base"])
            && !(disk["exists"] == true && disk["sha256"] == storage::hash(text.as_bytes()))
        {
            return Ok(json!({"kind":"conflict","disk":disk}));
        }
        if let Err(e) = self.validate_text(path, text) {
            return Ok(json!({"kind":"invalid","message":e.message}));
        }
        let target = storage::path(&self.root, path, false)?;
        if let Err(e) = storage::atomic(&target, text.as_bytes()) {
            return Ok(json!({"kind":"failed","message":e.message}));
        }
        let written = self.observe(path)?;
        self.state["drafts"].as_object_mut().unwrap().remove(path);
        if path == ".workspace/config.json" {
            self.reload_config();
        }
        Ok(json!({"kind":"saved","version":written,"text":text}))
    }
    fn reload_config(&mut self) {
        let cp = self.root.join(".workspace/config.json");
        match storage::read_json(&cp).and_then(|v| storage::config(&v)) {
            Ok(config) => {
                self.state["config"] = config;
                self.state["configProblem"] = V::Null;
            }
            Err(e) => {
                self.state["configProblem"] = json!(e.message);
            }
        }
        if let Ok(version) = storage::version(&cp) {
            self.state["disk"][".workspace/config.json"] = version;
        }
    }
    fn dirty(&self) -> Vec<V> {
        let mut refs: Vec<V> = self.state["drafts"]
            .as_object()
            .unwrap()
            .keys()
            .map(|p| json!({"kind":"file","path":p}))
            .collect();
        for (c, d) in self.state["composers"].as_object().unwrap() {
            if has_content(d) {
                refs.push(json!({"kind":"conversation","id":c}));
            }
        }
        refs
    }
    fn archive(&mut self, id: &str, decision: &str) -> Result<V> {
        let Some(i) = self.session_index(id) else {
            return Ok(json!({"kind":"notFound"}));
        };
        if !self.state["sessions"][i]["archivedAt"].is_null() {
            return Ok(json!({"kind":"archived","savedPaths":[]}));
        }
        let refs = layout::resources(&self.state["sessions"][i]["workbench"]);
        let dirty: Vec<_> = self
            .dirty()
            .into_iter()
            .filter(|r| refs.contains(r))
            .collect();
        if decision.is_empty() && !dirty.is_empty() {
            return Ok(json!({"kind":"needsDecision","resources":dirty}));
        }
        if !matches!(decision, "" | "save_all" | "keep_drafts" | "discard") {
            return Err(Error::invalid("invalid archive decision"));
        }
        let mut files: Vec<_> = dirty
            .iter()
            .filter(|r| r["kind"] == "file")
            .map(|r| s(&r["path"]).to_owned())
            .collect();
        files.sort();
        let mut saved = vec![];
        if decision == "save_all" {
            let mut conflicts = vec![];
            for p in &files {
                let d = self.state["drafts"][p].clone();
                let disk = self.observe(p)?;
                if !storage::same(&disk, &d["base"])
                    && !(disk["exists"] == true
                        && disk["sha256"] == storage::hash(s(&d["text"]).as_bytes()))
                {
                    conflicts.push(p);
                }
            }
            if !conflicts.is_empty() {
                return Ok(json!({"kind":"saveConflict","paths":conflicts}));
            }
            for p in &files {
                if let Err(e) = self.validate_text(p, s(&self.state["drafts"][p]["text"])) {
                    return Ok(json!({"kind":"invalid","path":p,"message":e.message}));
                }
            }
            for p in &files {
                let v = self.save(p)?;
                match s(&v["kind"]) {
                    "saved" | "unchanged" => saved.push(p.clone()),
                    "conflict" => return Ok(json!({"kind":"saveConflict","paths":[p]})),
                    "invalid" => {
                        return Ok(json!({"kind":"invalid","path":p,"message":v["message"]}));
                    }
                    _ => return Ok(v),
                }
            }
        } else if decision == "discard" {
            for p in &files {
                self.state["drafts"].as_object_mut().unwrap().remove(p);
            }
            for r in &dirty {
                if r["kind"] == "conversation" {
                    self.clear_composer(s(&r["id"]))?;
                }
            }
        }
        self.state["sessions"][i]["archivedAt"] = json!(now());
        if self.state["activeSessionId"] == id {
            self.state["activeSessionId"] = V::Null;
        }
        Ok(json!({"kind":"archived","savedPaths":saved}))
    }
    fn composer(&self, id: &str) -> V {
        self.state["composers"].get(id).cloned().unwrap_or_else(
            || json!({"format":1,"conversationId":id,"revision":0,"text":"","attachments":[]}),
        )
    }
    fn clear_composer(&mut self, id: &str) -> Result<V> {
        let mut d = self.composer(id);
        if has_content(&d) {
            d["revision"] = json!(
                d["revision"]
                    .as_u64()
                    .unwrap_or(0)
                    .checked_add(1)
                    .ok_or_else(|| Error::business("overflow", "composer revision overflow"))?
            );
            d["text"] = json!("");
            d["attachments"] = json!([]);
            self.state["composers"][id] = d.clone();
        }
        Ok(d)
    }
    pub fn maintenance(&mut self) -> Result<()> {
        let mut protected = HashSet::new();
        for r in self.dirty() {
            if let Some(best) = self
                .sessions()
                .iter()
                .filter(|v| {
                    v["archivedAt"].is_null() && layout::resources(&v["workbench"]).contains(&r)
                })
                .max_by(|a, b| {
                    a["lastUsedAt"]
                        .as_u64()
                        .cmp(&b["lastUsedAt"].as_u64())
                        .then(a["createdAt"].as_u64().cmp(&b["createdAt"].as_u64()))
                        .then(s(&a["id"]).cmp(s(&b["id"])))
                })
            {
                protected.insert(s(&best["id"]).to_owned());
            }
        }
        let time = now();
        let active = s(&self.state["activeSessionId"]).to_owned();
        let mut active_archived = false;
        for session in self.state["sessions"].as_array_mut().unwrap() {
            let retention = if session["name"].is_null() {
                86_400_000
            } else {
                7 * 86_400_000
            };
            if session["archivedAt"].is_null()
                && !protected.contains(s(&session["id"]))
                && time.saturating_sub(session["lastUsedAt"].as_u64().unwrap_or(time)) >= retention
            {
                session["archivedAt"] = json!(time);
                active_archived |= s(&session["id"]) == active;
            }
        }
        if active_archived {
            self.state["activeSessionId"] = V::Null;
        }
        self.refresh()?;
        self.purge_trash()?;
        Ok(())
    }
    pub fn refresh(&mut self) -> Result<()> {
        let mut paths: BTreeSet<String> = self.state["drafts"]
            .as_object()
            .unwrap()
            .keys()
            .cloned()
            .collect();
        paths.insert(".workspace/config.json".into());
        for session in self.sessions().iter().filter(|s| s["archivedAt"].is_null()) {
            for r in layout::resources(&session["workbench"]) {
                if r["kind"] == "file" {
                    paths.insert(s(&r["path"]).to_owned());
                }
            }
        }
        for p in paths {
            if let Ok(path) = storage::path(&self.root, &p, false) {
                if let Ok(metadata) = fs::metadata(path) {
                    let modified = metadata
                        .modified()
                        .unwrap_or(std::time::UNIX_EPOCH)
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_millis() as u64;
                    let known = &self.state["disk"][&p];
                    if metadata.is_file()
                        && known["exists"] == true
                        && known["size"].as_u64() == Some(metadata.len())
                        && known["modifiedAt"].as_u64() == Some(modified)
                    {
                        continue;
                    }
                }
            }
            let Ok(disk) = self.observe(&p) else {
                continue;
            };
            if self.state["drafts"][&p].is_object()
                && disk["exists"] == true
                && disk["sha256"] == storage::hash(s(&self.state["drafts"][&p]["text"]).as_bytes())
            {
                self.state["drafts"].as_object_mut().unwrap().remove(&p);
            }
        }
        self.reload_config();
        Ok(())
    }
    fn after_removal(&mut self, path: &str) -> Result<()> {
        let drafts = self.state["drafts"].clone();
        for session in self.state["sessions"].as_array_mut().unwrap() {
            let closing: Vec<_> = session["workbench"]["panels"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|p| {
                    matches!(s(&p["target"]["kind"]), "file" | "image" | "diff")
                        && within(s(&p["target"]["path"]), path)
                        && drafts.get(s(&p["target"]["path"])).is_none()
                })
                .map(|p| p["id"].clone())
                .collect();
            session["workbench"] = layout::apply(
                &session["workbench"],
                &json!({"type":"close","panelIds":closing}),
            );
        }
        self.refresh()
    }
    fn purge_trash(&self) -> Result<usize> {
        let mut count = 0;
        for e in fs::read_dir(self.root.join(".workspace/trash"))? {
            let e = e?;
            let meta = e.path().join("entry.json");
            if let Ok(v) = storage::read_json(&meta) {
                if now().saturating_sub(v["trashedAt"].as_u64().unwrap_or(now())) >= 7 * 86_400_000
                {
                    fs::remove_dir_all(e.path())?;
                    count += 1;
                }
            }
        }
        Ok(count)
    }
    fn file_op(&mut self, name: &str, a: &V) -> Result<V> {
        let result = (|| -> Result<V> {
            match name {
                "createFile" => {
                    let raw = required(a, "path")?;
                    let path = storage::path(&self.root, raw, false)?;
                    let data =
                        crate::protocol::decode_blob(a.get("data").and_then(V::as_str).unwrap_or(""), 65536)?;
                    storage::create_atomic(&path, &data)?;
                    self.refresh()?;
                }
                "createDirectory" => {
                    let path = storage::path(&self.root, required(a, "path")?, false)?;
                    fs::create_dir(path)?;
                }
                "deletePath" | "trashPath" => {
                    let raw = required(a, "path")?;
                    if raw.starts_with(".workspace/") {
                        return Err(Error::invalid("configuration cannot be deleted"));
                    }
                    let path = storage::path(&self.root, raw, false)?;
                    if name == "trashPath" {
                        let id = uuid::Uuid::new_v4().to_string();
                        let dir = self.root.join(".workspace/trash").join(&id);
                        fs::create_dir(&dir)?;
                        let entry = json!({"id":id,"path":raw,"trashedAt":now(),"isDirectory":path.is_dir()});
                        storage::write_json(&dir.join("entry.json"), &entry)?;
                        fs::rename(&path, dir.join("content"))?;
                        self.after_removal(raw)?;
                        return Ok(json!({"kind":"trashed","entry":entry}));
                    }
                    if path.is_dir() {
                        fs::remove_dir_all(path)?;
                    } else {
                        fs::remove_file(path)?;
                    }
                    self.after_removal(raw)?;
                }
                "movePath" | "copyPath" => {
                    let from = required(a, "from")?;
                    let to = required(a, "to")?;
                    if from.starts_with(".workspace/") || to.starts_with(".workspace/") {
                        return Err(Error::invalid("configuration cannot be moved or copied"));
                    }
                    let src = storage::path(&self.root, from, false)?;
                    let dst = storage::path(&self.root, to, false)?;
                    if dst.exists() {
                        return Err(Error::business("exists", "destination exists"));
                    }
                    if within(to, from) {
                        return Err(Error::invalid("cannot move or copy into itself"));
                    }
                    if name == "copyPath" {
                        storage::copy_tree(&src, &dst)?;
                    } else {
                        for path in self.state["drafts"].as_object().unwrap().keys() {
                            if within(path, to) && !within(path, from) {
                                return Err(Error::business(
                                    "draft_conflict",
                                    "destination has an unsaved draft",
                                ));
                            }
                            if within(path, from) {
                                let target = layout::rebase(path, from, to);
                                if self.state["drafts"].get(&target).is_some()
                                    && !within(&target, from)
                                {
                                    return Err(Error::business(
                                        "draft_conflict",
                                        "destination has an unsaved draft",
                                    ));
                                }
                            }
                        }
                        storage::publish_new(&src, &dst)?;
                        for field in ["drafts", "disk"] {
                            let old = self.state[field].as_object().unwrap().clone();
                            self.state[field] = json!({});
                            for (p, mut v) in old {
                                let new = layout::rebase(&p, from, to);
                                if field == "drafts" {
                                    v["path"] = json!(new);
                                }
                                self.state[field][new] = v;
                            }
                        }
                        for d in self.state["composers"]
                            .as_object_mut()
                            .unwrap()
                            .values_mut()
                        {
                            for at in d["attachments"].as_array_mut().unwrap() {
                                at["path"] = json!(layout::rebase(s(&at["path"]), from, to));
                            }
                        }
                        for session in self.state["sessions"].as_array_mut().unwrap() {
                            session["workbench"] = layout::apply(
                                &session["workbench"],
                                &json!({"type":"renamePath","from":from,"to":to}),
                            );
                        }
                        self.refresh()?;
                    }
                }
                "restoreFromTrash" => {
                    let id = storage::identifier(required(a, "id")?)?;
                    let dir = self.root.join(".workspace/trash").join(id);
                    let entry = storage::read_json(&dir.join("entry.json"))?;
                    let original = required(&entry, "path")?;
                    let mut raw = original.to_owned();
                    let mut path = storage::path(&self.root, &raw, false)?;
                    let p = Path::new(original);
                    let name = p.file_stem().and_then(|p| p.to_str()).unwrap_or("restored");
                    let ext = p
                        .extension()
                        .and_then(|e| e.to_str())
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
                        path = storage::path(&self.root, &raw, false)?;
                        n += 1;
                    }
                    fs::create_dir_all(path.parent().unwrap())?;
                    storage::publish_new(&dir.join("content"), &path)?;
                    fs::remove_dir_all(dir)?;
                    self.refresh()?;
                    return Ok(json!({"kind":"restored","path":raw}));
                }
                _ => return Err(Error::invalid("unknown file operation")),
            }
            Ok(json!({"kind":"done"}))
        })();
        Ok(result.unwrap_or_else(|e| json!({"kind":"failed","message":e.message})))
    }
    fn execute(&mut self, name: &str, a: &V) -> Result<V> {
        match name {
            "enterWorkbench" => {
                let id = self
                    .sessions()
                    .iter()
                    .find(|v| v["id"] == self.state["activeSessionId"] && v["archivedAt"].is_null())
                    .or_else(|| {
                        self.sessions()
                            .iter()
                            .filter(|v| v["archivedAt"].is_null())
                            .max_by_key(|v| v["lastUsedAt"].as_u64())
                    })
                    .map(|v| s(&v["id"]).to_owned());
                if let Some(id) = id {
                    self.activate(&id);
                    Ok(json!(id))
                } else {
                    self.create_session(None, layout::empty())
                }
            }
            "createSession" => self.create_session(a["name"].as_str(), layout::empty()),
            "openInSeparateSession" => {
                if !layout::valid_target(&a["target"]) {
                    return Err(Error::invalid("invalid target"));
                }
                self.create_session(
                    None,
                    layout::apply(
                        &layout::empty(),
                        &json!({"type":"enterSolo","target":a["target"]}),
                    ),
                )
            }
            "activateSession" | "restoreSession" => Ok(self.activate(required(a, "id")?)),
            "renameSession" => {
                let name = required(a, "name")?.trim();
                if name.is_empty() {
                    return Err(Error::invalid("name is empty"));
                }
                if let Some(i) = self.session_index(required(a, "id")?) {
                    self.state["sessions"][i]["name"] = json!(name);
                    Ok(json!(true))
                } else {
                    Ok(json!(false))
                }
            }
            "archiveSession" => self.archive(required(a, "id")?, s(&a["decision"])),
            "pinSession" | "unpinSession" => {
                let id = required(a, "id")?;
                let Some(i) = self.session_index(id) else {
                    return Ok(json!(false));
                };
                let mut pinned = self.state["pinned"].as_array().unwrap().clone();
                let existed = pinned.contains(&json!(id));
                pinned.retain(|p| p != id);
                if name == "pinSession" {
                    if self.state["sessions"][i]["name"].is_null() {
                        return Ok(json!(false));
                    }
                    let idx = a["index"].as_i64().unwrap_or(0).max(0) as usize;
                    pinned.insert(idx.min(pinned.len()), json!(id));
                }
                self.state["pinned"] = json!(pinned);
                Ok(json!(name == "pinSession" || existed))
            }
            "runMaintenance" => {
                self.maintenance()?;
                Ok(V::Null)
            }
            "flush" => Ok(json!(true)),
            "dismissNotice" => {
                let id = required(a, "id")?;
                self.state["notices"]
                    .as_array_mut()
                    .unwrap()
                    .retain(|n| n["id"] != id);
                Ok(V::Null)
            }
            "applyLayout" => {
                let id = a["sessionId"]
                    .as_str()
                    .unwrap_or(s(&self.state["activeSessionId"]));
                let Some(i) = self.session_index(id) else {
                    return Ok(V::Null);
                };
                if !self.state["sessions"][i]["archivedAt"].is_null() {
                    return Ok(V::Null);
                }
                let op = &a["op"];
                if !op.is_object() {
                    return Err(Error::invalid("missing layout op"));
                }
                if !LAYOUT_OPS.contains(&s(&op["type"])) {
                    return Err(Error::invalid("unknown layout operation"));
                }
                let w = layout::apply(&self.state["sessions"][i]["workbench"], op);
                if w != self.state["sessions"][i]["workbench"] {
                    self.state["sessions"][i]["workbench"] = w.clone();
                    touch(&mut self.state["sessions"][i]);
                }
                Ok(w)
            }
            "openFile" => {
                let raw = required(a, "path")?;
                let path = storage::path(&self.root, raw, false)?;
                let disk = self.observe(raw)?;
                let too_large = disk["size"].as_u64().unwrap_or(0) > 16 * 1024 * 1024;
                let bytes = if disk["exists"] == true && !too_large {
                    Some(fs::read(path)?)
                } else {
                    None
                };
                let text = bytes.as_ref().and_then(|b| {
                    if b.contains(&0) {
                        None
                    } else {
                        std::str::from_utf8(b).ok()
                    }
                });
                Ok(
                    json!({"path":raw,"disk":disk,"diskText":text,"binary":bytes.is_some()&&text.is_none(),"tooLarge":too_large,"draft":self.state["drafts"][raw]}),
                )
            }
            "editFile" => {
                self.edit(required(a, "path")?, required(a, "text")?, &a["shown"])?;
                Ok(V::Null)
            }
            "saveFile" => {
                let raw = required(a, "path")?;
                if let Some(text) = a["text"].as_str() {
                    let base = if self.state["drafts"][raw].is_object() {
                        self.state["drafts"][raw]["base"].clone()
                    } else if self.state["disk"][raw].is_object() {
                        self.state["disk"][raw].clone()
                    } else {
                        self.observe(raw)?
                    };
                    self.edit(raw, text, &base)?;
                }
                self.save(raw)
            }
            "resolveConflict" => {
                let path = required(a, "path")?;
                storage::path(&self.root, path, false)?;
                let version = self.observe(path)?;
                match required(a, "resolution")? {
                    "use_disk" => {
                        self.state["drafts"].as_object_mut().unwrap().remove(path);
                    }
                    "keep_mine" => {
                        if self.state["drafts"][path].is_object() {
                            self.state["drafts"][path]["base"] = version;
                            self.state["drafts"][path]["editedAt"] = json!(now());
                            self.state["drafts"][path]["revision"] =
                                json!(self.state["drafts"][path]["revision"].as_u64().unwrap() + 1);
                        }
                    }
                    _ => return Err(Error::invalid("unknown conflict resolution")),
                }
                Ok(V::Null)
            }
            "discardDraft" => {
                let p = required(a, "path")?;
                storage::path(&self.root, p, false)?;
                self.state["drafts"].as_object_mut().unwrap().remove(p);
                Ok(V::Null)
            }
            "deletePath" | "trashPath" | "createDirectory" | "movePath" | "copyPath"
            | "restoreFromTrash" | "createFile" => self.file_op(name, a),
            "purgeTrash" => Ok(json!(self.purge_trash()?)),
            "listDirectory" => {
                let raw = required(a, "path")?;
                let path = storage::path(&self.root, raw, true)?;
                let hidden = a["showHidden"] == true;
                let mut entries = vec![];
                for e in fs::read_dir(path)? {
                    let e = e?;
                    let name = e.file_name().to_string_lossy().into_owned();
                    if name == ".workspace"
                        || (!hidden && name.starts_with('.'))
                        || e.file_type()?.is_symlink()
                    {
                        continue;
                    }
                    let m = e.metadata()?;
                    entries.push(json!({"path":if raw.is_empty(){name}else{format!("{raw}/{name}")},"isDirectory":m.is_dir(),"size":m.len(),"modifiedAt":m.modified().unwrap_or(std::time::UNIX_EPOCH).duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis()as u64}));
                }
                entries.sort_by(|a, b| {
                    b["isDirectory"]
                        .as_bool()
                        .cmp(&a["isDirectory"].as_bool())
                        .then(
                            s(&a["path"])
                                .to_lowercase()
                                .cmp(&s(&b["path"]).to_lowercase()),
                        )
                });
                Ok(json!(entries))
            }
            "editComposer" => {
                let mut draft = a["draft"].clone();
                let id = required(&draft, "conversationId")?.to_owned();
                if id.trim().is_empty() {
                    return Err(Error::invalid("empty conversation id"));
                }
                required(&draft, "text")?;
                let Some(attachments) = draft["attachments"].as_array() else {
                    return Err(Error::invalid("attachments must be an array"));
                };
                for at in attachments {
                    storage::path(&self.root, required(at, "path")?, false)?;
                }
                draft["format"] = json!(1);
                let current = self.composer(&id);
                if current == draft {
                    return Ok(current);
                }
                if a["expectedRevision"] != current["revision"]
                    || draft["revision"].as_u64().unwrap_or(0)
                        <= current["revision"].as_u64().unwrap_or(0)
                {
                    return Err(Error::business("conflict", "composer revision changed"));
                }
                self.state["composers"][&id] = draft.clone();
                self.touch_resource(&json!({"kind":"conversation","id":id}));
                Ok(draft)
            }
            "acknowledgeComposer" => {
                let mut submitted = a["submitted"].clone();
                let id = required(&submitted, "conversationId")?.to_owned();
                submitted["format"] = json!(1);
                let current = self.composer(&id);
                if current == submitted {
                    self.clear_composer(&id)
                } else {
                    Ok(current)
                }
            }
            "discardComposer" | "removeConversation" => {
                let id = required(a, "conversationId")?;
                let cleared = self.clear_composer(id)?;
                if name == "removeConversation" {
                    for session in self.state["sessions"].as_array_mut().unwrap() {
                        let ids: Vec<V> = session["workbench"]["panels"]
                            .as_array()
                            .unwrap()
                            .iter()
                            .filter(|p| {
                                p["target"]["kind"] == "conversation" && p["target"]["id"] == id
                            })
                            .map(|p| p["id"].clone())
                            .collect();
                        session["workbench"] = layout::apply(
                            &session["workbench"],
                            &json!({"type":"close","panelIds":ids}),
                        );
                    }
                    Ok(V::Null)
                } else {
                    Ok(cleared)
                }
            }
            "updateConfig" => {
                if a["expectedRevision"].as_u64() != Some(self.revision) {
                    return Ok(json!({"kind":"conflict","revision":self.revision}));
                }
                let previous_config = self.state["config"].clone();
                self.reload_config();
                if self.state["configProblem"].is_string() {
                    return Ok(json!({"kind":"blocked","problem":self.state["configProblem"]}));
                }
                if self.state["config"] != previous_config {
                    return Ok(json!({"kind":"conflict","revision":self.revision+1}));
                }
                let mut input = self.state["config"].clone();
                storage::merge(&mut input, &a["config"]);
                match storage::config(&input) {
                    Ok(config) => {
                        storage::write_json(&self.root.join(".workspace/config.json"), &config)?;
                        self.reload_config();
                        Ok(json!({"kind":"updated","config":config}))
                    }
                    Err(e) => Ok(json!({"kind":"failed","message":e.message})),
                }
            }
            _ => Err(Error::invalid("unknown workspace command")),
        }
    }
}
pub fn required<'a>(v: &'a V, k: &str) -> Result<&'a str> {
    v[k].as_str()
        .ok_or_else(|| Error::invalid(&format!("expected string {k}")))
}
fn within(p: &str, parent: &str) -> bool {
    p == parent || p.starts_with(&format!("{parent}/"))
}
fn has_content(d: &V) -> bool {
    !s(&d["text"]).is_empty() || d["attachments"].as_array().is_some_and(|a| !a.is_empty())
}
fn touch(session: &mut V) {
    let time = now();
    let old = session["lastUsedAt"].as_u64().unwrap_or(time);
    if time <= old {
        return;
    }
    if time - old >= 1_800_000 {
        let frecency = session["usage"]["frecency"].as_f64().unwrap_or(1.0)
            * 0.5f64.powf(
                time.saturating_sub(session["usage"]["frecencyAt"].as_u64().unwrap_or(time)) as f64
                    / (7.0 * 86_400_000.0),
            )
            + 1.0;
        session["usage"] = json!({"uses":session["usage"]["uses"].as_u64().unwrap_or(1).saturating_add(1),"frecency":frecency,"frecencyAt":time});
    }
    session["lastUsedAt"] = json!(time);
}
fn validate_version(v: &V) -> Result<()> {
    if !v["exists"].is_boolean() || !v["size"].is_number() || !v["modifiedAt"].is_number() {
        return Err(Error::invalid("invalid DiskVersion"));
    }
    Ok(())
}
fn validate_state(v: &V) -> Result<()> {
    let st = &v["state"];
    if v["format"] != 1 || !v["revision"].is_u64() || !st.is_object() {
        return Err(Error::invalid("invalid state envelope"));
    }
    for k in ["sessions", "pinned", "notices"] {
        if !st[k].is_array() {
            return Err(Error::invalid("invalid state array"));
        }
    }
    for k in ["drafts", "composers", "disk", "config"] {
        if !st[k].is_object() {
            return Err(Error::invalid("invalid state map"));
        }
    }
    let mut ids = HashSet::new();
    for session in st["sessions"].as_array().unwrap() {
        if s(&session["id"]).is_empty()
            || !ids.insert(s(&session["id"]))
            || !session["createdAt"].is_u64()
            || !session["lastUsedAt"].is_u64()
        {
            return Err(Error::invalid("invalid session"));
        }
    }
    for (p, d) in st["drafts"].as_object().unwrap() {
        if d["format"] != 1 {
            return Err(Error::business(
                "unsupported_format",
                "unknown draft format; source preserved read-only",
            ));
        }
        if d["path"] != *p
            || !layout::valid_path(p)
            || !d["text"].is_string()
            || !d["revision"].is_u64()
        {
            return Err(Error::invalid("invalid draft"));
        }
        validate_version(&d["base"])?;
    }
    for (id, d) in st["composers"].as_object().unwrap() {
        if d["format"] != 1 {
            return Err(Error::business(
                "unsupported_format",
                "unknown composer format; source preserved read-only",
            ));
        }
        if d["conversationId"] != *id
            || !d["text"].is_string()
            || !d["revision"].is_u64()
            || !d["attachments"].is_array()
        {
            return Err(Error::invalid("invalid composer"));
        }
    }
    Ok(())
}
const LAYOUT_OPS: &[&str] = &[
    "open",
    "focus",
    "focusStack",
    "close",
    "move",
    "splitStack",
    "resizeSplit",
    "resetSplit",
    "resizeRegion",
    "setRegionCollapsed",
    "toggleRegion",
    "setMaximized",
    "switchParadigm",
    "promoteConversation",
    "returnToFiles",
    "enterSolo",
    "setChatSideStack",
    "retarget",
    "updateView",
    "updateExplorer",
    "renamePath",
];
