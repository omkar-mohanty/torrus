use serde::{Deserialize, Deserializer, Serialize};
use std::{ops::Deref, str::FromStr};

/// [ID] is for info hash, 20 byte identity for Peers.
#[derive(Debug, Clone, Copy)]
pub struct ID([u8; 20]);

impl Default for ID {
    fn default() -> Self {
        ID([0u8; 20])
    }
}

impl Deref for ID {
    type Target = [u8; 20];
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl FromStr for ID {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut out = [0u8; 20];
        if s.len() != 40 {
            anyhow::bail!("Expected size of 40");
        }
        hex::decode_to_slice(s, &mut out)?;
        Ok(ID(out))
    }
}

impl Serialize for ID {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_bytes(&self.0)
    }
}

impl<'de> Deserialize<'de> for ID {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct Visitor;

        impl<'de> serde::de::Visitor<'de> for Visitor {
            type Value = ID;
            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                write!(formatter, "A 20 byte slice or 20 byte string")
            }
            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                if v.len() != 40 {
                    return Err(E::invalid_length(40, &self));
                }
                let mut out = [0u8; 20];
                match hex::decode_to_slice(v, &mut out) {
                    Ok(_) => Ok(ID(out)),
                    Err(e) => Err(E::custom(e)),
                }
            }
            fn visit_borrowed_bytes<E>(self, v: &'de [u8]) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                self.visit_bytes(v)
            }
            fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                if v.len() != 20 {
                    return Err(E::invalid_length(20, &self));
                }
                let mut buf = [0u8; 20];
                buf.copy_from_slice(v);
                Ok(ID(buf))
            }
        }

        deserializer.deserialize_any(Visitor {})
    }
}
