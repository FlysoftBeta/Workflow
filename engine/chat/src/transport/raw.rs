//! Exact JSON tokens for vendor envelopes and IDs; known bodies decode to typed structs.
use crate::error::Result;
use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;
use std::fmt;
#[derive(Clone, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RawJson(Box<RawValue>);
impl fmt::Debug for RawJson {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("<opaque JSON>")
    }
}
impl PartialEq for RawJson {
    fn eq(&self, other: &Self) -> bool {
        self.text() == other.text()
    }
}
impl Eq for RawJson {}
impl RawJson {
    pub fn parse(text: &str) -> Result<Self> {
        Ok(Self(serde_json::from_str(text)?))
    }
    pub fn encode<T: Serialize + ?Sized>(value: &T) -> Result<Self> {
        Ok(Self(serde_json::value::to_raw_value(value)?))
    }
    pub fn text(&self) -> &str {
        self.0.get()
    }
    pub fn decode<T: serde::de::DeserializeOwned>(&self) -> Result<T> {
        Ok(workflow_environment::json::strict_json(
            self.text().as_bytes(),
        )?)
    }
    pub fn opaque(&self) -> Result<crate::model::OpaqueJson> {
        self.decode()
    }
    pub fn null() -> Self {
        Self::parse("null").unwrap()
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct RequestId(pub RawJson);
impl RequestId {
    pub fn text(&self) -> &str {
        self.0.text()
    }
    pub fn number(value: u64) -> Self {
        Self(RawJson::encode(&value).unwrap())
    }
    pub fn string(value: &str) -> Self {
        Self(RawJson::encode(value).unwrap())
    }
}
impl<'de> Deserialize<'de> for RequestId {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> std::result::Result<Self, D::Error> {
        let raw = RawJson::deserialize(d)?;
        let valid = if raw.text().starts_with('"') {
            serde_json::from_str::<String>(raw.text()).is_ok()
        } else {
            serde_json::from_str::<serde_json::Number>(raw.text()).is_ok()
        };
        if !valid {
            return Err(serde::de::Error::custom(
                "request ID must be a string or number",
            ));
        }
        Ok(Self(raw))
    }
}
pub fn present<'de, D: serde::Deserializer<'de>>(
    d: D,
) -> std::result::Result<Option<RawJson>, D::Error> {
    RawJson::deserialize(d).map(Some)
}
