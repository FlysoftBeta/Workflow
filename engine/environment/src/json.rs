//! Lossless unknown JSON payloads and duplicate-key-rejecting typed decoding.
use serde::{
    Deserialize, Serialize,
    de::{self, DeserializeOwned, MapAccess, Visitor},
};
use serde_json::Value as V;
use std::{collections::BTreeMap, fmt};
/// Unknown extension payload; known documents must have their own serde model.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(transparent)]
pub struct OpaqueJson(pub V);
impl schemars::JsonSchema for OpaqueJson {
    fn schema_name() -> std::borrow::Cow<'static, str> {
        "OpaqueJson".into()
    }
    fn json_schema(_: &mut schemars::SchemaGenerator) -> schemars::Schema {
        true.into()
    }
}
impl OpaqueJson {
    /// Deep merge extension members without exposing their representation to owners.
    pub fn merge(&mut self, from: Self) {
        fn values(to: &mut V, from: V) {
            match (to, from) {
                (V::Object(a), V::Object(b)) => {
                    for (k, v) in b {
                        if let Some(old) = a.get_mut(&k) {
                            values(old, v)
                        } else {
                            a.insert(k, v);
                        }
                    }
                }
                (a, b) => *a = b,
            }
        }
        values(&mut self.0, from.0);
    }
}
impl Default for OpaqueJson {
    fn default() -> Self {
        Self(V::Null)
    }
}
pub type OpaqueObject = BTreeMap<String, OpaqueJson>;
/// Validate the complete tree first, including duplicate keys in unknown fields,
/// then deserialize the validated tree without conflating objects and numbers.
pub fn strict_json<T: DeserializeOwned>(bytes: &[u8]) -> std::result::Result<T, serde_json::Error> {
    T::deserialize(Validated(strict_value(bytes)?))
}
/// A validated tree deserializer keeps real objects distinct from serde_json's
/// private arbitrary-precision number map. Flattened serde fields buffer this
/// distinction as a newtype, so even singleton reserved-looking objects survive.
struct Validated(V);
impl<'de> serde::Deserializer<'de> for Validated {
    type Error = serde_json::Error;
    fn deserialize_any<W: Visitor<'de>>(
        self,
        visitor: W,
    ) -> std::result::Result<W::Value, Self::Error> {
        use serde::de::IntoDeserializer;
        match self.0 {
            V::Null => visitor.visit_unit(),
            V::Bool(v) => visitor.visit_bool(v),
            V::String(v) => visitor.visit_string(v),
            V::Array(v) => visitor.visit_seq(serde::de::value::SeqDeserializer::new(
                v.into_iter().map(Validated),
            )),
            V::Object(v) => visitor.visit_map(serde::de::value::MapDeserializer::new(
                v.into_iter().map(|(k, v)| (k, Validated(v))),
            )),
            V::Number(v) => {
                if let Some(n) = v.as_u64().filter(|n| n.to_string() == v.to_string()) {
                    visitor.visit_u64(n)
                } else if let Some(n) = v.as_i64().filter(|n| n.to_string() == v.to_string()) {
                    visitor.visit_i64(n)
                } else if let Some(n) = v.as_f64().filter(|n| {
                    serde_json::Number::from_f64(*n)
                        .is_some_and(|exact| exact.to_string() == v.to_string())
                }) {
                    visitor.visit_f64(n)
                } else {
                    visitor.visit_newtype_struct(v.to_string().into_deserializer())
                }
            }
        }
    }
    fn deserialize_option<W: Visitor<'de>>(
        self,
        visitor: W,
    ) -> std::result::Result<W::Value, Self::Error> {
        if self.0.is_null() {
            visitor.visit_none()
        } else {
            visitor.visit_some(self)
        }
    }
    fn deserialize_newtype_struct<W: Visitor<'de>>(
        self,
        name: &'static str,
        visitor: W,
    ) -> std::result::Result<W::Value, Self::Error> {
        if name == "$serde_json::private::RawValue" {
            visitor.visit_map(serde::de::value::MapDeserializer::new(std::iter::once((
                name,
                self.0.to_string(),
            ))))
        } else {
            visitor.visit_newtype_struct(self)
        }
    }
    fn deserialize_enum<W: Visitor<'de>>(
        self,
        _: &'static str,
        _: &'static [&'static str],
        visitor: W,
    ) -> std::result::Result<W::Value, Self::Error> {
        use serde::de::IntoDeserializer;
        match self.0 {
            V::String(name) => visitor.visit_enum(name.into_deserializer()),
            V::Object(map) if map.len() == 1 => {
                let (name, value) = map.into_iter().next().unwrap();
                visitor.visit_enum(Variant {
                    name,
                    value: Validated(value),
                })
            }
            _ => Err(de::Error::custom("expected enum string or one-key object")),
        }
    }
    fn deserialize_i8<W: Visitor<'de>>(
        self,
        visitor: W,
    ) -> std::result::Result<W::Value, Self::Error> {
        serde::Deserializer::deserialize_i8(self.0, visitor)
    }
    fn deserialize_i16<W: Visitor<'de>>(
        self,
        visitor: W,
    ) -> std::result::Result<W::Value, Self::Error> {
        serde::Deserializer::deserialize_i16(self.0, visitor)
    }
    fn deserialize_i32<W: Visitor<'de>>(
        self,
        visitor: W,
    ) -> std::result::Result<W::Value, Self::Error> {
        serde::Deserializer::deserialize_i32(self.0, visitor)
    }
    fn deserialize_i64<W: Visitor<'de>>(
        self,
        visitor: W,
    ) -> std::result::Result<W::Value, Self::Error> {
        serde::Deserializer::deserialize_i64(self.0, visitor)
    }
    fn deserialize_u8<W: Visitor<'de>>(
        self,
        visitor: W,
    ) -> std::result::Result<W::Value, Self::Error> {
        serde::Deserializer::deserialize_u8(self.0, visitor)
    }
    fn deserialize_u16<W: Visitor<'de>>(
        self,
        visitor: W,
    ) -> std::result::Result<W::Value, Self::Error> {
        serde::Deserializer::deserialize_u16(self.0, visitor)
    }
    fn deserialize_u32<W: Visitor<'de>>(
        self,
        visitor: W,
    ) -> std::result::Result<W::Value, Self::Error> {
        serde::Deserializer::deserialize_u32(self.0, visitor)
    }
    fn deserialize_u64<W: Visitor<'de>>(
        self,
        visitor: W,
    ) -> std::result::Result<W::Value, Self::Error> {
        serde::Deserializer::deserialize_u64(self.0, visitor)
    }
    fn deserialize_f64<W: Visitor<'de>>(
        self,
        visitor: W,
    ) -> std::result::Result<W::Value, Self::Error> {
        serde::Deserializer::deserialize_f64(self.0, visitor)
    }
    fn deserialize_f32<W: Visitor<'de>>(
        self,
        visitor: W,
    ) -> std::result::Result<W::Value, Self::Error> {
        serde::Deserializer::deserialize_f32(self.0, visitor)
    }
    fn deserialize_i128<W: Visitor<'de>>(
        self,
        visitor: W,
    ) -> std::result::Result<W::Value, Self::Error> {
        serde::Deserializer::deserialize_i128(self.0, visitor)
    }
    fn deserialize_u128<W: Visitor<'de>>(
        self,
        visitor: W,
    ) -> std::result::Result<W::Value, Self::Error> {
        serde::Deserializer::deserialize_u128(self.0, visitor)
    }
    serde::forward_to_deserialize_any! { bool char str string bytes byte_buf unit unit_struct seq tuple tuple_struct map struct identifier ignored_any }
}
impl<'de> serde::de::IntoDeserializer<'de, serde_json::Error> for Validated {
    type Deserializer = Self;
    fn into_deserializer(self) -> Self {
        self
    }
}
struct Variant {
    name: String,
    value: Validated,
}
impl<'de> serde::de::EnumAccess<'de> for Variant {
    type Error = serde_json::Error;
    type Variant = Validated;
    fn variant_seed<S: serde::de::DeserializeSeed<'de>>(
        self,
        seed: S,
    ) -> std::result::Result<(S::Value, Self::Variant), Self::Error> {
        use serde::de::IntoDeserializer;
        let name = seed.deserialize(self.name.into_deserializer())?;
        Ok((name, self.value))
    }
}
impl<'de> serde::de::VariantAccess<'de> for Validated {
    type Error = serde_json::Error;
    fn unit_variant(self) -> std::result::Result<(), Self::Error> {
        Deserialize::deserialize(self)
    }
    fn newtype_variant_seed<S: serde::de::DeserializeSeed<'de>>(
        self,
        seed: S,
    ) -> std::result::Result<S::Value, Self::Error> {
        seed.deserialize(self)
    }
    fn tuple_variant<W: Visitor<'de>>(
        self,
        _: usize,
        visitor: W,
    ) -> std::result::Result<W::Value, Self::Error> {
        serde::Deserializer::deserialize_seq(self, visitor)
    }
    fn struct_variant<W: Visitor<'de>>(
        self,
        _: &'static [&'static str],
        visitor: W,
    ) -> std::result::Result<W::Value, Self::Error> {
        serde::Deserializer::deserialize_map(self, visitor)
    }
}
impl<'de> Deserialize<'de> for OpaqueJson {
    fn deserialize<D: serde::Deserializer<'de>>(
        deserializer: D,
    ) -> std::result::Result<Self, D::Error> {
        struct OpaqueVisitor;
        impl<'de> Visitor<'de> for OpaqueVisitor {
            type Value = OpaqueJson;
            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("a JSON value")
            }
            fn visit_unit<E: de::Error>(self) -> std::result::Result<Self::Value, E> {
                Ok(OpaqueJson(V::Null))
            }
            fn visit_none<E: de::Error>(self) -> std::result::Result<Self::Value, E> {
                self.visit_unit()
            }
            fn visit_bool<E: de::Error>(self, value: bool) -> std::result::Result<Self::Value, E> {
                Ok(OpaqueJson(V::Bool(value)))
            }
            fn visit_u64<E: de::Error>(self, value: u64) -> std::result::Result<Self::Value, E> {
                Ok(OpaqueJson(V::Number(value.into())))
            }
            fn visit_i64<E: de::Error>(self, value: i64) -> std::result::Result<Self::Value, E> {
                Ok(OpaqueJson(V::Number(value.into())))
            }
            fn visit_f64<E: de::Error>(self, value: f64) -> std::result::Result<Self::Value, E> {
                serde_json::Number::from_f64(value)
                    .map(|n| OpaqueJson(V::Number(n)))
                    .ok_or_else(|| E::custom("non-finite JSON number"))
            }
            fn visit_str<E: de::Error>(self, value: &str) -> std::result::Result<Self::Value, E> {
                Ok(OpaqueJson(V::String(value.into())))
            }
            fn visit_string<E: de::Error>(
                self,
                value: String,
            ) -> std::result::Result<Self::Value, E> {
                Ok(OpaqueJson(V::String(value)))
            }
            fn visit_newtype_struct<D: serde::Deserializer<'de>>(
                self,
                d: D,
            ) -> std::result::Result<Self::Value, D::Error> {
                let number = String::deserialize(d)?;
                Ok(OpaqueJson(V::Number(
                    exact_number(&number).map_err(de::Error::custom)?,
                )))
            }
            fn visit_seq<A: serde::de::SeqAccess<'de>>(
                self,
                mut seq: A,
            ) -> std::result::Result<Self::Value, A::Error> {
                let mut values = Vec::new();
                while let Some(value) = seq.next_element::<OpaqueJson>()? {
                    values.push(value.0);
                }
                Ok(OpaqueJson(V::Array(values)))
            }
            fn visit_map<A: MapAccess<'de>>(
                self,
                mut map: A,
            ) -> std::result::Result<Self::Value, A::Error> {
                let mut values = serde_json::Map::new();
                while let Some((key, value)) = map.next_entry::<String, OpaqueJson>()? {
                    if values.insert(key, value.0).is_some() {
                        return Err(de::Error::custom("duplicate object key"));
                    }
                }
                Ok(OpaqueJson(V::Object(values)))
            }
        }
        deserializer.deserialize_any(OpaqueVisitor)
    }
}

/// Canonical key order matches the historic serde_json object representation.
/// This is used only to calculate persisted declaration fingerprints.
pub(crate) fn canonical_bytes<T: Serialize>(value: &T) -> Vec<u8> {
    serde_json::to_vec(&serde_json::to_value(value).expect("serializable document"))
        .expect("serializable document")
}
// The exact serde_json version is pinned. RawValue and Number parsing validate
// the token before this constructor preserves lexical forms such as -0 and 1e+2.
fn exact_number(text: &str) -> std::result::Result<serde_json::Number, serde_json::Error> {
    let _: serde_json::Number = text.parse()?;
    Ok(serde_json::Number::from_string_unchecked(text.to_owned()))
}
/// Reject duplicate keys; serde_json's default Value visitor silently accepts them.
fn strict_value(bytes: &[u8]) -> std::result::Result<V, serde_json::Error> {
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
            _ => V::Number(exact_number(text)?),
        })
    }
    let raw: &RawValue = serde_json::from_slice(bytes)?;
    value(raw, 0)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[derive(Serialize, Deserialize)]
    struct Known {
        id: String,
        #[serde(flatten)]
        extra: OpaqueObject,
    }
    #[test]
    fn opaque_numeric_spelling_and_precision_are_preserved() {
        for raw in [
            "-0",
            "1.2300e+20",
            "0.12345678901234567890123456789",
            "123456789012345678901234567890",
        ] {
            let value: OpaqueJson = strict_json(raw.as_bytes()).unwrap();
            assert_eq!(serde_json::to_string(&value).unwrap(), raw);
        }
    }
    #[test]
    fn raw_numeric_ids_and_unknown_reserved_objects_coexist() {
        #[derive(Deserialize)]
        struct Envelope {
            id: Box<serde_json::value::RawValue>,
            params: OpaqueObject,
        }
        let parsed: Envelope = strict_json(br#"{"id":1.2300e+20,"params":{"unknown":{"$serde_json::private::Number":"123"},"number":123456789012345678901234567890}}"#).unwrap();
        assert_eq!(parsed.id.get(), "1.2300e+20");
        assert_eq!(
            serde_json::to_string(&parsed.params["unknown"]).unwrap(),
            r#"{"$serde_json::private::Number":"123"}"#
        );
        assert_eq!(
            serde_json::to_string(&parsed.params["number"]).unwrap(),
            "123456789012345678901234567890"
        );
    }
    #[test]
    fn unknown_fields_and_numbers_round_trip_without_weakening_validation() {
        let raw=br#"{"id":"one","unknown":{"n":123456789012345678901234567890,"$serde_json::private::Number":"not-a-number"}}"#;
        let parsed: Known = strict_json(raw).unwrap();
        assert_eq!(
            strict_value(&canonical_bytes(&parsed)).unwrap(),
            strict_value(raw).unwrap()
        );
        let singleton: Known = strict_json(
            br#"{"id":"one","unknown":{"$serde_json::private::Number":"not-a-number"}}"#,
        )
        .unwrap();
        assert!(
            String::from_utf8(canonical_bytes(&singleton))
                .unwrap()
                .contains("not-a-number")
        );
        assert!(strict_json::<Known>(br#"{"id":"one","extra":{"same":1,"same":2}}"#).is_err());
        assert!(strict_json::<Known>(b"{} {}").is_err());
        assert!(strict_json::<Known>(br#"{"id":1}"#).is_err());
    }
}
