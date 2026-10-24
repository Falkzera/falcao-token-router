//! Prepara o lançamento de uma sessão num grupo (≙ `SessionLauncher.swift`):
//! escolhe a conta certa, ativa-a e devolve o que o processo precisa para subir.
//! O processo em si fica na CLI — aqui é só a decisão, testável sem lançar nada.

use std::collections::HashMap;
use std::sync::Arc;

use super::account_model::Account;
use super::config_dir::ConfigDir;
use super::credential_store::CredentialStore;
use super::group_model::{AccountGroup, RouterConfig};
use super::provider::ProviderAdapter;
use super::rotation_engine::RotationEngine;
use crate::ids::Id;

/// O que a CLI precisa para subir a sessão no grupo.
#[derive(Clone, PartialEq, Debug)]
pub struct LaunchPlan {
    /// Os argumentos que o usuário passou depois do grupo.
    pub arguments: Vec<String>,
    /// O valor de `CLAUDE_CONFIG_DIR`, ou `None` no grupo padrão (que não exporta
    /// a variável — setá-la sobe a sessão deslogada).
    pub config_dir_env: Option<String>,
    /// A conta que ficou ativa para esta sessão.
    pub account: Account,
}

#[derive(Clone, PartialEq, Eq, Debug, thiserror::Error)]
pub enum LaunchError {
    #[error("o grupo {0} não tem contas — adicione uma no app")]
    EmptyGroup(String),
    #[error("nenhuma conta do grupo {0} pôde ser ativada — use Relogar no app")]
    NoUsableAccount(String),
}

pub struct SessionLauncher {
    engine: RotationEngine,
}

impl SessionLauncher {
    pub fn new(
        credentials: Arc<dyn CredentialStore>,
        adapters: Vec<Arc<dyn ProviderAdapter>>,
    ) -> Self {
        SessionLauncher {
            engine: RotationEngine::new(credentials, adapters),
        }
    }

    /// O perfil padrão (`~\.claude`) desta máquina — ver
    /// `RotationEngine::where_else_live`.
    pub fn with_default_profile(self, dir: ConfigDir) -> Self {
        SessionLauncher {
            engine: self.engine.with_default_profile(dir),
        }
    }

    /// Acha um grupo pelo nome, sem diferenciar maiúsculas nem espaços nas pontas.
    pub fn group_named<'c>(name: &str, config: &'c RouterConfig) -> Option<&'c AccountGroup> {
        let wanted = name.trim().to_lowercase();
        config
            .groups
            .iter()
            .find(|g| g.name.trim().to_lowercase() == wanted)
    }

    /// Escolhe a conta que vai servir a sessão e a ativa.
    ///
    /// Ordem: mantém a ativa se ela ainda tem folga; senão a próxima com folga na
    /// ordem de preferência; senão a ativa mesmo; senão (grupo nunca ativado) a
    /// primeira — é melhor subir numa conta cheia do que recusar o lançamento.
    pub fn prepare(
        &self,
        group: &AccountGroup,
        config: &RouterConfig,
        usage: &HashMap<Id, f64>,
        arguments: Vec<String>,
    ) -> Result<LaunchPlan, LaunchError> {
        let accounts = config.accounts_in(group);
        let Some(first) = accounts.first() else {
            return Err(LaunchError::EmptyGroup(group.name.clone()));
        };
        let active = self.engine.active_account(group, config);
        let threshold = group.threshold_percent / 100.0;

        // Sem ativa e ninguém com folga: a primeira que PODE servir, não a
        // primeira da lista às cegas — sem credencial na casa, ou viva noutro
        // perfil, ela fazia o lançamento recusar com outra conta disponível.
        let chosen = match active {
            Some(a) if usage.get(&a.id).is_some_and(|u| *u < threshold) => a,
            _ => match self.engine.next_account(group, config, usage) {
                Some(next) => next,
                None => match active {
                    Some(a) => a,
                    None => accounts
                        .iter()
                        .copied()
                        .find(|a| self.engine.can_serve(a, group, config))
                        .unwrap_or(first),
                },
            },
        };

        self.engine
            .activate(chosen, group, config)
            .map_err(|_| LaunchError::NoUsableAccount(group.name.clone()))?;

        Ok(LaunchPlan {
            arguments,
            config_dir_env: group.config_dir.environment_value().map(String::from),
            account: chosen.clone(),
        })
    }
}
