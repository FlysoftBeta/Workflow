use crate::model::OpaqueJson;
use std::fmt;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ErrorKind {
    InvalidArgument,
    NotFound,
    BackendUnavailable,
    RequestExpired,
    DecisionNotOffered,
    SendConflict,
    SendAmbiguous,
    /// A turn is running and the operation needs an idle conversation.
    TurnActive,
    Vendor,
    /// The operation is not valid in the current state (Kotlin `check` failures).
    State,
    Store,
    UnsupportedFormat,
    Timeout,
    Closed,
}
/// A failure returned to the requester. Messages may quote vendor text, so they are never logged.
#[derive(Clone, Debug, PartialEq)]
pub struct ChatError {
    pub kind: ErrorKind,
    pub message: String,
    /// The vendor's JSON-RPC error code and data, when the vendor rejected a request.
    pub code: Option<i64>,
    pub data: Option<OpaqueJson>,
}
pub type Result<T> = std::result::Result<T, ChatError>;
impl ChatError {
    pub fn new(kind: ErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
            code: None,
            data: None,
        }
    }
    pub fn vendor(code: i64, message: impl Into<String>, data: Option<OpaqueJson>) -> Self {
        Self {
            kind: ErrorKind::Vendor,
            message: message.into(),
            code: Some(code),
            data,
        }
    }
    pub fn invalid(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::InvalidArgument, message)
    }
    pub fn unavailable(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::BackendUnavailable, message)
    }
    pub fn state(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::State, message)
    }
}
impl fmt::Display for ChatError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(&self.message)
    }
}
impl std::error::Error for ChatError {}
impl From<workflow_environment::Error> for ChatError {
    fn from(e: workflow_environment::Error) -> Self {
        Self::new(ErrorKind::Store, e.message)
    }
}
impl From<serde_json::Error> for ChatError {
    fn from(e: serde_json::Error) -> Self {
        Self::new(ErrorKind::InvalidArgument, e.to_string())
    }
}
impl From<std::io::Error> for ChatError {
    fn from(e: std::io::Error) -> Self {
        Self::new(ErrorKind::Closed, e.to_string())
    }
}
