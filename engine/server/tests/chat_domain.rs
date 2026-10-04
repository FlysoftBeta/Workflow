//! Server composes the public Rust Chat API in production; keep its domain cases in the Server
//! suite too.
#[path = "../../chat/tests/parity.rs"]
mod parity;
#[path = "../../chat/tests/service.rs"]
mod service;
#[path = "../../chat/tests/transport.rs"]
mod transport;
