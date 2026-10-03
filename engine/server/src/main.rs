mod chat;
mod contracts;
mod local_services;
mod protocol;
mod state;
mod transport;
use contracts::{Call, Reply};
use protocol::*;
use std::{
    collections::{BTreeMap, HashSet},
    path::PathBuf,
    sync::{
        Arc, Condvar, Mutex,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    time::{Duration, Instant},
};
use workflow_environment::{
    self as environment, Store, persist, runtime::Processes, store::keys, tools,
};
use workflow_filework::Uploads;
use workflow_terminal::Terminals;
struct Server {
    workspace: Mutex<state::Workspace>,
    changed: Condvar,
    environment: Arc<Mutex<environment::Environment>>,
    processes: Arc<Processes>,
    chat: chat::Chat,
    terminals: Terminals,
    execution: Mutex<()>,
    uploads: Mutex<Uploads>,
    closed: AtomicBool,
    workers: AtomicUsize,
}
struct ServiceContext<'a> {
    workspace: &'a mut state::Workspace,
    changed: &'a Condvar,
}
impl local_services::ServicesContext for ServiceContext<'_> {
    fn store(&self) -> &Store {
        &self.workspace.store
    }
    fn writable(&self) -> bool {
        self.workspace.writable
    }
    fn commit(&mut self) -> environment::Result<u64> {
        self.workspace.commit(self.workspace.clone())?;
        self.changed.notify_all();
        Ok(self.workspace.revision)
    }
    fn document(
        &mut self,
        request: local_services::DocumentRequest,
    ) -> environment::Result<local_services::Document> {
        let (op, key, document, expected_revision) = match request {
            local_services::DocumentRequest::Read { key } => {
                (contracts::documents::Operation::Read, key, None, None)
            }
            local_services::DocumentRequest::Write {
                key,
                document,
                expected_revision,
            } => (
                contracts::documents::Operation::Write,
                key,
                Some(document),
                Some(expected_revision),
            ),
        };
        let a = DocumentParams {
            namespace: "services.proxy".into(),
            key: key.into(),
            document,
            expected_revision,
            extra: Default::default(),
        };
        let result =
            contracts::documents::call(&self.workspace.store, op, &a, self.workspace.writable)?;
        if result.changed {
            let mut next = self.workspace.clone();
            next.refresh()?;
            self.workspace.commit(next)?;
            self.changed.notify_all();
        }
        Ok(local_services::Document {
            document: result.document.document,
            revision: result.document.revision,
        })
    }
}
impl Server {
    fn hello(&self) -> HelloResult {
        HelloResult::new(&self.workspace.lock().unwrap().root)
    }
    fn chat(self: &Arc<Self>, method: &str, params: &OpaqueObject) -> Result<Reply> {
        if !self.environment.lock().unwrap().status().usable {
            environment::Environment::reconcile(&self.environment, false)?;
            return Err(Error::business(
                "environment_preparing",
                "chat requires a ready environment",
            ));
        }
        let weak = Arc::downgrade(self);
        let host = {
            let _execution = self.execution.lock().unwrap();
            if self.closed.load(Ordering::SeqCst) {
                return Err(Error::business("closed", "workspace stopped"));
            }
            self.chat.ensure(
                &self.environment,
                Arc::new(move |method, params| {
                    let server = weak
                        .upgrade()
                        .ok_or_else(|| Error::business("closed", "workspace stopped"))?;
                    if server.closed.load(Ordering::SeqCst) {
                        return Err(Error::business("closed", "workspace stopped"));
                    }
                    if method == "hello" {
                        let p: HelloParams = protocol::params(params)?;
                        if p.protocol != PROTOCOL || p.client_id.is_empty() {
                            return Err(Error::business(
                                "protocol_mismatch",
                                "invalid internal chat handshake",
                            ));
                        }
                        return opaque(&server.hello());
                    }
                    opaque(&server.call(method, params)?)
                }),
            )?
        };
        Ok(Reply::Chat(host.request(method, params)?))
    }
    fn document(&self, op: contracts::documents::Operation, a: DocumentParams) -> Result<Reply> {
        let mut w = self.workspace.lock().unwrap();
        let result = contracts::documents::call(&w.store, op, &a, w.writable)?;
        if result.changed {
            let mut next = w.clone();
            next.refresh()?;
            w.commit(next)?;
            self.changed.notify_all();
        }
        if matches!(op, contracts::documents::Operation::Read) {
            Ok(Reply::Document(result.document))
        } else {
            let revision = if matches!(op, contracts::documents::Operation::Quarantine)
                && !a.namespace.starts_with("services.")
            {
                w.revision
            } else {
                result.document.revision
            };
            Ok(Reply::Revision(Revision { revision }))
        }
    }
    fn service(&self, request: local_services::Request) -> Result<Reply> {
        let mut w = self.workspace.lock().unwrap();
        let report = matches!(&request, local_services::Request::Report(_));
        let result = local_services::call(
            &mut ServiceContext {
                workspace: &mut w,
                changed: &self.changed,
            },
            request,
        )?;
        self.changed.notify_all();
        Ok(if report {
            Reply::ServiceReport(contracts::ServiceReportReply::Proxy(result))
        } else {
            Reply::Service(result)
        })
    }
    fn call(self: &Arc<Self>, method: &str, params: &OpaqueObject) -> Result<Reply> {
        let request = Call::decode(method, params)?;
        if request.requires_writable() && !self.workspace.lock().unwrap().writable {
            return Err(Error::business(
                "read_only",
                "workspace schema is unsupported",
            ));
        }
        use Call::*;
        Ok(match request {
            Hello(_) => {
                return Err(Error::business(
                    "already_initialized",
                    "hello has already completed",
                ));
            }
            ChatSnapshot(_) | ChatWatch(_) | ChatCommand(_) => return self.chat(method, params),
            Snapshot(_) => Reply::Snapshot(self.workspace.lock().unwrap().snapshot()),
            Watch(a) => {
                if a.timeout_ms > 30000 {
                    return Err(Error::invalid("timeoutMs exceeds 30000"));
                }
                let deadline = Instant::now() + Duration::from_millis(a.timeout_ms);
                let mut w = self.workspace.lock().unwrap();
                while w.revision <= a.after_revision
                    && Instant::now() < deadline
                    && !self.closed.load(Ordering::SeqCst)
                {
                    w = self
                        .changed
                        .wait_timeout(w, deadline.saturating_duration_since(Instant::now()))
                        .unwrap()
                        .0;
                }
                Reply::Snapshot(w.snapshot())
            }
            Command(command) => {
                let result = self.workspace.lock().unwrap().command_typed(command)?;
                self.changed.notify_all();
                Reply::Command(result)
            }
            ReadFile(a) => {
                let w = self.workspace.lock().unwrap();
                let data = w.filework.read_chunk(&a.path, a.offset, a.length)?;
                Reply::Blob(BlobResult {
                    data: encode_blob(&data.data),
                    next_offset: data.next_offset,
                    eof: data.eof,
                    size: data.size,
                })
            }
            BeginUpload(a) => {
                let w = self.workspace.lock().unwrap();
                Reply::UploadStarted(w.filework.begin_upload(
                    &mut self.uploads.lock().unwrap(),
                    a.destination,
                    a.size,
                )?)
            }
            UploadChunk(a) => {
                let bytes = decode_blob(&a.data, MAX_BLOB)?;
                let w = self.workspace.lock().unwrap();
                Reply::UploadProgress(w.filework.upload_chunk(
                    &mut self.uploads.lock().unwrap(),
                    &a.upload_id,
                    a.offset,
                    &bytes,
                )?)
            }
            CommitUpload(a) => {
                let mut w = self.workspace.lock().unwrap();
                let result = w
                    .filework
                    .commit_upload(&mut self.uploads.lock().unwrap(), &a.upload_id)?;
                if matches!(result, workflow_filework::UploadOutcome::Done { .. }) {
                    let mut next = w.clone();
                    next.refresh()?;
                    w.commit(next)?;
                    self.changed.notify_all();
                }
                Reply::UploadOutcome(result)
            }
            CancelUpload(a) => {
                let w = self.workspace.lock().unwrap();
                w.filework
                    .cancel_upload(&mut self.uploads.lock().unwrap(), &a.upload_id);
                Reply::Cancel(CancelResult { cancelled: true })
            }
            ReadDocument(a) => return self.document(contracts::documents::Operation::Read, a),
            WriteDocument(a) => return self.document(contracts::documents::Operation::Write, a),
            QuarantineDocument(a) => {
                return self.document(contracts::documents::Operation::Quarantine, a);
            }
            RegisterExecutor(a) => return self.service(local_services::Request::Register(a)),
            RetireExecutor(a) => return self.service(local_services::Request::Retire(a)),
            ServiceCommand(a) => return self.service(local_services::Request::Command(a)),
            CompleteService(a) => return self.service(local_services::Request::Complete(a)),
            ServiceReport(ServiceReportParams::Proxy(a)) => {
                return self.service(local_services::Request::Report(
                    local_services::ReportRequest {
                        service_id: "proxy".into(),
                        epoch: a.epoch,
                        state: a.state,
                        extra: a.extra,
                    },
                ));
            }
            ServiceReport(a) => {
                let (id, state) = match a {
                    ServiceReportParams::Network(a) => {
                        ("network".to_string(), ReportedState::Network(a.state))
                    }
                    ServiceReportParams::Extension(a) => {
                        (a.service_id.0, ReportedState::Extension(a.state))
                    }
                    ServiceReportParams::Proxy(_) => unreachable!(),
                };
                let mut w = self.workspace.lock().unwrap();
                if let ReportedState::Network(network) = &state {
                    if network.dns_servers.len() > 16 {
                        return Err(Error::invalid("invalid network report"));
                    }
                    let addresses = network
                        .dns_servers
                        .iter()
                        .map(|v| {
                            v.parse::<std::net::IpAddr>().map_err(|_| {
                                Error::invalid("network.dnsServers contains an invalid IP address")
                            })
                        })
                        .collect::<Result<Vec<_>>>()?;
                    w.store.write_resolver(&addresses)?;
                }
                w.store.write(
                    &keys::service_report(&id)?,
                    &protocol::ServiceReport {
                        format: 1,
                        state: state.clone(),
                        reported_at: persist::now(),
                        epoch: None,
                        extra: Default::default(),
                    },
                )?;
                let next = w.clone();
                w.commit(next)?;
                self.changed.notify_all();
                Reply::ServiceReport(contracts::ServiceReportReply::Other(ServiceReportResult {
                    revision: w.revision,
                    service_id: id,
                    state,
                }))
            }
            ServicesStatus(_) => {
                let w = self.workspace.lock().unwrap();
                let mut services = BTreeMap::new();
                for key in w.store.list(keys::SERVICE_REPORTS)? {
                    if let Some(name) = key.rsplit('/').next().and_then(|n| n.strip_suffix(".json"))
                    {
                        let record = match name {
                            "network" => w.store.read(&key)?.map(StoredReport::Network),
                            "proxy" => w.store.read(&key)?.map(StoredReport::Proxy),
                            _ => w.store.read(&key)?.map(StoredReport::Extension),
                        };
                        if let Some(record) = record {
                            services.insert(name.into(), record);
                        }
                    }
                }
                Reply::Services(ServicesResult {
                    services,
                    desired: w.services_config(),
                    control: ServiceControl {
                        proxy: local_services::projection(&w.store)?,
                    },
                })
            }
            ClientConfig(a) => {
                persist::identifier(&a.client_id)?;
                Reply::ClientConfig(self.workspace.lock().unwrap().client_config())
            }
            ToolsStatus(_) => Reply::Tools(tools::status(&self.environment)?),
            InstallTool(a) => Reply::Tools(tools::install(
                &self.environment,
                self.processes.clone(),
                &a.tool_id,
                a.retry,
            )?),
            EnvironmentStatus(_) => Reply::Environment(self.environment.lock().unwrap().status()),
            Reconcile(a) => Reply::Environment(environment::Environment::reconcile(
                &self.environment,
                a.retry,
            )?),
            RestartEnvironment(_) => {
                let _execution = self.execution.lock().unwrap();
                self.terminals.refresh(&self.processes)?;
                let due = self.terminals.running();
                let mut environment = self.environment.lock().unwrap();
                if !environment.has_verified_pending()? {
                    return Ok(Reply::Environment(environment.status()));
                }
                self.chat.stop();
                self.processes.stop_all();
                let deadline = Instant::now() + Duration::from_secs(10);
                while self.processes.running.load(Ordering::SeqCst) > 0 && Instant::now() < deadline
                {
                    std::thread::sleep(Duration::from_millis(20));
                }
                if self.processes.running.load(Ordering::SeqCst) > 0 {
                    return Err(Error::business(
                        "stop_timeout",
                        "environment processes did not exit before restart deadline",
                    ));
                }
                let result = environment.restart();
                drop(environment);
                self.terminals
                    .restore(&due, &self.processes, &self.environment);
                Reply::Environment(result?)
            }
            Spawn(a) => {
                let _execution = self.execution.lock().unwrap();
                Reply::Spawned(self.processes.spawn(&self.environment, &a)?)
            }
            ReadProcess(a) => Reply::ProcessRead(
                self.processes
                    .read(
                        &a.process_id,
                        a.stream,
                        a.offset,
                        a.max_bytes.min(MAX_BLOB),
                        a.wait_ms,
                    )?
                    .into(),
            ),
            WriteProcess(a) => Reply::Written(WrittenResult {
                written: self
                    .processes
                    .write(&a.process_id, &decode_blob(&a.data, MAX_BLOB)?)?,
            }),
            ResizeProcess(a) => {
                Reply::Dimensions(self.processes.resize(&a.process_id, a.rows, a.columns)?)
            }
            StopProcess(a) => {
                self.processes.stop(&a.process_id, a.force)?;
                Reply::Stopping(StoppingResult { stopping: true })
            }
            WaitProcess(a) => Reply::Wait(self.processes.wait(&a.process_id, a.timeout_ms)?),
            CreateTerminal(a) => {
                let _execution = self.execution.lock().unwrap();
                Reply::Terminal(
                    self.terminals
                        .create(&self.processes, &self.environment, &a)?,
                )
            }
            AttachTerminal(a) => {
                let _execution = self.execution.lock().unwrap();
                Reply::Terminal(self.terminals.attach(
                    &self.processes,
                    &self.environment,
                    &a.terminal_id,
                    &a.options,
                )?)
            }
            RestartTerminal(a) => {
                let _execution = self.execution.lock().unwrap();
                Reply::Terminal(self.terminals.restart(
                    &self.processes,
                    &self.environment,
                    &a.terminal_id,
                    &a.options,
                )?)
            }
            ResolveTerminalPaths(a) => {
                if a.candidates.len() > 128 || a.candidates.iter().any(|s| s.len() > 4096) {
                    return Err(Error::invalid("terminal path batch exceeds limit"));
                }
                self.terminals.refresh(&self.processes)?;
                let context = self
                    .terminals
                    .resolution_context(&a.terminal_id, a.generation)?;
                let w = self.workspace.lock().unwrap();
                let paths = a
                    .candidates
                    .iter()
                    .map(|text| {
                        let mut result = workflow_terminal::ResolvedPath::rejected(text);
                        for (path, line, column) in
                            workflow_terminal::candidates(&context.cwd, text)
                        {
                            if let Ok(Some(kind)) = w.filework.existing_path_kind(&path) {
                                result.path = Some(path);
                                result.kind = Some(match kind {
                                    workflow_filework::ExistingPathKind::Directory => {
                                        workflow_terminal::PathKind::Directory
                                    }
                                    workflow_filework::ExistingPathKind::File => {
                                        workflow_terminal::PathKind::File
                                    }
                                });
                                result.line = line;
                                result.column = column;
                                break;
                            }
                        }
                        result
                    })
                    .collect();
                drop(w);
                let current = self
                    .terminals
                    .resolution_context(&a.terminal_id, a.generation)?;
                if current.cwd != context.cwd {
                    return Err(Error::business(
                        "stale_terminal",
                        "terminal working directory changed",
                    ));
                }
                Reply::TerminalPaths(workflow_terminal::ResolvedPaths {
                    terminal_id: a.terminal_id,
                    generation: a.generation,
                    cwd: context.cwd,
                    paths,
                })
            }
            TerminalStatus(a) => Reply::Terminal(self.terminals.status(&a.terminal_id)?),
            RenameTerminal(a) => {
                Reply::Terminal(self.terminals.rename(&a.terminal_id, a.title.as_deref())?)
            }
            ClearTerminal(a) => {
                Reply::Terminal(self.terminals.clear(&self.processes, &a.terminal_id)?)
            }
            ReadTerminal(a) => Reply::TerminalRead(
                self.terminals
                    .read(
                        &self.processes,
                        &a.terminal_id,
                        &workflow_terminal::ReadOptions {
                            generation: a.generation,
                            offset: a.offset,
                            max_bytes: a.max_bytes,
                            wait_ms: a.wait_ms,
                            extra: a.extra,
                        },
                    )?
                    .into(),
            ),
            WriteTerminal(a) => Reply::Written(WrittenResult {
                written: self.terminals.write(
                    &self.processes,
                    &a.terminal_id,
                    &decode_blob(&a.data, MAX_BLOB)?,
                )?,
            }),
            ResizeTerminal(a) => Reply::Dimensions(self.terminals.resize(
                &self.processes,
                &a.terminal_id,
                a.rows,
                a.columns,
            )?),
            StopTerminal(a) => {
                self.terminals
                    .stop(&self.processes, &a.terminal_id, a.force)?;
                Reply::Stopping(StoppingResult { stopping: true })
            }
            WaitTerminal(a) => Reply::Wait(self.terminals.wait(
                &self.processes,
                &a.terminal_id,
                a.timeout_ms,
            )?),
        })
    }
}
fn serve(options: environment::Options) -> Result<()> {
    let store = Store::open(&options.root)?;
    let _lock = store.lock()?;
    let workspace = state::Workspace::load(options.root.clone())?;
    let terminals = Terminals::load(&options.root)?;
    let running = Arc::new(AtomicUsize::new(0));
    let environment = Arc::new(Mutex::new(environment::Environment::load(
        options,
        running.clone(),
    )?));
    let server = Arc::new(Server {
        workspace: Mutex::new(workspace),
        changed: Condvar::new(),
        environment,
        processes: Arc::new(Processes::new(running)),
        chat: chat::Chat::default(),
        terminals,
        execution: Mutex::new(()),
        uploads: Mutex::new(Uploads::default()),
        closed: AtomicBool::new(false),
        workers: AtomicUsize::new(0),
    });
    let watcher = server.clone();
    let monitor = std::thread::spawn(move || {
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
            let references = watcher.workspace.lock().unwrap().terminal_references();
            {
                let _execution = watcher.execution.lock().unwrap();
                let _ = watcher.terminals.reap(&references, &watcher.processes);
            }
            environment::Environment::reconcile_changed(&watcher.environment);
        }
    });
    let result = transport::run(&server);
    server.closed.store(true, Ordering::SeqCst);
    server.changed.notify_all();
    {
        let _execution = server.execution.lock().unwrap();
        server.chat.stop();
        server.processes.stop_all();
    }
    *server.uploads.lock().unwrap() = Uploads::default();
    let _ = monitor.join();
    result
}
fn options(mut args: impl Iterator<Item = String>) -> Result<environment::Options> {
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
            "--tools" => o.tools = Some(value.into()),
            "--image" => o.image = Some(value.into()),
            "--image-index" => o.image_index = Some(value.into()),
            _ => return Err(Error::invalid("unknown CLI option")),
        }
    }
    Ok(o.prepare()?)
}
fn run() -> Result<()> {
    let mut args = std::env::args().skip(1);
    match args.next().as_deref() {
        Some("serve") => serve(options(args)?),
        Some("export-contract") => {
            if args.next().as_deref() != Some("--out") {
                return Err(Error::invalid("export-contract requires --out DIR"));
            }
            let out = args
                .next()
                .ok_or_else(|| Error::invalid("missing export directory"))?;
            if args.next().is_some() {
                return Err(Error::invalid("unexpected export argument"));
            }
            contracts::export(std::path::Path::new(&out))
        }
        _ => Err(Error::invalid(
            "usage: workflow-engine serve --root DIR [--runtime FILE --loader FILE --apk APK | --image FILE --image-index FILE]",
        )),
    }
}
fn main() {
    if let Err(error) = run() {
        eprintln!("workflow-engine: {}", error.message);
        std::process::exit(78);
    }
}
#[cfg(test)]
mod tests;
