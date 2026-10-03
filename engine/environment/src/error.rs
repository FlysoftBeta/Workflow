//! Domain failures. The Server maps these to its protocol error envelope.
use std::{fmt, io};
#[derive(Debug, Clone)]
pub struct Error {
    pub kind: String,
    pub message: String,
}
pub type Result<T> = std::result::Result<T, Error>;
impl Error {
    pub fn invalid(message: &str) -> Self {
        Self {
            kind: "invalid_params".into(),
            message: message.into(),
        }
    }
    pub fn business(kind: &str, message: &str) -> Self {
        Self {
            kind: kind.into(),
            message: message.into(),
        }
    }
}
impl From<io::Error> for Error {
    fn from(error: io::Error) -> Self {
        Self::business("io", &error.to_string())
    }
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.kind, self.message)
    }
}
impl std::error::Error for Error {}
