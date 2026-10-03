use std::io::{self, BufRead, BufReader, Read};
#[derive(Debug, PartialEq, Eq)]
pub enum Line {
    Text(String),
    TooLarge(usize),
    BadEncoding(usize),
    Eof,
}
/// Drain overlong lines without allocating their body; preserve the following frame.
pub struct LineReader<R: Read> {
    reader: BufReader<R>,
    max: usize,
}
impl<R: Read> LineReader<R> {
    pub fn new(reader: R, max: usize) -> Self {
        Self {
            reader: BufReader::with_capacity(65536, reader),
            max,
        }
    }
    pub fn next_line(&mut self) -> io::Result<Line> {
        loop {
            let mut bytes = Vec::new();
            let mut count = 0;
            let mut oversized = false;
            loop {
                let available = self.reader.fill_buf()?;
                if available.is_empty() {
                    if count == 0 {
                        return Ok(Line::Eof);
                    }
                    break;
                }
                let end = available.iter().position(|b| *b == b'\n');
                let n = end.unwrap_or(available.len());
                count += n;
                if count > self.max {
                    oversized = true;
                    bytes.clear();
                }
                if !oversized {
                    bytes.extend_from_slice(&available[..n]);
                }
                self.reader.consume(n + usize::from(end.is_some()));
                if end.is_some() {
                    break;
                }
            }
            if oversized {
                return Ok(Line::TooLarge(count));
            }
            if bytes.last() == Some(&b'\r') {
                bytes.pop();
            }
            if bytes.is_empty() {
                continue;
            }
            let len = bytes.len();
            return Ok(match String::from_utf8(bytes) {
                Ok(s) => Line::Text(s),
                Err(_) => Line::BadEncoding(len),
            });
        }
    }
}
