//! `ProviderEnv`: o ambiente sem nada que desvie a sessão da conta do grupo.
//!
//! Portados de `EngineTests.swift` (suíte "ProviderEnv", 2), mais o que o
//! Windows exige: nome de variável **sem caixa** (o ambiente do Windows não
//! distingue) e a lista estendida — as variáveis de credencial/endpoint achadas
//! no JS do binário 2.1.280, inclusive `CLAUDE_SECURESTORAGE_CONFIG_DIR`, que
//! desviaria a leitura da credencial para outra pasta.

use std::ffi::OsString;

use router_core::engine::provider_env::ProviderEnv;

fn env(pairs: &[(&str, &str)]) -> Vec<(OsString, OsString)> {
    pairs
        .iter()
        .map(|(k, v)| (OsString::from(k), OsString::from(v)))
        .collect()
}

/// Procura como o Windows procura: sem caixa.
fn get(env: &[(OsString, OsString)], key: &str) -> Option<String> {
    env.iter()
        .find(|(k, _)| k.to_string_lossy().eq_ignore_ascii_case(key))
        .map(|(_, v)| v.to_string_lossy().into_owned())
}

#[test]
fn strips_the_proxy_and_keeps_the_rest() {
    let direct = ProviderEnv::direct(env(&[
        ("HTTPS_PROXY", "http://127.0.0.1:3456"),
        ("https_proxy", "http://127.0.0.1:3456"),
        ("NODE_EXTRA_CA_CERTS", "C:/x/proxy-ca.pem"),
        ("PATH", "C:/Windows/system32"),
        ("USERPROFILE", "C:/Users/exemplo"),
    ]));
    assert_eq!(get(&direct, "HTTPS_PROXY"), None);
    assert_eq!(get(&direct, "NODE_EXTRA_CA_CERTS"), None);
    assert_eq!(get(&direct, "PATH").as_deref(), Some("C:/Windows/system32"));
    assert_eq!(
        get(&direct, "USERPROFILE").as_deref(),
        Some("C:/Users/exemplo")
    );
}

/// Chave de API vence a conta OAuth: a sessão sobe servida por ela, o rodízio
/// ativa uma conta que ninguém usa, e o sensor ainda carimba o consumo da chave
/// no e-mail do perfil. O que não decide QUEM atende (a escolha de modelo) fica.
#[test]
fn strips_credential_and_endpoint_overrides() {
    let direct = ProviderEnv::direct(env(&[
        ("ANTHROPIC_API_KEY", "sk-falsa"),
        ("ANTHROPIC_AUTH_TOKEN", "tok"),
        ("ANTHROPIC_BASE_URL", "https://gateway.exemplo/v1"),
        ("ANTHROPIC_CUSTOM_HEADERS", "Authorization: Bearer x"),
        ("ANTHROPIC_BEDROCK_BASE_URL", "x"),
        ("ANTHROPIC_VERTEX_BASE_URL", "x"),
        ("CLAUDE_CODE_USE_BEDROCK", "1"),
        ("CLAUDE_CODE_USE_VERTEX", "1"),
        ("ANTHROPIC_MODEL", "claude-opus-5"),
        ("PATH", "C:/Windows"),
    ]));
    for key in ProviderEnv::CREDENTIAL_KEYS {
        assert_eq!(get(&direct, key), None, "{key} sobreviveu");
    }
    assert_eq!(
        get(&direct, "ANTHROPIC_MODEL").as_deref(),
        Some("claude-opus-5")
    );
    assert_eq!(get(&direct, "PATH").as_deref(), Some("C:/Windows"));
}

/// No Windows `Https_Proxy` e `HTTPS_PROXY` são a MESMA variável: a remoção não
/// pode depender da caixa em que alguém a exportou.
#[test]
fn removal_ignores_case_like_windows_does() {
    let direct = ProviderEnv::direct(env(&[
        ("Https_Proxy", "http://127.0.0.1:3456"),
        ("anthropic_api_key", "sk-falsa"),
        ("Claude_SecureStorage_Config_Dir", "C:/Users/exemplo/outra"),
        ("Path", "C:/Windows"),
    ]));
    assert_eq!(direct.len(), 1, "{direct:?}");
    assert_eq!(get(&direct, "PATH").as_deref(), Some("C:/Windows"));
}

/// A lista que o macOS ainda não tem: token OAuth por variável, arquivo ou
/// descritor, perfil/organização alternativos, identidade federada, Foundry e
/// Bedrock por token — e `CLAUDE_SECURESTORAGE_CONFIG_DIR`, que o binário
/// consulta ANTES do `CLAUDE_CONFIG_DIR` para achar a credencial.
#[test]
fn strips_the_windows_extended_list() {
    let mut pairs: Vec<(&str, &str)> = ProviderEnv::WINDOWS_KEYS
        .iter()
        .map(|k| (*k, "x"))
        .collect();
    pairs.push(("ANTHROPIC_FOUNDRY_API_KEY", "x"));
    pairs.push(("ANTHROPIC_FOUNDRY_RESOURCE", "x"));
    pairs.push(("ANTHROPIC_AWS_WORKSPACE_ID", "x"));
    pairs.push(("CLAUDE_CONFIG_DIR", "C:/Users/exemplo/grupo"));

    let direct = ProviderEnv::direct(env(&pairs));

    assert!(ProviderEnv::WINDOWS_KEYS.contains(&"CLAUDE_SECURESTORAGE_CONFIG_DIR"));
    assert!(ProviderEnv::WINDOWS_KEYS.contains(&"CLAUDE_CODE_OAUTH_TOKEN"));
    // Só o perfil sobra: ele é do router, não um desvio.
    assert_eq!(direct.len(), 1, "{direct:?}");
    assert_eq!(
        get(&direct, "CLAUDE_CONFIG_DIR").as_deref(),
        Some("C:/Users/exemplo/grupo")
    );
}

/// A sonda e o login rodados de DENTRO de uma sessão do Claude Code herdariam as
/// variáveis dela e se comportariam como sessão aninhada.
#[test]
fn the_nested_session_scrub_drops_claude_code_variables() {
    let scrubbed = ProviderEnv::without_nested_session(env(&[
        ("CLAUDE_CODE_ENTRYPOINT", "cli"),
        ("CLAUDECODE", "1"),
        ("claudecode", "1"),
        ("Claude_Code_SSE_Port", "1234"),
        ("CLAUDE_CONFIG_DIR", "C:/Users/exemplo/grupo"),
        ("PATH", "C:/Windows"),
    ]));
    assert_eq!(scrubbed.len(), 2, "{scrubbed:?}");
    assert!(get(&scrubbed, "CLAUDE_CONFIG_DIR").is_some());
    assert!(get(&scrubbed, "PATH").is_some());
}
