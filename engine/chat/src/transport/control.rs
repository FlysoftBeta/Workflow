//! Claude Code stream-json plus its control protocol, one JSON object per line in both directions.
//!
//! Our `control_request`s carry our own `request_id`; only the matching `control_response`
//! completes them. The CLI echoes our own responses on stdout, so a response for an ID we are not
//! waiting on is ignored and counted. CLI `control_request`s keep their `request_id` verbatim and
//! are answered only through `respond` or `respond_error` by the owner. A
//! `control_cancel_request` withdraws one; `keep_alive` is dropped.
use super::channel::{self, LineEvent, StderrTail, Threads, Writer};
use crate::{
    error::{ChatError, ErrorKind, Result},
    model::{OpaqueJson, OpaqueObject},
    ports::AgentStdio,
    wire::{self, Json, Str},
};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
        mpsc,
    },
    time::Duration,
};

pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(240);

pub enum ControlInbound {
    Message {
        kind: String,
        raw: OpaqueJson,
    },
    Request {
        id: String,
        subtype: String,
        request: OpaqueJson,
        raw: OpaqueJson,
    },
    Cancel {
        id: String,
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
    #[serde(rename = "type")]
    kind: Str,
    request_id: Str,
    request: Json,
    response: Json,
}
#[derive(Default, Deserialize)]
#[serde(default)]
struct ResponseBody {
    request_id: Str,
    subtype: Str,
    error: Str,
    response: Json,
}
#[derive(Default, Deserialize)]
#[serde(default)]
struct Subtype {
    subtype: Str,
}
#[derive(Serialize)]
struct RequestBody<'a> {
    subtype: &'a str,
    #[serde(flatten)]
    fields: &'a OpaqueObject,
}
#[derive(Serialize)]
struct Outgoing<'a> {
    #[serde(rename = "type")]
    kind: &'static str,
    request_id: &'a str,
    request: RequestBody<'a>,
}
#[derive(Serialize)]
struct Answer<'a> {
    subtype: &'static str,
    request_id: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    response: Option<&'a OpaqueJson>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<&'a str>,
}
#[derive(Serialize)]
struct AnswerFrame<'a> {
    #[serde(rename = "type")]
    kind: &'static str,
    response: Answer<'a>,
}

type Reply = Result<OpaqueJson>;
#[derive(Default)]
struct State {
    pending_out: HashMap<String, mpsc::Sender<Reply>>,
    pending_in: HashMap<String, String>,
    closed: Option<String>,
}

pub struct ControlConnection {
    writer: Writer,
    state: Mutex<State>,
    ignored: AtomicU64,
    stderr: Arc<StderrTail>,
    threads: Threads,
    ids: Box<dyn Fn() -> String + Send + Sync>,
}
pub type ControlHandler = Box<dyn FnMut(&Arc<ControlConnection>, ControlInbound) + Send>;

/// `wf_<random>_<n>`: our control request IDs never collide with the CLI's.
pub fn default_ids() -> Box<dyn Fn() -> String + Send + Sync> {
    let prefix = uuid::Uuid::new_v4().to_string()[..8].to_string();
    let counter = AtomicU64::new(0);
    Box::new(move || format!("wf_{prefix}_{}", counter.fetch_add(1, Ordering::SeqCst) + 1))
}

impl ControlConnection {
    pub fn open(
        stdio: AgentStdio,
        ids: Box<dyn Fn() -> String + Send + Sync>,
        mut handler: ControlHandler,
    ) -> Arc<Self> {
        let connection = Arc::new(Self {
            writer: Writer::new(stdio.stdin),
            state: Mutex::new(State::default()),
            ignored: AtomicU64::new(0),
            stderr: Arc::new(StderrTail::default()),
            threads: Threads::default(),
            ids,
        });
        let (sender, receiver) = mpsc::sync_channel::<ControlInbound>(channel::INBOUND_CAPACITY);
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
            .add(channel::spawn_dispatcher(receiver, move |m| handler(&owner, m)));
        connection
    }

    fn route(&self, event: LineEvent) -> Option<ControlInbound> {
        let raw = match event {
            LineEvent::Frame(raw) => raw,
            LineEvent::Malformed { preview, reason } => {
                return Some(ControlInbound::Malformed { preview, reason });
            }
        };
        let frame: Envelope = wire::project(&raw);
        let kind = frame.kind.or("");
        match kind.as_str() {
            "control_response" => {
                let response: Option<ResponseBody> = frame.response.object().map(wire::project);
                let waiter = response.as_ref().and_then(|r| {
                    r.request_id
                        .get()
                        .and_then(|id| self.state.lock().unwrap().pending_out.remove(id))
                });
                match (response, waiter) {
                    (Some(response), Some(waiter)) => {
                        let reply = if response.subtype.get() == Some("error") {
                            Err(ChatError::new(
                                ErrorKind::Vendor,
                                response.error.or("control request failed"),
                            ))
                        } else {
                            Ok(response
                                .response
                                .object()
                                .cloned()
                                .unwrap_or_else(wire::empty_object))
                        };
                        let _ = waiter.send(reply);
                    }
                    _ => {
                        self.ignored.fetch_add(1, Ordering::SeqCst);
                    }
                }
                None
            }
            "control_request" => {
                let request = frame.request.object().cloned();
                let subtype = request
                    .as_ref()
                    .and_then(|r| wire::project::<Subtype>(r).subtype.owned());
                match (frame.request_id.owned(), request, subtype) {
                    (Some(id), Some(request), Some(subtype)) => {
                        self.state
                            .lock()
                            .unwrap()
                            .pending_in
                            .insert(id.clone(), subtype.clone());
                        Some(ControlInbound::Request {
                            id,
                            subtype,
                            request,
                            raw,
                        })
                    }
                    _ => Some(ControlInbound::Message {
                        kind: "control_request".into(),
                        raw,
                    }),
                }
            }
            "control_cancel_request" => match frame.request_id.owned() {
                Some(id) => {
                    self.state.lock().unwrap().pending_in.remove(&id);
                    Some(ControlInbound::Cancel { id, raw })
                }
                None => Some(ControlInbound::Message {
                    kind: "control_cancel_request".into(),
                    raw,
                }),
            },
            "keep_alive" => None,
            _ => Some(ControlInbound::Message { kind, raw }),
        }
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

    /// Sends `control_request{subtype, ...fields}` and returns the success `response` object.
    pub fn control(&self, subtype: &str, fields: &OpaqueObject) -> Result<OpaqueJson> {
        self.control_timeout(subtype, fields, DEFAULT_TIMEOUT)
    }
    pub fn control_timeout(
        &self,
        subtype: &str,
        fields: &OpaqueObject,
        timeout: Duration,
    ) -> Result<OpaqueJson> {
        let id = (self.ids)();
        let (sender, receiver) = mpsc::channel();
        {
            let mut state = self.state.lock().unwrap();
            if let Some(reason) = &state.closed {
                return Err(ChatError::new(ErrorKind::Closed, reason.clone()));
            }
            state.pending_out.insert(id.clone(), sender);
        }
        let sent = self.writer.send(&Outgoing {
            kind: "control_request",
            request_id: &id,
            request: RequestBody { subtype, fields },
        });
        let result = match sent {
            Ok(()) => receiver.recv_timeout(timeout).unwrap_or_else(|e| match e {
                mpsc::RecvTimeoutError::Timeout => Err(ChatError::new(
                    ErrorKind::Timeout,
                    format!("{subtype} was not answered in time"),
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

    /// Writes a stream-json message such as a `user` prompt.
    pub fn send(&self, message: &OpaqueJson) -> Result<()> {
        self.writer.send(message)
    }

    fn take(&self, id: &str) -> Result<()> {
        self.state
            .lock()
            .unwrap()
            .pending_in
            .remove(id)
            .map(|_| ())
            .ok_or_else(|| {
                ChatError::new(
                    ErrorKind::RequestExpired,
                    format!("no open control request {id}"),
                )
            })
    }

    pub fn respond(&self, id: &str, response: Option<&OpaqueJson>) -> Result<()> {
        self.take(id)?;
        self.writer.send(&AnswerFrame {
            kind: "control_response",
            response: Answer {
                subtype: "success",
                request_id: id,
                response,
                error: None,
            },
        })
    }

    pub fn respond_error(&self, id: &str, error: &str) -> Result<()> {
        self.take(id)?;
        self.writer.send(&AnswerFrame {
            kind: "control_response",
            response: Answer {
                subtype: "error",
                request_id: id,
                response: None,
                error: Some(error),
            },
        })
    }

    /// Forgets an inbound request without answering it (an undeclared `request_user_dialog`).
    pub fn forget(&self, id: &str) {
        self.state.lock().unwrap().pending_in.remove(id);
    }
    pub fn open_inbound_requests(&self) -> Vec<String> {
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
    pub fn ignored_responses(&self) -> u64 {
        self.ignored.load(Ordering::SeqCst)
    }
    pub fn closed_reason(&self) -> Option<String> {
        self.state.lock().unwrap().closed.clone()
    }
    pub fn stderr_tail(&self, chars: usize) -> String {
        self.stderr.last(chars)
    }
    pub fn close_input(&self) {
        self.writer.close();
    }
    pub fn join(&self) {
        self.threads.join();
    }
}

