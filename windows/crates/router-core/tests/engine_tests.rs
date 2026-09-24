//! Porte de `UsagePercentTests`, `RouterConfigTests` e a parte de `ConfigDir`
//! que vale no Windows (assimetria do `.claude.json`, valor de ambiente).

mod common;
use common::*;

use std::path::PathBuf;

use router_core::{AccountGroup, ConfigDir, RouterConfig, UsagePercent};

// --- UsagePercent: uma formatação só (arredonda, não trunca) ---

#[test]
fn usage_percent_rounds_rather_than_truncates() {
    assert_eq!(UsagePercent::value(0.666), 67);
    assert_eq!(UsagePercent::value(0.664), 66);
    assert_eq!(UsagePercent::text(0.666), "67%");
}

#[test]
fn usage_percent_edges_do_not_slip() {
    assert_eq!(UsagePercent::text(0.0), "0%");
    assert_eq!(UsagePercent::text(1.0), "100%");
    assert_eq!(UsagePercent::text(0.005), "1%");
}

// --- ConfigDir: assimetria do .claude.json e valor de ambiente (Windows) ---

#[test]
fn default_profile_has_no_env_and_json_beside() {
    let dir = ConfigDir::standard("C:/Users/exemplo");
    assert_eq!(dir.environment_value(), None);
    // No padrão o `.claude.json` fica AO LADO do diretório.
    assert_eq!(
        dir.global_config_path(),
        PathBuf::from("C:/Users/exemplo/.claude.json")
    );
}

#[test]
fn dedicated_profile_exports_env_and_json_inside() {
    let dir = ConfigDir::dedicated("C:/Users/exemplo/.claude-trabalho");
    assert_eq!(
        dir.environment_value(),
        Some("C:/Users/exemplo/.claude-trabalho")
    );
    // Num perfil dedicado o `.claude.json` fica DENTRO.
    assert_eq!(
        dir.global_config_path(),
        PathBuf::from("C:/Users/exemplo/.claude-trabalho/.claude.json")
    );
}

// --- RouterConfig: sobrevive a um ciclo de codificação ---

#[test]
fn router_config_round_trips() {
    let mut a = account("a@k.com", "C:/Users/exemplo/.claude-a");
    a.nickname = Some("principal".to_string());
    let mut group = AccountGroup::new("trabalho", ConfigDir::standard("C:/Users/exemplo"));
    group.account_ids = vec![a.id];
    let config = RouterConfig {
        accounts: vec![a.clone()],
        groups: vec![group],
        share_history: true,
        version: 1,
    };

    let data = serde_json::to_vec(&config).unwrap();
    let back: RouterConfig = serde_json::from_slice(&data).unwrap();

    assert_eq!(back, config);
    assert_eq!(
        back.account(a.id).unwrap().nickname.as_deref(),
        Some("principal")
    );
    assert_eq!(back.default_group().unwrap().name, "trabalho");
}

#[test]
fn config_keys_are_camel_case_with_uppercase_ids() {
    let mut group = AccountGroup::new("g", ConfigDir::standard("C:/Users/exemplo"));
    group.account_ids = vec![account("a@k.com", "C:/x").id];
    let config = RouterConfig {
        groups: vec![group],
        ..Default::default()
    };
    let json = serde_json::to_string(&config).unwrap();
    // Chaves na grafia que o macOS grava.
    assert!(
        json.contains("\"accountIDs\""),
        "esperava accountIDs: {json}"
    );
    assert!(json.contains("\"configDir\""));
    assert!(json.contains("\"thresholdPercent\""));
    assert!(json.contains("\"autoRotate\""));
    assert!(json.contains("\"shareHistory\""));
    assert!(json.contains("\"isDefault\""));
    // UUID em MAIÚSCULAS.
    let id_str = config.groups[0].id.to_string();
    assert_eq!(id_str, id_str.to_uppercase());
    assert!(json.contains(&id_str));
}

#[test]
fn preserves_unknown_oauth_fields() {
    // O `oauthAccount` cru guarda campos que o app não conhece — e não finge conhecer.
    let mut raw = serde_json::Map::new();
    raw.insert(
        "emailAddress".into(),
        serde_json::Value::String("a@k.com".into()),
    );
    raw.insert(
        "organizationUuid".into(),
        serde_json::Value::String("abc-123".into()),
    );
    raw.insert(
        "campoNovoQueNaoConhecemos".into(),
        serde_json::Value::Bool(true),
    );
    let mut identity = ident("a@k.com");
    identity.raw = raw;

    let data = serde_json::to_vec(&identity).unwrap();
    let back: router_core::AccountIdentity = serde_json::from_slice(&data).unwrap();
    assert_eq!(
        back.raw.get("campoNovoQueNaoConhecemos"),
        Some(&serde_json::Value::Bool(true))
    );
}
