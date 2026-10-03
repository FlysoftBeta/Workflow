//! Local stdio transport; remote transports remain abstractions for a later request.
use crate::{
    Server,
    protocol::{self, Error, HelloParams, Request, Result, RpcId, line, response},
};
use serde::Deserialize;
use std::{
    io,
    sync::{Arc, Mutex, atomic::Ordering},
};
#[derive(Deserialize)]
struct IdProbe {
    #[serde(default)]
    id: Option<Box<serde_json::value::RawValue>>,
}
fn error(code: i32, kind: &str, message: &str) -> Error {
    Error {
        code,
        kind: kind.into(),
        message: message.into(),
    }
}
fn reject(writer: &Arc<Mutex<io::Stdout>>, id: RpcId, err: Error) {
    response::<()>(writer, id, Err(err));
}
pub fn run(server: &Arc<Server>) -> Result<()> {
    let writer = Arc::new(Mutex::new(io::stdout()));
    let mut reader = io::BufReader::new(io::stdin());
    let mut hello = false;
    loop {
        let bytes = match line(&mut reader) {
            Ok(Some(bytes)) => bytes,
            Ok(None) => break,
            Err(_) => {
                reject(
                    &writer,
                    RpcId::null(),
                    error(-32700, "frame_too_large", "JSON frame exceeds limit"),
                );
                break;
            }
        };
        if protocol::strict_json::<serde::de::IgnoredAny>(&bytes).is_err() {
            reject(
                &writer,
                RpcId::null(),
                error(-32700, "parse_error", "Invalid JSON"),
            );
            if !hello {
                break;
            }
            continue;
        }
        let request = protocol::strict_json::<Request>(&bytes);
        let request = match request {
            Ok(request) if request.id.as_ref().is_none_or(RpcId::valid) => request,
            _ => {
                let id = protocol::strict_json::<IdProbe>(&bytes)
                    .ok()
                    .and_then(|v| v.id)
                    .map(RpcId)
                    .filter(RpcId::valid)
                    .unwrap_or_else(RpcId::null);
                reject(
                    &writer,
                    id,
                    error(-32600, "invalid_request", "Invalid JSON-RPC request"),
                );
                if !hello {
                    break;
                }
                continue;
            }
        };
        let Request {
            id, method, params, ..
        } = request;
        if !hello {
            let greeting = protocol::params::<HelloParams>(&params).ok();
            if method != "hello"
                || id.is_none()
                || greeting.is_none_or(|p| {
                    p.protocol != protocol::PROTOCOL || p.client_id.trim().is_empty()
                })
            {
                reject(
                    &writer,
                    id.unwrap_or_else(RpcId::null),
                    Error::business(
                        "protocol_mismatch",
                        "first request must be hello with exact workflow.workspace/1 protocol and clientId",
                    ),
                );
                break;
            }
            hello = true;
            response(&writer, id.unwrap(), Ok(server.hello()));
            continue;
        }
        if method == "hello" {
            if let Some(id) = id {
                reject(
                    &writer,
                    id,
                    Error::business("already_initialized", "hello has already completed"),
                );
            }
            continue;
        }
        if concurrent(&method) {
            if server.workers.fetch_add(1, Ordering::SeqCst) >= 128 {
                server.workers.fetch_sub(1, Ordering::SeqCst);
                if let Some(id) = id {
                    reject(
                        &writer,
                        id,
                        Error::business("busy", "too many pending requests"),
                    );
                }
                continue;
            }
            let server = server.clone();
            let writer = writer.clone();
            std::thread::spawn(move || {
                let result = server.call(&method, &params);
                if let Some(id) = id {
                    response(&writer, id, result);
                }
                server.workers.fetch_sub(1, Ordering::SeqCst);
            });
        } else {
            let result = server.call(&method, &params);
            if let Some(id) = id {
                response(&writer, id, result);
            }
        }
    }
    Ok(())
}
fn concurrent(method: &str) -> bool {
    matches!(
        method,
        "environment.tools.status"
            | "environment.tools.install"
            | "terminal.create"
            | "terminal.attach"
            | "terminal.restart"
            | "terminal.read"
            | "terminal.wait"
            | "terminal.write"
            | "chat.snapshot"
            | "chat.watch"
            | "chat.command"
            | "workspace.watch"
            | "process.read"
            | "process.wait"
            | "process.write"
            | "process.spawn"
            | "environment.status"
            | "environment.reconcile"
            | "environment.restart"
    )
}
