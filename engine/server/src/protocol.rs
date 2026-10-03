//! JSON-RPC framing, strict JSON decoding, binary chunks and error envelopes.
//! No workspace state or filesystem policy belongs in this module.
use base64::Engine;
use serde::{
    Deserialize,
    de::{self, MapAccess, Visitor},
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
    use serde_json::value::RawValue;
    // Borrow child slices: validation never rounds numbers or interprets a user's object
    // as serde_json's private arbitrary-precision-number representation.
    struct Object<'a>(Vec<(String, &'a RawValue)>);
    impl<'de> Deserialize<'de> for Object<'de> {
        fn deserialize<D: serde::Deserializer<'de>>(d: D) -> std::result::Result<Self, D::Error> {
            struct ObjectVisitor;
            impl<'de> Visitor<'de> for ObjectVisitor {
                type Value = Object<'de>;
                fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                    f.write_str("an object")
                }
                fn visit_map<A: MapAccess<'de>>(
                    self,
                    mut a: A,
                ) -> std::result::Result<Self::Value, A::Error> {
                    let mut fields = Vec::new();
                    let mut keys = std::collections::HashSet::new();
                    while let Some((key, value)) = a.next_entry::<String, &'de RawValue>()? {
                        if !keys.insert(key.clone()) {
                            return Err(de::Error::custom("duplicate object key"));
                        }
                        fields.push((key, value));
                    }
                    Ok(Object(fields))
                }
            }
            d.deserialize_map(ObjectVisitor)
        }
    }
    fn value(raw: &RawValue, depth: usize) -> std::result::Result<V, serde_json::Error> {
        if depth > 128 {
            return Err(de::Error::custom("JSON nesting exceeds limit"));
        }
        let text = raw.get();
        Ok(match text.as_bytes().first() {
            Some(b'{') => {
                let object: Object<'_> = serde_json::from_str(text)?;
                let mut fields = serde_json::Map::new();
                for (key, raw) in object.0 {
                    fields.insert(key, value(raw, depth + 1)?);
                }
                V::Object(fields)
            }
            Some(b'[') => {
                let items: Vec<&RawValue> = serde_json::from_str(text)?;
                V::Array(
                    items
                        .into_iter()
                        .map(|v| value(v, depth + 1))
                        .collect::<std::result::Result<_, _>>()?,
                )
            }
            Some(b'"') => V::String(serde_json::from_str(text)?),
            Some(b't') => V::Bool(true),
            Some(b'f') => V::Bool(false),
            Some(b'n') => V::Null,
            _ => V::Number(text.parse()?),
        })
    }
    let raw: &RawValue = serde_json::from_slice(bytes)?;
    value(raw, 0)
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
    fn opaque_numbers_and_reserved_looking_fields_are_lossless() {
        let raw=br#"{"id":123456789012345678901234567890,"precise":0.12345678901234567890123456789,"$serde_json::private::Number":"not-a-number"}"#;
        let value = strict_json(raw).unwrap();
        assert_eq!(value["id"].to_string(), "123456789012345678901234567890");
        assert_eq!(
            value["precise"].to_string(),
            "0.12345678901234567890123456789"
        );
        assert_eq!(value["$serde_json::private::Number"], "not-a-number");
        assert!(strict_json(br#"{"outer":{"same":1,"same":2}}"#).is_err());
    }
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
