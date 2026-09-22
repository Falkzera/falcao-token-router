//! Onde tudo do produto mora no disco.
//!
//! No Windows a raiz é `%LOCALAPPDATA%\com.synqo.falcao-router` (Local, não
//! Roaming — credencial não pode viajar entre máquinas). O namespace de dados
//! **diverge do bundle id de propósito e não pode ser "corrigido"** (é o mesmo do
//! macOS; mudá-lo tornaria toda credencial inalcançável de uma vez).

use std::env;
use std::path::PathBuf;

use super::config_dir::ConfigDir;
use super::group_usage::GroupUsageStore;
use crate::ids::Id;

#[derive(Clone, Debug)]
pub struct RouterPaths {
    pub base: PathBuf,
}

impl RouterPaths {
    /// Namespace de armazenamento. Não muda quando o app é renomeado.
    pub const BUNDLE_ID: &'static str = "com.synqo.falcao-router";

    /// Resolve a raiz: `ROUTER_APP_SUPPORT` (teste/override) → `%LOCALAPPDATA%`.
    pub fn new() -> Self {
        Self::with_app_support(None)
    }

    /// Raiz explícita (para teste), senão a resolução padrão.
    pub fn with_app_support(app_support: Option<PathBuf>) -> Self {
        let root = app_support
            .or_else(|| env::var_os("ROUTER_APP_SUPPORT").map(PathBuf::from))
            .or_else(|| env::var_os("LOCALAPPDATA").map(PathBuf::from))
            .unwrap_or_else(env::temp_dir);
        RouterPaths {
            base: root.join(Self::BUNDLE_ID),
        }
    }

    /// O `RouterConfig` inteiro, num JSON.
    pub fn config_file(&self) -> PathBuf {
        self.base.join("config.json")
    }

    /// Onde o sensor grava as amostras de uso: `<base>\usage`.
    pub fn usage_dir(&self) -> PathBuf {
        GroupUsageStore::directory(&self.base)
    }

    /// A casa de cada conta — o perfil onde ela faz `auth login`. Uma por conta.
    pub fn account_home(&self, id: Id) -> ConfigDir {
        ConfigDir::dedicated(
            self.base
                .join("accounts")
                .join(id.to_string())
                .to_string_lossy()
                .into_owned(),
        )
    }

    /// O perfil onde as sessões de um grupo rodam. O grupo padrão usa o
    /// `<home>\.claude` (sem variável); os demais, um perfil dedicado.
    pub fn group_config_dir(&self, id: Id, is_default: bool, home: &str) -> ConfigDir {
        if is_default {
            ConfigDir::standard(home)
        } else {
            ConfigDir::dedicated(
                self.base
                    .join("groups")
                    .join(id.to_string())
                    .to_string_lossy()
                    .into_owned(),
            )
        }
    }
}

impl Default for RouterPaths {
    fn default() -> Self {
        Self::new()
    }
}
