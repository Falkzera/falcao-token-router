//! O que é específico do Claude Code, atrás do [`ProviderAdapter`].
//!
//! Concentra num lugar só o que se descobriu observando o Claude Code — o
//! `.claude.json` com a identidade, onde ele mora, o que precisa estar nele para
//! um perfil dedicado não abrir o assistente de login. Nada disto está
//! documentado; quando uma versão do Claude Code mudar algo, muda-se aqui.

use std::path::PathBuf;

use serde_json::{Map, Value};

use super::account_model::AccountIdentity;
use super::config_dir::ConfigDir;
use super::provider::{IdentityError, Provider, ProviderAdapter};
use crate::platform::atomic_write::read_retrying;
use crate::platform::json_file::{edit_object, JsonFileError};

pub struct AnthropicAdapter;

impl AnthropicAdapter {
    /// A parte pura da leitura, sem I/O — testável com bytes.
    pub fn identity_from_bytes(data: &[u8]) -> Option<AccountIdentity> {
        let root: Value = serde_json::from_slice(data).ok()?;
        let oauth = root.get("oauthAccount")?.as_object()?;
        let email = oauth.get("emailAddress")?.as_str()?.to_string();
        let organization_name = oauth
            .get("organizationName")
            .and_then(Value::as_str)
            .map(String::from);
        // O tier fica em `organizationRateLimitTier`; o de usuário é o fallback
        // para conta pessoal sem organização.
        let rate_limit_tier = oauth
            .get("organizationRateLimitTier")
            .and_then(Value::as_str)
            .or_else(|| oauth.get("userRateLimitTier").and_then(Value::as_str))
            .map(String::from);

        Some(AccountIdentity {
            email,
            organization_name,
            rate_limit_tier,
            raw: oauth.clone(),
        })
    }

    /// O "splice": põe a identidade no objeto raiz do `.claude.json` mexendo só
    /// no que é atrelado à conta. Com o `preserve_order` do `serde_json`, o
    /// `insert` numa chave existente troca o valor **no lugar**, e a remoção é
    /// por `shift` (o `remove` padrão trocaria a última chave de posição).
    pub fn splice_identity(root: &mut Map<String, Value>, identity: &AccountIdentity) {
        root.insert(
            "oauthAccount".to_string(),
            Value::Object(identity.raw.clone()),
        );
        // Sem esta chave o Claude Code trata o perfil como primeira execução e
        // abre o assistente de login — sem nem consultar a credencial. Identidade
        // escrita = conta já autenticada, então o onboarding está feito.
        root.insert("hasCompletedOnboarding".to_string(), Value::Bool(true));
        // O cache de uso pertence à conta anterior; deixá-lo carimbaria a nova
        // com números que não são dela até o Claude Code sobrescrever.
        root.shift_remove("cachedUsageUtilization");
    }
}

impl ProviderAdapter for AnthropicAdapter {
    fn provider(&self) -> Provider {
        Provider::Anthropic
    }

    /// `<perfil>\.credentials.json` — no padrão, `%USERPROFILE%\.claude\.credentials.json`.
    /// Conferido no Windows 11 com o Claude Code 2.1.280 (22/09/2026): arquivo,
    /// nada no Credential Manager. (`CLAUDE_SECURESTORAGE_CONFIG_DIR` desviaria a
    /// leitura para outra pasta; o router a remove de todo processo que lança.)
    fn credential_location(&self, dir: &ConfigDir) -> PathBuf {
        dir.path().join(".credentials.json")
    }

    fn identity(&self, dir: &ConfigDir) -> Option<AccountIdentity> {
        let data = read_retrying(&dir.global_config_path()).ok()?;
        Self::identity_from_bytes(&data)
    }

    /// Grava a identidade de forma **cirúrgica**: lê o `.claude.json` que houver,
    /// troca só `oauthAccount`/`hasCompletedOnboarding`/`cachedUsageUtilization`
    /// e regrava preservando todas as outras chaves, na ordem em que estavam
    /// (inclusive as que só diferem em caixa, que o Windows tem). Um arquivo
    /// existente e ilegível é **recusado**, nunca substituído; ausente ou vazio
    /// vira um objeto novo (grupo dedicado que nunca foi ativado — a pasta do
    /// perfil é criada). Depois de gravar, relê para conferir.
    fn write_identity(
        &self,
        identity: &AccountIdentity,
        dir: &ConfigDir,
    ) -> Result<(), IdentityError> {
        let path = dir.global_config_path();
        edit_object(&path, |root| Self::splice_identity(root, identity)).map_err(|e| match e {
            JsonFileError::Unreadable { path, reason } => {
                IdentityError::Unreadable { path, reason }
            }
            JsonFileError::Io { path, source } => IdentityError::Io { path, source },
        })?;

        match self.identity(dir) {
            Some(back) if back.email == identity.email => Ok(()),
            _ => Err(IdentityError::NotPersisted { path }),
        }
    }

    fn launch_command(&self) -> (String, Vec<String>) {
        ("claude".to_string(), Vec::new())
    }
}
