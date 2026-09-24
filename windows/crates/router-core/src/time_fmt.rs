//! Datas ISO-8601 **sem fração de segundo** (`2026-09-18T12:00:00Z`).
//!
//! O decodificador `.iso8601` do Swift recusa fração, então a amostra é escrita
//! sem ela — e o porte segue igual. Na leitura, aceitamos qualquer RFC-3339
//! (com ou sem fração, `Z` ou offset), por robustez.

use chrono::{DateTime, SecondsFormat, Utc};
use serde::{Deserialize, Deserializer, Serializer};

fn parse(s: &str) -> Result<DateTime<Utc>, chrono::ParseError> {
    DateTime::parse_from_rfc3339(s).map(|dt| dt.with_timezone(&Utc))
}

fn format(dt: &DateTime<Utc>) -> String {
    // Secs = sem fração; use_z = true → sufixo `Z` em vez de `+00:00`.
    dt.to_rfc3339_opts(SecondsFormat::Secs, true)
}

/// Data obrigatória (ex.: `sampledAt`).
pub mod required {
    use super::*;

    pub fn serialize<S: Serializer>(dt: &DateTime<Utc>, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&format(dt))
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<DateTime<Utc>, D::Error> {
        let s = String::deserialize(d)?;
        parse(&s).map_err(serde::de::Error::custom)
    }
}

/// Data opcional (ex.: `fiveHourResetsAt`). Ausente vira `None`; nunca é escrita
/// quando `None`.
pub mod optional {
    use super::*;

    pub fn serialize<S: Serializer>(dt: &Option<DateTime<Utc>>, s: S) -> Result<S::Ok, S::Error> {
        match dt {
            Some(dt) => s.serialize_str(&format(dt)),
            None => s.serialize_none(),
        }
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Option<DateTime<Utc>>, D::Error> {
        let opt = Option::<String>::deserialize(d)?;
        match opt {
            Some(s) => parse(&s).map(Some).map_err(serde::de::Error::custom),
            None => Ok(None),
        }
    }
}
