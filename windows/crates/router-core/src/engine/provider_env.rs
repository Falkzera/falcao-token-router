//! O ambiente para falar com o provedor **direto**, sem intermediário
//! (≙ `ProviderEnv.swift`, com a lista estendida do Windows).
//!
//! A máquina pode ter um proxy exportado globalmente, uma chave de API ou um
//! endpoint alternativo — e qualquer um deles faz a sessão do grupo ser atendida
//! por outra coisa que não a conta OAuth que o grupo ativou. Pior: o sensor lê o
//! `rate_limits` dessa sessão e carimba o consumo na conta do perfil, e o rodízio
//! passa a decidir com número falso. O login, a sonda e o lançamento removem
//! exatamente estas variáveis.
//!
//! No Windows o nome de variável **não tem caixa** (`Https_Proxy` é o mesmo que
//! `HTTPS_PROXY`), então a remoção também não tem.

use std::ffi::{OsStr, OsString};

pub struct ProviderEnv;

impl ProviderEnv {
    /// Variáveis que redirecionam requisições por um proxy (a lista do macOS).
    pub const PROXY_KEYS: &'static [&'static str] = &[
        "HTTP_PROXY",
        "HTTPS_PROXY",
        "ALL_PROXY",
        "http_proxy",
        "https_proxy",
        "all_proxy",
        "NODE_EXTRA_CA_CERTS",
    ];

    /// Variáveis que trocam **quem atende** a sessão — credencial ou endpoint
    /// (a lista do macOS).
    pub const CREDENTIAL_KEYS: &'static [&'static str] = &[
        "ANTHROPIC_API_KEY",
        "ANTHROPIC_AUTH_TOKEN",
        "ANTHROPIC_CUSTOM_HEADERS",
        "ANTHROPIC_BASE_URL",
        "ANTHROPIC_BEDROCK_BASE_URL",
        "ANTHROPIC_VERTEX_BASE_URL",
        "CLAUDE_CODE_USE_BEDROCK",
        "CLAUDE_CODE_USE_VERTEX",
    ];

    /// O que o JS do binário 2.1.280 também aceita como credencial ou endpoint,
    /// e o macOS ainda não remove (22/09/2026). `CLAUDE_SECURESTORAGE_CONFIG_DIR`
    /// é consultada ANTES do `CLAUDE_CONFIG_DIR` para achar a credencial — com
    /// ela setada, a sessão do grupo leria a credencial de outra pasta.
    pub const WINDOWS_KEYS: &'static [&'static str] = &[
        "CLAUDE_CODE_OAUTH_TOKEN",
        "CLAUDE_CODE_OAUTH_TOKEN_FILE_DESCRIPTOR",
        "CLAUDE_CODE_OAUTH_REFRESH_TOKEN",
        "CLAUDE_CODE_API_KEY_FILE_DESCRIPTOR",
        "CLAUDE_CODE_SESSION_ACCESS_TOKEN",
        "CLAUDE_CODE_CUSTOM_OAUTH_URL",
        "CLAUDE_SECURESTORAGE_CONFIG_DIR",
        "ANTHROPIC_PROFILE",
        "ANTHROPIC_CONFIG_DIR",
        "ANTHROPIC_IDENTITY_TOKEN",
        "ANTHROPIC_IDENTITY_TOKEN_FILE",
        "ANTHROPIC_FEDERATION_RULE_ID",
        "ANTHROPIC_ORGANIZATION_ID",
        "CLAUDE_CODE_USE_FOUNDRY",
        "AWS_BEARER_TOKEN_BEDROCK",
    ];

    /// Famílias inteiras de configuração de endpoint alternativo.
    pub const WINDOWS_PREFIXES: &'static [&'static str] = &["ANTHROPIC_FOUNDRY_", "ANTHROPIC_AWS_"];

    /// Esta variável desvia a sessão da conta do grupo?
    pub fn redirects(key: &OsStr) -> bool {
        let key = key.to_string_lossy().to_ascii_uppercase();
        Self::PROXY_KEYS
            .iter()
            .chain(Self::CREDENTIAL_KEYS)
            .chain(Self::WINDOWS_KEYS)
            .any(|k| k.eq_ignore_ascii_case(&key))
            || Self::WINDOWS_PREFIXES.iter().any(|p| key.starts_with(p))
    }

    /// O ambiente sem nada que desvie a sessão da conta do grupo.
    pub fn direct(
        env: impl IntoIterator<Item = (OsString, OsString)>,
    ) -> Vec<(OsString, OsString)> {
        env.into_iter()
            .filter(|(key, _)| !Self::redirects(key))
            .collect()
    }

    /// Sem as variáveis de uma sessão do Claude Code em volta (`CLAUDE_CODE*`,
    /// `CLAUDECODE`): a sonda e o login lançados de DENTRO de uma sessão as
    /// herdariam e se comportariam como sessão aninhada. (O `launch` não tira:
    /// o macOS não tira, e quem aninha é o usuário.)
    pub fn without_nested_session(
        env: impl IntoIterator<Item = (OsString, OsString)>,
    ) -> Vec<(OsString, OsString)> {
        env.into_iter()
            .filter(|(key, _)| {
                let key = key.to_string_lossy().to_ascii_uppercase();
                !(key.starts_with("CLAUDE_CODE") || key == "CLAUDECODE")
            })
            .collect()
    }

    /// Troca (ou remove, com `None`) uma variável, sem caixa.
    pub fn with_var(
        env: impl IntoIterator<Item = (OsString, OsString)>,
        key: &str,
        value: Option<&OsStr>,
    ) -> Vec<(OsString, OsString)> {
        let mut out: Vec<(OsString, OsString)> = env
            .into_iter()
            .filter(|(k, _)| !k.to_string_lossy().eq_ignore_ascii_case(key))
            .collect();
        if let Some(value) = value {
            out.push((OsString::from(key), value.to_os_string()));
        }
        out
    }
}
