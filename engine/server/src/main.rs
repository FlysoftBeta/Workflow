mod chat;
mod environment;
mod home_stage;
mod imports;
mod layout;
mod local_services;
mod process;
mod protocol;
mod storage;
mod terminal;
mod tools;
mod workspace;
use protocol::{
    Error, MAX_BLOB, MAX_FRAME, PROTOCOL, Result, RpcId, decode_blob, encode_blob, line, response,
    strict_json,
};
use serde_json::{Value as V, json};
use std::{
    collections::{HashMap, HashSet},
    fs::{self, File, OpenOptions},
    io::{self, Read, Seek, SeekFrom, Write},
    os::fd::AsRawFd,
    path::PathBuf,
    sync::{
        Arc, Condvar, Mutex,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    time::{Duration, Instant},
};
struct Upload {
    path: String,
    temp: PathBuf,
    size: u64,
    offset: u64,
    unique: Option<(String, String)>,
}
struct Server {
    workspace: Mutex<workspace::Workspace>,
    changed: Condvar,
    environment: Arc<Mutex<environment::Environment>>,
    processes: Arc<process::Processes>,
    chat: chat::Chat,
    terminals: terminal::Terminals,
    execution: Mutex<()>,
    uploads: Mutex<HashMap<String, Upload>>,
    closed: AtomicBool,
    workers: AtomicUsize,
}
impl Server {
    fn service_document(
        &self,
        w: &mut workspace::Workspace,
        method: &str,
        service: &str,
        key: &str,
        a: &V,
    ) -> Result<V> {
        let raw = format!(".workspace/services/{service}/{key}");
        let canonical = storage::path(&w.root, &raw, false)?;
        let sidecar = w
            .root
            .join(".workspace/documents")
            .join(format!("services.{service}"))
            .join(format!("{key}.json"));
        let document = match File::open(&canonical) {
            Ok(file) => {
                let mut bytes = vec![];
                file.take(16 * 1024 * 1024 + 1).read_to_end(&mut bytes)?;
                if bytes.len() > 16 * 1024 * 1024 {
                    return Err(Error::business(
                        "too_large",
                        "service document exceeds 16 MiB",
                    ));
                }
                Some(String::from_utf8(bytes).map_err(|_| {
                    Error::business("invalid_text", "service document is not UTF-8")
                })?)
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => None,
            Err(error) => return Err(error.into()),
        };
        let sha = document.as_ref().map(|d| storage::hash(d.as_bytes()));
        let mut meta = if sidecar.exists() {
            let meta = storage::read_json(&sidecar)?;
            if meta["format"] != 1 || !meta["revision"].is_u64() {
                return Err(Error::business(
                    "unsupported_format",
                    "invalid service document sidecar",
                ));
            }
            meta
        } else {
            json!({"format":1,"revision":0,"sha256":null})
        };
        if meta["sha256"] != json!(sha) {
            meta["revision"] = json!(
                meta["revision"]
                    .as_u64()
                    .unwrap()
                    .checked_add(1)
                    .ok_or_else(|| Error::business("overflow", "document revision overflow"))?
            );
            meta["sha256"] = json!(sha);
            if w.writable {
                storage::write_json(&sidecar, &meta)?;
                let mut next = w.clone();
                next.refresh()?;
                w.commit(next)?;
                self.changed.notify_all();
            }
        }
        if method == "documents.read" {
            return Ok(json!({"document":document,"revision":meta["revision"]}));
        }
        if let Some(expected) = a.get("expectedRevision") {
            if *expected != meta["revision"] {
                return Err(Error::business("conflict", "service document changed"));
            }
        }
        if method == "documents.quarantine" {
            if canonical.exists() {
                fs::rename(
                    &canonical,
                    w.root
                        .join(".workspace/corrupt")
                        .join(format!("{}-{service}-{key}", uuid::Uuid::new_v4())),
                )?;
            }
            meta["sha256"] = V::Null;
        } else {
            let text = workspace::required(a, "document")?;
            if text.len() > 16 * 1024 * 1024 {
                return Err(Error::invalid("service document exceeds 16 MiB"));
            }
            storage::atomic(&canonical, text.as_bytes())?;
            meta["sha256"] = json!(storage::hash(text.as_bytes()));
        }
        meta["revision"] = json!(
            meta["revision"]
                .as_u64()
                .unwrap()
                .checked_add(1)
                .ok_or_else(|| Error::business("overflow", "document revision overflow"))?
        );
        storage::write_json(&sidecar, &meta)?;
        let mut next = w.clone();
        next.refresh()?;
        w.commit(next)?;
        self.changed.notify_all();
        Ok(json!({"revision":meta["revision"]}))
    }
    fn call(self: &Arc<Self>, method: &str, a: &V) -> Result<V> {
        if matches!(
            method,
            "documents.write"
                | "documents.quarantine"
                | "services.report"
                | "files.upload.begin"
                | "files.upload.chunk"
                | "files.upload.commit"
                | "files.upload.cancel"
                | "environment.reconcile"
                | "environment.restart"
                | "process.spawn"
                | "chat.command"
                | "environment.tools.install"
                | "terminal.create"
                | "terminal.attach"
                | "terminal.restart"
                | "terminal.rename"
                | "terminal.clear"
        ) && !self.workspace.lock().unwrap().writable
        {
            return Err(Error::business(
                "read_only",
                "workspace schema is unsupported",
            ));
        }
        match method {
            "chat.snapshot" | "chat.watch" | "chat.command" => {
                let weak = Arc::downgrade(self);
                self.chat.request(&self.environment, Arc::new(move |method, params| {
                    let server = weak.upgrade().ok_or_else(|| Error::business("closed", "workspace stopped"))?;
                    if server.closed.load(Ordering::SeqCst) { return Err(Error::business("closed", "workspace stopped")); }
                    if method == "hello" {
                        if params["protocol"] != PROTOCOL || params["clientId"].as_str().is_none_or(str::is_empty) {
                            return Err(Error::business("protocol_mismatch", "invalid internal chat handshake"));
                        }
                        let w = server.workspace.lock().unwrap();
                        return Ok(json!({"protocol":PROTOCOL,"engineVersion":"1.0.0","workspaceRoot":w.root,"capabilities":{"workspace":true,"files":true,"documents":true,"environment":true,"processes":true,"pty":true,"services":true,"chat":true,"maxFrameBytes":MAX_FRAME,"maxBlobChunkBytes":MAX_BLOB}}));
                    }
                    server.call(method, params)
                }), method, a)
            }
            "workspace.snapshot" => Ok(self.workspace.lock().unwrap().snapshot()),
            "workspace.watch" => {
                let after = a["afterRevision"]
                    .as_u64()
                    .ok_or_else(|| Error::invalid("afterRevision must be an integer"))?;
                let timeout = a["timeoutMs"].as_u64().unwrap_or(30000);
                if timeout > 30000 {
                    return Err(Error::invalid("timeoutMs exceeds 30000"));
                }
                let deadline = Instant::now() + Duration::from_millis(timeout);
                let mut w = self.workspace.lock().unwrap();
                while w.revision <= after
                    && Instant::now() < deadline
                    && !self.closed.load(Ordering::SeqCst)
                {
                    w = self
                        .changed
                        .wait_timeout(w, deadline.saturating_duration_since(Instant::now()))
                        .unwrap()
                        .0;
                }
                Ok(w.snapshot())
            }
            "workspace.command" => {
                let name = workspace::required(a, "name")?;
                let args = a.get("args").cloned().unwrap_or(json!({}));
                if !args.is_object() {
                    return Err(Error::invalid("command args must be object"));
                }
                let result = self.workspace.lock().unwrap().command(name, &args);
                self.changed.notify_all();
                result
            }
            "files.read" => {
                let root = self.workspace.lock().unwrap().root.clone();
                let p = storage::path(&root, workspace::required(a, "path")?, false)?;
                let offset = a["offset"].as_u64().unwrap_or(0);
                let length = a["length"].as_u64().unwrap_or(MAX_BLOB as u64);
                if length > MAX_BLOB as u64 {
                    return Err(Error::invalid("length exceeds 65536"));
                }
                let mut f = File::open(p)?;
                let size = f.metadata()?.len();
                if offset > size {
                    return Err(Error::invalid("offset exceeds file size"));
                }
                f.seek(SeekFrom::Start(offset))?;
                let mut data = vec![];
                f.take(length).read_to_end(&mut data)?;
                let next = offset + data.len() as u64;
                Ok(
                    json!({"data":encode_blob(&data),"nextOffset":next,"eof":next==size,"size":size}),
                )
            }
            "files.upload.begin" => {
                let root = self.workspace.lock().unwrap().root.clone();
                let unique = if a.get("path").is_none() {
                    let directory = workspace::required(a, "directory")?;
                    let name = workspace::required(a, "name")?;
                    imports::validate(&root, directory, name)?;
                    Some((directory.to_owned(), name.to_owned()))
                } else {
                    None
                };
                let raw = if let Some((directory, name)) = &unique {
                    if directory.is_empty() {
                        name.clone()
                    } else {
                        format!("{directory}/{name}")
                    }
                } else {
                    workspace::required(a, "path")?.to_owned()
                };
                let p = storage::path(&root, &raw, false)?;
                if raw.starts_with(".workspace/") && !raw.starts_with(".workspace/services/") {
                    return Err(Error::invalid(
                        "upload is only for user files and explicit service assets",
                    ));
                }
                if p.exists() && unique.is_none() {
                    return Err(Error::business("exists", "destination already exists"));
                }
                let size = a["size"]
                    .as_u64()
                    .ok_or_else(|| Error::invalid("size must be nonnegative integer"))?;
                if size > 8 * 1024 * 1024 * 1024 {
                    return Err(Error::invalid("upload exceeds 8 GiB"));
                }
                let mut uploads = self.uploads.lock().unwrap();
                if uploads.len() >= 16 {
                    return Err(Error::business("limit", "too many uploads"));
                }
                let id = uuid::Uuid::new_v4().to_string();
                let temp = root.join(".workspace/uploads").join(&id);
                OpenOptions::new()
                    .create_new(true)
                    .write(true)
                    .open(&temp)?;
                uploads.insert(
                    id.clone(),
                    Upload {
                        path: raw.into(),
                        temp,
                        size,
                        offset: 0,
                        unique,
                    },
                );
                Ok(json!({"uploadId":id}))
            }
            "files.upload.chunk" => {
                let mut uploads = self.uploads.lock().unwrap();
                let upload = uploads
                    .get_mut(workspace::required(a, "uploadId")?)
                    .ok_or_else(|| Error::business("not_found", "upload not found"))?;
                if a["offset"].as_u64() != Some(upload.offset) {
                    return Err(Error::business(
                        "invalid_offset",
                        "upload offset is not nextOffset",
                    ));
                }
                let data = decode_blob(workspace::required(a, "data")?, MAX_BLOB)?;
                if upload.offset + data.len() as u64 > upload.size {
                    return Err(Error::invalid("upload exceeds declared size"));
                }
                let mut f = OpenOptions::new().append(true).open(&upload.temp)?;
                f.write_all(&data)?;
                upload.offset += data.len() as u64;
                Ok(json!({"nextOffset":upload.offset}))
            }
            "files.upload.commit" => {
                let mut uploads = self.uploads.lock().unwrap();
                let id = workspace::required(a, "uploadId")?;
                let upload = uploads
                    .get(id)
                    .ok_or_else(|| Error::business("not_found", "upload not found"))?;
                if upload.offset != upload.size {
                    return Err(Error::business(
                        "incomplete",
                        "upload has not reached declared size",
                    ));
                }
                let mut w = self.workspace.lock().unwrap();
                File::open(&upload.temp)?.sync_all()?;
                let final_path = if let Some((directory, name)) = &upload.unique {
                    imports::publish(&w.root, &upload.temp, directory, name)?
                } else {
                    let p = storage::path(&w.root, &upload.path, false)?;
                    fs::create_dir_all(p.parent().unwrap())?;
                    if let Err(e) = storage::publish_new(&upload.temp, &p) {
                        return Ok(json!({"kind":"failed","message":e.to_string()}));
                    }
                    File::open(p.parent().unwrap())?.sync_all()?;
                    upload.path.clone()
                };
                File::open(upload.temp.parent().unwrap())?.sync_all()?;
                uploads.remove(id);
                let mut candidate = w.clone();
                candidate.refresh()?;
                w.commit(candidate)?;
                self.changed.notify_all();
                Ok(json!({"kind":"done","path":final_path}))
            }
            "files.upload.cancel" => {
                if let Some(u) = self
                    .uploads
                    .lock()
                    .unwrap()
                    .remove(workspace::required(a, "uploadId")?)
                {
                    fs::remove_file(u.temp)?;
                }
                Ok(json!({"cancelled":true}))
            }
            "documents.read" | "documents.write" | "documents.quarantine" => {
                let namespace = storage::identifier(workspace::required(a, "namespace")?)?;
                let key = storage::identifier(workspace::required(a, "key")?)?;
                let mut w = self.workspace.lock().unwrap();
                if let Some(service) = namespace.strip_prefix("services.") {
                    storage::identifier(service)?;
                    return self.service_document(&mut w, method, service, key, a);
                }
                let path = w
                    .root
                    .join(".workspace/documents")
                    .join(namespace)
                    .join(format!("{key}.json"));
                if method == "documents.quarantine" {
                    if path.exists() {
                        fs::rename(
                            path,
                            w.root
                                .join(".workspace/corrupt")
                                .join(format!("{}-{namespace}-{key}.json", uuid::Uuid::new_v4())),
                        )?;
                    }
                    let next = w.clone();
                    w.commit(next)?;
                    self.changed.notify_all();
                    return Ok(json!({"revision":w.revision}));
                }
                let old = if path.exists() {
                    let old = storage::read_json(&path)?;
                    if old["format"] != 1
                        || !old["revision"].is_u64()
                        || (!old["document"].is_string() && !old["document"].is_null())
                    {
                        return Err(Error::business(
                            "unsupported_format",
                            "invalid opaque document envelope",
                        ));
                    }
                    old
                } else {
                    json!({"format":1,"revision":0,"document":null})
                };
                if method == "documents.read" {
                    return Ok(json!({"document":old["document"],"revision":old["revision"]}));
                }
                if let Some(expected) = a.get("expectedRevision") {
                    if *expected != old["revision"] {
                        return Err(Error::business("conflict", "document revision changed"));
                    }
                }
                let document = workspace::required(a, "document")?;
                if document.len() > 16 * 1024 * 1024 {
                    return Err(Error::invalid("document exceeds 16 MiB"));
                }
                let revision = old["revision"]
                    .as_u64()
                    .unwrap()
                    .checked_add(1)
                    .ok_or_else(|| Error::business("overflow", "document revision overflow"))?;
                storage::write_json(
                    &path,
                    &json!({"format":1,"revision":revision,"document":document}),
                )?;
                let next = w.clone();
                w.commit(next)?;
                self.changed.notify_all();
                Ok(json!({"revision":revision}))
            }
            "services.executor.register" | "services.executor.retire" | "services.command" | "services.complete" => {
                let mut w = self.workspace.lock().unwrap();
                if !w.writable { return Err(Error::business("read_only", "workspace is read only")); }
                let result = local_services::call(&mut w, method, a, &mut |w, m, key, args| self.service_document(w, m, "proxy", key, args));
                self.changed.notify_all();
                result
            }
            "services.report" if a["serviceId"] == "proxy" => {
                let mut w = self.workspace.lock().unwrap();
                let result = local_services::call(&mut w, method, a, &mut |w, m, key, args| self.service_document(w, m, "proxy", key, args));
                self.changed.notify_all();
                result
            }
            "services.report" => {
                let id = storage::identifier(workspace::required(a, "serviceId")?)?;
                if !a["state"].is_object() {
                    return Err(Error::invalid("service state must be object"));
                }
                let mut w = self.workspace.lock().unwrap();
                if id == "network" {
                    let dns = a["state"]["dnsServers"]
                        .as_array()
                        .ok_or_else(|| Error::invalid("network.dnsServers must be an array"))?;
                    if !a["state"]["connected"].is_boolean() || dns.len() > 16 {
                        return Err(Error::invalid("invalid network report"));
                    }
                    let mut text = String::new();
                    for value in dns {
                        let address = value
                            .as_str()
                            .and_then(|v| v.parse::<std::net::IpAddr>().ok())
                            .ok_or_else(|| {
                                Error::invalid("network.dnsServers contains an invalid IP address")
                            })?;
                        text.push_str(&format!("nameserver {address}\n"));
                    }
                    let resolver = w.root.join(".workspace/environment/network/resolv.conf");
                    storage::atomic(&resolver, text.as_bytes())?;
                    use std::os::unix::fs::PermissionsExt;
                    fs::set_permissions(resolver, fs::Permissions::from_mode(0o644))?;
                }
                storage::write_json(
                    &w.root
                        .join(".workspace/state/services")
                        .join(format!("{id}.json")),
                    &json!({"format":1,"state":a["state"],"reportedAt":storage::now()}),
                )?;
                let next = w.clone();
                w.commit(next)?;
                self.changed.notify_all();
                Ok(json!({"revision":w.revision,"serviceId":id,"state":a["state"]}))
            }
            "services.status" => {
                let w = self.workspace.lock().unwrap();
                let mut states = serde_json::Map::new();
                for e in fs::read_dir(w.root.join(".workspace/state/services"))? {
                    let e = e?;
                    let name = e.file_name().to_string_lossy().into_owned();
                    if let Some(id) = name.strip_suffix(".json") {
                        states.insert(id.into(), storage::read_json(&e.path())?);
                    }
                }
                Ok(json!({"services":states,"desired":w.state["config"]["services"],"control":{"proxy":local_services::projection(&w)?}}))
            }
            "client.config" => {
                let _ = storage::identifier(workspace::required(a, "clientId")?)?;
                let w = self.workspace.lock().unwrap();
                let c = &w.state["config"];
                Ok(
                    json!({"revision":w.revision,"config":{"appearance":c["appearance"],"overlay":c["overlay"],"launcher":c["launcher"],"terminal":c["terminal"]}}),
                )
            }
            "environment.tools.status" => tools::status(&self.environment),
            "environment.tools.install" => tools::install(
                &self.environment,
                self.processes.clone(),
                workspace::required(a, "toolId")?,
                a["retry"] == true,
            ),
            "terminal.create" | "terminal.attach" | "terminal.restart" => {
                let _execution = self.execution.lock().unwrap();
                self.terminals
                    .call(&self.processes, &self.environment, method, a)
            }
            "terminal.status" | "terminal.read" | "terminal.write" | "terminal.resize"
            | "terminal.stop" | "terminal.wait" | "terminal.rename" | "terminal.clear" => self
                .terminals
                .call(&self.processes, &self.environment, method, a),
            "environment.status" => Ok(self.environment.lock().unwrap().status()),
            "environment.reconcile" => {
                environment::Environment::reconcile(&self.environment, a["retry"] == true)
            }
            "environment.restart" => {
                let _execution = self.execution.lock().unwrap();
                self.terminals.refresh(&self.processes)?;
                let due = self.terminals.running();
                let mut environment = self.environment.lock().unwrap();
                if !environment.has_verified_pending()? {
                    return Ok(environment.status());
                }
                {
                    self.chat.stop();
                    self.processes.stop_all();
                    let deadline = Instant::now() + Duration::from_secs(10);
                    while self.processes.running.load(Ordering::SeqCst) > 0
                        && Instant::now() < deadline
                    {
                        std::thread::sleep(Duration::from_millis(20));
                    }
                    if self.processes.running.load(Ordering::SeqCst) > 0 {
                        return Err(Error::business(
                            "stop_timeout",
                            "environment processes did not exit before restart deadline",
                        ));
                    }
                }
                let result = environment.restart();
                drop(environment);
                self.terminals
                    .restore(&due, &self.processes, &self.environment);
                result
            }
            "process.spawn" => self.processes.spawn(&self.environment, a),
            "process.read" | "process.write" | "process.resize" | "process.stop"
            | "process.wait" => self.processes.request(method, a),
            _ => Err(Error::method()),
        }
    }
}
fn serve(options: environment::Options) -> Result<()> {
    fs::create_dir_all(options.root.join(".workspace"))?;
    let lock = OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(options.root.join(".workspace/engine.lock"))?;
    if unsafe { libc::flock(lock.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } != 0 {
        return Err(Error::business(
            "busy",
            "another engine owns this workspace",
        ));
    }
    let workspace = workspace::Workspace::load(options.root.clone())?;
    let terminals = terminal::Terminals::load(&options.root)?;
    let running = Arc::new(AtomicUsize::new(0));
    let environment = Arc::new(Mutex::new(environment::Environment::load(
        options,
        running.clone(),
    )?));
    let server = Arc::new(Server {
        workspace: Mutex::new(workspace),
        changed: Condvar::new(),
        environment,
        processes: Arc::new(process::Processes::new(running)),
        chat: chat::Chat::default(),
        terminals,
        execution: Mutex::new(()),
        uploads: Mutex::new(HashMap::new()),
        closed: AtomicBool::new(false),
        workers: AtomicUsize::new(0),
    });
    let writer = Arc::new(Mutex::new(io::stdout()));
    let mut reader = io::BufReader::new(io::stdin());
    let mut hello = false;
    let watcher = server.clone();
    std::thread::spawn(move || {
        let mut maintenance = Instant::now();
        while !watcher.closed.load(Ordering::SeqCst) {
            std::thread::sleep(Duration::from_millis(750));
            if let Ok(mut w) = watcher.workspace.lock() {
                if !w.writable {
                    continue;
                }
                let mut next = w.clone();
                let result = if maintenance.elapsed() >= Duration::from_secs(3600) {
                    maintenance = Instant::now();
                    next.maintenance()
                } else {
                    next.refresh()
                };
                if result.is_ok() && next.state != w.state {
                    let _ = w.commit(next);
                    watcher.changed.notify_all();
                }
            }
            let references = terminal::references(&watcher.workspace.lock().unwrap().state);
            {
                let _execution = watcher.execution.lock().unwrap();
                let _ = watcher.terminals.reap(&references, &watcher.processes);
            }
            environment::Environment::reconcile_changed(&watcher.environment);
        }
    });
    loop {
        let bytes = match line(&mut reader) {
            Ok(Some(b)) => b,
            Ok(None) => break,
            Err(_) => {
                response(
                    &writer,
                    V::Null,
                    Err(Error {
                        code: -32700,
                        kind: "frame_too_large".into(),
                        message: "JSON frame exceeds limit".into(),
                    }),
                );
                break;
            }
        };
        let request = match strict_json(&bytes) {
            Ok(v) => v,
            Err(_) => {
                response(
                    &writer,
                    V::Null,
                    Err(Error {
                        code: -32700,
                        kind: "parse_error".into(),
                        message: "Invalid JSON".into(),
                    }),
                );
                if !hello {
                    break;
                }
                continue;
            }
        };
        // Preserve number IDs as raw JSON, including integers beyond f64/u64 precision.
        let raw_fields =
            serde_json::from_slice::<HashMap<String, &serde_json::value::RawValue>>(&bytes).ok();
        let id = raw_fields
            .as_ref()
            .and_then(|fields| fields.get("id"))
            .map(|raw| RpcId(raw.get().to_owned()));
        if !request.is_object()
            || request["jsonrpc"] != "2.0"
            || !request["method"].is_string()
            || request
                .get("id")
                .is_some_and(|v| !v.is_null() && !v.is_string() && !v.is_number())
            || request.get("params").is_some_and(|v| !v.is_object())
        {
            response(
                &writer,
                id.unwrap_or_else(RpcId::null),
                Err(Error {
                    code: -32600,
                    kind: "invalid_request".into(),
                    message: "Invalid JSON-RPC request".into(),
                }),
            );
            if !hello {
                break;
            }
            continue;
        }
        let method = request["method"].as_str().unwrap().to_owned();
        let params = request.get("params").cloned().unwrap_or(json!({}));
        if !hello {
            if method != "hello"
                || params["protocol"] != PROTOCOL
                || params["clientId"]
                    .as_str()
                    .is_none_or(|s| s.trim().is_empty())
                || id.is_none()
            {
                response(
                    &writer,
                    id.unwrap_or_else(RpcId::null),
                    Err(Error::business(
                        "protocol_mismatch",
                        "first request must be hello with exact workflow.workspace/1 protocol and clientId",
                    )),
                );
                break;
            }
            hello = true;
            let w = server.workspace.lock().unwrap();
            response(
                &writer,
                id.unwrap(),
                Ok(
                    json!({"protocol":PROTOCOL,"engineVersion":"1.0.0","workspaceRoot":w.root,"capabilities":{"workspace":true,"files":true,"documents":true,"environment":true,"processes":true,"pty":true,"services":true,"chat":true,"maxFrameBytes":MAX_FRAME,"maxBlobChunkBytes":MAX_BLOB}}),
                ),
            );
            continue;
        }
        if method == "hello" {
            if let Some(id) = id {
                response(
                    &writer,
                    id,
                    Err(Error::business(
                        "already_initialized",
                        "hello has already completed",
                    )),
                );
            }
            continue;
        }
        let concurrent = matches!(
            method.as_str(),
            "environment.tools.status"
                | "environment.tools.install"
                | "terminal.create"
                | "terminal.attach"
                | "terminal.restart"
                | "terminal.read"
                | "terminal.wait"
                | "terminal.write"
                | "chat.snapshot"
                | "chat.watch"
                | "chat.command"
                | "workspace.watch"
                | "process.read"
                | "process.wait"
                | "process.write"
                | "process.spawn"
                | "environment.status"
                | "environment.reconcile"
                | "environment.restart"
        );
        if concurrent {
            if server.workers.fetch_add(1, Ordering::SeqCst) >= 128 {
                server.workers.fetch_sub(1, Ordering::SeqCst);
                if let Some(id) = id {
                    response(
                        &writer,
                        id,
                        Err(Error::business("busy", "too many pending requests")),
                    );
                }
                continue;
            }
            let server = server.clone();
            let writer = writer.clone();
            std::thread::spawn(move || {
                let result = server.call(&method, &params);
                if let Some(id) = id {
                    response(&writer, id, result);
                }
                server.workers.fetch_sub(1, Ordering::SeqCst);
            });
        } else {
            let result = server.call(&method, &params);
            if let Some(id) = id {
                response(&writer, id, result);
            }
        }
    }
    server.closed.store(true, Ordering::SeqCst);
    server.changed.notify_all();
    server.chat.stop();
    server.processes.stop_all();
    for (_, u) in server.uploads.lock().unwrap().drain() {
        let _ = fs::remove_file(u.temp);
    }
    Ok(())
}
fn options() -> Result<environment::Options> {
    let mut args = std::env::args().skip(1);
    if args.next().as_deref() != Some("serve") {
        return Err(Error::invalid(
            "usage: workflow-engine serve --root DIR [--runtime FILE --loader FILE --apk APK --native-dir DIR | --image FILE --image-index FILE]",
        ));
    }
    let mut o = environment::Options::default();
    let mut seen = HashSet::new();
    while let Some(flag) = args.next() {
        if !seen.insert(flag.clone()) {
            return Err(Error::invalid("duplicate CLI flag"));
        }
        let value = args
            .next()
            .ok_or_else(|| Error::invalid("missing CLI argument"))?;
        match flag.as_str() {
            "--root" => o.root = PathBuf::from(value),
            "--runtime" => o.runtime = Some(value.into()),
            "--loader" => o.loader = Some(value.into()),
            "--apk" => o.apk = Some(value.into()),
            "--native-dir" => o.native_dir = Some(value.into()),
            "--tools" => o.tools = Some(value.into()),
            "--image" => o.image = Some(value.into()),
            "--image-index" => o.image_index = Some(value.into()),
            _ => return Err(Error::invalid("unknown CLI option")),
        }
    }
    if o.root.as_os_str().is_empty() {
        return Err(Error::invalid("--root is required"));
    }
    fs::create_dir_all(&o.root)?;
    o.root = o.root.canonicalize()?;
    for path in [
        &mut o.runtime,
        &mut o.loader,
        &mut o.apk,
        &mut o.native_dir,
        &mut o.tools,
        &mut o.image,
        &mut o.image_index,
    ]
    .into_iter()
    .flatten()
    {
        *path = path.canonicalize()?;
    }
    if o.image.is_some() != o.image_index.is_some() {
        return Err(Error::invalid(
            "--image and --image-index must be provided together",
        ));
    }
    Ok(o)
}
fn main() {
    if let Err(e) = options().and_then(serve) {
        eprintln!("workflow-engine: {}", e.message);
        std::process::exit(78);
    }
}
#[cfg(test)]
mod tests;
