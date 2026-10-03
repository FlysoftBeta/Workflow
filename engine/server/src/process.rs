use crate::{
    environment::Environment,
    protocol::{Error, Result, decode_blob, encode_blob},
    workspace::required,
};
use serde_json::{Value as V, json};
use std::{
    collections::{HashMap, VecDeque},
    fs::File,
    io::{Read, Write},
    os::{
        fd::{AsRawFd, FromRawFd},
        unix::process::CommandExt,
    },
    process::Stdio,
    sync::{
        Arc, Condvar, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
    time::{Duration, Instant},
};
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
    pub fn spawn(&self, environment: &Arc<Mutex<Environment>>, a: &V) -> Result<V> {
        let argv = if let Some(argv) = a.get("argv") {
            let Some(argv) = argv.as_array() else {
                return Err(Error::invalid("argv must be an array"));
            };
            if argv.is_empty()
                || argv
                    .iter()
                    .any(|v| !v.is_string() || v.as_str().unwrap().contains('\0'))
            {
                return Err(Error::invalid(
                    "argv must contain executable and string arguments",
                ));
            }
            argv.iter()
                .map(|v| v.as_str().unwrap().to_owned())
                .collect::<Vec<_>>()
        } else {
            vec!["/bin/bash".into(), "--login".into()]
        };
        let terminal = a["terminal"].as_bool().unwrap_or(false);
        let (rows, cols) = dimensions(a)?;
        let env = environment.lock().unwrap();
        let mut items = self.items.lock().unwrap();
        if items.len() >= 128 {
            items.retain(|_, p| p.output.lock().unwrap().exit.is_none());
            if items.len() >= 128 {
                return Err(Error::business("limit", "too many running processes"));
            }
        }

        let mut command =
            env.process_command(&argv, a["cwd"].as_str().unwrap_or("/workspace"), &a["env"])?;
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
        Ok(json!({"processId":id}))
    }
    fn get(&self, a: &V) -> Result<Arc<Process>> {
        self.items
            .lock()
            .unwrap()
            .get(required(a, "processId")?)
            .cloned()
            .ok_or_else(|| Error::business("not_found", "process not found"))
    }
    pub fn request(&self, method: &str, a: &V) -> Result<V> {
        let p = self.get(a)?;
        match method {
            "process.read" => {
                let stderr = match a["stream"].as_str().unwrap_or("stdout") {
                    "stdout" => false,
                    "stderr" => true,
                    _ => return Err(Error::invalid("invalid process stream")),
                };
                let offset = a["offset"].as_u64().unwrap_or(0);
                let max = a["maxBytes"].as_u64().unwrap_or(65536).min(65536) as usize;
                let wait = a["waitMs"].as_u64().unwrap_or(0);
                if wait > 1000 {
                    return Err(Error::invalid("waitMs exceeds 1000"));
                }
                let mut out = p.output.lock().unwrap();
                let ring = if stderr { &out.stderr } else { &out.stdout };
                if offset >= ring.next() && !ring.eof && wait > 0 {
                    out = p
                        .changed
                        .wait_timeout(out, Duration::from_millis(wait))
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
                let bytes: Vec<u8> = ring
                    .bytes
                    .iter()
                    .skip((start - ring.start) as usize)
                    .take(max)
                    .copied()
                    .collect();
                let next = start + bytes.len() as u64;
                let mut v = json!({"data":encode_blob(&bytes),"startOffset":start,"nextOffset":next,"eof":ring.eof&&next==ring.next()});
                if let Some(code) = out.exit {
                    v["exitCode"] = json!(code);
                }
                Ok(v)
            }
            "process.write" => {
                let data = decode_blob(required(a, "data")?, 65536)?;
                let mut input = p.input.lock().unwrap();
                let Some(input) = input.as_mut() else {
                    return Err(Error::business("closed", "process input is closed"));
                };
                input.write_all(&data)?;
                input.flush()?;
                Ok(json!({"written":data.len()}))
            }
            "process.resize" => {
                let (rows, cols) = dimensions(a)?;
                let Some(pty) = &p.pty else {
                    return Err(Error::business(
                        "not_terminal",
                        "process does not have a PTY",
                    ));
                };
                let size = libc::winsize {
                    ws_row: rows,
                    ws_col: cols,
                    ws_xpixel: 0,
                    ws_ypixel: 0,
                };
                if unsafe { libc::ioctl(pty.lock().unwrap().as_raw_fd(), libc::TIOCSWINSZ, &size) }
                    < 0
                {
                    return Err(std::io::Error::last_os_error().into());
                }
                Ok(json!({"rows":rows,"columns":cols}))
            }
            "process.stop" => {
                if p.output.lock().unwrap().exit.is_none() {
                    let signal = if a["force"] == true {
                        libc::SIGKILL
                    } else {
                        libc::SIGTERM
                    };
                    if unsafe { libc::kill(-p.pid, signal) } < 0 {
                        let e = std::io::Error::last_os_error();
                        if e.raw_os_error() != Some(libc::ESRCH) {
                            return Err(e.into());
                        }
                    }
                }
                Ok(json!({"stopping":true}))
            }
            "process.wait" => {
                let timeout = a["timeoutMs"].as_u64().unwrap_or(0);
                if timeout > 30000 {
                    return Err(Error::invalid("timeoutMs exceeds 30000"));
                }
                let deadline = Instant::now() + Duration::from_millis(timeout);
                let mut out = p.output.lock().unwrap();
                while out.exit.is_none() && Instant::now() < deadline {
                    out = p
                        .changed
                        .wait_timeout(out, deadline.saturating_duration_since(Instant::now()))
                        .unwrap()
                        .0;
                }
                Ok(if let Some(code) = out.exit {
                    json!({"running":false,"exitCode":code})
                } else {
                    json!({"running":true})
                })
            }
            _ => Err(Error::method()),
        }
    }
    pub fn clear_output(&self, id: &str) -> Result<()> {
        let p = self.get(&json!({"processId":id}))?;
        let mut out = p.output.lock().unwrap();
        let next = out.stdout.next();
        out.stdout.bytes.clear(); out.stdout.start = next;
        Ok(())
    }
    pub fn stop_all(&self) {
        for p in self.items.lock().unwrap().values() {
            if p.output.lock().unwrap().exit.is_none() {
                unsafe {
                    libc::kill(-p.pid, libc::SIGKILL);
                }
            }
        }
    }
}
fn dimensions(a: &V) -> Result<(u16, u16)> {
    let rows = a["rows"].as_u64().unwrap_or(24);
    let cols = a["columns"].as_u64().unwrap_or(80);
    if rows == 0 || cols == 0 || rows > 1000 || cols > 1000 {
        return Err(Error::invalid("PTY dimensions must be 1..1000"));
    }
    Ok((rows as u16, cols as u16))
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
