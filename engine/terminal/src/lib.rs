//! Engine-owned terminal resources. View attachment never owns process lifetime.
//! Persistence and PTY operations go exclusively through workflow-environment.
mod model;
mod path;

pub use model::{Metadata, ReadOptions, ReadResult, StartOptions, Status};
pub use path::{resolve_workspace_path, working_directory};

use model::Saved;
use std::{
    collections::{HashMap, HashSet},
    path::Path,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use workflow_environment::{
    Environment, Error, Result, Store,
    json::OpaqueObject,
    runtime::{Dimensions, OutputStream, Processes, ReadOutput, SpawnOptions, WaitStatus},
    store::keys::TERMINAL_STATE as STATE_KEY,
};

const REAP_GRACE: Duration = Duration::from_secs(15);

struct Terminal {
    metadata: Metadata,
    process: Option<String>,
    scan: u64,
    osc: Vec<u8>,
    unreferenced: Option<Instant>,
}
impl Terminal {
    fn new(metadata: Metadata) -> Self {
        Self {
            metadata,
            process: None,
            scan: 0,
            osc: Vec::new(),
            unreferenced: Some(Instant::now()),
        }
    }
    fn next_generation(&mut self) {
        self.scan = 0;
        self.osc.clear();
        self.metadata.generation += 1;
    }
}

pub struct Terminals {
    store: Store,
    items: Mutex<HashMap<String, Terminal>>,
    extra: OpaqueObject,
    failure: Option<Error>,
}
impl Terminals {
    /// Invalid terminal state blocks terminal mutations, without blocking workspace startup.
    pub fn load(root: &Path) -> Result<Self> {
        let store = Store::open(root)?;
        let loaded = (|| -> Result<_> {
            let saved = store.read::<Saved>(STATE_KEY)?.unwrap_or_default();
            if saved.format != 1 {
                return Err(Error::business(
                    "unsupported_format",
                    "unsupported terminal state",
                ));
            }
            let mut items = HashMap::new();
            for mut metadata in saved.terminals {
                workflow_environment::persist::identifier(&metadata.id)?;
                metadata.status = Status::Ended;
                metadata.exit_code = None;
                items.insert(metadata.id.clone(), Terminal::new(metadata));
            }
            Ok((items, saved.extra))
        })();
        let (items, extra, failure) = match loaded {
            Ok((items, extra)) => (items, extra, None),
            Err(error) => (HashMap::new(), OpaqueObject::default(), Some(error)),
        };
        Ok(Self {
            store,
            items: Mutex::new(items),
            extra,
            failure,
        })
    }
    fn ensure_loaded(&self) -> Result<()> {
        match &self.failure {
            Some(error) => Err(error.clone()),
            None => Ok(()),
        }
    }
    fn save(&self, items: &HashMap<String, Terminal>) -> Result<()> {
        self.store.write(
            STATE_KEY,
            &Saved {
                format: 1,
                terminals: items
                    .values()
                    .map(|terminal| terminal.metadata.clone())
                    .collect(),
                extra: self.extra.clone(),
            },
        )
    }
    pub fn create(
        &self,
        processes: &Processes,
        environment: &Arc<Mutex<Environment>>,
        options: &StartOptions,
    ) -> Result<Metadata> {
        self.ensure_loaded()?;
        let mut items = self.items.lock().unwrap();
        let id = format!("t-{}", uuid::Uuid::new_v4());
        let ordinal = items
            .values()
            .map(|t| t.metadata.ordinal)
            .max()
            .unwrap_or(0)
            + 1;
        let mut terminal = Terminal::new(Metadata::new(
            id.clone(),
            ordinal,
            working_directory(options.directory.as_deref().unwrap_or(""))?,
        ));
        start(&mut terminal, processes, environment, options)?;
        let metadata = terminal.metadata.clone();
        items.insert(id, terminal);
        self.save(&items)?;
        Ok(metadata)
    }
    pub fn attach(
        &self,
        processes: &Processes,
        environment: &Arc<Mutex<Environment>>,
        id: &str,
        options: &StartOptions,
    ) -> Result<Metadata> {
        self.mutate(id, |terminal| {
            if terminal.process.is_none()
                && terminal.metadata.status != Status::Failed
                && terminal.metadata.exit_code.is_none()
            {
                start(terminal, processes, environment, options)?;
            }
            Ok(())
        })
    }
    pub fn restart(
        &self,
        processes: &Processes,
        environment: &Arc<Mutex<Environment>>,
        id: &str,
        options: &StartOptions,
    ) -> Result<Metadata> {
        self.mutate(id, |terminal| {
            if let Some(process) = &terminal.process {
                processes.stop(process, true)?;
                if processes.wait(process, 10000)?.running {
                    return Err(Error::business("stop_timeout", "terminal did not stop"));
                }
            }
            start(terminal, processes, environment, options)
        })
    }
    pub fn status(&self, id: &str) -> Result<Metadata> {
        self.mutate(id, |_| Ok(()))
    }
    pub fn rename(&self, id: &str, title: Option<&str>) -> Result<Metadata> {
        self.mutate(id, |terminal| {
            terminal.metadata.custom_title = title
                .map(|s| s.trim().chars().take(80).collect::<String>())
                .filter(|s| !s.is_empty());
            Ok(())
        })
    }
    pub fn clear(&self, processes: &Processes, id: &str) -> Result<Metadata> {
        self.mutate(id, |terminal| {
            if let Some(process) = &terminal.process {
                processes.clear_output(process)?;
            }
            terminal.next_generation();
            Ok(())
        })
    }
    fn mutate(
        &self,
        id: &str,
        operation: impl FnOnce(&mut Terminal) -> Result<()>,
    ) -> Result<Metadata> {
        self.ensure_loaded()?;
        let mut items = self.items.lock().unwrap();
        let terminal = items.get_mut(id).ok_or_else(not_found)?;
        operation(terminal)?;
        let metadata = terminal.metadata.clone();
        self.save(&items)?;
        Ok(metadata)
    }
    fn with_process<T>(&self, id: &str, operation: impl FnOnce(&str) -> Result<T>) -> Result<T> {
        self.ensure_loaded()?;
        let items = self.items.lock().unwrap();
        let process = items
            .get(id)
            .ok_or_else(not_found)?
            .process
            .as_deref()
            .ok_or_else(|| Error::business("closed", "terminal has no process"))?;
        operation(process)
    }
    pub fn write(&self, processes: &Processes, id: &str, data: &[u8]) -> Result<usize> {
        self.with_process(id, |process| processes.write(process, data))
    }
    pub fn resize(
        &self,
        processes: &Processes,
        id: &str,
        rows: u16,
        columns: u16,
    ) -> Result<Dimensions> {
        self.ensure_loaded()?;
        let mut items = self.items.lock().unwrap();
        let terminal = items.get_mut(id).ok_or_else(not_found)?;
        let process = terminal
            .process
            .as_deref()
            .ok_or_else(|| Error::business("closed", "terminal has no process"))?;
        let dimensions = processes.resize(process, rows, columns)?;
        terminal.metadata.rows = dimensions.rows;
        terminal.metadata.columns = dimensions.columns;
        // Keep the existing behavior: process resize updates the projection immediately;
        // a later terminal-state save persists it with the rest of the metadata.
        Ok(dimensions)
    }
    pub fn stop(&self, processes: &Processes, id: &str, force: bool) -> Result<()> {
        self.with_process(id, |process| processes.stop(process, force))
    }
    pub fn wait(&self, processes: &Processes, id: &str, timeout_ms: u64) -> Result<WaitStatus> {
        self.ensure_loaded()?;
        self.refresh(processes)?;
        let (process, metadata) = self.snapshot(id)?;
        // Do not hold the resource registry while waiting for the process owner.
        match process {
            Some(process) => processes.wait(&process, timeout_ms),
            None => Ok(WaitStatus {
                running: false,
                exit_code: metadata.exit_code,
            }),
        }
    }
    pub fn read(
        &self,
        processes: &Processes,
        id: &str,
        options: &ReadOptions,
    ) -> Result<ReadResult> {
        self.ensure_loaded()?;
        self.refresh(processes)?;
        let (process, terminal) = self.snapshot(id)?;
        let reset = options.generation != Some(terminal.generation);
        let output = match process {
            Some(process) => processes.read(
                &process,
                OutputStream::Stdout,
                if reset { 0 } else { options.offset },
                options.max_bytes,
                options.wait_ms,
            )?,
            None => ReadOutput {
                data: Vec::new(),
                start_offset: 0,
                next_offset: 0,
                eof: true,
                exit_code: None,
            },
        };
        Ok(ReadResult {
            data: output.data,
            start_offset: output.start_offset,
            next_offset: output.next_offset,
            eof: output.eof,
            exit_code: output.exit_code,
            reset,
            terminal,
        })
    }
    fn snapshot(&self, id: &str) -> Result<(Option<String>, Metadata)> {
        let items = self.items.lock().unwrap();
        let terminal = items.get(id).ok_or_else(not_found)?;
        Ok((terminal.process.clone(), terminal.metadata.clone()))
    }
    pub fn refresh(&self, processes: &Processes) -> Result<()> {
        if self.failure.is_some() {
            return Ok(());
        }
        let mut items = self.items.lock().unwrap();
        let mut changed = false;
        for terminal in items.values_mut() {
            let Some(process) = terminal.process.clone() else {
                continue;
            };
            let before = terminal.metadata.clone();
            for _ in 0..16 {
                let output =
                    match processes.read(&process, OutputStream::Stdout, terminal.scan, 65536, 0) {
                        Ok(output) => output,
                        Err(error) if error.kind == "not_found" => {
                            terminal.process = None;
                            if terminal.metadata.status == Status::Running {
                                terminal.metadata.status = Status::Failed;
                                terminal.metadata.error =
                                    Some("terminal process record has expired".into());
                            }
                            break;
                        }
                        Err(error) => return Err(error),
                    };
                if output.start_offset != terminal.scan {
                    terminal.osc.clear();
                }
                terminal.scan = output.next_offset;
                for byte in output.data.iter().copied() {
                    scan_osc(terminal, byte);
                }
                if let Some(code) = output.exit_code {
                    terminal.metadata.status = Status::Ended;
                    terminal.metadata.exit_code = Some(code);
                }
                if output.data.len() < 65536 {
                    break;
                }
            }
            changed |= before != terminal.metadata;
        }
        if changed {
            self.save(&items)?;
        }
        Ok(())
    }
    pub fn reap(&self, references: &HashSet<String>, processes: &Processes) -> Result<()> {
        self.refresh(processes)?;
        let mut items = self.items.lock().unwrap();
        if reap_items(&mut items, references, Instant::now(), |id| {
            let _ = processes.stop(id, true);
        }) {
            self.save(&items)?;
        }
        Ok(())
    }
    pub fn running(&self) -> Vec<String> {
        self.items
            .lock()
            .unwrap()
            .iter()
            .filter(|(_, t)| t.metadata.status == Status::Running)
            .map(|(id, _)| id.clone())
            .collect()
    }
    /// Rehydrate resources after Environment has activated a new runtime generation.
    pub fn restore(
        &self,
        ids: &[String],
        processes: &Processes,
        environment: &Arc<Mutex<Environment>>,
    ) {
        if self.failure.is_some() {
            return;
        }
        let mut items = self.items.lock().unwrap();
        for id in ids {
            if let Some(terminal) = items.get_mut(id) {
                terminal.process = None;
                if let Err(error) =
                    start(terminal, processes, environment, &StartOptions::default())
                {
                    terminal.metadata.status = Status::Failed;
                    terminal.metadata.error = Some(error.message);
                }
            }
        }
        let _ = self.save(&items);
    }
}

fn not_found() -> Error {
    Error::business("not_found", "terminal not found")
}
fn start(
    terminal: &mut Terminal,
    processes: &Processes,
    environment: &Arc<Mutex<Environment>>,
    options: &StartOptions,
) -> Result<()> {
    let rows = options.rows.unwrap_or(terminal.metadata.rows);
    let columns = options.columns.unwrap_or(terminal.metadata.columns);
    let directory = match &options.directory {
        Some(relative) => working_directory(relative)?,
        None => terminal.metadata.cwd.clone(),
    };
    let spawned = processes.spawn(
        environment,
        &SpawnOptions {
            cwd: directory.clone(),
            terminal: true,
            rows,
            columns,
            env: [
                ("TERM".into(), "xterm-256color".into()),
                ("COLORTERM".into(), "truecolor".into()),
                (
                    "PROMPT_COMMAND".into(),
                    "printf '\\033]7;file://localhost%s\\007' \"$PWD\"".into(),
                ),
            ]
            .into(),
            ..SpawnOptions::default()
        },
    )?;
    terminal.process = Some(spawned.process_id);
    terminal.next_generation();
    terminal.metadata.cwd = directory;
    terminal.metadata.rows = rows;
    terminal.metadata.columns = columns;
    terminal.metadata.status = Status::Running;
    terminal.metadata.exit_code = None;
    terminal.metadata.error = None;
    Ok(())
}
fn reap_items(
    items: &mut HashMap<String, Terminal>,
    references: &HashSet<String>,
    now: Instant,
    mut stop: impl FnMut(&str),
) -> bool {
    let mut changed = false;
    for (id, terminal) in items.iter_mut() {
        if references.contains(id) {
            terminal.unreferenced = None;
            continue;
        }
        if terminal.process.is_none() {
            continue;
        }
        let since = terminal.unreferenced.get_or_insert(now);
        if now.saturating_duration_since(*since) >= REAP_GRACE {
            if let Some(process) = terminal.process.take() {
                stop(&process);
            }
            terminal.metadata.status = Status::Ended;
            terminal.metadata.exit_code = None;
            changed = true;
        }
    }
    changed
}
fn scan_osc(terminal: &mut Terminal, byte: u8) {
    if terminal.osc.is_empty() {
        if byte == 27 {
            terminal.osc.push(byte);
        }
        return;
    }
    if terminal.osc.len() == 1 && byte != b']' {
        terminal.osc.clear();
        return;
    }
    terminal.osc.push(byte);
    if terminal.osc.len() > 8192 {
        terminal.osc.clear();
        return;
    }
    let end = if byte == 7 {
        Some(terminal.osc.len() - 1)
    } else if terminal.osc.ends_with(b"\x1b\\") {
        Some(terminal.osc.len() - 2)
    } else {
        None
    };
    if let Some(end) = end {
        let text = String::from_utf8_lossy(&terminal.osc[2..end]);
        if let Some((kind, payload)) = text.split_once(';') {
            match kind {
                "0" | "2" => {
                    terminal.metadata.title = Some(payload.trim().chars().take(80).collect())
                }
                "7" => {
                    if let Some(uri) = payload.strip_prefix("file://") {
                        if let Some(i) = uri.find('/') {
                            if let Some(path) = path::decode_cwd(&uri[i..]) {
                                terminal.metadata.cwd = path;
                            }
                        }
                    }
                }
                _ => {}
            }
        }
        terminal.osc.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicUsize;
    fn processes() -> Processes {
        Processes::new(Arc::new(AtomicUsize::new(0)))
    }
    fn metadata() -> Metadata {
        Metadata::new("t-test".into(), 1, "/workspace".into())
    }
    #[test]
    fn invalid_terminal_metadata_is_preserved_without_blocking_workspace_startup() {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::open(dir.path()).unwrap();
        store
            .write(
                STATE_KEY,
                &Saved {
                    format: 99,
                    ..Saved::default()
                },
            )
            .unwrap();
        let terminals = Terminals::load(dir.path()).unwrap();
        assert_eq!(
            terminals.failure.as_ref().unwrap().kind,
            "unsupported_format"
        );
        assert_eq!(store.read::<Saved>(STATE_KEY).unwrap().unwrap().format, 99);
        assert!(terminals.refresh(&processes()).is_ok());
        assert_eq!(
            terminals.rename("t-test", Some("new")).unwrap_err().kind,
            "unsupported_format"
        );
    }
    #[test]
    fn osc_state_is_owned_without_client() {
        let mut terminal = Terminal::new(metadata());
        for byte in b"\x1b]2;Build\x07\x1b]7;file://localhost/workspace/src\x1b\\" {
            scan_osc(&mut terminal, *byte);
        }
        assert_eq!(terminal.metadata.title.as_deref(), Some("Build"));
        assert_eq!(terminal.metadata.cwd, "/workspace/src");
    }
    #[test]
    fn resource_registry_reloads_titles_and_unknown_fields_but_not_dead_processes() {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::open(dir.path()).unwrap();
        let saved: Saved = serde_json::from_str(r#"{"format":1,"future":{"revision":9},"terminals":[{"id":"t-test","customTitle":"keep","status":"running","vendor":{"tracking":true}}]}"#).unwrap();
        store.write(STATE_KEY, &saved).unwrap();
        let restored = Terminals::load(dir.path()).unwrap();
        let initial = restored.status("t-test").unwrap();
        assert_eq!(initial.custom_title.as_deref(), Some("keep"));
        assert_eq!(initial.status, Status::Ended);
        assert!(restored.items.lock().unwrap()["t-test"].process.is_none());
        restored.rename("t-test", None).unwrap();
        let round_trip = store.read::<Saved>(STATE_KEY).unwrap().unwrap();
        assert_eq!(round_trip.extra, saved.extra);
        assert_eq!(round_trip.terminals[0].extra, saved.terminals[0].extra);
    }
    #[test]
    fn clear_advances_generation_and_read_resets_without_a_process() {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::open(dir.path()).unwrap();
        store
            .write(
                STATE_KEY,
                &Saved {
                    terminals: vec![metadata()],
                    ..Saved::default()
                },
            )
            .unwrap();
        let registry = Terminals::load(dir.path()).unwrap();
        let processes = processes();
        assert_eq!(registry.clear(&processes, "t-test").unwrap().generation, 1);
        let reset = registry
            .read(
                &processes,
                "t-test",
                &ReadOptions {
                    generation: Some(0),
                    offset: 900,
                    ..ReadOptions::default()
                },
            )
            .unwrap();
        assert!(reset.reset && reset.eof);
        assert_eq!(reset.terminal.generation, 1);
        assert_eq!(reset.next_offset, 0);
        assert!(
            !registry
                .read(
                    &processes,
                    "t-test",
                    &ReadOptions {
                        generation: Some(1),
                        ..ReadOptions::default()
                    }
                )
                .unwrap()
                .reset
        );
    }
    #[test]
    fn referenced_resources_and_fifteen_second_grace_control_cleanup() {
        let now = Instant::now();
        let mut terminal = Terminal::new(metadata());
        terminal.process = Some("process-test".into());
        terminal.metadata.status = Status::Running;
        terminal.unreferenced = Some(now - REAP_GRACE);
        let mut items = HashMap::from([("t-test".into(), terminal)]);
        let mut stopped = Vec::new();
        let references = HashSet::from(["t-test".into()]);
        assert!(!reap_items(&mut items, &references, now, |id| stopped
            .push(id.to_owned())));
        assert!(!reap_items(&mut items, &HashSet::new(), now, |id| stopped
            .push(id.to_owned())));
        assert!(!reap_items(
            &mut items,
            &HashSet::new(),
            now + Duration::from_secs(14),
            |id| stopped.push(id.to_owned())
        ));
        assert!(reap_items(
            &mut items,
            &HashSet::new(),
            now + REAP_GRACE,
            |id| stopped.push(id.to_owned())
        ));
        assert_eq!(stopped, vec!["process-test"]);
        assert_eq!(items["t-test"].metadata.status, Status::Ended);
        assert!(items["t-test"].process.is_none());
    }
}
