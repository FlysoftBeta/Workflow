//! Sole API for guest commands, supervision, PTYs and process output.
use crate::{Environment, Error, Options, Result, access, persist};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, HashMap, VecDeque},
    fs::{self, File},
    io::{Read, Write},
    os::{
        fd::{AsRawFd, FromRawFd},
        unix::{fs::PermissionsExt, process::CommandExt},
    },
    path::Path,
    process::{ChildStderr, ChildStdin, ChildStdout, Command, Stdio},
    sync::{
        Arc, Condvar, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
    time::{Duration, Instant},
};
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(default, rename_all = "camelCase")]
pub struct CommandSpec {
    pub argv: Vec<String>,
    pub cwd: String,
    pub env: BTreeMap<String, String>,
}
impl Default for CommandSpec {
    fn default() -> Self {
        Self {
            argv: vec!["/bin/bash".into(), "--login".into()],
            cwd: "/workspace".into(),
            env: BTreeMap::new(),
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(default, rename_all = "camelCase")]
pub struct SpawnOptions {
    pub argv: Vec<String>,
    pub cwd: String,
    pub env: BTreeMap<String, String>,
    pub terminal: bool,
    pub rows: u16,
    pub columns: u16,
}
impl Default for SpawnOptions {
    fn default() -> Self {
        let c = CommandSpec::default();
        Self {
            argv: c.argv,
            cwd: c.cwd,
            env: c.env,
            terminal: false,
            rows: 24,
            columns: 80,
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Spawned {
    pub process_id: String,
}
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum OutputStream {
    #[default]
    Stdout,
    Stderr,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ReadOutput {
    pub data: Vec<u8>,
    pub start_offset: u64,
    pub next_offset: u64,
    pub eof: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<i32>,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct WaitStatus {
    pub running: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<i32>,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, JsonSchema)]
pub struct Dimensions {
    pub rows: u16,
    pub columns: u16,
}
pub fn dimensions(rows: u16, columns: u16) -> Result<Dimensions> {
    if rows == 0 || columns == 0 || rows > 1000 || columns > 1000 {
        return Err(Error::invalid("PTY dimensions must be 1..1000"));
    }
    Ok(Dimensions { rows, columns })
}
pub(crate) fn env_key(key: &str) -> bool {
    !key.is_empty()
        && key
            .bytes()
            .enumerate()
            .all(|(i, b)| b == b'_' || b.is_ascii_alphabetic() || (i > 0 && b.is_ascii_digit()))
}
pub(crate) fn validate_command(
    argv: &[String],
    cwd: &str,
    env: &BTreeMap<String, String>,
) -> Result<()> {
    if argv.is_empty() || argv.iter().any(|a| a.contains('\0')) {
        return Err(Error::invalid(
            "argv must contain executable and string arguments",
        ));
    }
    if !cwd.starts_with('/') || cwd.contains('\0') || cwd.split('/').any(|part| part == "..") {
        return Err(Error::invalid(
            "cwd must be a normalized guest absolute path",
        ));
    }
    if env.iter().any(|(k, v)| !env_key(k) || v.contains('\0')) {
        return Err(Error::invalid("invalid process environment"));
    }
    Ok(())
}
const RING_CAP: usize = 4 * 1024 * 1024;
#[derive(Default)]
struct Ring {
    bytes: VecDeque<u8>,
    start: u64,
    eof: bool,
}
impl Ring {
    fn push(&mut self, b: &[u8]) {
        self.bytes.extend(b);
        let remove = self.bytes.len().saturating_sub(RING_CAP);
        self.bytes.drain(..remove);
        self.start += remove as u64;
    }
    fn next(&self) -> u64 {
        self.start + self.bytes.len() as u64
    }
}
struct Output {
    stdout: Ring,
    stderr: Ring,
    exit: Option<i32>,
}
pub struct Process {
    pid: i32,
    output: Mutex<Output>,
    changed: Condvar,
    input: Mutex<Option<Box<dyn Write + Send>>>,
    pty: Option<Mutex<File>>,
}
pub struct Processes {
    items: Mutex<HashMap<String, Arc<Process>>>,
    pub running: Arc<AtomicUsize>,
}
impl Processes {
    pub fn new(running: Arc<AtomicUsize>) -> Self {
        Self {
            items: Mutex::new(HashMap::new()),
            running,
        }
    }
    pub fn spawn(
        &self,
        environment: &Arc<Mutex<Environment>>,
        a: &SpawnOptions,
    ) -> Result<Spawned> {
        validate_command(&a.argv, &a.cwd, &a.env)?;
        let argv = &a.argv;
        let terminal = a.terminal;
        let Dimensions {
            rows,
            columns: cols,
        } = dimensions(a.rows, a.columns)?;
        let env = environment.lock().unwrap();
        let mut items = self.items.lock().unwrap();
        if items.len() >= 128 {
            items.retain(|_, p| p.output.lock().unwrap().exit.is_none());
            if items.len() >= 128 {
                return Err(Error::business("limit", "too many running processes"));
            }
        }

        let mut command = env.process_command(argv, &a.cwd, &a.env)?;
        let mut pty = None;
        let mut master_read = None;
        if terminal {
            let mut master = -1;
            let mut slave = -1;
            let size = libc::winsize {
                ws_row: rows,
                ws_col: cols,
                ws_xpixel: 0,
                ws_ypixel: 0,
            };
            let rc = unsafe {
                libc::openpty(
                    &mut master,
                    &mut slave,
                    std::ptr::null_mut(),
                    std::ptr::null(),
                    &size,
                )
            };
            if rc != 0 {
                return Err(std::io::Error::last_os_error().into());
            }
            let master = unsafe { File::from_raw_fd(master) };
            let slave = unsafe { File::from_raw_fd(slave) };
            command
                .stdin(slave.try_clone()?)
                .stdout(slave.try_clone()?)
                .stderr(slave);
            unsafe {
                command.pre_exec(|| {
                    if libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGKILL) < 0 {
                        return Err(std::io::Error::last_os_error());
                    }
                    if libc::setsid() < 0 {
                        return Err(std::io::Error::last_os_error());
                    }
                    if libc::ioctl(0, libc::TIOCSCTTY, 0) < 0 {
                        return Err(std::io::Error::last_os_error());
                    }
                    Ok(())
                });
            }
            master_read = Some(master.try_clone()?);
            pty = Some(master);
        } else {
            command
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped());
            unsafe {
                command.pre_exec(|| {
                    if libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGKILL) < 0 {
                        return Err(std::io::Error::last_os_error());
                    }
                    if libc::setsid() < 0 {
                        return Err(std::io::Error::last_os_error());
                    }
                    Ok(())
                });
            }
        }
        // Linux parent-death signals are tied to the spawning *thread*. Keep that
        // thread alive as the child's reaper instead of spawning from an RPC worker.
        let (spawn_tx, spawn_rx) = std::sync::mpsc::sync_channel(1);
        let (owner_tx, owner_rx) = std::sync::mpsc::sync_channel::<Arc<Process>>(1);
        let counter = self.running.clone();
        std::thread::spawn(move || {
            let mut child = match command.spawn() {
                Ok(child) => child,
                Err(error) => {
                    let _ = spawn_tx.send(Err(error));
                    return;
                }
            };
            let pid = child.id() as i32;
            let pipes = (
                pid,
                child.stdin.take(),
                child.stdout.take(),
                child.stderr.take(),
            );
            if spawn_tx.send(Ok(pipes)).is_err() {
                let _ = child.kill();
                let _ = child.wait();
                return;
            }
            let process = match owner_rx.recv() {
                Ok(p) => p,
                Err(_) => {
                    let _ = child.kill();
                    let _ = child.wait();
                    return;
                }
            };
            let code = match child.wait() {
                Ok(status) => {
                    use std::os::unix::process::ExitStatusExt;
                    status
                        .code()
                        .unwrap_or_else(|| 128 + status.signal().unwrap_or(0))
                }
                Err(_) => 125,
            };
            process.output.lock().unwrap().exit = Some(code);
            process.input.lock().unwrap().take();
            counter.fetch_sub(1, Ordering::SeqCst);
            process.changed.notify_all();
        });
        let (pid, stdin, stdout, stderr) = spawn_rx
            .recv()
            .map_err(|_| Error::business("spawn_failed", "process owner disconnected"))??;
        let input: Box<dyn Write + Send> = if let Some(p) = &pty {
            Box::new(p.try_clone()?)
        } else {
            Box::new(stdin.unwrap())
        };
        let process = Arc::new(Process {
            pid,
            output: Mutex::new(Output {
                stdout: Ring::default(),
                stderr: Ring {
                    eof: terminal,
                    ..Ring::default()
                },
                exit: None,
            }),
            changed: Condvar::new(),
            input: Mutex::new(Some(input)),
            pty: pty.map(Mutex::new),
        });
        self.running.fetch_add(1, Ordering::SeqCst);
        owner_tx
            .send(process.clone())
            .map_err(|_| Error::business("spawn_failed", "process owner disconnected"))?;
        drop(env);
        let id = uuid::Uuid::new_v4().to_string();
        items.insert(id.clone(), process.clone());
        drop(items);
        if let Some(master) = master_read {
            reader(process.clone(), master, false);
        } else {
            reader(process.clone(), stdout.unwrap(), false);
            reader(process.clone(), stderr.unwrap(), true);
        }
        Ok(Spawned { process_id: id })
    }

    fn get(&self, id: &str) -> Result<Arc<Process>> {
        self.items
            .lock()
            .unwrap()
            .get(id)
            .cloned()
            .ok_or_else(|| Error::business("not_found", "process not found"))
    }
    pub fn read(
        &self,
        id: &str,
        stream: OutputStream,
        offset: u64,
        max_bytes: usize,
        wait_ms: u64,
    ) -> Result<ReadOutput> {
        if wait_ms > 1000 {
            return Err(Error::invalid("waitMs exceeds 1000"));
        }
        let p = self.get(id)?;
        let stderr = matches!(stream, OutputStream::Stderr);
        let mut out = p.output.lock().unwrap();
        let ring = if stderr { &out.stderr } else { &out.stdout };
        if offset >= ring.next() && !ring.eof && wait_ms > 0 {
            out = p
                .changed
                .wait_timeout(out, Duration::from_millis(wait_ms))
                .unwrap()
                .0;
        }
        let ring = if stderr { &out.stderr } else { &out.stdout };
        if offset > ring.next() {
            return Err(Error::business(
                "invalid_offset",
                "offset exceeds produced bytes",
            ));
        }
        let start = offset.max(ring.start);
        let data: Vec<u8> = ring
            .bytes
            .iter()
            .skip((start - ring.start) as usize)
            .take(max_bytes.min(65536))
            .copied()
            .collect();
        let next = start + data.len() as u64;
        Ok(ReadOutput {
            data,
            start_offset: start,
            next_offset: next,
            eof: ring.eof && next == ring.next(),
            exit_code: out.exit,
        })
    }
    pub fn write(&self, id: &str, data: &[u8]) -> Result<usize> {
        if data.len() > 65536 {
            return Err(Error::invalid("blob exceeds chunk limit"));
        }
        let p = self.get(id)?;
        let mut input = p.input.lock().unwrap();
        let input = input
            .as_mut()
            .ok_or_else(|| Error::business("closed", "process input is closed"))?;
        input.write_all(data)?;
        input.flush()?;
        Ok(data.len())
    }
    pub fn resize(&self, id: &str, rows: u16, columns: u16) -> Result<Dimensions> {
        let dimensions = dimensions(rows, columns)?;
        let p = self.get(id)?;
        let pty = p
            .pty
            .as_ref()
            .ok_or_else(|| Error::business("not_terminal", "process does not have a PTY"))?;
        let size = libc::winsize {
            ws_row: rows,
            ws_col: columns,
            ws_xpixel: 0,
            ws_ypixel: 0,
        };
        if unsafe { libc::ioctl(pty.lock().unwrap().as_raw_fd(), libc::TIOCSWINSZ, &size) } < 0 {
            return Err(std::io::Error::last_os_error().into());
        }
        Ok(dimensions)
    }
    pub fn stop(&self, id: &str, force: bool) -> Result<()> {
        let p = self.get(id)?;
        if p.output.lock().unwrap().exit.is_none() {
            signal_group(p.pid, force)?;
        }
        Ok(())
    }
    pub fn wait(&self, id: &str, timeout_ms: u64) -> Result<WaitStatus> {
        if timeout_ms > 30000 {
            return Err(Error::invalid("timeoutMs exceeds 30000"));
        }
        let p = self.get(id)?;
        let deadline = Instant::now() + Duration::from_millis(timeout_ms);
        let mut out = p.output.lock().unwrap();
        while out.exit.is_none() && Instant::now() < deadline {
            out = p
                .changed
                .wait_timeout(out, deadline.saturating_duration_since(Instant::now()))
                .unwrap()
                .0;
        }
        Ok(WaitStatus {
            running: out.exit.is_none(),
            exit_code: out.exit,
        })
    }
    pub fn clear_output(&self, id: &str) -> Result<()> {
        let p = self.get(id)?;
        let mut out = p.output.lock().unwrap();
        let next = out.stdout.next();
        out.stdout.bytes.clear();
        out.stdout.start = next;
        Ok(())
    }
    pub fn stop_all(&self) {
        for p in self.items.lock().unwrap().values() {
            if p.output.lock().unwrap().exit.is_none() {
                let _ = signal_group(p.pid, true);
            }
        }
    }
}
fn signal_group(pid: i32, force: bool) -> Result<()> {
    if unsafe { libc::kill(-pid, if force { libc::SIGKILL } else { libc::SIGTERM }) } < 0 {
        let error = std::io::Error::last_os_error();
        if error.raw_os_error() != Some(libc::ESRCH) {
            return Err(error.into());
        }
    }
    Ok(())
}
fn reader(p: Arc<Process>, mut source: impl Read + Send + 'static, stderr: bool) {
    std::thread::spawn(move || {
        let mut bytes = [0u8; 8192];
        loop {
            match source.read(&mut bytes) {
                Ok(0) => break,
                Ok(n) => {
                    let mut o = p.output.lock().unwrap();
                    let ring = if stderr { &mut o.stderr } else { &mut o.stdout };
                    ring.push(&bytes[..n]);
                    drop(o);
                    p.changed.notify_all();
                }
                Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(_) => break,
            }
        }
        let mut o = p.output.lock().unwrap();
        if stderr {
            o.stderr.eof = true;
        } else {
            o.stdout.eof = true;
        }
        drop(o);
        p.changed.notify_all();
    });
}

struct HandleState {
    pid: i32,
    exit: Mutex<Option<i32>>,
    changed: Condvar,
}
#[derive(Clone)]
pub struct ProcessHandle(Arc<HandleState>);
impl ProcessHandle {
    pub fn stop(&self, force: bool) -> Result<()> {
        if self.running() {
            signal_group(self.0.pid, force)?;
        }
        Ok(())
    }
    pub fn running(&self) -> bool {
        self.0.exit.lock().unwrap().is_none()
    }
    pub fn wait(&self, timeout_ms: u64) -> Result<WaitStatus> {
        if timeout_ms > 30000 {
            return Err(Error::invalid("timeoutMs exceeds 30000"));
        }
        let deadline = Instant::now() + Duration::from_millis(timeout_ms);
        let mut exit = self.0.exit.lock().unwrap();
        while exit.is_none() && Instant::now() < deadline {
            exit = self
                .0
                .changed
                .wait_timeout(exit, deadline.saturating_duration_since(Instant::now()))
                .unwrap()
                .0;
        }
        Ok(WaitStatus {
            running: exit.is_none(),
            exit_code: *exit,
        })
    }
}
pub struct PipedProcess {
    pub stdin: ChildStdin,
    pub stdout: ChildStdout,
    pub stderr: ChildStderr,
    pub handle: ProcessHandle,
}
/// Spawn and reap on one persistent thread: PDEATHSIG belongs to the spawning thread.
pub fn spawn_piped(
    environment: &Arc<Mutex<Environment>>,
    request: &CommandSpec,
) -> Result<PipedProcess> {
    let environment = environment.lock().unwrap();
    let command = environment.process_command(&request.argv, &request.cwd, &request.env)?;
    spawn_piped_command(command, environment.running.clone())
}
fn spawn_piped_command(mut command: Command, running: Arc<AtomicUsize>) -> Result<PipedProcess> {
    command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    unsafe {
        command.pre_exec(|| {
            if libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGKILL) < 0 || libc::setsid() < 0 {
                return Err(std::io::Error::last_os_error());
            }
            Ok(())
        });
    }
    let (sender, receiver) = std::sync::mpsc::sync_channel(1);
    std::thread::spawn(move || {
        let mut child = match command.spawn() {
            Ok(child) => child,
            Err(error) => {
                let _ = sender.send(Err(error));
                return;
            }
        };
        let handle = ProcessHandle(Arc::new(HandleState {
            pid: child.id() as i32,
            exit: Mutex::new(None),
            changed: Condvar::new(),
        }));
        let process = PipedProcess {
            stdin: child.stdin.take().unwrap(),
            stdout: child.stdout.take().unwrap(),
            stderr: child.stderr.take().unwrap(),
            handle: handle.clone(),
        };
        running.fetch_add(1, Ordering::SeqCst);
        if sender.send(Ok(process)).is_err() {
            let _ = child.kill();
        }
        let code = match child.wait() {
            Ok(status) => {
                use std::os::unix::process::ExitStatusExt;
                status
                    .code()
                    .unwrap_or_else(|| 128 + status.signal().unwrap_or(0))
            }
            Err(_) => 125,
        };
        *handle.0.exit.lock().unwrap() = Some(code);
        running.fetch_sub(1, Ordering::SeqCst);
        handle.0.changed.notify_all();
    });
    receiver
        .recv()
        .map_err(|_| Error::business("spawn_failed", "process owner disconnected"))?
        .map_err(Into::into)
}
pub(crate) fn probe(
    environment: &Arc<Mutex<Environment>>,
    argv: Vec<String>,
    expected: &str,
) -> Result<()> {
    let PipedProcess {
        stdin,
        stdout,
        stderr,
        handle,
    } = spawn_piped(
        environment,
        &CommandSpec {
            argv,
            cwd: "/home/work".into(),
            env: BTreeMap::new(),
        },
    )?;
    drop(stdin);
    fn drain(mut pipe: impl Read) -> Vec<u8> {
        let mut data = Vec::new();
        let mut buf = [0; 8192];
        while let Ok(n) = pipe.read(&mut buf) {
            if n == 0 {
                break;
            }
            let take = n.min(65536usize.saturating_sub(data.len()));
            data.extend_from_slice(&buf[..take]);
        }
        data
    }
    let stdout = std::thread::spawn(move || drain(stdout));
    let stderr = std::thread::spawn(move || drain(stderr));
    let result = handle.wait(30000)?;
    if result.running {
        let _ = handle.stop(true);
        let _ = handle.wait(1000);
        return Err(Error::business(
            "invalid_tools",
            "tool version check timed out",
        ));
    }
    let mut bytes = stdout.join().unwrap_or_default();
    bytes.extend(stderr.join().unwrap_or_default());
    if result.exit_code != Some(0) || !String::from_utf8_lossy(&bytes).contains(expected) {
        return Err(Error::business(
            "invalid_tools",
            "tool execution/version check failed",
        ));
    }
    Ok(())
}
pub(crate) fn install_generation(
    opts: &Options,
    image: &Path,
    index: &Path,
    generation: &Path,
) -> Result<()> {
    let mut command = Command::new(runtime_binary(opts)?);
    command
        .arg("install")
        .arg("--image")
        .arg(image)
        .arg("--index")
        .arg(index)
        .arg("--target")
        .arg(generation)
        .arg("--profile")
        .arg("workspace")
        .arg("--quiet");
    checked(command, "image install")?;
    Ok(())
}
pub(crate) fn verify_generation(opts: &Options, generation: &Path) -> Result<()> {
    let mut command = Command::new(runtime_binary(opts)?);
    command
        .arg("verify")
        .arg("--generation")
        .arg(generation)
        .arg("--quiet");
    checked(command, "generation verification")?;
    Ok(())
}
fn runtime_binary(opts: &Options) -> Result<&Path> {
    opts.runtime
        .as_deref()
        .filter(|p| p.is_file())
        .ok_or_else(|| {
            Error::business(
                "runtime_unavailable",
                "workspace runtime executable is not configured",
            )
        })
}
pub(crate) fn guest_command(
    opts: &Options,
    generation: &Path,
    user: &str,
    cwd: &str,
    env: &BTreeMap<String, String>,
    interactive: bool,
) -> Result<Command> {
    let mut c = Command::new(runtime_binary(opts)?);
    c.arg("run")
        .arg("--root")
        .arg(generation.join("rootfs"))
        .arg("--user")
        .arg(user)
        .arg("--cwd")
        .arg(cwd);
    let sockets = std::env::temp_dir().join(format!(
        "wf-sock-{}",
        &persist::hash(opts.root.to_string_lossy().as_bytes())[..12]
    ));
    fs::create_dir_all(&sockets)?;
    c.arg("--socket-dir").arg(&sockets);
    if let Some(loader) = &opts.loader {
        c.arg("--loader").arg(loader);
    }
    let toolchains = crate::lifecycle::store_dir(opts, "toolchains");
    fs::create_dir_all(&toolchains)?;
    c.arg("--bind")
        .arg(format!("{}:/opt/toolchains", toolchains.display()));
    if !interactive && generation.join("post-home").is_dir() {
        c.arg("--bind").arg(format!(
            "{}:/home/work",
            generation.join("post-home").display()
        ));
    }
    if interactive {
        let home = crate::lifecycle::store_dir(opts, "home/work");
        fs::create_dir_all(&home)?;
        c.arg("--bind")
            .arg(format!("{}:/home/work", home.display()));
        // Agent homes are visible workspace configuration, shared by chat and terminals. The
        // runtime resolves nested binds by the longest guest prefix.
        for agent in access::AGENT_HOMES {
            let host = opts.root.join(access::DIRECTORY).join(agent.key());
            fs::create_dir_all(&host)?;
            fs::set_permissions(&host, fs::Permissions::from_mode(0o700))?;
            c.arg("--bind")
                .arg(format!("{}:{}", host.display(), agent.guest));
        }
        c.arg("--bind")
            .arg(format!("{}:{}", opts.root.display(), access::GUEST_WORKSPACE));
        // Hides apply to resolved guest paths. The agents tree is masked below /workspace and
        // exists only at its home and tool mounts, so workspace walks never reach credentials.
        for mask in access::guest_masks() {
            c.arg("--hide").arg(mask);
        }
    }
    let resolver = opts
        .root
        .join(access::DIRECTORY)
        .join(crate::store::keys::RESOLVER);
    if resolver.is_file() {
        c.arg("--bind")
            .arg(format!("{}:/etc/resolv.conf", resolver.display()));
    }
    crate::tools::bind(opts, &mut c)?;
    c.env_clear();
    if interactive {
        for agent in access::AGENT_HOMES {
            c.env(agent.variable, agent.guest);
        }
    }
    c.env("PATH","/opt/toolchains/active/python/bin:/opt/toolchains/active/node/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin").env("HOME",if user=="root"{"/root"}else{"/home/work"}).env("USER",user).env("LANG","C.UTF-8").env("TERM","xterm-256color").env("NVM_DIR","/opt/toolchains/nvm").env("UV_PYTHON_INSTALL_DIR","/opt/toolchains/uv/python").env("UV_CACHE_DIR","/opt/toolchains/uv/cache").env("TMPDIR","/tmp");
    // Apply caller variables with the guest env executable. Loader variables must
    // never affect the host runtime before it has entered the environment.
    c.arg("--").arg("/usr/bin/env");
    for (k, v) in env {
        c.arg(format!("{k}={v}"));
    }
    Ok(c)
}
pub(crate) fn checked(mut c: Command, label: &str) -> Result<Vec<u8>> {
    unsafe {
        c.pre_exec(|| {
            if libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGKILL) < 0 {
                return Err(std::io::Error::last_os_error());
            }
            Ok(())
        });
    }
    c.stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = c.spawn()?;
    let stdout = child.stdout.take().unwrap();
    let stderr = child.stderr.take().unwrap();
    let err_reader = std::thread::spawn(move || {
        let mut data = Vec::new();
        let mut pipe = stderr;
        let mut buf = [0u8; 8192];
        while let Ok(n) = pipe.read(&mut buf) {
            if n == 0 {
                break;
            }
            if data.len() < 65536 {
                let take = n.min(65536 - data.len());
                data.extend_from_slice(&buf[..take]);
            }
        }
    });
    let out_reader = std::thread::spawn(move || {
        let mut data = Vec::new();
        let mut pipe = stdout;
        let mut buf = [0u8; 8192];
        while let Ok(n) = pipe.read(&mut buf) {
            if n == 0 {
                break;
            }
            if data.len() < 1024 * 1024 {
                let take = n.min(1024 * 1024 - data.len());
                data.extend_from_slice(&buf[..take]);
            }
        }
        data
    });
    let status = child.wait()?;
    let _ = err_reader.join();
    let output = out_reader.join().unwrap_or_default();
    if !status.success() {
        return Err(Error::business(
            "build_failed",
            &format!("{label} exited with {}", status.code().unwrap_or(128)),
        ));
    }
    Ok(output)
}

/// Only explicit test fixtures may execute a host process through this supervisor.
#[cfg(feature = "test-support")]
pub fn spawn_fixture(command: Command, running: Arc<AtomicUsize>) -> Result<PipedProcess> {
    spawn_piped_command(command, running)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;
    #[test]
    fn ring_reports_clipped_offsets_and_clear_keeps_cursor_monotonic() {
        let mut ring = Ring::default();
        ring.push(&vec![b'x'; RING_CAP + 13]);
        assert_eq!(ring.start, 13);
        assert_eq!(ring.bytes.len(), RING_CAP);
        assert_eq!(ring.next(), RING_CAP as u64 + 13);
    }
    #[test]
    fn pipe_owner_outlives_request_thread_and_tracks_exit() {
        let running = Arc::new(AtomicUsize::new(0));
        let counter = running.clone();
        let process = std::thread::spawn(move || {
            let mut command = Command::new("/bin/sh");
            command.args([
                "-c",
                "read line; printf 'reply:%s' \"$line\"; printf error >&2; exit 7",
            ]);
            spawn_piped_command(command, counter).unwrap()
        })
        .join()
        .unwrap();
        let PipedProcess {
            mut stdin,
            mut stdout,
            mut stderr,
            handle,
        } = process;
        assert!(handle.running());
        assert_eq!(running.load(Ordering::SeqCst), 1);
        stdin.write_all(b"hello\n").unwrap();
        drop(stdin);
        let done = handle.wait(5000).unwrap();
        assert!(!done.running);
        assert_eq!(done.exit_code, Some(7));
        let mut output = String::new();
        stdout.read_to_string(&mut output).unwrap();
        assert_eq!(output, "reply:hello");
        let mut error = String::new();
        stderr.read_to_string(&mut error).unwrap();
        assert_eq!(error, "error");
        assert_eq!(running.load(Ordering::SeqCst), 0);
    }
    #[test]
    fn process_handle_stops_group_and_wait_is_bounded() {
        let running = Arc::new(AtomicUsize::new(0));
        let mut command = Command::new("/bin/sh");
        command.args(["-c", "sleep 30"]);
        let process = spawn_piped_command(command, running.clone()).unwrap();
        assert!(process.handle.wait(0).unwrap().running);
        process.handle.stop(true).unwrap();
        assert_eq!(process.handle.wait(5000).unwrap().exit_code, Some(137));
        assert_eq!(running.load(Ordering::SeqCst), 0);
    }
    #[test]
    fn bundled_launcher_and_guest_variables_do_not_mutate_host_environment() {
        let temp = tempfile::tempdir().unwrap();
        let opts = Options {
            root: temp.path().into(),
            runtime: Some("/bin/true".into()),
            tools: Some(temp.path().join("payload")),
            ..Options::default()
        };
        crate::tools::fixture(&temp.path().join("payload"));
        let generation = temp.path().join("generation");
        let command = guest_command(
            &opts,
            &generation,
            "work",
            "/workspace",
            &BTreeMap::from([("LD_PRELOAD".into(), "guest-only.so".into())]),
            true,
        )
        .unwrap();
        let launcher = temp.path().join(".workspace/cache/tools/launchers/codex");
        let binding = format!("{}:/usr/local/bin/codex", launcher.display());
        assert!(command.get_args().any(|arg| arg == binding.as_str()));
        assert!(
            command
                .get_args()
                .any(|arg| arg == "LD_PRELOAD=guest-only.so")
        );
        assert!(!command.get_envs().any(|(key, _)| key == "LD_PRELOAD"));
        assert_eq!(
            fs::read(&launcher).unwrap(),
            include_bytes!("../guest/codex")
        );
        assert_eq!(
            fs::metadata(&launcher).unwrap().permissions().mode() & 0o777,
            0o755
        );
        assert!(!generation.exists());
    }
    fn arguments(command: &Command) -> Vec<String> {
        command
            .get_args()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect()
    }
    fn values(args: &[String], flag: &str) -> Vec<String> {
        args.windows(2)
            .filter(|pair| pair[0] == flag)
            .map(|pair| pair[1].clone())
            .collect()
    }
    #[test]
    fn interactive_guests_share_visible_agent_homes_and_mask_private_state() {
        let temp = tempfile::tempdir().unwrap();
        let opts = Options {
            root: temp.path().into(),
            runtime: Some("/bin/true".into()),
            tools: Some(temp.path().join("payload")),
            ..Options::default()
        };
        crate::tools::fixture(&temp.path().join("payload"));
        let generation = temp.path().join("generation");
        let command =
            guest_command(&opts, &generation, "work", "/workspace", &BTreeMap::new(), true)
                .unwrap();
        let args = arguments(&command);
        let binds = values(&args, "--bind");
        let root = temp.path().display().to_string();
        for (id, guest) in [("codex", "/home/work/.codex"), ("claude", "/home/work/.claude")] {
            let host = temp.path().join(".workspace/agents").join(id);
            assert!(binds.contains(&format!("{}:{guest}", host.display())), "{binds:?}");
            assert_eq!(
                fs::metadata(&host).unwrap().permissions().mode() & 0o777,
                0o700
            );
        }
        let payload = binds
            .iter()
            .find(|bind| bind.ends_with(":/opt/workflow/tools"))
            .unwrap();
        assert!(payload.starts_with(&format!("{root}/.workspace/cache/tools/payload/")));
        assert!(binds.contains(&format!(
            "{root}/.workspace/cache/tools/claude/2.1.283:/opt/workflow/tools/claude"
        )));
        assert!(binds.contains(&format!("{root}:/workspace")));
        assert!(!binds.iter().any(|bind| bind.contains("/.workspace/agents/tools")));
        assert!(binds.contains(&format!(
            "{root}/.workspace/cache/toolchains:/opt/toolchains"
        )));
        assert!(binds.contains(&format!(
            "{root}/.workspace/environment/stores/home/work:/home/work"
        )));
        let hides = values(&args, "--hide");
        assert_eq!(hides, crate::access::guest_masks());
        for private in [
            "state",
            "environment",
            "documents",
            "corrupt",
            "trash",
            "engine.lock",
            "agents",
            "cache",
        ] {
            assert!(hides.contains(&format!("/workspace/.workspace/{private}")), "{private}");
        }
        for visible in ["config.json", "proxy", "services"] {
            assert!(
                !hides.iter().any(|hide| hide == &format!("/workspace/.workspace/{visible}")),
                "{visible}"
            );
        }
        let envs: BTreeMap<String, String> = command
            .get_envs()
            .filter_map(|(k, v)| {
                Some((k.to_string_lossy().into_owned(), v?.to_string_lossy().into_owned()))
            })
            .collect();
        assert_eq!(envs["CODEX_HOME"], "/home/work/.codex");
        assert_eq!(envs["CLAUDE_CONFIG_DIR"], "/home/work/.claude");
        assert_eq!(envs["HOME"], "/home/work");
        assert!(!temp.path().join(".workspace/agents/tools").exists());
        assert!(!temp.path().join(".workspace/environment/tools").exists());
    }
    #[test]
    fn provisioning_guests_see_neither_the_workspace_nor_agent_homes() {
        let temp = tempfile::tempdir().unwrap();
        let opts = Options {
            root: temp.path().into(),
            runtime: Some("/bin/true".into()),
            ..Options::default()
        };
        let command = guest_command(
            &opts,
            &temp.path().join("generation"),
            "work",
            "/home/work",
            &BTreeMap::new(),
            false,
        )
        .unwrap();
        let args = arguments(&command);
        assert!(values(&args, "--hide").is_empty());
        assert!(
            !values(&args, "--bind")
                .iter()
                .any(|bind| bind.ends_with(":/workspace") || bind.contains("/home/work/."))
        );
        assert!(!command.get_envs().any(|(key, _)| key == "CODEX_HOME"));
        assert!(!temp.path().join(".workspace/agents/codex").exists());
    }
    #[test]
    fn command_validation_rejects_parent_traversal_nuls_and_bad_env_keys() {
        assert!(validate_command(&[], "/", &BTreeMap::new()).is_err());
        assert!(validate_command(&["bash".into()], "/workspace/../etc", &BTreeMap::new()).is_err());
        assert!(
            validate_command(
                &["bash".into()],
                "/workspace",
                &BTreeMap::from([("1BAD".into(), "x".into())])
            )
            .is_err()
        );
        assert!(dimensions(0, 80).is_err());
        assert!(dimensions(24, 1001).is_err());
    }
}
