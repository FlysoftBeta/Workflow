//! Tolerant typed projections of vendor JSON.
//!
//! Vendor frames are parsed once into [`OpaqueJson`], which keeps every field and number token, and
//! then projected into small typed structs built from the field types below. Each type accepts any
//! JSON value and yields nothing for a value of another kind, exactly like the accessors of the
//! retained Kotlin adapters (`str`, `long`, `double`, `bool`, `obj`, `arr`, `strings`). A projection
//! therefore never fails because one field has an unexpected type, and unknown fields stay in the
//! opaque frame that the neutral model retains as `raw`.
use crate::model::OpaqueJson;
use serde::{
    Deserialize, Deserializer, Serialize,
    de::{self, DeserializeOwned, IgnoredAny, MapAccess, SeqAccess, Visitor},
};
use std::{fmt, marker::PhantomData};

/// Projects a vendor value. Only a non-object input can fail, and it yields the default projection.
pub fn project<T: DeserializeOwned + Default>(value: &OpaqueJson) -> T {
    T::deserialize(ValueDe(&value.0)).unwrap_or_default()
}
/// Decodes a vendor value strictly (for request bodies that must be well formed).
pub fn decode<T: DeserializeOwned>(value: &OpaqueJson) -> Result<T, serde_json::Error> {
    T::deserialize(ValueDe(&value.0))
}
/// Projects an optional vendor value.
pub fn project_opt<T: DeserializeOwned + Default>(value: Option<&OpaqueJson>) -> T {
    value.map(project).unwrap_or_default()
}
/// Encodes an outbound typed body as an opaque value for the transport.
pub fn encode<T: Serialize + ?Sized>(value: &T) -> OpaqueJson {
    OpaqueJson(serde_json::to_value(value).expect("typed vendor bodies always encode"))
}
/// Parses one vendor line. Duplicate members keep the last value, as the retained adapters did, and
/// every number keeps its exact token (`-0`, `1.2300e+20`, integers beyond 64 bits).
pub fn parse(text: &str) -> Result<OpaqueJson, String> {
    exact(text, 0).map(OpaqueJson).map_err(|e| e.to_string())
}
fn exact(text: &str, depth: usize) -> Result<serde_json::Value, serde_json::Error> {
    use serde_json::{Value as J, value::RawValue};
    if depth > 256 {
        return Err(de::Error::custom("JSON nesting exceeds limit"));
    }
    let raw: &RawValue = serde_json::from_str(text)?;
    let text = raw.get();
    Ok(match text.as_bytes().first() {
        Some(b'{') => {
            let members: Vec<(String, &RawValue)> = serde_json::from_str::<Ordered>(text)?.0;
            let mut map = serde_json::Map::new();
            for (key, value) in members {
                map.insert(key, exact(value.get(), depth + 1)?);
            }
            J::Object(map)
        }
        Some(b'[') => J::Array(
            serde_json::from_str::<Vec<&RawValue>>(text)?
                .into_iter()
                .map(|v| exact(v.get(), depth + 1))
                .collect::<Result<_, _>>()?,
        ),
        Some(b'"') => J::String(serde_json::from_str(text)?),
        Some(b't') => J::Bool(true),
        Some(b'f') => J::Bool(false),
        Some(b'n') => J::Null,
        _ => {
            let _: serde_json::Number = text.parse()?;
            J::Number(serde_json::Number::from_string_unchecked(text.to_owned()))
        }
    })
}
/// Object members in document order, borrowing their raw values.
struct Ordered<'a>(Vec<(String, &'a serde_json::value::RawValue)>);
impl<'de: 'a, 'a> Deserialize<'de> for Ordered<'a> {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct V<'a>(PhantomData<&'a ()>);
        impl<'de: 'a, 'a> Visitor<'de> for V<'a> {
            type Value = Ordered<'a>;
            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("an object")
            }
            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
                let mut out = Vec::new();
                while let Some(entry) =
                    map.next_entry::<String, &'de serde_json::value::RawValue>()?
                {
                    out.push(entry);
                }
                Ok(Ordered(out))
            }
        }
        d.deserialize_map(V(PhantomData))
    }
}
/// Exact JSON text, used for request-ID map keys and display details.
pub fn text(value: &OpaqueJson) -> String {
    serde_json::to_string(value).unwrap_or_default()
}
pub fn is_object(value: &OpaqueJson) -> bool {
    value.0.is_object()
}
pub fn is_array(value: &OpaqueJson) -> bool {
    value.0.is_array()
}
pub fn is_null(value: &OpaqueJson) -> bool {
    value.0.is_null()
}
pub fn string_value(value: &str) -> OpaqueJson {
    OpaqueJson(serde_json::Value::String(value.into()))
}
pub fn empty_object() -> OpaqueJson {
    OpaqueJson(serde_json::Value::Object(Default::default()))
}

/// A JSON string; other kinds are absent.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Str(pub Option<String>);
impl Str {
    pub fn get(&self) -> Option<&str> {
        self.0.as_deref()
    }
    pub fn owned(&self) -> Option<String> {
        self.0.clone()
    }
    pub fn or(&self, fallback: &str) -> String {
        self.0.clone().unwrap_or_else(|| fallback.into())
    }
}
impl<'de> Deserialize<'de> for Str {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct V;
        impl<'de> Visitor<'de> for V {
            type Value = Option<String>;
            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("any JSON value")
            }
            fn visit_str<E: de::Error>(self, v: &str) -> Result<Self::Value, E> {
                Ok(Some(v.into()))
            }
            fn visit_string<E: de::Error>(self, v: String) -> Result<Self::Value, E> {
                Ok(Some(v))
            }
            fn visit_bool<E: de::Error>(self, _: bool) -> Result<Self::Value, E> {
                Ok(None)
            }
            fn visit_unit<E: de::Error>(self) -> Result<Self::Value, E> {
                Ok(None)
            }
            fn visit_none<E: de::Error>(self) -> Result<Self::Value, E> {
                Ok(None)
            }
            fn visit_i64<E: de::Error>(self, _: i64) -> Result<Self::Value, E> {
                Ok(None)
            }
            fn visit_u64<E: de::Error>(self, _: u64) -> Result<Self::Value, E> {
                Ok(None)
            }
            fn visit_i128<E: de::Error>(self, _: i128) -> Result<Self::Value, E> {
                Ok(None)
            }
            fn visit_u128<E: de::Error>(self, _: u128) -> Result<Self::Value, E> {
                Ok(None)
            }
            fn visit_f64<E: de::Error>(self, _: f64) -> Result<Self::Value, E> {
                Ok(None)
            }
            fn visit_newtype_struct<D: Deserializer<'de>>(
                self,
                d: D,
            ) -> Result<Self::Value, D::Error> {
                IgnoredAny::deserialize(d).map(|_| None)
            }
            fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
                while seq.next_element::<IgnoredAny>()?.is_some() {}
                Ok(None)
            }
            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
                while map.next_entry::<IgnoredAny, IgnoredAny>()?.is_some() {}
                Ok(None)
            }
        }
        d.deserialize_any(V).map(Str)
    }
}

/// A non-string integral JSON number within the signed 64-bit range.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Long(pub Option<i64>);
impl Long {
    pub fn get(&self) -> Option<i64> {
        self.0
    }
    /// Kotlin `Long.toInt()` truncation.
    pub fn int(&self) -> Option<i32> {
        self.0.map(|v| v as i32)
    }
}
impl<'de> Deserialize<'de> for Long {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct V;
        impl<'de> Visitor<'de> for V {
            type Value = Option<i64>;
            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("any JSON value")
            }
            fn visit_i64<E: de::Error>(self, v: i64) -> Result<Self::Value, E> {
                Ok(Some(v))
            }
            fn visit_u64<E: de::Error>(self, v: u64) -> Result<Self::Value, E> {
                Ok(i64::try_from(v).ok())
            }
            fn visit_bool<E: de::Error>(self, _: bool) -> Result<Self::Value, E> {
                Ok(None)
            }
            fn visit_unit<E: de::Error>(self) -> Result<Self::Value, E> {
                Ok(None)
            }
            fn visit_none<E: de::Error>(self) -> Result<Self::Value, E> {
                Ok(None)
            }
            fn visit_str<E: de::Error>(self, _: &str) -> Result<Self::Value, E> {
                Ok(None)
            }
            fn visit_i128<E: de::Error>(self, _: i128) -> Result<Self::Value, E> {
                Ok(None)
            }
            fn visit_u128<E: de::Error>(self, _: u128) -> Result<Self::Value, E> {
                Ok(None)
            }
            fn visit_f64<E: de::Error>(self, _: f64) -> Result<Self::Value, E> {
                Ok(None)
            }
            fn visit_newtype_struct<D: Deserializer<'de>>(
                self,
                d: D,
            ) -> Result<Self::Value, D::Error> {
                IgnoredAny::deserialize(d).map(|_| None)
            }
            fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
                while seq.next_element::<IgnoredAny>()?.is_some() {}
                Ok(None)
            }
            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
                while map.next_entry::<IgnoredAny, IgnoredAny>()?.is_some() {}
                Ok(None)
            }
        }
        d.deserialize_any(V).map(Long)
    }
}

/// A non-string JSON number.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Double(pub Option<f64>);
impl<'de> Deserialize<'de> for Double {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct V;
        impl<'de> Visitor<'de> for V {
            type Value = Option<f64>;
            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("any JSON value")
            }
            fn visit_i64<E: de::Error>(self, v: i64) -> Result<Self::Value, E> {
                Ok(Some(v as f64))
            }
            fn visit_u64<E: de::Error>(self, v: u64) -> Result<Self::Value, E> {
                Ok(Some(v as f64))
            }
            fn visit_i128<E: de::Error>(self, v: i128) -> Result<Self::Value, E> {
                Ok(Some(v as f64))
            }
            fn visit_u128<E: de::Error>(self, v: u128) -> Result<Self::Value, E> {
                Ok(Some(v as f64))
            }
            fn visit_f64<E: de::Error>(self, v: f64) -> Result<Self::Value, E> {
                Ok(Some(v))
            }
            fn visit_bool<E: de::Error>(self, _: bool) -> Result<Self::Value, E> {
                Ok(None)
            }
            fn visit_unit<E: de::Error>(self) -> Result<Self::Value, E> {
                Ok(None)
            }
            fn visit_none<E: de::Error>(self) -> Result<Self::Value, E> {
                Ok(None)
            }
            fn visit_str<E: de::Error>(self, _: &str) -> Result<Self::Value, E> {
                Ok(None)
            }
            fn visit_newtype_struct<D: Deserializer<'de>>(
                self,
                d: D,
            ) -> Result<Self::Value, D::Error> {
                Ok(String::deserialize(d)?.parse::<f64>().ok())
            }
            fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
                while seq.next_element::<IgnoredAny>()?.is_some() {}
                Ok(None)
            }
            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
                // An arbitrary-precision number token that does not fit f64 exactly.
                let mut out = None;
                while let Some((key, value)) = map.next_entry::<String, IgnoredOrString>()? {
                    if key == "$serde_json::private::Number" {
                        out = value.0.and_then(|v| v.parse::<f64>().ok());
                    }
                }
                Ok(out)
            }
        }
        d.deserialize_any(V).map(Double)
    }
}
#[derive(Default)]
struct IgnoredOrString(Option<String>);
impl<'de> Deserialize<'de> for IgnoredOrString {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        Str::deserialize(d).map(|s| IgnoredOrString(s.0))
    }
}

/// A JSON boolean.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Bool(pub Option<bool>);
impl Bool {
    pub fn get(&self) -> Option<bool> {
        self.0
    }
    pub fn is_true(&self) -> bool {
        self.0 == Some(true)
    }
    pub fn is_false(&self) -> bool {
        self.0 == Some(false)
    }
}
impl<'de> Deserialize<'de> for Bool {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct V;
        impl<'de> Visitor<'de> for V {
            type Value = Option<bool>;
            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("any JSON value")
            }
            fn visit_bool<E: de::Error>(self, v: bool) -> Result<Self::Value, E> {
                Ok(Some(v))
            }
            fn visit_unit<E: de::Error>(self) -> Result<Self::Value, E> {
                Ok(None)
            }
            fn visit_none<E: de::Error>(self) -> Result<Self::Value, E> {
                Ok(None)
            }
            fn visit_str<E: de::Error>(self, _: &str) -> Result<Self::Value, E> {
                Ok(None)
            }
            fn visit_i64<E: de::Error>(self, _: i64) -> Result<Self::Value, E> {
                Ok(None)
            }
            fn visit_u64<E: de::Error>(self, _: u64) -> Result<Self::Value, E> {
                Ok(None)
            }
            fn visit_i128<E: de::Error>(self, _: i128) -> Result<Self::Value, E> {
                Ok(None)
            }
            fn visit_u128<E: de::Error>(self, _: u128) -> Result<Self::Value, E> {
                Ok(None)
            }
            fn visit_f64<E: de::Error>(self, _: f64) -> Result<Self::Value, E> {
                Ok(None)
            }
            fn visit_newtype_struct<D: Deserializer<'de>>(
                self,
                d: D,
            ) -> Result<Self::Value, D::Error> {
                IgnoredAny::deserialize(d).map(|_| None)
            }
            fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
                while seq.next_element::<IgnoredAny>()?.is_some() {}
                Ok(None)
            }
            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
                while map.next_entry::<IgnoredAny, IgnoredAny>()?.is_some() {}
                Ok(None)
            }
        }
        d.deserialize_any(V).map(Bool)
    }
}

/// The strings of a JSON array; other members and other kinds are skipped.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Strings(pub Vec<String>);
impl<'de> Deserialize<'de> for Strings {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let items: Arr<Str> = Arr::deserialize(d)?;
        Ok(Strings(
            items
                .0
                .unwrap_or_default()
                .into_iter()
                .filter_map(|s| s.0)
                .collect(),
        ))
    }
}

/// Any JSON value, distinguishing a missing member (`None`) from an explicit `null`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Json(pub Option<OpaqueJson>);
impl Json {
    pub fn get(&self) -> Option<&OpaqueJson> {
        self.0.as_ref()
    }
    /// Present and not JSON null (Kotlin `isNullish == false`).
    pub fn value(&self) -> Option<&OpaqueJson> {
        self.0.as_ref().filter(|v| !v.0.is_null())
    }
    pub fn object(&self) -> Option<&OpaqueJson> {
        self.0.as_ref().filter(|v| v.0.is_object())
    }
    pub fn array(&self) -> Option<&OpaqueJson> {
        self.0.as_ref().filter(|v| v.0.is_array())
    }
    pub fn is_nullish(&self) -> bool {
        self.value().is_none()
    }
    /// The members of an array value, including `null` members; empty for other kinds.
    pub fn elements(&self) -> Vec<OpaqueJson> {
        self.array()
            .map(|v| {
                project::<Arr<Json>>(v)
                    .0
                    .unwrap_or_default()
                    .into_iter()
                    .map(|m| m.0.unwrap_or_default())
                    .collect()
            })
            .unwrap_or_default()
    }
    /// The string value, if this is a JSON string.
    pub fn string(&self) -> Option<String> {
        self.0.as_ref().and_then(|v| project::<Str>(v).0)
    }
    /// Names of an object's members (sorted by the opaque representation).
    pub fn keys(&self) -> Vec<String> {
        self.object()
            .and_then(|v| project::<Members>(v).0)
            .map(|m| m.into_iter().map(|(k, _)| k).collect())
            .unwrap_or_default()
    }
    /// A primitive's text (Kotlin `JsonPrimitive.content`), or the encoded JSON otherwise.
    pub fn display_text(&self) -> Option<String> {
        let v = self.0.as_ref()?;
        Some(match &v.0 {
            serde_json::Value::String(s) => s.clone(),
            serde_json::Value::Null => "null".into(),
            serde_json::Value::Bool(b) => b.to_string(),
            serde_json::Value::Number(n) => n.to_string(),
            _ => text(v),
        })
    }
}
impl<'de> Deserialize<'de> for Json {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        OpaqueJson::deserialize(d).map(|v| Json(Some(v)))
    }
}

/// A JSON object projected into `T`; other kinds are absent.
#[derive(Clone, Debug, PartialEq)]
pub struct Obj<T>(pub Option<T>);
impl<T> Default for Obj<T> {
    fn default() -> Self {
        Obj(None)
    }
}
impl<T: Default> Obj<T> {
    pub fn get(&self) -> Option<&T> {
        self.0.as_ref()
    }
    pub fn or_default(&self) -> T
    where
        T: Clone,
    {
        self.0.clone().unwrap_or_default()
    }
}
impl<'de, T: Deserialize<'de>> Deserialize<'de> for Obj<T> {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct V<T>(PhantomData<T>);
        impl<'de, T: Deserialize<'de>> Visitor<'de> for V<T> {
            type Value = Option<T>;
            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("any JSON value")
            }
            fn visit_map<A: MapAccess<'de>>(self, map: A) -> Result<Self::Value, A::Error> {
                T::deserialize(de::value::MapAccessDeserializer::new(map)).map(Some)
            }
            fn visit_bool<E: de::Error>(self, _: bool) -> Result<Self::Value, E> {
                Ok(None)
            }
            fn visit_unit<E: de::Error>(self) -> Result<Self::Value, E> {
                Ok(None)
            }
            fn visit_none<E: de::Error>(self) -> Result<Self::Value, E> {
                Ok(None)
            }
            fn visit_str<E: de::Error>(self, _: &str) -> Result<Self::Value, E> {
                Ok(None)
            }
            fn visit_i64<E: de::Error>(self, _: i64) -> Result<Self::Value, E> {
                Ok(None)
            }
            fn visit_u64<E: de::Error>(self, _: u64) -> Result<Self::Value, E> {
                Ok(None)
            }
            fn visit_i128<E: de::Error>(self, _: i128) -> Result<Self::Value, E> {
                Ok(None)
            }
            fn visit_u128<E: de::Error>(self, _: u128) -> Result<Self::Value, E> {
                Ok(None)
            }
            fn visit_f64<E: de::Error>(self, _: f64) -> Result<Self::Value, E> {
                Ok(None)
            }
            fn visit_newtype_struct<D: Deserializer<'de>>(
                self,
                d: D,
            ) -> Result<Self::Value, D::Error> {
                IgnoredAny::deserialize(d).map(|_| None)
            }
            fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
                while seq.next_element::<IgnoredAny>()?.is_some() {}
                Ok(None)
            }
        }
        d.deserialize_any(V(PhantomData)).map(Obj)
    }
}

/// A JSON array whose members are projected into `T`; other kinds are absent.
#[derive(Clone, Debug, PartialEq)]
pub struct Arr<T>(pub Option<Vec<T>>);
impl<T> Default for Arr<T> {
    fn default() -> Self {
        Arr(None)
    }
}
impl<T> Arr<T> {
    pub fn items(&self) -> &[T] {
        self.0.as_deref().unwrap_or(&[])
    }
    pub fn is_present(&self) -> bool {
        self.0.is_some()
    }
}
impl<'de, T: Deserialize<'de>> Deserialize<'de> for Arr<T> {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct V<T>(PhantomData<T>);
        impl<'de, T: Deserialize<'de>> Visitor<'de> for V<T> {
            type Value = Option<Vec<T>>;
            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("any JSON value")
            }
            fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
                let mut out = Vec::new();
                while let Some(v) = seq.next_element::<T>()? {
                    out.push(v);
                }
                Ok(Some(out))
            }
            fn visit_bool<E: de::Error>(self, _: bool) -> Result<Self::Value, E> {
                Ok(None)
            }
            fn visit_unit<E: de::Error>(self) -> Result<Self::Value, E> {
                Ok(None)
            }
            fn visit_none<E: de::Error>(self) -> Result<Self::Value, E> {
                Ok(None)
            }
            fn visit_str<E: de::Error>(self, _: &str) -> Result<Self::Value, E> {
                Ok(None)
            }
            fn visit_i64<E: de::Error>(self, _: i64) -> Result<Self::Value, E> {
                Ok(None)
            }
            fn visit_u64<E: de::Error>(self, _: u64) -> Result<Self::Value, E> {
                Ok(None)
            }
            fn visit_i128<E: de::Error>(self, _: i128) -> Result<Self::Value, E> {
                Ok(None)
            }
            fn visit_u128<E: de::Error>(self, _: u128) -> Result<Self::Value, E> {
                Ok(None)
            }
            fn visit_f64<E: de::Error>(self, _: f64) -> Result<Self::Value, E> {
                Ok(None)
            }
            fn visit_newtype_struct<D: Deserializer<'de>>(
                self,
                d: D,
            ) -> Result<Self::Value, D::Error> {
                IgnoredAny::deserialize(d).map(|_| None)
            }
            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
                while map.next_entry::<IgnoredAny, IgnoredAny>()?.is_some() {}
                Ok(None)
            }
        }
        d.deserialize_any(V(PhantomData)).map(Arr)
    }
}

/// Field names of a JSON object in document order, with their values kept opaque.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Members(pub Option<Vec<(String, OpaqueJson)>>);
impl<'de> Deserialize<'de> for Members {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let object: Obj<std::collections::BTreeMap<String, OpaqueJson>> = Obj::deserialize(d)?;
        Ok(Members(object.0.map(|m| m.into_iter().collect())))
    }
}

/// A borrowing deserializer over a parsed vendor tree. Numbers that a primitive cannot represent
/// exactly are offered as a newtype holding their exact token, which [`OpaqueJson`] keeps verbatim
/// and the tolerant primitives above treat as absent. serde_json's own `&Value` deserializer would
/// instead expose its private arbitrary-precision map and corrupt such numbers in opaque fields.
struct ValueDe<'de>(&'de serde_json::Value);
impl<'de> de::IntoDeserializer<'de, serde_json::Error> for ValueDe<'de> {
    type Deserializer = Self;
    fn into_deserializer(self) -> Self {
        self
    }
}
impl<'de> Deserializer<'de> for ValueDe<'de> {
    type Error = serde_json::Error;
    fn deserialize_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        use serde_json::Value as J;
        match self.0 {
            J::Null => visitor.visit_unit(),
            J::Bool(b) => visitor.visit_bool(*b),
            J::String(s) => visitor.visit_borrowed_str(s),
            J::Array(items) => {
                visitor.visit_seq(de::value::SeqDeserializer::new(items.iter().map(ValueDe)))
            }
            J::Object(map) => visitor.visit_map(de::value::MapDeserializer::new(
                map.iter().map(|(k, v)| (k.as_str(), ValueDe(v))),
            )),
            J::Number(n) => {
                let token = n.to_string();
                if let Some(v) = n.as_u64().filter(|v| v.to_string() == token) {
                    visitor.visit_u64(v)
                } else if let Some(v) = n.as_i64().filter(|v| v.to_string() == token) {
                    visitor.visit_i64(v)
                } else if let Some(v) = n.as_f64().filter(|v| {
                    serde_json::Number::from_f64(*v).is_some_and(|exact| exact.to_string() == token)
                }) {
                    visitor.visit_f64(v)
                } else {
                    use de::IntoDeserializer;
                    visitor.visit_newtype_struct(token.into_deserializer())
                }
            }
        }
    }
    fn deserialize_option<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        if self.0.is_null() {
            visitor.visit_none()
        } else {
            visitor.visit_some(self)
        }
    }
    fn deserialize_newtype_struct<V: Visitor<'de>>(
        self,
        _: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        visitor.visit_newtype_struct(self)
    }
    fn deserialize_enum<V: Visitor<'de>>(
        self,
        _: &'static str,
        _: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        use de::IntoDeserializer;
        match self.0 {
            serde_json::Value::String(name) => {
                visitor.visit_enum(name.as_str().into_deserializer())
            }
            serde_json::Value::Object(map) if map.len() == 1 => {
                let (name, value) = map.iter().next().unwrap();
                visitor.visit_enum(EnumDe(name, ValueDe(value)))
            }
            _ => Err(de::Error::custom("expected enum string or one-key object")),
        }
    }
    serde::forward_to_deserialize_any! { bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string bytes byte_buf unit unit_struct seq tuple tuple_struct map struct identifier ignored_any }
}
struct EnumDe<'de>(&'de str, ValueDe<'de>);
impl<'de> de::EnumAccess<'de> for EnumDe<'de> {
    type Error = serde_json::Error;
    type Variant = ValueDe<'de>;
    fn variant_seed<S: de::DeserializeSeed<'de>>(
        self,
        seed: S,
    ) -> Result<(S::Value, Self::Variant), Self::Error> {
        use de::IntoDeserializer;
        Ok((seed.deserialize(self.0.into_deserializer())?, self.1))
    }
}
impl<'de> de::VariantAccess<'de> for ValueDe<'de> {
    type Error = serde_json::Error;
    fn unit_variant(self) -> Result<(), Self::Error> {
        Deserialize::deserialize(self)
    }
    fn newtype_variant_seed<S: de::DeserializeSeed<'de>>(
        self,
        seed: S,
    ) -> Result<S::Value, Self::Error> {
        seed.deserialize(self)
    }
    fn tuple_variant<V: Visitor<'de>>(self, _: usize, visitor: V) -> Result<V::Value, Self::Error> {
        self.deserialize_any(visitor)
    }
    fn struct_variant<V: Visitor<'de>>(
        self,
        _: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        self.deserialize_any(visitor)
    }
}
