//! Ajudantes compartilhados pelos testes de integração. Fixtures anonimizadas:
//! `conta1@exemplo.com`, `C:\Users\exemplo`, org `Acme`.
#![allow(dead_code)]

use chrono::{DateTime, TimeZone, Utc};
use router_core::{Account, AccountIdentity, ConfigDir, Id, Provider};

/// Epoch (segundos) → data UTC, sem fração.
pub fn ts(secs: i64) -> DateTime<Utc> {
    Utc.timestamp_opt(secs, 0).single().unwrap()
}

pub fn ident(email: &str) -> AccountIdentity {
    let mut raw = serde_json::Map::new();
    raw.insert(
        "emailAddress".to_string(),
        serde_json::Value::String(email.to_string()),
    );
    AccountIdentity {
        email: email.to_string(),
        organization_name: Some("Acme".to_string()),
        rate_limit_tier: Some("default_claude_max_5x".to_string()),
        raw,
    }
}

pub fn account(email: &str, home: &str) -> Account {
    Account {
        id: Id::new(),
        provider: Provider::Anthropic,
        identity: ident(email),
        home: ConfigDir::dedicated(home),
        nickname: None,
    }
}
