//! Provedor de IA cujo CLI o app sabe rotacionar, e o que o motor precisa saber
//! dele. A v1 traz só o `.anthropic` (Claude Code). O tipo existe agora porque
//! decide o formato do arquivo de configuração — cada conta e grupo carrega o
//! provedor a que pertence.

use std::io;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use super::account_model::AccountIdentity;
use super::config_dir::ConfigDir;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Provider {
    #[default]
    Anthropic,
}

impl Provider {
    /// Nome que aparece na UI. Não traduzível: é marca.
    pub fn display_name(&self) -> &'static str {
        match self {
            Provider::Anthropic => "Claude",
        }
    }
}

/// Por que a identidade não pôde ser gravada no perfil.
#[derive(Debug, thiserror::Error)]
pub enum IdentityError {
    /// O `.claude.json` existe e não é um objeto JSON legível. É **recusado** —
    /// nunca substituído. (No macOS ele virava `{}` e o arquivo inteiro do
    /// usuário se perdia: projetos, confiança de pasta, tudo.)
    #[error("{} ilegível ({reason}) — recusado, nada foi escrito", path.display())]
    Unreadable { path: PathBuf, reason: String },
    /// Falha de E/S ao gravar.
    #[error("falha ao gravar {}: {source}", path.display())]
    Io {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    /// Gravou, mas a releitura não mostra a identidade escrita.
    #[error("{} gravado, mas a releitura não mostra a identidade", path.display())]
    NotPersisted { path: PathBuf },
}

/// O que o motor precisa saber de um provedor para rotacionar suas contas, sem
/// conhecer nada específico dele. O `RotationEngine` fala só com isto; trocar de
/// provedor é trocar a implementação, não o motor.
pub trait ProviderAdapter: Send + Sync {
    fn provider(&self) -> Provider;

    /// Onde mora a credencial de um perfil (≙ `keychainService(forConfigDir:)`).
    ///
    /// No Windows é um ARQUIVO dentro do perfil, tanto no padrão quanto num
    /// dedicado — sem o hash do caminho que o item de chaveiro do macOS tem.
    fn credential_location(&self, dir: &ConfigDir) -> PathBuf;

    /// A conta com que um perfil está logado, lida do disco (nunca da rede).
    /// `None` quando o perfil não tem identidade legível.
    fn identity(&self, dir: &ConfigDir) -> Option<AccountIdentity>;

    /// Grava, no perfil de destino, a identidade da conta que está sendo
    /// ativada — para a tela do provedor mostrar a conta certa e o uso não vir
    /// carimbado da conta anterior.
    fn write_identity(
        &self,
        identity: &AccountIdentity,
        dir: &ConfigDir,
    ) -> Result<(), IdentityError>;

    /// O comando e os argumentos para iniciar uma sessão num grupo. O ambiente
    /// (incluindo o `CLAUDE_CONFIG_DIR`, quando o grupo não é o padrão) é montado
    /// por quem chama.
    fn launch_command(&self) -> (String, Vec<String>);
}
