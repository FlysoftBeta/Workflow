//! Codex App-Server JSON-RPC over JSONL (no `jsonrpc` member on the wire).
//!
//! Two independent ID spaces: our numeric request IDs wait for their responses; the server's
//! request IDs are kept as their exact JSON value and echoed verbatim. A server request is never
//! answered here: the owner decides, and for approvals only the user decides. Responses are routed
//! on the reader thread, so a busy inbound handler can never block a pending request.
use super::channel::{self, LineEvent, StderrTail, Threads, Writer};
use crate::{
    error::{ChatError, ErrorKind, Result},
    model::OpaqueJson,
    ports::AgentStdio,
    wire::{self, Json, Long, Str},
};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex, mpsc},
    time::Duration,
};

pub const METHOD_NOT_FOUND: i64 = -32601;
pub const INVALID_PARAMS: i64 = -32602;
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(240);

pub enum RpcInbound {
    Notification {
        method: String,
        params: Option<OpaqueJson>,
        raw: OpaqueJson,
    },
    Request {
        id: OpaqueJson,
        method: String,
        params: Option<OpaqueJson>,
        raw: OpaqueJson,
    },
    /// A response for an ID we are not waiting on, or a frame that fits no JSON-RPC shape.
    Stray {
        reason: String,
        raw: OpaqueJson,
    },
    Malformed {
        preview: String,
        reason: String,
    },
}

#[derive(Default, Deserialize)]
#[serde(default)]
struct Envelope {
    method: Str,
    id: Json,
    params: Json,
    result: Json,
    error: Json,
}
#[derive(Clone, Default, Deserialize)]
#[serde(default)]
struct ErrorBody {
    code: Long,
    message: Str,
    data: Json,
}
#[derive(Serialize)]
struct Request<'a> {
    id: u64,
    method: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    params: Option<&'a OpaqueJson>,
}
#[derive(Serialize)]
struct Notification<'a> {
    method: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    params: Option<&'a OpaqueJson>,
}
#[derive(Serialize)]
struct Success<'a> {
    id: &'a OpaqueJson,
    result: &'a OpaqueJson,
}
#[derive(Serialize)]
struct Failure<'a> {
    id: &'a OpaqueJson,
    error: FailureBody<'a>,
}
#[derive(Serialize)]
struct FailureBody<'a> {
    code: i64,
    message: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<&'a OpaqueJson>,
}

type Reply = Result<OpaqueJson>;
#[derive(Default)]
struct State {
    next: u64,
    pending_out: HashMap<u64, mpsc::Sender<Reply>>,
    pending_in: HashMap<String, String>,
    closed: Option<String>,
}

pub struct RpcConnection {
    writer: Writer,
    state: Mutex<State>,
    stderr: Arc<StderrTail>,
    threads: Threads,
}
pub type RpcHandler = Box<dyn FnMut(&Arc<RpcConnection>, RpcInbound) + Send>;

impl RpcConnection {
    /// Starts the reader, stderr and dispatcher threads. `handler` sees inbound messages in order.
    pub fn open(stdio: AgentStdio, mut handler: RpcHandler) -> Arc<Self> {
        let connection = Arc::new(Self {
            writer: Writer::new(stdio.stdin),
            state: Mutex::new(State {
                next: 1,
                ..Default::default()
            }),
            stderr: Arc::new(StderrTail::default()),
            threads: Threads::default(),
        });
        let (sender, receiver) = mpsc::sync_channel::<RpcInbound>(channel::INBOUND_CAPACITY);
        let routing = connection.clone();
        let closing = connection.clone();
        connection.threads.add(channel::spawn_reader(
            stdio.stdout,
            move |event| {
                if let Some(inbound) = routing.route(event) {
                    let _ = sender.send(inbound);
                }
            },
            move |failure| closing.close(failure.unwrap_or_else(|| "process closed stdout".into())),
        ));
        connection.threads.add(channel::spawn_stderr(
            stdio.stderr,
            connection.stderr.clone(),
        ));
        let owner = connection.clone();
        connection
            .threads
            .add(channel::spawn_dispatcher(receiver, move |m| {
                handler(&owner, m)
            }));
        connection
    }

    fn route(&self, event: LineEvent) -> Option<RpcInbound> {
        let raw = match event {
            LineEvent::Frame(raw) => raw,
            LineEvent::Malformed { preview, reason } => {
                return Some(RpcInbound::Malformed { preview, reason });
            }
        };
        let frame: Envelope = wire::project(&raw);
        let id = frame.id.0.clone();
        if let Some(method) = frame.method.0.clone() {
            return Some(match id.filter(|id| !wire::is_null(id)) {
                Some(id) => {
                    self.state
                        .lock()
                        .unwrap()
                        .pending_in
                        .insert(wire::text(&id), method.clone());
                    RpcInbound::Request {
                        id,
                        method,
                        params: frame.params.0,
                        raw,
                    }
                }
                None => RpcInbound::Notification {
                    method,
                    params: frame.params.0,
                    raw,
                },
            });
        }
        let Some(id) = id else {
            return Some(RpcInbound::Stray {
                reason: "not a JSON-RPC message".into(),
                raw,
            });
        };
        if frame.result.0.is_none() && frame.error.0.is_none() {
            return Some(RpcInbound::Stray {
                reason: "not a JSON-RPC message".into(),
                raw,
            });
        }
        // Our IDs are JSON numbers; a string ID can never be ours.
        let ours = wire::project::<Long>(&id)
            .0
            .and_then(|n| u64::try_from(n).ok());
        let waiter = ours.and_then(|n| self.state.lock().unwrap().pending_out.remove(&n));
        let Some(waiter) = waiter else {
            return Some(RpcInbound::Stray {
                reason: format!("response for unknown id {}", wire::text(&id)),
                raw,
            });
        };
        let reply = match frame.error.object().map(wire::project::<ErrorBody>) {
            Some(error) => Err(ChatError::vendor(
                error.code.get().unwrap_or(-32000),
                error.message.or("error"),
                error.data.0,
            )),
            None => Ok(frame
                .result
                .0
                .unwrap_or(OpaqueJson(serde_json::Value::Null))),
        };
        let _ = waiter.send(reply);
        None
    }

    fn close(&self, reason: String) {
        let mut state = self.state.lock().unwrap();
        if state.closed.is_none() {
            state.closed = Some(reason.clone());
        }
        for (_, waiter) in state.pending_out.drain() {
            let _ = waiter.send(Err(ChatError::new(ErrorKind::Closed, reason.clone())));
        }
        state.pending_in.clear();
    }

    /// Sends a request and blocks until its response, the process exits or the timeout passes.
    pub fn request(&self, method: &str, params: Option<OpaqueJson>) -> Result<OpaqueJson> {
        self.request_timeout(method, params, DEFAULT_TIMEOUT)
    }
    pub fn request_timeout(
        &self,
        method: &str,
        params: Option<OpaqueJson>,
        timeout: Duration,
    ) -> Result<OpaqueJson> {
        let (sender, receiver) = mpsc::channel();
        let id = {
            let mut state = self.state.lock().unwrap();
            if let Some(reason) = &state.closed {
                return Err(ChatError::new(ErrorKind::Closed, reason.clone()));
            }
            let id = state.next;
            state.next += 1;
            state.pending_out.insert(id, sender);
            id
        };
        let sent = self.writer.send(&Request {
            id,
            method,
            params: params.as_ref(),
        });
        let result = match sent {
            Ok(()) => receiver.recv_timeout(timeout).unwrap_or_else(|e| match e {
                mpsc::RecvTimeoutError::Timeout => Err(ChatError::new(
                    ErrorKind::Timeout,
                    format!("{method} was not answered in time"),
                )),
                mpsc::RecvTimeoutError::Disconnected => {
                    Err(ChatError::new(ErrorKind::Closed, "connection closed"))
                }
            }),
            Err(e) => Err(e),
        };
        self.state.lock().unwrap().pending_out.remove(&id);
        result
    }

    pub fn notify(&self, method: &str, params: Option<OpaqueJson>) -> Result<()> {
        self.writer.send(&Notification {
            method,
            params: params.as_ref(),
        })
    }

    fn take(&self, id: &OpaqueJson) -> Result<()> {
        self.state
            .lock()
            .unwrap()
            .pending_in
            .remove(&wire::text(id))
            .map(|_| ())
            .ok_or_else(|| {
                ChatError::new(
                    ErrorKind::RequestExpired,
                    format!("no open server request with id {}", wire::text(id)),
                )
            })
    }

    /// Answers server request `id` verbatim. Fails if the ID is not an open server request.
    pub fn respond(&self, id: &OpaqueJson, result: &OpaqueJson) -> Result<()> {
        self.take(id)?;
        self.writer.send(&Success { id, result })
    }

    pub fn respond_error(
        &self,
        id: &OpaqueJson,
        code: i64,
        message: &str,
        data: Option<&OpaqueJson>,
    ) -> Result<()> {
        self.take(id)?;
        self.writer.send(&Failure {
            id,
            error: FailureBody {
                code,
                message,
                data,
            },
        })
    }

    /// The server resolved request `id` itself; it must not be answered anymore.
    pub fn forget(&self, id: &OpaqueJson) {
        self.state
            .lock()
            .unwrap()
            .pending_in
            .remove(&wire::text(id));
    }

    /// Exact JSON text of the server requests not answered yet.
    pub fn open_server_requests(&self) -> Vec<String> {
        let mut ids: Vec<_> = self
            .state
            .lock()
            .unwrap()
            .pending_in
            .keys()
            .cloned()
            .collect();
        ids.sort();
        ids
    }

    pub fn closed_reason(&self) -> Option<String> {
        self.state.lock().unwrap().closed.clone()
    }
    pub fn stderr_tail(&self, chars: usize) -> String {
        self.stderr.last(chars)
    }
    /// Closes stdin; reading continues until the child closes stdout.
    pub fn close_input(&self) {
        self.writer.close();
    }
    /// Waits for the reader, stderr and dispatcher threads to finish.
    pub fn join(&self) {
        self.threads.join();
    }
}
