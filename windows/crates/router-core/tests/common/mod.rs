//! Ajudantes compartilhados pelos testes de integração. Fixtures anonimizadas:
//! `conta1@exemplo.com`, `C:\Users\exemplo`, org `Acme`.
#![allow(dead_code)]

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

use chrono::{DateTime, TimeZone, Utc};
use router_core::engine::credential_store::{CredentialBlob, CredentialError, CredentialStore};
use router_core::engine::provider::{IdentityError, Provider, ProviderAdapter};
use router_core::{Account, AccountIdentity, ConfigDir, Id};

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

/// Um blob de teste. O fake não valida forma — como o `FakeKeychain` do Swift,
/// que guardava qualquer string.
pub fn cred(text: &str) -> CredentialBlob {
    CredentialBlob::from_bytes(text.as_bytes().to_vec())
}

/// Credenciais em memória (≙ `FakeKeychain`): escrever nunca falha, e as
/// escritas e remoções ficam registradas para os testes conferirem.
#[derive(Default)]
pub struct FakeStore {
    items: Mutex<HashMap<PathBuf, CredentialBlob>>,
    writes: Mutex<Vec<(PathBuf, CredentialBlob)>>,
    deletes: Mutex<Vec<PathBuf>>,
}

impl FakeStore {
    pub fn new() -> Self {
        Self::default()
    }

    /// Quantas escritas foram feitas num lugar.
    pub fn writes_to(&self, location: &Path) -> usize {
        self.writes
            .lock()
            .unwrap()
            .iter()
            .filter(|(p, _)| p == location)
            .count()
    }

    pub fn deleted(&self, location: &Path) -> bool {
        self.deletes.lock().unwrap().iter().any(|p| p == location)
    }
}

impl CredentialStore for FakeStore {
    fn read(&self, location: &Path) -> Option<CredentialBlob> {
        self.items.lock().unwrap().get(location).cloned()
    }

    fn write(&self, blob: &CredentialBlob, location: &Path) -> Result<(), CredentialError> {
        self.items
            .lock()
            .unwrap()
            .insert(location.to_path_buf(), blob.clone());
        self.writes
            .lock()
            .unwrap()
            .push((location.to_path_buf(), blob.clone()));
        Ok(())
    }

    fn exists(&self, location: &Path) -> bool {
        self.items.lock().unwrap().contains_key(location)
    }

    fn delete(&self, location: &Path) {
        self.items.lock().unwrap().remove(location);
        self.deletes.lock().unwrap().push(location.to_path_buf());
    }
}

/// Adapter falso (≙ `FakeAdapter`): identidade em memória por `ConfigDir.raw`,
/// para o motor não precisar de arquivo. O "lugar" da credencial segue a mesma
/// regra de distinguir padrão e dedicado.
#[derive(Default)]
pub struct FakeAdapter {
    identities: Mutex<HashMap<String, AccountIdentity>>,
    fail_identity_writes: AtomicBool,
}

impl FakeAdapter {
    pub fn new() -> Self {
        Self::default()
    }

    /// Faz toda escrita de identidade falhar (para o caminho de erro do motor).
    pub fn fail_identity_writes(&self) {
        self.fail_identity_writes.store(true, Ordering::SeqCst);
    }
}

impl ProviderAdapter for FakeAdapter {
    fn provider(&self) -> Provider {
        Provider::Anthropic
    }

    fn credential_location(&self, dir: &ConfigDir) -> PathBuf {
        if dir.is_default {
            PathBuf::from("svc-default")
        } else {
            PathBuf::from(format!("svc-{}", dir.raw))
        }
    }

    fn identity(&self, dir: &ConfigDir) -> Option<AccountIdentity> {
        self.identities.lock().unwrap().get(&dir.raw).cloned()
    }

    fn write_identity(
        &self,
        identity: &AccountIdentity,
        dir: &ConfigDir,
    ) -> Result<(), IdentityError> {
        if self.fail_identity_writes.load(Ordering::SeqCst) {
            return Err(IdentityError::NotPersisted {
                path: PathBuf::from(&dir.raw),
            });
        }
        self.identities
            .lock()
            .unwrap()
            .insert(dir.raw.clone(), identity.clone());
        Ok(())
    }

    fn launch_command(&self) -> (String, Vec<String>) {
        ("claude".to_string(), Vec::new())
    }
}
