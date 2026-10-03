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
    Vendor,
    Store,
    UnsupportedFormat,
    Closed,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChatError {
    pub kind: ErrorKind,
    pub message: String,
}
pub type Result<T> = std::result::Result<T, ChatError>;
impl ChatError {
    pub fn new(kind: ErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
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
