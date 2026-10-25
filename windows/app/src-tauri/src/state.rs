//! O estado do app, um só por processo: o store do motor (a MESMA montagem da
//! CLI — credencial em arquivo com a guarda do perfil padrão), o idioma, a aba
//! que a janela deve abrir e o que a medição (botão e agenda) precisa lembrar.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard};

use chrono::{DateTime, Utc};
use router_core::engine::anthropic_adapter::AnthropicAdapter;
use router_core::engine::credential_store::FileCredentialStore;
use router_core::engine::default_profile_guard::DefaultProfileGuard;
use router_core::engine::provider::ProviderAdapter;
use router_core::engine::router_config_store::RouterConfigStore;
use router_core::engine::router_paths::RouterPaths;
use router_core::Id;
use serde::{Deserialize, Serialize};

use crate::locale::{self, Locale};

/// As abas da janela única.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HomeTab {
    Groups,
    History,
    Settings,
}

/// Para onde a janela vai: a aba e, no Histórico, a conta a mostrar — o atalho
/// de cada conta (21/10/2026), que pode vir do painel da bandeja, outra janela.
/// É o que o evento `navigate` leva e o que a janela que ainda vai abrir lê.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Navigation {
    pub tab: HomeTab,
    #[serde(default)]
    pub account: Option<String>,
}

impl Navigation {
    pub fn to(tab: HomeTab) -> Self {
        Navigation { tab, account: None }
    }
}

pub struct AppState {
    store: Mutex<RouterConfigStore>,
    pub locale: Locale,
    /// A home do usuário (`%USERPROFILE%`), de onde sai o perfil padrão.
    pub home: String,
    /// O destino pedido para a PRÓXIMA abertura da janela (quem pede é a
    /// bandeja, antes de a janela existir).
    pending: Mutex<Option<Navigation>>,
    /// O grupo que a sonda está medindo agora (fora da trava do store: a
    /// sonda leva segundos por conta e a bandeja segue respondendo).
    measuring: Mutex<Option<Id>>,
    /// A última tentativa da medição automática em cada conta, nesta execução
    /// do app — o que segura uma conta deslogada (ver `MeasureSchedule`).
    measure_attempts: Mutex<HashMap<Id, DateTime<Utc>>>,
}

/// Um `Mutex` envenenado (pânico noutra thread) não pode derrubar a bandeja:
/// o dado continua lá, e o app segue.
fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

impl AppState {
    pub fn open() -> Self {
        let paths = RouterPaths::new();
        let home = std::env::var("USERPROFILE")
            .or_else(|_| std::env::var("HOME"))
            .unwrap_or_default();
        // A guarda: antes da 1ª escrita do router no `~\.claude`, o login que
        // estava lá é copiado para `<base>\backups\`.
        let credentials = Arc::new(FileCredentialStore::with_guard(
            DefaultProfileGuard::for_home(&home, &paths.base),
        ));
        let adapters: Vec<Arc<dyn ProviderAdapter>> = vec![Arc::new(AnthropicAdapter)];
        let store = RouterConfigStore::new(paths, home.clone(), credentials, adapters);
        AppState {
            store: Mutex::new(store),
            locale: locale::system(),
            home,
            pending: Mutex::new(None),
            measuring: Mutex::new(None),
            measure_attempts: Mutex::new(HashMap::new()),
        }
    }

    pub fn measuring(&self) -> Option<Id> {
        *lock(&self.measuring)
    }

    pub fn set_measuring(&self, group: Option<Id>) {
        *lock(&self.measuring) = group;
    }

    /// Marca o grupo como "medindo" se nada está sendo medido. Atômico: o
    /// botão e a agenda de 5 em 5 horas nunca medem ao mesmo tempo.
    pub fn try_start_measuring(&self, group: Id) -> bool {
        let mut measuring = lock(&self.measuring);
        if measuring.is_some() {
            return false;
        }
        *measuring = Some(group);
        true
    }

    pub fn measure_attempts(&self) -> HashMap<Id, DateTime<Utc>> {
        lock(&self.measure_attempts).clone()
    }

    pub fn record_attempts(&self, accounts: impl IntoIterator<Item = Id>, at: DateTime<Utc>) {
        let mut attempts = lock(&self.measure_attempts);
        for id in accounts {
            attempts.insert(id, at);
        }
    }

    pub fn store(&self) -> MutexGuard<'_, RouterConfigStore> {
        lock(&self.store)
    }

    pub fn request(&self, navigation: Navigation) {
        *lock(&self.pending) = Some(navigation);
    }

    /// O destino pedido (uma vez); sem pedido, Grupos.
    pub fn take_request(&self) -> Navigation {
        lock(&self.pending)
            .take()
            .unwrap_or(Navigation::to(HomeTab::Groups))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// O atalho de cada conta leva a conta junto com a aba — no evento
    /// `navigate` e no `app_info` da janela que ainda vai abrir.
    #[test]
    fn the_history_shortcut_travels_with_the_account() {
        let shortcut = Navigation {
            tab: HomeTab::History,
            account: Some("AAAAAAAA-0000-4000-8000-000000000001".into()),
        };
        assert_eq!(
            serde_json::to_value(&shortcut).unwrap(),
            json!({"tab": "history", "account": "AAAAAAAA-0000-4000-8000-000000000001"})
        );
        assert_eq!(
            serde_json::to_value(Navigation::to(HomeTab::Settings)).unwrap(),
            json!({"tab": "settings", "account": null})
        );
    }
}
