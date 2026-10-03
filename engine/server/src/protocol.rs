//! JSON-RPC framing, strict JSON decoding, binary chunks and error envelopes.
//! No workspace state or filesystem policy belongs in this module.
use base64::Engine;
use serde::{
    Deserialize,
    de::{self, MapAccess, SeqAccess, Visitor},
};
use serde_json::{Value as V, json};
use std::{
    fmt,
    io::{self, BufRead, Write},
    sync::{Arc, Mutex},
};

pub(crate) const PROTOCOL: &str = "workflow.workspace/1";
pub(crate) const MAX_FRAME: usize = 32 * 1024 * 1024;
pub(crate) const MAX_BLOB: usize = 65536;
#[derive(Debug, Clone)]
pub struct Error {
    pub(crate) code: i32,
    pub kind: String,
    pub message: String,
}
pub type Result<T> = std::result::Result<T, Error>;
impl Error {
    pub fn invalid(message: &str) -> Self {
        Self {
            code: -32602,
            kind: "invalid_params".into(),
            message: message.into(),
        }
    }
    pub fn business(kind: &str, message: &str) -> Self {
        Self {
            code: -32000,
            kind: kind.into(),
            message: message.into(),
        }
    }
    pub(crate) fn method() -> Self {
        Self {
            code: -32601,
            kind: "unknown_method".into(),
            message: "Method not found".into(),
        }
    }
}
impl From<io::Error> for Error {
    fn from(e: io::Error) -> Self {
        Self::business("io", &e.to_string())
    }
}
/// Reject duplicate keys; serde_json's default Value visitor silently accepts them.
pub fn strict_json(bytes: &[u8]) -> std::result::Result<V, serde_json::Error> {
    struct Strict(V);
    impl<'de> Deserialize<'de> for Strict {
        fn deserialize<D: serde::Deserializer<'de>>(d: D) -> std::result::Result<Self, D::Error> {
            struct Vis;
            impl<'de> Visitor<'de> for Vis {
                type Value = Strict;
                fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                    write!(f, "JSON value")
                }
                fn visit_bool<E: de::Error>(self, v: bool) -> std::result::Result<Strict, E> {
                    Ok(Strict(json!(v)))
                }
                fn visit_i64<E: de::Error>(self, v: i64) -> std::result::Result<Strict, E> {
                    Ok(Strict(json!(v)))
                }
                fn visit_u64<E: de::Error>(self, v: u64) -> std::result::Result<Strict, E> {
                    Ok(Strict(json!(v)))
                }
                fn visit_f64<E: de::Error>(self, v: f64) -> std::result::Result<Strict, E> {
                    serde_json::Number::from_f64(v)
                        .map(|n| Strict(V::Number(n)))
                        .ok_or_else(|| E::custom("non-finite number"))
                }
                fn visit_str<E: de::Error>(self, v: &str) -> std::result::Result<Strict, E> {
                    Ok(Strict(json!(v)))
                }
                fn visit_string<E: de::Error>(self, v: String) -> std::result::Result<Strict, E> {
                    Ok(Strict(json!(v)))
                }
                fn visit_unit<E: de::Error>(self) -> std::result::Result<Strict, E> {
                    Ok(Strict(V::Null))
                }
                fn visit_none<E: de::Error>(self) -> std::result::Result<Strict, E> {
                    Ok(Strict(V::Null))
                }
                fn visit_seq<A: SeqAccess<'de>>(
                    self,
                    mut a: A,
                ) -> std::result::Result<Strict, A::Error> {
                    let mut out = vec![];
                    while let Some(v) = a.next_element::<Strict>()? {
                        out.push(v.0);
                    }
                    Ok(Strict(V::Array(out)))
                }
                fn visit_map<A: MapAccess<'de>>(
                    self,
                    mut a: A,
                ) -> std::result::Result<Strict, A::Error> {
                    let mut out = serde_json::Map::new();
                    while let Some((k, v)) = a.next_entry::<String, Strict>()? {
                        if out.insert(k, v.0).is_some() {
                            return Err(de::Error::custom("duplicate object key"));
                        }
                    }
                    Ok(Strict(V::Object(out)))
                }
            }
            d.deserialize_any(Vis)
        }
    }
    let mut decoder = serde_json::Deserializer::from_slice(bytes);
    let v = Strict::deserialize(&mut decoder)?;
    decoder.end()?;
    Ok(v.0)
}
pub fn decode_blob(s: &str, max: usize) -> Result<Vec<u8>> {
    if s.len() > max.div_ceil(3) * 4 {
        return Err(Error::invalid("blob exceeds chunk limit"));
    }
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(s)
        .map_err(|_| Error::invalid("invalid base64"))?;
    if bytes.len() > max {
        return Err(Error::invalid("blob exceeds chunk limit"));
    }
    Ok(bytes)
}
pub fn encode_blob(bytes: &[u8]) -> String {
    base64::engine::general_purpose::STANDARD.encode(bytes)
}
pub(crate) struct RpcId(pub(crate) String);
impl RpcId {
    pub(crate) fn null() -> Self {
        Self("null".into())
    }
}
impl From<V> for RpcId {
    fn from(value: V) -> Self {
        Self(value.to_string())
    }
}
pub(crate) fn response(writer: &Arc<Mutex<io::Stdout>>, id: impl Into<RpcId>, result: Result<V>) {
    let id = id.into();
    let (field, value) = match result {
        Ok(value) => ("result", value),
        Err(error) => (
            "error",
            json!({"code":error.code,"message":error.message,"data":{"kind":error.kind}}),
        ),
    };
    let mut bytes = format!("{{\"jsonrpc\":\"2.0\",\"id\":{},\"{field}\":", id.0).into_bytes();
    serde_json::to_writer(&mut bytes, &value).unwrap();
    bytes.push(b'}');
    if bytes.len() > MAX_FRAME {
        bytes=format!("{{\"jsonrpc\":\"2.0\",\"id\":{},\"error\":{{\"code\":-32000,\"message\":\"response exceeds frame limit\",\"data\":{{\"kind\":\"too_large\"}}}}}}",id.0).into_bytes();
    }
    let mut stdout = writer.lock().unwrap();
    let _ = stdout.write_all(&bytes);
    let _ = stdout.write_all(b"\n");
    let _ = stdout.flush();
}
pub(crate) fn line(reader: &mut impl BufRead) -> io::Result<Option<Vec<u8>>> {
    let mut out = Vec::new();
    let mut oversized = false;
    loop {
        let buf = reader.fill_buf()?;
        if buf.is_empty() {
            return if out.is_empty() && !oversized {
                Ok(None)
            } else if oversized {
                Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "frame too large",
                ))
            } else {
                Ok(Some(out))
            };
        }
        let take = buf
            .iter()
            .position(|b| *b == b'\n')
            .map(|p| p + 1)
            .unwrap_or(buf.len());
        let ended = buf[take - 1] == b'\n';
        if out.len() + take > MAX_FRAME + 1 {
            oversized = true;
        }
        if !oversized {
            out.extend_from_slice(&buf[..take]);
        }
        reader.consume(take);
        if ended {
            if oversized {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "frame too large",
                ));
            }
            out.pop();
            if out.last() == Some(&b'\r') {
                out.pop();
            }
            return Ok(Some(out));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strict_json_rejects_duplicate_keys_and_trailing_documents() {
        assert!(strict_json(br#"{"a":1,"a":2}"#).is_err());
        assert!(strict_json(b"{} {}").is_err());
        assert!(strict_json(br#"{"id":{"unknown":[1,true,null]}}"#).is_ok());
    }
    #[test]
    fn bounded_frames_and_blobs() {
        let mut reader = io::Cursor::new(b"{\"x\":1}\r\n{}\n".to_vec());
        assert_eq!(line(&mut reader).unwrap().unwrap(), b"{\"x\":1}");
        assert_eq!(line(&mut reader).unwrap().unwrap(), b"{}");
        assert!(line(&mut reader).unwrap().is_none());
        assert!(decode_blob(&encode_blob(&vec![0; 65537]), 65536).is_err());
        assert_eq!(
            decode_blob(&encode_blob(&[0, 255, 3]), 65536).unwrap(),
            vec![0, 255, 3]
        );
        let mut over = io::Cursor::new(vec![b'a'; MAX_FRAME + 2]);
        assert!(line(&mut over).is_err());
    }
}
