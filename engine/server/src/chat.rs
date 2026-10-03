//! Engine-owned guest chat service supervision and private bidirectional RPC.
//! The Android client never receives the private process channel or executable paths.
use crate::{
    environment::Environment,
    protocol::{Error, MAX_FRAME, Result, line, strict_json},
};
use serde_json::{Value as V, json};
use std::{
    collections::HashMap,
    io::{BufReader, Read, Write},
    os::unix::process::CommandExt,
    process::{ChildStdin, Command, Stdio},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering},
        mpsc,
    },
    time::Duration,
};

type Callback = Arc<dyn Fn(&str, &V) -> Result<V> + Send + Sync>;

#[derive(Default)]
pub struct Chat {
    host: Mutex<Option<Arc<Host>>>,
}
impl Chat {
    pub fn request(
        &self,
        environment: &Arc<Mutex<Environment>>,
        callback: Callback,
        method: &str,
        params: &V,
    ) -> Result<V> {
        let host = {
            let mut slot = self.host.lock().unwrap();
            if slot
                .as_ref()
                .is_none_or(|h| !h.alive.load(Ordering::SeqCst))
            {
                let environment = environment.lock().unwrap();
                let argv = [
                    "/opt/workflow/tools/jre/bin/java",
                    "-Xms16m",
                    "-Xmx256m",
                    "-XX:ActiveProcessorCount=2",
                    "-XX:+UseSerialGC",
                    "-XX:-UsePerfData",
                    "-jar",
                    "/opt/workflow/tools/chat/workflow-chat.jar",
                ]
                .map(String::from);
                let command = environment.process_command(&argv, "/workspace", &json!({}))?;
                *slot = Some(Host::spawn(command, environment.running.clone(), callback)?);
            }
            slot.as_ref().unwrap().clone()
        };
        host.request(method, params)
    }
    pub fn stop(&self) {
        if let Some(host) = self.host.lock().unwrap().take() {
            host.stop();
        }
    }
}

struct Host {
    input: Mutex<ChildStdin>,
    pending: Mutex<HashMap<String, mpsc::Sender<Result<V>>>>,
    next: AtomicU64,
    alive: AtomicBool,
    callbacks: AtomicUsize,
    pid: i32,
}
impl Host {
    fn spawn(
        mut command: Command,
        running: Arc<AtomicUsize>,
        callback: Callback,
    ) -> Result<Arc<Self>> {
        command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        unsafe {
            command.pre_exec(|| {
                if libc::setsid() < 0 {
                    return Err(std::io::Error::last_os_error());
                }
                if libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGKILL) < 0 {
                    return Err(std::io::Error::last_os_error());
                }
                Ok(())
            });
        }
        // PR_SET_PDEATHSIG follows the spawning thread, so that thread also reaps.
        let (spawn_tx, spawn_rx) = mpsc::sync_channel(1);
        let (owner_tx, owner_rx) = mpsc::sync_channel::<Arc<Host>>(1);
        std::thread::spawn(move || {
            let mut child = match command.spawn() {
                Ok(child) => child,
                Err(error) => { let _ = spawn_tx.send(Err(error)); return; }
            };
            let pipes = (child.id() as i32, child.stdin.take().unwrap(), child.stdout.take().unwrap(), child.stderr.take().unwrap());
            if spawn_tx.send(Ok(pipes)).is_err() { let _ = child.kill(); let _ = child.wait(); return; }
            let host = match owner_rx.recv() {
                Ok(host) => host,
                Err(_) => { let _ = child.kill(); let _ = child.wait(); return; }
            };
            running.fetch_add(1, Ordering::SeqCst);
            let _ = child.wait();
            host.fail();
            running.fetch_sub(1, Ordering::SeqCst);
        });
        let (pid, stdin, stdout, mut stderr) = spawn_rx.recv().map_err(|_| Error::business("chat_start", "chat process owner exited"))??;
        let host = Arc::new(Self {
            input: Mutex::new(stdin), pending: Mutex::new(HashMap::new()),
            next: AtomicU64::new(0), alive: AtomicBool::new(true), callbacks: AtomicUsize::new(0), pid,
        });
        owner_tx.send(host.clone()).map_err(|_| Error::business("chat_start", "chat process owner exited"))?;
        // Drain diagnostics without retaining secrets or printing vendor output.
        std::thread::spawn(move || {
            let mut b = [0u8; 8192];
            while stderr.read(&mut b).is_ok_and(|n| n != 0) {}
        });
        let reader = host.clone();
        std::thread::spawn(move || {
            let mut input = BufReader::new(stdout);
            while let Ok(Some(bytes)) = line(&mut input) {
                let frame = match strict_json(&bytes) {
                    Ok(v) => v,
                    Err(_) => break,
                };
                if frame["jsonrpc"] != "2.0" {
                    break;
                }
                if let Some(method) = frame["method"].as_str() {
                    let Some(id) = frame.get("id").filter(|id| id.is_string()).cloned() else {
                        break;
                    };
                    if reader.callbacks.fetch_add(1, Ordering::SeqCst) >= 64 {
                        reader.callbacks.fetch_sub(1, Ordering::SeqCst);
                        let _ = reader
                            .reply(id, Err(Error::business("limit", "too many chat callbacks")));
                        continue;
                    }
                    let method = method.to_owned();
                    let params = frame.get("params").cloned().unwrap_or(json!({}));
                    let child = reader.clone();
                    let callback = callback.clone();
                    std::thread::spawn(move || {
                        let result = if allowed_callback(&method, &params) {
                            callback(&method, &params)
                        } else {
                            Err(Error::method())
                        };
                        let _ = child.reply(id, result);
                        child.callbacks.fetch_sub(1, Ordering::SeqCst);
                    });
                } else if let Some(id) = frame["id"].as_str() {
                    if let Some(waiter) = reader.pending.lock().unwrap().remove(id) {
                        let result = if let Some(error) = frame.get("error") {
                            Err(Error {
                                code: error["code"].as_i64().unwrap_or(-32000) as i32,
                                kind: error["data"]["kind"]
                                    .as_str()
                                    .unwrap_or("chat_failed")
                                    .into(),
                                message: error["message"]
                                    .as_str()
                                    .unwrap_or("chat command failed")
                                    .into(),
                            })
                        } else if let Some(result) = frame.get("result") {
                            Ok(result.clone())
                        } else {
                            Err(Error::business("chat_protocol", "invalid chat response"))
                        };
                        let _ = waiter.send(result);
                    }
                } else {
                    break;
                }
            }
            reader.stop();
        });
        Ok(host)
    }
    fn write(&self, value: &V) -> Result<()> {
        let bytes = serde_json::to_vec(value).map_err(|_| Error::invalid("invalid chat frame"))?;
        if bytes.len() > MAX_FRAME {
            return Err(Error::business("too_large", "chat frame exceeds limit"));
        }
        let mut input = self.input.lock().unwrap();
        input.write_all(&bytes)?;
        input.write_all(b"\n")?;
        input.flush()?;
        Ok(())
    }
    fn reply(&self, id: V, result: Result<V>) -> Result<()> {
        self.write(&match result {
            Ok(value) => json!({"jsonrpc":"2.0","id":id,"result":value}),
            Err(e) => json!({"jsonrpc":"2.0","id":id,"error":{"code":e.code,"message":e.message,"data":{"kind":e.kind}}}),
        })
    }
    fn request(&self, method: &str, params: &V) -> Result<V> {
        let id = format!("chat{}", self.next.fetch_add(1, Ordering::SeqCst));
        let (tx, rx) = mpsc::channel();
        {
            let mut pending = self.pending.lock().unwrap();
            if !self.alive.load(Ordering::SeqCst) {
                return Err(Error::business("chat_exited", "chat service exited"));
            }
            if pending.len() >= 64 {
                return Err(Error::business("limit", "too many chat commands"));
            }
            pending.insert(id.clone(), tx);
        }
        let result = self.write(&json!({"jsonrpc":"2.0","id":id,"method":method,"params":params}));
        let result = match result {
            Ok(()) => rx
                .recv_timeout(Duration::from_secs(240))
                .unwrap_or_else(|_| {
                    Err(Error::business(
                        "chat_timeout",
                        "chat command outcome is unknown; refresh state before retrying",
                    ))
                }),
            Err(e) => Err(e),
        };
        self.pending.lock().unwrap().remove(&id);
        result
    }
    fn fail(&self) {
        let mut pending = self.pending.lock().unwrap();
        self.alive.store(false, Ordering::SeqCst);
        for (_, waiter) in pending.drain() {
            let _ = waiter.send(Err(Error::business("chat_exited", "chat service exited")));
        }
    }
    fn stop(&self) {
        if self.alive.swap(false, Ordering::SeqCst) {
            unsafe {
                libc::kill(-self.pid, libc::SIGKILL);
            }
        }
        self.fail();
    }
}
fn allowed_callback(method: &str, params: &V) -> bool {
    match method {
        "hello"
        | "workspace.snapshot"
        | "workspace.watch"
        | "workspace.command"
        | "files.read"
        | "environment.tools.status"
        | "environment.tools.install" => true,
        "documents.read" | "documents.write" | "documents.quarantine" => {
            params["namespace"] == "chat"
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn callbacks_do_not_expose_process_launch_or_recursive_chat() {
        assert!(!allowed_callback("process.spawn", &json!({})));
        assert!(!allowed_callback("chat.command", &json!({})));
        assert!(!allowed_callback(
            "documents.write",
            &json!({"namespace":"services.proxy"})
        ));
        assert!(allowed_callback(
            "documents.write",
            &json!({"namespace":"chat"})
        ));
    }
    #[test]
    fn host_routes_callback_while_request_is_waiting_and_expires_waiters_on_exit() {
        let mut command = Command::new("python3");
        command.arg("-u").arg("-c").arg(
            r#"
import sys,json
request=json.loads(sys.stdin.readline())
print(json.dumps({'jsonrpc':'2.0','id':'r1','method':'workspace.snapshot','params':{}}),flush=True)
callback=json.loads(sys.stdin.readline())
print(json.dumps({'jsonrpc':'2.0','id':request['id'],'result':callback['result']}),flush=True)
sys.stdin.readline()
"#,
        );
        let running = Arc::new(AtomicUsize::new(0));
        let host = Host::spawn(
            command,
            running.clone(),
            Arc::new(|_, _| Ok(json!({"revision":7}))),
        )
        .unwrap();
        assert_eq!(
            host.request("chat.snapshot", &json!({})).unwrap(),
            json!({"revision":7})
        );
        host.stop();
        assert!(host.request("chat.snapshot", &json!({})).is_err());
    }
}
