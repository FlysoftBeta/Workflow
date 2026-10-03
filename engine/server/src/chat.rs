//! Temporary round-1 bridge to the unchanged Kotlin service. Runtime owns its process.
use crate::protocol::{
    self, Error, MAX_FRAME, OpaqueJson, OpaqueObject, Request, Response, Result, RpcError, RpcId,
    Version, line,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    io::{BufReader, Read, Write},
    process::ChildStdin,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering},
        mpsc,
    },
    time::Duration,
};
use workflow_environment::{
    Environment,
    runtime::{self, CommandSpec, PipedProcess, ProcessHandle},
};
type Callback = Arc<dyn Fn(&str, &OpaqueObject) -> Result<OpaqueJson> + Send + Sync>;
#[derive(Default)]
pub struct Chat {
    host: Mutex<Option<Arc<Host>>>,
}
impl Chat {
    pub fn ensure(
        &self,
        environment: &Arc<Mutex<Environment>>,
        callback: Callback,
    ) -> Result<Arc<Host>> {
        let mut slot = self.host.lock().unwrap();
        if slot
            .as_ref()
            .is_none_or(|h| !h.alive.load(Ordering::SeqCst))
        {
            let request = CommandSpec {
                argv: [
                    "/opt/workflow/tools/jre/bin/java",
                    "-Xms16m",
                    "-Xmx256m",
                    "-XX:ActiveProcessorCount=2",
                    "-XX:+UseSerialGC",
                    "-XX:-UsePerfData",
                    "-jar",
                    "/opt/workflow/tools/chat/workflow-chat.jar",
                ]
                .map(String::from)
                .to_vec(),
                cwd: "/workspace".into(),
                env: Default::default(),
            };
            *slot = Some(Host::spawn(
                runtime::spawn_piped(environment, &request)?,
                callback,
            )?);
        }
        Ok(slot.as_ref().unwrap().clone())
    }
    pub fn stop(&self) {
        if let Some(host) = self.host.lock().unwrap().take() {
            host.stop();
        }
    }
}
fn present_payload<'de, D: serde::Deserializer<'de>>(
    d: D,
) -> std::result::Result<Option<OpaqueJson>, D::Error> {
    OpaqueJson::deserialize(d).map(Some)
}
#[derive(Deserialize)]
struct PrivateFrame {
    jsonrpc: Version,
    id: Option<String>,
    method: Option<String>,
    #[serde(default)]
    params: OpaqueObject,
    #[serde(default, deserialize_with = "present_payload")]
    result: Option<OpaqueJson>,
    error: Option<RpcError>,
    #[serde(flatten)]
    _extra: OpaqueObject,
}
pub(crate) struct Host {
    input: Mutex<ChildStdin>,
    pending: Mutex<HashMap<String, mpsc::Sender<Result<OpaqueJson>>>>,
    next: AtomicU64,
    alive: AtomicBool,
    callbacks: AtomicUsize,
    process: ProcessHandle,
}
impl Host {
    fn spawn(process: PipedProcess, callback: Callback) -> Result<Arc<Self>> {
        let PipedProcess {
            stdin,
            stdout,
            mut stderr,
            handle,
        } = process;
        let host = Arc::new(Self {
            input: Mutex::new(stdin),
            pending: Mutex::new(HashMap::new()),
            next: AtomicU64::new(0),
            alive: AtomicBool::new(true),
            callbacks: AtomicUsize::new(0),
            process: handle,
        });
        // Drain without retaining credentials or printing backend diagnostics.
        std::thread::spawn(move || {
            let mut b = [0u8; 8192];
            while stderr.read(&mut b).is_ok_and(|n| n != 0) {}
        });
        let reader = host.clone();
        std::thread::spawn(move || {
            let mut input = BufReader::new(stdout);
            while let Ok(Some(bytes)) = line(&mut input) {
                let frame: PrivateFrame = match protocol::parse(&bytes) {
                    Ok(v) => v,
                    Err(_) => break,
                };
                let _ = frame.jsonrpc;
                let Some(id) = frame.id else { break };
                if let Some(method) = frame.method {
                    if reader.callbacks.fetch_add(1, Ordering::SeqCst) >= 64 {
                        reader.callbacks.fetch_sub(1, Ordering::SeqCst);
                        let _ = reader.reply(
                            &id,
                            Err(Error::business("limit", "too many chat callbacks")),
                        );
                        continue;
                    }
                    let child = reader.clone();
                    let callback = callback.clone();
                    std::thread::spawn(move || {
                        let result = if allowed_callback(&method, &frame.params) {
                            callback(&method, &frame.params)
                        } else {
                            Err(protocol::unknown_method())
                        };
                        let _ = child.reply(&id, result);
                        child.callbacks.fetch_sub(1, Ordering::SeqCst);
                    });
                } else if let Some(waiter) = reader.pending.lock().unwrap().remove(&id) {
                    let result = match (frame.error, frame.result) {
                        (Some(error), None) => Err(error.into()),
                        (None, Some(result)) => Ok(result),
                        _ => Err(Error::business("chat_protocol", "invalid chat response")),
                    };
                    let _ = waiter.send(result);
                }
            }
            reader.stop();
        });
        Ok(host)
    }
    fn write(&self, value: &impl Serialize) -> Result<()> {
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
    fn reply(&self, id: &str, result: Result<OpaqueJson>) -> Result<()> {
        self.write(&Response::new(RpcId::string(id), result))
    }
    pub fn request(&self, method: &str, params: &OpaqueObject) -> Result<OpaqueJson> {
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
        let result = self.write(&Request {
            jsonrpc: Version::V2,
            id: Some(RpcId::string(&id)),
            method: method.into(),
            params: params.clone(),
            extra: Default::default(),
        });
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
            let _ = self.process.stop(true);
        }
        self.fail();
    }
}
fn allowed_callback(method: &str, params: &OpaqueObject) -> bool {
    match method {
        "hello"
        | "workspace.snapshot"
        | "workspace.watch"
        | "workspace.command"
        | "files.read"
        | "environment.tools.status"
        | "environment.tools.install" => true,
        "documents.read" | "documents.write" | "documents.quarantine" => {
            protocol::params::<protocol::DocumentParams>(params)
                .is_ok_and(|p| p.namespace == "chat")
        }
        _ => false,
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn callbacks_do_not_expose_process_launch_or_recursive_chat() {
        let params = protocol::object(&protocol::DocumentParams {
            namespace: "chat".into(),
            key: "index".into(),
            document: None,
            expected_revision: None,
            extra: Default::default(),
        })
        .unwrap();
        assert!(allowed_callback("documents.write", &params));
        assert!(!allowed_callback("process.spawn", &params));
        assert!(!allowed_callback("chat.command", &params));
    }
    #[test]
    fn callback_while_request_waits_and_waiter_expires_on_exit() {
        let mut command = std::process::Command::new("python3");
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
        let process = runtime::spawn_fixture(command, Arc::new(AtomicUsize::new(0))).unwrap();
        let host = Host::spawn(
            process,
            Arc::new(|_, _| protocol::opaque(&protocol::Revision { revision: 7 })),
        )
        .unwrap();
        let result = host
            .request("chat.snapshot", &OpaqueObject::default())
            .unwrap();
        let revision: protocol::Revision =
            serde_json::from_slice(&serde_json::to_vec(&result).unwrap()).unwrap();
        assert_eq!(revision.revision, 7);
        host.stop();
        assert!(host.request("chat.snapshot", &Default::default()).is_err());
    }
}
