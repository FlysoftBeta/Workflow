//! Server consumes the public Rust Chat API. Keep these cases active in the
//! Server suite while the original JVM service remains the production adapter.
#[path = "../../chat/tests/parity.rs"]
mod parity;
#[path = "../../chat/tests/service.rs"]
mod service;
