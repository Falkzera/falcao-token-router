//! O que é específico do Claude Code. Nesta fatia (o sensor) só a leitura de
//! identidade: o `oauthAccount.emailAddress` do `.claude.json` do perfil. A
//! escrita de identidade e o resto do adapter entram na Fase 4.

use serde_json::Value;

use super::account_model::AccountIdentity;
use super::config_dir::ConfigDir;

pub struct AnthropicAdapter;

impl AnthropicAdapter {
    /// A conta com que um perfil está logado, lida do disco (nunca da rede).
    /// `None` quando o perfil não tem identidade legível.
    pub fn identity(dir: &ConfigDir) -> Option<AccountIdentity> {
        let data = std::fs::read(dir.global_config_path()).ok()?;
        Self::identity_from_bytes(&data)
    }

    /// A parte pura, sem I/O — testável com bytes.
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
}
