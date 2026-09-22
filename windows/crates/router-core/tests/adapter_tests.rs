//! `AnthropicAdapter`: lê a identidade (`oauthAccount`) do `.claude.json` e a
//! grava de volta de forma **cirúrgica**.
//!
//! Portados de `EngineTests.swift` (suíte "AnthropicAdapter (real, arquivo)") e
//! acrescidos das regressões do Windows: ordem das chaves preservada, chaves que
//! só diferem em caixa preservadas, arquivo ilegível RECUSADO (nunca substituído).

use std::path::Path;

use serde_json::{json, Value};

use router_core::engine::anthropic_adapter::AnthropicAdapter;
use router_core::engine::provider::{IdentityError, ProviderAdapter};
use router_core::{AccountIdentity, ConfigDir};

fn identity(email: &str) -> AccountIdentity {
    let mut raw = serde_json::Map::new();
    raw.insert("emailAddress".into(), json!(email));
    raw.insert("organizationName".into(), json!("Acme"));
    AccountIdentity {
        email: email.to_string(),
        organization_name: Some("Acme".to_string()),
        rate_limit_tier: None,
        raw,
    }
}

fn dedicated(path: &Path) -> ConfigDir {
    ConfigDir::dedicated(path.to_string_lossy().into_owned())
}

fn read_root(dir: &ConfigDir) -> serde_json::Map<String, Value> {
    let data = std::fs::read(dir.global_config_path()).unwrap();
    match serde_json::from_slice(&data).unwrap() {
        Value::Object(map) => map,
        other => panic!("raiz não é objeto: {other}"),
    }
}

// --- Leitura (identity_from_bytes) ---

#[test]
fn reads_email_org_and_org_tier() {
    let json = br#"{
        "oauthAccount": {
            "emailAddress": "conta1@exemplo.com",
            "organizationName": "Acme",
            "organizationRateLimitTier": "default_claude_max_20x",
            "userRateLimitTier": "default_claude_pro"
        },
        "hasCompletedOnboarding": true
    }"#;
    let id = AnthropicAdapter::identity_from_bytes(json).unwrap();
    assert_eq!(id.email, "conta1@exemplo.com");
    assert_eq!(id.organization_name.as_deref(), Some("Acme"));
    // O tier de ORG vence o de usuário.
    assert_eq!(
        id.rate_limit_tier.as_deref(),
        Some("default_claude_max_20x")
    );
    // O cru é preservado inteiro.
    assert!(id.raw.contains_key("organizationRateLimitTier"));
}

#[test]
fn falls_back_to_user_tier_without_org() {
    let json = br#"{"oauthAccount":{"emailAddress":"p@exemplo.com","userRateLimitTier":"default_claude_pro"}}"#;
    let id = AnthropicAdapter::identity_from_bytes(json).unwrap();
    assert_eq!(id.rate_limit_tier.as_deref(), Some("default_claude_pro"));
    assert_eq!(id.organization_name, None);
}

#[test]
fn no_oauth_account_is_none() {
    assert!(AnthropicAdapter::identity_from_bytes(br#"{"hasCompletedOnboarding":true}"#).is_none());
    assert!(AnthropicAdapter::identity_from_bytes(br#"{"oauthAccount":{}}"#).is_none());
    assert!(AnthropicAdapter::identity_from_bytes(b"not json").is_none());
}

// --- Escrita (write_identity) — portados do macOS ---

/// Regressão do macOS: escrever a identidade num perfil dedicado que ainda não
/// existe no disco tem de criar o diretório, não falhar. (Pegou um bug real no
/// `router launch` do primeiro grupo dedicado.)
#[test]
fn write_identity_creates_the_profile_directory() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = dedicated(&tmp.path().join("grupo-novo"));

    AnthropicAdapter
        .write_identity(&identity("z@exemplo.com"), &dir)
        .unwrap();

    assert_eq!(
        AnthropicAdapter.identity(&dir).map(|i| i.email).as_deref(),
        Some("z@exemplo.com")
    );
}

/// A regravação preserva o resto do `.claude.json`, some com o cache de uso da
/// conta anterior e marca o onboarding — sem ele o Claude Code abre o assistente
/// de login num perfil dedicado, sem nem olhar a credencial.
#[test]
fn write_identity_preserves_the_file_and_clears_the_usage_cache() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = dedicated(tmp.path());
    std::fs::write(
        dir.global_config_path(),
        serde_json::to_vec(&json!({
            "oauthAccount": {"emailAddress": "antiga@exemplo.com"},
            "cachedUsageUtilization": {"fetchedAtMs": 1},
            "projects": {"C:/Users/exemplo/x": {"hasTrustDialogAccepted": true}},
        }))
        .unwrap(),
    )
    .unwrap();

    AnthropicAdapter
        .write_identity(&identity("nova@exemplo.com"), &dir)
        .unwrap();

    let root = read_root(&dir);
    assert_eq!(root["oauthAccount"]["emailAddress"], "nova@exemplo.com");
    assert!(
        !root.contains_key("cachedUsageUtilization"),
        "cache não limpo"
    );
    assert!(root.contains_key("projects"), "projects sumiu");
    assert_eq!(root["hasCompletedOnboarding"], true);
}

// --- Escrita — regressões do Windows ---

/// Cirúrgico: só `oauthAccount`, `hasCompletedOnboarding` e o cache mudam; as
/// outras chaves ficam onde estavam. O macOS regrava com as chaves ordenadas; o
/// porte não (o `serde_json` com `preserve_order`, e a remoção por `shift`).
#[test]
fn write_identity_keeps_every_other_key_in_place() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = dedicated(tmp.path());
    std::fs::write(
        dir.global_config_path(),
        br#"{"zeta":1,"oauthAccount":{"emailAddress":"antiga@exemplo.com"},"alfa":2,"cachedUsageUtilization":{"x":1},"meio":3,"hasCompletedOnboarding":false}"#,
    )
    .unwrap();

    AnthropicAdapter
        .write_identity(&identity("nova@exemplo.com"), &dir)
        .unwrap();

    let keys: Vec<String> = read_root(&dir).keys().cloned().collect();
    assert_eq!(
        keys,
        [
            "zeta",
            "oauthAccount",
            "alfa",
            "meio",
            "hasCompletedOnboarding"
        ]
    );
}

/// O `.claude.json` real do Windows tem chaves de projeto que só diferem em
/// caixa (`C:/…` e `c:/…`). As duas são dados do usuário e ficam.
#[test]
fn write_identity_keeps_keys_that_differ_only_in_case() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = dedicated(tmp.path());
    std::fs::write(
        dir.global_config_path(),
        br#"{"projects":{"C:/Users/exemplo/app":{"a":1},"c:/Users/exemplo/app":{"b":2}}}"#,
    )
    .unwrap();

    AnthropicAdapter
        .write_identity(&identity("nova@exemplo.com"), &dir)
        .unwrap();

    let root = read_root(&dir);
    let projects = root["projects"].as_object().unwrap();
    assert_eq!(projects.len(), 2);
    assert_eq!(projects["C:/Users/exemplo/app"]["a"], 1);
    assert_eq!(projects["c:/Users/exemplo/app"]["b"], 2);
}

/// Defeito do macOS que o porte não copia: lá, um `.claude.json` ilegível vira
/// `{}` e é regravado com só a identidade — somem os projetos, a confiança de
/// pasta, tudo. Aqui o arquivo é RECUSADO e fica intacto, byte a byte.
#[test]
fn write_identity_refuses_an_unreadable_file_and_leaves_it_untouched() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = dedicated(tmp.path());
    let broken = b"{\"projects\": {\"C:/Users/exemplo/app\": {\"a\": 1}},  <- truncado";
    std::fs::write(dir.global_config_path(), broken).unwrap();

    let err = AnthropicAdapter
        .write_identity(&identity("nova@exemplo.com"), &dir)
        .unwrap_err();

    assert!(matches!(err, IdentityError::Unreadable { .. }), "{err:?}");
    assert_eq!(std::fs::read(dir.global_config_path()).unwrap(), broken);
}

/// Raiz que não é objeto também é ilegível para este propósito.
#[test]
fn write_identity_refuses_a_root_that_is_not_an_object() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = dedicated(tmp.path());
    std::fs::write(dir.global_config_path(), b"[1,2,3]").unwrap();

    let err = AnthropicAdapter
        .write_identity(&identity("nova@exemplo.com"), &dir)
        .unwrap_err();

    assert!(matches!(err, IdentityError::Unreadable { .. }), "{err:?}");
    assert_eq!(std::fs::read(dir.global_config_path()).unwrap(), b"[1,2,3]");
}

/// Arquivo vazio (truncado por uma queda) não tem dado do usuário a perder:
/// conta como ausente, em vez de travar a ativação para sempre.
#[test]
fn write_identity_treats_an_empty_file_as_absent() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = dedicated(tmp.path());
    std::fs::write(dir.global_config_path(), b"  \r\n").unwrap();

    AnthropicAdapter
        .write_identity(&identity("nova@exemplo.com"), &dir)
        .unwrap();

    assert_eq!(
        read_root(&dir)["oauthAccount"]["emailAddress"],
        "nova@exemplo.com"
    );
}

/// O `oauthAccount` gravado é o cru inteiro da conta — campos que este app não
/// conhece seguem junto, porque é o Claude Code quem os lê.
#[test]
fn write_identity_writes_the_raw_oauth_account_whole() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = dedicated(tmp.path());
    let mut id = identity("nova@exemplo.com");
    id.raw
        .insert("campoNovoQueNaoConhecemos".into(), json!({"x": [1, 2]}));

    AnthropicAdapter.write_identity(&id, &dir).unwrap();

    assert_eq!(
        read_root(&dir)["oauthAccount"]["campoNovoQueNaoConhecemos"],
        json!({"x": [1, 2]})
    );
}

/// No perfil padrão o `.claude.json` mora AO LADO do diretório (`<home>\.claude.json`).
#[test]
fn write_identity_in_the_default_profile_writes_beside_the_directory() {
    let home = tempfile::tempdir().unwrap();
    let dir = ConfigDir::standard(&home.path().to_string_lossy());

    AnthropicAdapter
        .write_identity(&identity("padrao@exemplo.com"), &dir)
        .unwrap();

    assert!(home.path().join(".claude.json").exists());
    assert!(!home.path().join(".claude").join(".claude.json").exists());
    assert_eq!(
        AnthropicAdapter.identity(&dir).map(|i| i.email).as_deref(),
        Some("padrao@exemplo.com")
    );
}
