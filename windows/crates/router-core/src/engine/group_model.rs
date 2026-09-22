//! Um grupo de contas que rotacionam entre si, e a configuração inteira do app
//! (`config.json`). Formato idêntico ao do macOS — é o contrato do PORTING.md.

use serde::{Deserialize, Serialize};

use super::account_model::Account;
use super::config_dir::ConfigDir;
use super::provider::Provider;
use crate::ids::Id;

pub const CURRENT_VERSION: i64 = 1;

fn default_threshold() -> f64 {
    90.0
}
fn default_true() -> bool {
    true
}
fn default_version() -> i64 {
    CURRENT_VERSION
}

/// Um grupo de contas que rotacionam entre si. Cada grupo tem o seu perfil e a
/// sua lista ordenada, então vários rotacionam em paralelo sem um pisar no outro.
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountGroup {
    pub id: Id,
    pub name: String,
    #[serde(default)]
    pub provider: Provider,

    /// As contas na **ordem de preferência**: a primeira disponível é a escolhida.
    /// Chave `accountIDs` (com o "ID" maiúsculo, como o macOS grava).
    #[serde(rename = "accountIDs")]
    pub account_ids: Vec<Id>,

    /// O perfil onde as sessões deste grupo rodam. Um único grupo pode ser o
    /// padrão (sem variável de ambiente); os demais têm perfil dedicado.
    pub config_dir: ConfigDir,

    /// A partir de quanto de uso trocar de conta (por grupo).
    #[serde(default = "default_threshold")]
    pub threshold_percent: f64,

    /// Rotação automática ligada.
    #[serde(default = "default_true")]
    pub auto_rotate: bool,
}

impl AccountGroup {
    pub fn new(name: impl Into<String>, config_dir: ConfigDir) -> Self {
        AccountGroup {
            id: Id::new(),
            name: name.into(),
            provider: Provider::Anthropic,
            account_ids: Vec::new(),
            config_dir,
            threshold_percent: 90.0,
            auto_rotate: true,
        }
    }
}

/// A configuração inteira do app, persistida como um JSON só (`config.json`).
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RouterConfig {
    #[serde(default)]
    pub accounts: Vec<Account>,
    #[serde(default)]
    pub groups: Vec<AccountGroup>,

    /// Um só histórico entre os grupos (`--resume` enxerga tudo), ou cada grupo
    /// com o seu. Padrão de fábrica é compartilhado.
    #[serde(default = "default_true")]
    pub share_history: bool,

    /// Versão do formato, para migrar sem perder configuração.
    #[serde(default = "default_version")]
    pub version: i64,
}

impl Default for RouterConfig {
    fn default() -> Self {
        RouterConfig {
            accounts: Vec::new(),
            groups: Vec::new(),
            share_history: true,
            version: CURRENT_VERSION,
        }
    }
}

impl RouterConfig {
    pub fn account(&self, id: Id) -> Option<&Account> {
        self.accounts.iter().find(|a| a.id == id)
    }

    /// As contas de um grupo, na ordem de preferência, ignorando IDs órfãos.
    pub fn accounts_in(&self, group: &AccountGroup) -> Vec<&Account> {
        group
            .account_ids
            .iter()
            .filter_map(|id| self.accounts.iter().find(|a| a.id == *id))
            .collect()
    }

    /// O grupo marcado como padrão — o que roda no `~/.claude`. No máximo um.
    pub fn default_group(&self) -> Option<&AccountGroup> {
        self.groups.iter().find(|g| g.config_dir.is_default)
    }
}
