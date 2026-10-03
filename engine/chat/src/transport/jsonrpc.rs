//! Separate inbound/outbound ID spaces. Routing never writes an approval response.
use super::raw::{RawJson, RequestId, present};
use crate::error::{ChatError, ErrorKind, Result};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    sync::mpsc::{self, Receiver, Sender},
};
#[derive(Deserialize)]
struct Envelope {
    id: Option<RequestId>,
    method: Option<String>,
    params: Option<RawJson>,
    #[serde(default, deserialize_with = "present")]
    result: Option<RawJson>,
    error: Option<VendorError>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VendorError {
    pub code: i64,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<RawJson>,
}
#[derive(Clone, Debug)]
pub enum Inbound {
    Request {
        id: RequestId,
        method: String,
        params: Option<RawJson>,
        raw: RawJson,
    },
    Notification {
        method: String,
        params: Option<RawJson>,
        raw: RawJson,
    },
    Stray {
        raw: RawJson,
    },
}
#[derive(Serialize)]
struct Request<'a> {
    id: RequestId,
    method: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    params: Option<&'a RawJson>,
}
#[derive(Serialize)]
struct Success<'a> {
    id: &'a RequestId,
    result: &'a RawJson,
}
#[derive(Serialize)]
struct Failure<'a> {
    id: &'a RequestId,
    error: VendorError,
}
#[derive(Default)]
pub struct RpcRouter {
    next: u64,
    pending_out: BTreeMap<String, Sender<Result<RawJson>>>,
    pending_in: BTreeMap<String, String>,
    closed: bool,
}
impl RpcRouter {
    pub fn begin(
        &mut self,
        method: &str,
        params: Option<&RawJson>,
    ) -> Result<(RequestId, RawJson, Receiver<Result<RawJson>>)> {
        if self.closed {
            return Err(ChatError::new(ErrorKind::Closed, "connection closed"));
        }
        self.next = self
            .next
            .checked_add(1)
            .ok_or_else(|| ChatError::new(ErrorKind::Closed, "request ID exhausted"))?;
        let id = RequestId::number(self.next);
        let (send, receive) = mpsc::channel();
        self.pending_out.insert(id.text().into(), send);
        let frame = RawJson::encode(&Request {
            id: id.clone(),
            method,
            params,
        })?;
        Ok((id, frame, receive))
    }
    pub fn cancel_outgoing(&mut self, id: &RequestId) {
        self.pending_out.remove(id.text());
    }
    pub fn route(&mut self, raw: RawJson) -> Result<Option<Inbound>> {
        let Ok(frame) = serde_json::from_str::<Envelope>(raw.text()) else {
            return Ok(Some(Inbound::Stray { raw }));
        };
        if let Some(method) = frame.method {
            return Ok(Some(if let Some(id) = frame.id {
                self.pending_in.insert(id.text().into(), method.clone());
                Inbound::Request {
                    id,
                    method,
                    params: frame.params,
                    raw,
                }
            } else {
                Inbound::Notification {
                    method,
                    params: frame.params,
                    raw,
                }
            }));
        }
        if let Some(id) = frame
            .id
            .filter(|_| frame.result.is_some() || frame.error.is_some())
        {
            if let Some(waiter) = self.pending_out.remove(id.text()) {
                let reply = match frame.error {
                    Some(error) => {
                        Err(ChatError::new(ErrorKind::Vendor, error.message).with_vendor(raw))
                    }
                    None => Ok(frame.result.unwrap_or_else(RawJson::null)),
                };
                let _ = waiter.send(reply);
                return Ok(None);
            }
        }
        Ok(Some(Inbound::Stray { raw }))
    }
    pub fn open_requests(&self) -> Vec<String> {
        self.pending_in.keys().cloned().collect()
    }
    pub fn forget(&mut self, id: &RequestId) {
        self.pending_in.remove(id.text());
    }
    fn take(&mut self, id: &RequestId) -> Result<()> {
        self.pending_in
            .remove(id.text())
            .map(|_| ())
            .ok_or_else(|| ChatError::new(ErrorKind::RequestExpired, "request is no longer open"))
    }
    /// Called only for an explicit user decision or a separately allowlisted factual/negative capability answer.
    pub fn respond(&mut self, id: &RequestId, result: &RawJson) -> Result<RawJson> {
        self.take(id)?;
        RawJson::encode(&Success { id, result })
    }
    pub fn reject(&mut self, id: &RequestId, code: i64, message: &str) -> Result<RawJson> {
        self.take(id)?;
        RawJson::encode(&Failure {
            id,
            error: VendorError {
                code,
                message: message.into(),
                data: None,
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
