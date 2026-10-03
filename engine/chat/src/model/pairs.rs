//! Insertion-ordered structured-key maps encoded as Kotlin alternating arrays.
use serde::{
    Deserialize, Serialize,
    de::{Error, SeqAccess, Visitor},
    ser::SerializeSeq,
};
use std::fmt;

#[derive(Clone, Debug, PartialEq)]
pub struct Pairs<K, V>(pub Vec<(K, V)>);
impl<K, V> Default for Pairs<K, V> {
    fn default() -> Self {
        Self(Vec::new())
    }
}
impl<K: PartialEq, V> Pairs<K, V> {
    pub fn get(&self, key: &K) -> Option<&V> {
        self.0.iter().find(|(k, _)| k == key).map(|(_, v)| v)
    }
    pub fn get_mut(&mut self, key: &K) -> Option<&mut V> {
        self.0.iter_mut().find(|(k, _)| k == key).map(|(_, v)| v)
    }
    pub fn insert(&mut self, key: K, value: V) {
        if let Some(old) = self.get_mut(&key) {
            *old = value;
        } else {
            self.0.push((key, value));
        }
    }
    pub fn remove(&mut self, key: &K) -> Option<V> {
        self.0
            .iter()
            .position(|(k, _)| k == key)
            .map(|i| self.0.remove(i).1)
    }
    pub fn values(&self) -> impl Iterator<Item = &V> {
        self.0.iter().map(|(_, v)| v)
    }
    pub fn values_mut(&mut self) -> impl Iterator<Item = &mut V> {
        self.0.iter_mut().map(|(_, v)| v)
    }
}
impl<K: Serialize, V: Serialize> Serialize for Pairs<K, V> {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let mut seq = s.serialize_seq(Some(self.0.len() * 2))?;
        for (key, value) in &self.0 {
            seq.serialize_element(key)?;
            seq.serialize_element(value)?;
        }
        seq.end()
    }
}
impl<'de, K: Deserialize<'de> + PartialEq, V: Deserialize<'de>> Deserialize<'de> for Pairs<K, V> {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct PairsVisitor<K, V>(std::marker::PhantomData<(K, V)>);
        impl<'de, K: Deserialize<'de> + PartialEq, V: Deserialize<'de>> Visitor<'de>
            for PairsVisitor<K, V>
        {
            type Value = Pairs<K, V>;
            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("an alternating key/value array")
            }
            fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
                let mut result = Pairs::default();
                while let Some(key) = seq.next_element::<K>()? {
                    let value = seq
                        .next_element::<V>()?
                        .ok_or_else(|| A::Error::custom("structured map lacks value"))?;
                    if result.get(&key).is_some() {
                        return Err(A::Error::custom("duplicate structured map key"));
                    }
                    result.insert(key, value);
                }
                Ok(result)
            }
        }
        d.deserialize_seq(PairsVisitor(std::marker::PhantomData))
    }
}
