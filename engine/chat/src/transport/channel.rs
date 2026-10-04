//! Newline-delimited JSON over a guest child's stdio.
//!
//! One reader thread splits stdout into bounded lines and routes each frame; one thread keeps a
//! bounded stderr tail for diagnostics, which is never logged; a dispatcher thread hands inbound
//! messages to the owner in order. Writes are serialized and flushed per frame.
use super::lines::{Line, LineReader};
use crate::{
    error::{ChatError, ErrorKind, Result},
    model::OpaqueJson,
    wire,
};
use serde::Serialize;
use std::{
    io::{BufRead, BufReader, Read, Write},
    sync::{Arc, Mutex, mpsc},
    thread::JoinHandle,
};

pub const MAX_FRAME_BYTES: usize = 64 * 1024 * 1024;
pub const STDERR_TAIL_CHARS: usize = 32 * 1024;
pub const INBOUND_CAPACITY: usize = 1024;
const PREVIEW_CHARS: usize = 2000;

pub enum LineEvent {
    Frame(OpaqueJson),
    Malformed { preview: String, reason: String },
}

/// Serialized, flushed frame writes. Closing drops stdin so the child sees EOF.
pub struct Writer {
    inner: Mutex<Option<Box<dyn Write + Send>>>,
}
impl Writer {
    pub fn new(stdin: Box<dyn Write + Send>) -> Self {
        Self {
            inner: Mutex::new(Some(stdin)),
        }
    }
    pub fn send<T: Serialize + ?Sized>(&self, frame: &T) -> Result<()> {
        let mut bytes = serde_json::to_vec(frame)?;
        bytes.push(b'\n');
        let mut inner = self.inner.lock().unwrap();
        let out = inner
            .as_mut()
            .ok_or_else(|| ChatError::new(ErrorKind::Closed, "channel closed"))?;
        out.write_all(&bytes)
            .and_then(|_| out.flush())
            .map_err(|e| ChatError::new(ErrorKind::Closed, e.to_string()))
    }
    pub fn close(&self) {
        self.inner.lock().unwrap().take();
    }
}

/// The last characters of stderr. It may contain credentials, so it is only shown to the user.
#[derive(Default)]
pub struct StderrTail(Mutex<String>);
impl StderrTail {
    fn push(&self, line: &str) {
        let mut tail = self.0.lock().unwrap();
        tail.push_str(line);
        tail.push('\n');
        let count = tail.chars().count();
        if count > STDERR_TAIL_CHARS {
            let cut = tail
                .char_indices()
                .nth(count - STDERR_TAIL_CHARS)
                .map(|(i, _)| i)
                .unwrap_or(0);
            tail.drain(..cut);
        }
    }
    pub fn text(&self) -> String {
        self.0.lock().unwrap().clone()
    }
    /// The last `chars` characters, as shown in process failure states.
    pub fn last(&self, chars: usize) -> String {
        take_last(&self.text(), chars)
    }
}
pub fn take_last(text: &str, chars: usize) -> String {
    let count = text.chars().count();
    text.chars().skip(count.saturating_sub(chars)).collect()
}
pub fn take_first(text: &str, chars: usize) -> String {
    text.chars().take(chars).collect()
}

/// Reads stdout until EOF, calling `on` for each frame. Returns the reason the stream ended.
pub fn spawn_reader(
    stdout: Box<dyn Read + Send>,
    mut on: impl FnMut(LineEvent) + Send + 'static,
    done: impl FnOnce(Option<String>) + Send + 'static,
) -> JoinHandle<()> {
    std::thread::spawn(move || {
        let mut reader = LineReader::new(stdout, MAX_FRAME_BYTES);
        let failure = loop {
            match reader.next_line() {
                Ok(Line::Eof) => break None,
                Ok(Line::TooLarge(n)) => on(LineEvent::Malformed {
                    preview: String::new(),
                    reason: format!("frame of {n} bytes exceeds {MAX_FRAME_BYTES}"),
                }),
                Ok(Line::BadEncoding(n)) => on(LineEvent::Malformed {
                    preview: String::new(),
                    reason: format!("invalid UTF-8 ({n} bytes)"),
                }),
                Ok(Line::Text(text)) => on(match wire::parse(&text) {
                    Ok(value) if wire::is_object(&value) => LineEvent::Frame(value),
                    Ok(_) => LineEvent::Malformed {
                        preview: take_first(&text, PREVIEW_CHARS),
                        reason: "not a JSON object".into(),
                    },
                    Err(reason) => LineEvent::Malformed {
                        preview: take_first(&text, PREVIEW_CHARS),
                        reason,
                    },
                }),
                Err(e) => break Some(e.to_string()),
            }
        };
        done(failure);
    })
}

pub fn spawn_stderr(stderr: Box<dyn Read + Send>, tail: Arc<StderrTail>) -> JoinHandle<()> {
    std::thread::spawn(move || {
        let mut reader = BufReader::new(stderr);
        let mut line = Vec::new();
        loop {
            line.clear();
            match reader.read_until(b'\n', &mut line) {
                Ok(0) | Err(_) => break,
                Ok(_) => {
                    if line.last() == Some(&b'\n') {
                        line.pop();
                    }
                    if line.last() == Some(&b'\r') {
                        line.pop();
                    }
                    tail.push(&String::from_utf8_lossy(&line));
                }
            }
        }
    })
}

/// Runs `handler` for every queued inbound message, in arrival order, until the queue closes.
pub fn spawn_dispatcher<T: Send + 'static>(
    receiver: mpsc::Receiver<T>,
    mut handler: impl FnMut(T) + Send + 'static,
) -> JoinHandle<()> {
    std::thread::spawn(move || {
        for message in receiver {
            handler(message);
        }
    })
}

/// Joins the threads of a connection; a thread that panicked is ignored.
#[derive(Default)]
pub struct Threads(Mutex<Vec<JoinHandle<()>>>);
impl Threads {
    pub fn add(&self, handle: JoinHandle<()>) {
        self.0.lock().unwrap().push(handle);
    }
    pub fn join(&self) {
        let handles = std::mem::take(&mut *self.0.lock().unwrap());
        let me = std::thread::current().id();
        for h in handles {
            if h.thread().id() != me {
                let _ = h.join();
            }
        }
    }
}
