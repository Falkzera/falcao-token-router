//! O ato de trocar a conta ativa de um grupo, e as regras que impedem os erros
//! que já mataram contas (≙ `RotationEngine.swift`, regra por regra).
//!
//! A troca em si é uma cópia: a credencial da **casa** da conta (o perfil onde
//! ela fez login) vai para o perfil do **grupo**, e a identidade é gravada no
//! `.claude.json` do grupo. No Windows a credencial é o arquivo
//! `<perfil>\.credentials.json`; a sessão viva relê o arquivo quando o mtime
//! muda e passa a atender pela conta nova — sem reiniciar, sem `--resume`
//! (confirmado no spike de 22/09/2026).
//!
//! Duas regras separam isto do rodízio caseiro que falhou:
//!
//! - **Espelhar antes de trocar.** Enquanto uma conta está ativa num grupo, é a
//!   cópia do GRUPO que o Claude Code renova; a casa fica para trás. Antes de
//!   ativar outra conta, o token fresco do grupo volta para a casa da que sai.
//! - **Uma conta, um lugar.** A mesma conta em dois grupos ativos seria duas
//!   cópias de um refresh token que gira — a falha silenciosa que derrubou
//!   contas. O motor recusa.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use super::account_model::Account;
use super::anthropic_adapter::AnthropicAdapter;
use super::config_dir::ConfigDir;
use super::credential_store::CredentialStore;
use super::group_model::{AccountGroup, RouterConfig};
use super::provider::{Provider, ProviderAdapter};
use crate::ids::Id;

#[derive(Clone, PartialEq, Eq, Debug, thiserror::Error)]
pub enum RotationError {
    /// A conta não tem credencial legível na casa. Saída: relogar.
    #[error("a conta {account_id} não tem credencial na casa — relogue")]
    NoCredential { account_id: Id },
    /// A conta já serve outro grupo agora. Ativar aqui duplicaria o token.
    #[error("a conta {account_id} já está ativa no grupo {group_id}")]
    AccountBusyElsewhere { account_id: Id, group_id: Id },
    /// A conta é a do `~\.claude` e nenhum grupo usa esse perfil: o `claude`
    /// puro continua servido por ela. Ativar aqui duplicaria o token.
    #[error("a conta {account_id} é a do perfil padrão, que nenhum grupo usa")]
    AccountLiveInDefaultProfile { account_id: Id },
    /// Falha ao escrever a credencial ou o `.claude.json`.
    #[error("falha ao escrever: {0}")]
    WriteFailed(String),
}

pub struct RotationEngine {
    credentials: Arc<dyn CredentialStore>,
    adapters: Vec<Arc<dyn ProviderAdapter>>,
    /// O `~\.claude` — o perfil do `claude` puro (ver `where_else_live`). O
    /// store e a CLI sempre o passam; sem ele a regra do perfil padrão não vale.
    default_profile: Option<ConfigDir>,
}

impl RotationEngine {
    pub fn new(
        credentials: Arc<dyn CredentialStore>,
        adapters: Vec<Arc<dyn ProviderAdapter>>,
    ) -> Self {
        RotationEngine {
            credentials,
            adapters,
            default_profile: None,
        }
    }

    /// O perfil padrão (`~\.claude`) desta máquina.
    pub fn with_default_profile(mut self, dir: ConfigDir) -> Self {
        self.default_profile = Some(dir);
        self
    }

    fn adapter(&self, provider: Provider) -> Arc<dyn ProviderAdapter> {
        self.adapters
            .iter()
            .find(|a| a.provider() == provider)
            .cloned()
            .unwrap_or_else(|| Arc::new(AnthropicAdapter))
    }

    /// A conta que serve um grupo agora, comparando a identidade gravada no
    /// perfil do grupo com as contas do grupo. `None` se o grupo está vazio ou
    /// não tem identidade legível.
    pub fn active_account<'c>(
        &self,
        group: &AccountGroup,
        config: &'c RouterConfig,
    ) -> Option<&'c Account> {
        let identity = self.adapter(group.provider).identity(&group.config_dir)?;
        config
            .accounts_in(group)
            .into_iter()
            .find(|a| a.identity.email == identity.email)
    }

    /// Ativa uma conta no grupo: espelha a que sai, recusa duplicação, copia a
    /// credencial para o grupo e grava a identidade.
    ///
    /// Idempotente: reativar a conta que já está ativa não faz nada além de
    /// garantir que a casa está em dia com o grupo.
    pub fn activate(
        &self,
        account: &Account,
        group: &AccountGroup,
        config: &RouterConfig,
    ) -> Result<Account, RotationError> {
        let adapter = self.adapter(group.provider);
        let group_location = adapter.credential_location(&group.config_dir);

        // Já é a ativa deste grupo? Então a cópia do GRUPO é a viva — é nela que
        // o Claude Code renova, e o refresh token GIRA a cada renovação. Copiar a
        // casa por cima mataria o token girado (o "Login expired" de 26/ago no
        // macOS). O movimento certo é o inverso: vivo → casa.
        if self
            .active_account(group, config)
            .is_some_and(|current| current.id == account.id)
        {
            self.mirror_group_to_home(account, &group_location)?;
            return Ok(account.clone());
        }

        // Recusa duplicação: a conta não pode estar viva em OUTRO perfil.
        if let Some(busy) = self.where_else_live(account, group, config) {
            return Err(busy);
        }

        let home_location = adapter.credential_location(&account.home);
        let secret = self
            .credentials
            .read(&home_location)
            .ok_or(RotationError::NoCredential {
                account_id: account.id,
            })?;

        // Espelha a conta que sai: o token fresco do grupo volta para a casa
        // dela, para nunca ativarmos uma cópia vencida mais tarde. Se o espelho
        // falha, a troca NÃO acontece: sobrescrever o grupo agora perderia o
        // único token vivo da conta que sai.
        if let Some(leaving) = self.active_account(group, config) {
            if leaving.id != account.id {
                self.mirror_group_to_home(leaving, &group_location)?;
            }
        }

        // Credencial (mtime novo: é o que faz a sessão viva reler) e identidade
        // andam juntas. Com a credencial nova e a identidade velha, o próximo
        // espelho gravaria a credencial desta conta na casa da que saiu (um
        // `.claude.json` que a sessão regravou no meio basta — `NotPersisted`).
        // Se a identidade não grava, o grupo volta ao que era.
        let previous = self.credentials.read(&group_location);
        self.credentials
            .write(&secret, &group_location)
            .map_err(|e| RotationError::WriteFailed(e.to_string()))?;
        if let Err(e) = adapter.write_identity(&account.identity, &group.config_dir) {
            match previous {
                Some(previous) => {
                    let _ = self.credentials.write(&previous, &group_location);
                }
                None => self.credentials.delete(&group_location),
            }
            return Err(RotationError::WriteFailed(e.to_string()));
        }
        Ok(account.clone())
    }

    /// Onde mais esta conta está viva agora, fora do perfil deste grupo — ou
    /// `None` se em lugar nenhum.
    ///
    /// Dois lugares contam: o perfil de outro grupo com a identidade dela, e o
    /// `~\.claude` quando ele não é o perfil deste grupo nem de grupo nenhum —
    /// depois de "deixar o ~\.claude livre", a conta que estava lá continua
    /// servindo o `claude` puro. Até 10/2026 só o primeiro contava, e a rotação
    /// a ativava também no perfil dedicado novo: duas cópias de um refresh
    /// token que gira.
    pub fn where_else_live(
        &self,
        account: &Account,
        group: &AccountGroup,
        config: &RouterConfig,
    ) -> Option<RotationError> {
        for other in config.groups.iter().filter(|g| g.id != group.id) {
            if self
                .active_account(other, config)
                .is_some_and(|busy| busy.id == account.id)
            {
                return Some(RotationError::AccountBusyElsewhere {
                    account_id: account.id,
                    group_id: other.id,
                });
            }
        }
        let default = self.default_profile.as_ref()?;
        let default_is_a_groups = config
            .groups
            .iter()
            .any(|g| g.config_dir.raw == default.raw);
        if group.config_dir.raw != default.raw
            && !default_is_a_groups
            && self
                .adapter(account.provider)
                .identity(default)
                .is_some_and(|present| present.email == account.identity.email)
        {
            return Some(RotationError::AccountLiveInDefaultProfile {
                account_id: account.id,
            });
        }
        None
    }

    /// A conta pode ser ativada neste grupo agora: tem credencial na casa e não
    /// está viva em outro perfil. A rotação só escolhe quem pode — escolher quem
    /// não pode fazia o grupo insistir na mesma recusa a cada volta.
    pub fn can_serve(
        &self,
        account: &Account,
        group: &AccountGroup,
        config: &RouterConfig,
    ) -> bool {
        if self
            .active_account(group, config)
            .is_some_and(|active| active.id == account.id)
        {
            return true;
        }
        self.where_else_live(account, group, config).is_none()
            && self
                .credentials
                .exists(&self.home_credential_location(account))
    }

    /// Esvazia um perfil DEDICADO: apaga a credencial e tira a identidade do
    /// `.claude.json`. Chamado quando um grupo troca de perfil. Nunca toca o
    /// `~\.claude` — o login dele é do `claude` puro.
    pub fn reset_profile(&self, dir: &ConfigDir, provider: Provider) {
        if dir.is_default
            || self
                .default_profile
                .as_ref()
                .is_some_and(|d| d.raw == dir.raw)
        {
            return;
        }
        let adapter = self.adapter(provider);
        self.credentials.delete(&adapter.credential_location(dir));
        let _ = adapter.clear_identity(dir);
    }

    /// Copia a credencial viva do grupo de volta para a casa da conta.
    ///
    /// Chamado antes de cada troca, e num ciclo periódico enquanto a conta está
    /// ativa, para que a casa nunca fique muito atrás do token que gira. Uma
    /// cópia do grupo incompleta (pega no meio de uma escrita) nem é lida pelo
    /// store — e então não há espelho.
    pub fn mirror_group_to_home(
        &self,
        account: &Account,
        group_location: &Path,
    ) -> Result<(), RotationError> {
        let home_location = self
            .adapter(account.provider)
            .credential_location(&account.home);
        let Some(fresh) = self.credentials.read(group_location) else {
            return Ok(());
        };
        // Só escreve se mudou: no Windows cada escrita é um mtime novo.
        if self.credentials.read(&home_location).as_ref() != Some(&fresh) {
            self.credentials
                .write(&fresh, &home_location)
                .map_err(|e| RotationError::WriteFailed(e.to_string()))?;
        }
        Ok(())
    }

    /// Depois de um RELOGIN de conta ativa: a casa tem a credencial nova e o
    /// grupo guarda a morta que motivou o relogin — o único caso em que
    /// casa→grupo com a conta já ativa é o movimento certo. (O `activate`
    /// normal faz o inverso de propósito.)
    pub fn push_home_to_group(&self, account: &Account, group: &AccountGroup) {
        let adapter = self.adapter(group.provider);
        let Some(secret) = self
            .credentials
            .read(&adapter.credential_location(&account.home))
        else {
            return;
        };
        let _ = self
            .credentials
            .write(&secret, &adapter.credential_location(&group.config_dir));
        let _ = adapter.write_identity(&account.identity, &group.config_dir);
    }

    /// O ciclo periódico do espelhamento: o token vivo do grupo volta para a
    /// casa da conta ativa. Sem isto a casa apodrece — o refresh token dela é
    /// invalidado na primeira renovação que o Claude Code faz no grupo.
    pub fn mirror_active(&self, group: &AccountGroup, config: &RouterConfig) {
        let Some(active) = self.active_account(group, config) else {
            return;
        };
        let location = self
            .adapter(group.provider)
            .credential_location(&group.config_dir);
        // Periódico: uma falha aqui tenta de novo na próxima volta.
        let _ = self.mirror_group_to_home(active, &location);
    }

    /// Onde mora a credencial-mãe de uma conta — pelo **provedor da conta**,
    /// como o motor usa em todo lugar (o store precisa disto para apagá-la).
    pub fn home_credential_location(&self, account: &Account) -> PathBuf {
        self.adapter(account.provider)
            .credential_location(&account.home)
    }

    /// Onde mora a credencial de um perfil que ainda não é de conta nenhuma (a
    /// casa de um login pendente) — pelo provedor da v1, o único.
    pub fn credential_location_of(&self, dir: &ConfigDir) -> PathBuf {
        self.adapter(Provider::Anthropic).credential_location(dir)
    }

    /// Por qual perfil sondar esta conta — e **nunca** pela casa de uma conta
    /// ativa.
    ///
    /// Enquanto a conta serve um grupo, quem tem a credencial viva é o GRUPO. A
    /// casa fica com uma cópia atrás; sondá-la faria o `claude` tentar renovar
    /// com o refresh token velho — se ele ainda valer, a renovação gira a cadeia
    /// e invalida a cópia do grupo, e a sessão viva cai em "Login expired" no
    /// meio do trabalho. Conta ociosa é o caso seguro: a casa é a única cópia.
    pub fn probe_config_dir(&self, account: &Account, config: &RouterConfig) -> ConfigDir {
        config
            .groups
            .iter()
            .find(|g| {
                self.active_account(g, config)
                    .is_some_and(|a| a.id == account.id)
            })
            .map(|g| g.config_dir.clone())
            .unwrap_or_else(|| account.home.clone())
    }

    /// A próxima conta que o grupo deve usar, dado o uso de cada uma: a primeira
    /// na ordem de preferência com uso conhecido **abaixo do limiar** — ou sem
    /// medição nenhuma, presumida fresca. `None` se nenhuma qualifica, e quem
    /// chama mantém a atual (pior é "não trocou", nunca "travou").
    pub fn next_account<'c>(
        &self,
        group: &AccountGroup,
        config: &'c RouterConfig,
        usage: &HashMap<Id, f64>,
    ) -> Option<&'c Account> {
        let threshold = group.threshold_percent / 100.0;
        config.accounts_in(group).into_iter().find(|account| {
            // Sem amostra = presumida fresca. No sensor passivo, conta nunca
            // usada não tem medição — exigir amostra criava um deadlock (só mede
            // quem serve; só serve quem é escolhida).
            usage.get(&account.id).is_none_or(|used| *used < threshold)
                && self.can_serve(account, group, config)
        })
    }

    /// Decide se um grupo deve trocar agora, e para qual conta.
    ///
    /// Com histerese: só troca se a conta ativa passou do limiar **e** existe um
    /// destino melhor. Com a ativa abaixo do limiar, fica onde está mesmo que
    /// outra esteja mais folgada — trocar por pouco só reconstrói cache à toa.
    pub fn rotation_target<'c>(
        &self,
        group: &AccountGroup,
        config: &'c RouterConfig,
        usage: &HashMap<Id, f64>,
    ) -> Option<&'c Account> {
        if !group.auto_rotate {
            return None;
        }
        let threshold = group.threshold_percent / 100.0;
        let active = self.active_account(group, config);

        // Ativa sem amostra conta como fresca, igual a qualquer candidata —
        // senão um "Usar" manual numa conta ainda não medida era desfeito na
        // volta seguinte.
        if let Some(active) = active {
            if usage.get(&active.id).is_none_or(|used| *used < threshold) {
                return None; // ativa ainda tem folga; não mexe
            }
        }
        // Perfil com um login que não é conta do router (o `~\.claude` de quem
        // já usava o `claude` antes do app): a rotação não o troca sozinha.
        // Ativar ali é escolha explícita ("Usar", ou `claude <grupo>`).
        if active.is_none() {
            if let Some(present) = self.adapter(group.provider).identity(&group.config_dir) {
                if !config
                    .accounts
                    .iter()
                    .any(|a| a.identity.email == present.email)
                {
                    return None;
                }
            }
        }
        // Ninguém qualifica: mantém a atual.
        let target = self.next_account(group, config, usage)?;
        if active.is_some_and(|a| a.id == target.id) {
            None
        } else {
            Some(target)
        }
    }
}
