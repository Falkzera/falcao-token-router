//! Identificador de conta/grupo.
//!
//! Um UUID que serializa em **MAIÚSCULAS** (`uuidString` do Swift é maiúsculo, e
//! o `config.json` que o macOS grava tem os ids assim). Na leitura aceita
//! qualquer caixa, para não recusar um arquivo escrito por outra ferramenta.

use std::fmt;
use std::ops::Deref;

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use uuid::Uuid;

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Id(pub Uuid);

impl Id {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Id(Uuid::new_v4())
    }

    pub fn parse(s: &str) -> Result<Self, uuid::Error> {
        Ok(Id(Uuid::parse_str(s)?))
    }
}

impl Deref for Id {
    type Target = Uuid;
    fn deref(&self) -> &Uuid {
        &self.0
    }
}

impl fmt::Display for Id {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Maiúsculas, como o `uuidString` do Swift.
        write!(f, "{}", self.0.hyphenated().to_string().to_uppercase())
    }
}

impl fmt::Debug for Id {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Id({self})")
    }
}

impl Serialize for Id {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for Id {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let s = String::deserialize(d)?;
        Uuid::parse_str(&s)
            .map(Id)
            .map_err(serde::de::Error::custom)
    }
}
