//! O estado do app, um só por processo: o store do motor (a MESMA montagem da
//! CLI — credencial em arquivo com a guarda do perfil padrão), o idioma e a aba
//! que a janela deve abrir.

use std::sync::{Arc, Mutex, MutexGuard};

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
    Settings,
}

pub struct AppState {
    store: Mutex<RouterConfigStore>,
    pub locale: Locale,
    /// A home do usuário (`%USERPROFILE%`), de onde sai o perfil padrão.
    pub home: String,
    /// A aba pedida para a PRÓXIMA abertura da janela (quem pede é a bandeja,
    /// antes de a janela existir).
    pending_tab: Mutex<Option<HomeTab>>,
    /// O grupo que a sonda está medindo agora (fora da trava do store: a
    /// sonda leva segundos por conta e a bandeja segue respondendo).
    measuring: Mutex<Option<Id>>,
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
            pending_tab: Mutex::new(None),
            measuring: Mutex::new(None),
        }
    }

    pub fn measuring(&self) -> Option<Id> {
        *lock(&self.measuring)
    }

    pub fn set_measuring(&self, group: Option<Id>) {
        *lock(&self.measuring) = group;
    }

    pub fn store(&self) -> MutexGuard<'_, RouterConfigStore> {
        lock(&self.store)
    }

    pub fn request_tab(&self, tab: HomeTab) {
        *lock(&self.pending_tab) = Some(tab);
    }

    /// A aba pedida (uma vez); sem pedido, Grupos.
    pub fn take_tab(&self) -> HomeTab {
        lock(&self.pending_tab).take().unwrap_or(HomeTab::Groups)
    }
}
