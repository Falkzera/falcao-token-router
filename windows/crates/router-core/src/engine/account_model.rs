//! A identidade de uma conta (lida do `.claude.json`, nunca da rede) e a conta
//! que o usuário adicionou a um grupo. A credencial **nunca** vive aqui.

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use super::config_dir::ConfigDir;
use super::provider::Provider;
use crate::ids::Id;

/// A identidade de uma conta. Guarda o `oauthAccount` cru inteiro (opaco) porque
/// é ele que o Claude Code lê; os campos abaixo são extraídos por cima para a UI.
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountIdentity {
    pub email: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub organization_name: Option<String>,
    /// `default_claude_max_5x`, `default_claude_max_20x`, etc. — decide o plano
    /// sem chamada nenhuma.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rate_limit_tier: Option<String>,
    /// O `oauthAccount` cru, para regravar idêntico no perfil de destino.
    pub raw: Map<String, Value>,
}

impl AccountIdentity {
    /// Nome curto: o que cabe numa linha de lista (parte local do e-mail).
    pub fn short_name(&self) -> &str {
        match self.email.split_once('@') {
            Some((local, _)) => local,
            None => &self.email,
        }
    }
}

/// Uma conta que o usuário adicionou a um grupo. Só o suficiente para achar a
/// credencial (na casa) e mostrar a conta: identidade e o caminho da casa.
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct Account {
    pub id: Id,
    pub provider: Provider,
    /// Identidade capturada quando a conta foi adicionada — rótulo estável mesmo
    /// antes da primeira ativação.
    pub identity: AccountIdentity,
    /// O perfil onde esta conta fez `auth login` — onde a credencial-mãe mora.
    pub home: ConfigDir,
    /// Apelido opcional que o usuário deu.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nickname: Option<String>,
}

impl Account {
    /// O que mostrar: o apelido se houver, senão o nome curto do e-mail.
    pub fn label(&self) -> &str {
        self.nickname
            .as_deref()
            .unwrap_or_else(|| self.identity.short_name())
    }
}
