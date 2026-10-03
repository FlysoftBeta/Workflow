//! Claude control directions. Echoes cannot complete unrelated outgoing requests.
use super::raw::{RawJson, present};
use crate::error::{ChatError, ErrorKind, Result};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    sync::mpsc::{self, Receiver, Sender},
};
#[derive(Deserialize)]
struct Envelope {
    #[serde(rename = "type")]
    kind: Option<String>,
    request_id: Option<String>,
    request: Option<RawJson>,
    response: Option<ControlResponse>,
}
#[derive(Deserialize)]
struct Subtype {
    subtype: String,
}
#[derive(Deserialize)]
struct ControlResponse {
    request_id: Option<String>,
    subtype: Option<String>,
    error: Option<String>,
    #[serde(default, deserialize_with = "present")]
    response: Option<RawJson>,
}
#[derive(Clone, Debug)]
pub enum Inbound {
    Request {
        id: String,
        subtype: String,
        request: RawJson,
        raw: RawJson,
    },
    Cancel {
        id: String,
        raw: RawJson,
    },
    Message {
        kind: String,
        raw: RawJson,
    },
}
#[derive(Serialize)]
struct Outgoing<'a> {
    #[serde(rename = "type")]
    kind: &'static str,
    request_id: &'a str,
    request: &'a RawJson,
}
#[derive(Serialize)]
struct ResponseBody<'a> {
    subtype: &'static str,
    request_id: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    response: Option<&'a RawJson>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<&'a str>,
}
#[derive(Serialize)]
struct Response<'a> {
    #[serde(rename = "type")]
    kind: &'static str,
    response: ResponseBody<'a>,
}
#[derive(Default)]
pub struct ControlRouter {
    pending_out: BTreeMap<String, Sender<Result<RawJson>>>,
    pending_in: BTreeMap<String, String>,
    pub ignored_responses: u64,
    closed: bool,
}
impl ControlRouter {
    pub fn begin(
        &mut self,
        id: &str,
        request: &RawJson,
    ) -> Result<(RawJson, Receiver<Result<RawJson>>)> {
        if self.closed {
            return Err(ChatError::new(ErrorKind::Closed, "connection closed"));
        }
        if self.pending_out.contains_key(id) {
            return Err(ChatError::new(
                ErrorKind::InvalidArgument,
                "duplicate control ID",
            ));
        }
        let (send, receive) = mpsc::channel();
        self.pending_out.insert(id.into(), send);
        Ok((
            RawJson::encode(&Outgoing {
                kind: "control_request",
                request_id: id,
                request,
            })?,
            receive,
        ))
    }
    pub fn cancel_outgoing(&mut self, id: &str) {
        self.pending_out.remove(id);
    }
    pub fn route(&mut self, raw: RawJson) -> Result<Option<Inbound>> {
        let Ok(frame) = serde_json::from_str::<Envelope>(raw.text()) else {
            return Ok(Some(Inbound::Message {
                kind: String::new(),
                raw,
            }));
        };
        let kind = frame.kind.unwrap_or_default();
        match kind.as_str() {
            "control_response" => {
                if let Some(response) = frame.response {
                    if let Some(waiter) = response
                        .request_id
                        .and_then(|id| self.pending_out.remove(&id))
                    {
                        let result = if response.subtype.as_deref() == Some("error") {
                            Err(ChatError::new(
                                ErrorKind::Vendor,
                                response
                                    .error
                                    .unwrap_or_else(|| "control request failed".into()),
                            )
                            .with_vendor(raw))
                        } else {
                            Ok(response.response.unwrap_or(RawJson::parse("{}")?))
                        };
                        let _ = waiter.send(result);
                        return Ok(None);
                    }
                }
                self.ignored_responses += 1;
                Ok(None)
            }
            "control_request" => {
                if let (Some(id), Some(request)) = (frame.request_id, frame.request) {
                    if let Ok(Subtype { subtype }) = request.decode() {
                        self.pending_in.insert(id.clone(), subtype.clone());
                        return Ok(Some(Inbound::Request {
                            id,
                            subtype,
                            request,
                            raw,
                        }));
                    }
                }
                Ok(Some(Inbound::Message { kind, raw }))
            }
            "control_cancel_request" => {
                if let Some(id) = frame.request_id {
                    self.pending_in.remove(&id);
                    Ok(Some(Inbound::Cancel { id, raw }))
                } else {
                    Ok(Some(Inbound::Message { kind, raw }))
                }
            }
            "keep_alive" => Ok(None),
            _ => Ok(Some(Inbound::Message { kind, raw })),
        }
    }
    pub fn open_requests(&self) -> Vec<String> {
        self.pending_in.keys().cloned().collect()
    }
    pub fn forget(&mut self, id: &str) {
        self.pending_in.remove(id);
    }
    fn take(&mut self, id: &str) -> Result<()> {
        self.pending_in.remove(id).map(|_| ()).ok_or_else(|| {
            ChatError::new(
                ErrorKind::RequestExpired,
                "control request is no longer open",
            )
        })
    }
    pub fn respond(&mut self, id: &str, payload: Option<&RawJson>) -> Result<RawJson> {
        self.take(id)?;
        RawJson::encode(&Response {
            kind: "control_response",
            response: ResponseBody {
                subtype: "success",
                request_id: id,
                response: payload,
                error: None,
            },
        })
    }
    pub fn reject(&mut self, id: &str, error: &str) -> Result<RawJson> {
        self.take(id)?;
        RawJson::encode(&Response {
            kind: "control_response",
            response: ResponseBody {
                subtype: "error",
                request_id: id,
                response: None,
                error: Some(error),
            },
        })
    }
    pub fn close(&mut self) {
        self.closed = true;
        for (_, waiter) in std::mem::take(&mut self.pending_out) {
            let _ = waiter.send(Err(ChatError::new(
                ErrorKind::Closed,
                "process closed stdout",
            )));
        }
        self.pending_in.clear();
    }
}
