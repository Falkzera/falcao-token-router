//! `SessionLauncher`: escolhe a conta que vai servir a sessão, ativa-a e devolve
//! o plano de lançamento. O processo em si fica na CLI — aqui é só a decisão,
//! testável sem lançar nada.
//!
//! Portados de `LauncherTests.swift` (suíte "SessionLauncher", 6), mais os dois
//! ramos da decisão que o Swift não fixava.

mod common;
use common::*;

use std::collections::HashMap;
use std::sync::Arc;

use router_core::engine::credential_store::CredentialStore;
use router_core::engine::provider::ProviderAdapter;
use router_core::engine::session_launcher::{LaunchError, SessionLauncher};
use router_core::{Account, AccountGroup, ConfigDir, Id, RouterConfig};

struct Fixture {
    launcher: SessionLauncher,
    config: RouterConfig,
    group: AccountGroup,
    a: Account,
    b: Account,
}

fn fixture() -> Fixture {
    let store = Arc::new(FakeStore::new());
    let adapter = Arc::new(FakeAdapter::new());
    let a = account("a@exemplo.com", "C:/Users/exemplo/.claude-a");
    let b = account("b@exemplo.com", "C:/Users/exemplo/.claude-b");
    store
        .write(&cred("ca"), &adapter.credential_location(&a.home))
        .unwrap();
    store
        .write(&cred("cb"), &adapter.credential_location(&b.home))
        .unwrap();
    let mut group = AccountGroup::new("Trabalho", ConfigDir::standard("C:/Users/exemplo"));
    group.account_ids = vec![a.id, b.id];
    group.threshold_percent = 90.0;
    let config = RouterConfig {
        accounts: vec![a.clone(), b.clone()],
        groups: vec![group.clone()],
        ..Default::default()
    };
    let launcher = SessionLauncher::new(store, vec![adapter as Arc<dyn ProviderAdapter>]);
    Fixture {
        launcher,
        config,
        group,
        a,
        b,
    }
}

fn usage(pairs: &[(Id, f64)]) -> HashMap<Id, f64> {
    pairs.iter().copied().collect()
}

#[test]
fn finds_a_group_ignoring_case_and_surrounding_spaces() {
    let f = fixture();
    assert_eq!(
        SessionLauncher::group_named(" trabalho ", &f.config).map(|g| g.name.as_str()),
        Some("Trabalho")
    );
    assert!(SessionLauncher::group_named("pessoal", &f.config).is_none());
}

#[test]
fn an_empty_group_is_a_named_error() {
    let launcher = SessionLauncher::new(
        Arc::new(FakeStore::new()),
        vec![Arc::new(FakeAdapter::new()) as Arc<dyn ProviderAdapter>],
    );
    let group = AccountGroup::new("g", ConfigDir::standard("C:/Users/exemplo"));
    let config = RouterConfig {
        groups: vec![group.clone()],
        ..Default::default()
    };

    let err = launcher
        .prepare(&group, &config, &HashMap::new(), Vec::new())
        .unwrap_err();
    assert_eq!(err, LaunchError::EmptyGroup("g".into()));
}

#[test]
fn starts_with_the_first_account_on_a_cold_start() {
    let f = fixture();
    let plan = f
        .launcher
        .prepare(&f.group, &f.config, &HashMap::new(), Vec::new())
        .unwrap();

    assert_eq!(plan.account.id, f.a.id);
    // O grupo padrão não exporta CLAUDE_CONFIG_DIR.
    assert_eq!(plan.config_dir_env, None);
    assert_eq!(plan.executable, "claude");
}

#[test]
fn keeps_the_active_account_while_it_has_headroom() {
    let f = fixture();
    f.launcher
        .prepare(&f.group, &f.config, &HashMap::new(), Vec::new())
        .unwrap(); // ativa `a`

    let plan = f
        .launcher
        .prepare(&f.group, &f.config, &usage(&[(f.a.id, 0.5)]), Vec::new())
        .unwrap();
    assert_eq!(plan.account.id, f.a.id);
}

#[test]
fn jumps_to_the_next_with_headroom_when_the_active_is_over() {
    let f = fixture();
    f.launcher
        .prepare(&f.group, &f.config, &HashMap::new(), Vec::new())
        .unwrap(); // ativa `a`

    let plan = f
        .launcher
        .prepare(
            &f.group,
            &f.config,
            &usage(&[(f.a.id, 0.95), (f.b.id, 0.1)]),
            vec!["--resume".into()],
        )
        .unwrap();
    assert_eq!(plan.account.id, f.b.id);
    assert_eq!(plan.arguments, vec!["--resume".to_string()]);
}

#[test]
fn a_dedicated_group_exports_claude_config_dir() {
    let store = Arc::new(FakeStore::new());
    let adapter = Arc::new(FakeAdapter::new());
    let a = account("a@exemplo.com", "C:/Users/exemplo/.claude-a");
    store
        .write(&cred("ca"), &adapter.credential_location(&a.home))
        .unwrap();
    let mut group = AccountGroup::new(
        "pessoal",
        ConfigDir::dedicated("C:/Users/exemplo/grupos/pessoal"),
    );
    group.account_ids = vec![a.id];
    let config = RouterConfig {
        accounts: vec![a],
        groups: vec![group.clone()],
        ..Default::default()
    };
    let launcher = SessionLauncher::new(store, vec![adapter as Arc<dyn ProviderAdapter>]);

    let plan = launcher
        .prepare(&group, &config, &HashMap::new(), Vec::new())
        .unwrap();
    assert_eq!(
        plan.config_dir_env.as_deref(),
        Some("C:/Users/exemplo/grupos/pessoal")
    );
}

// --- Os dois ramos que o Swift não fixava ---

/// Ninguém com folga: melhor subir na conta cheia do que recusar o lançamento
/// (o usuário pode trocar de modelo, ou só ler).
#[test]
fn stays_on_the_active_when_nobody_has_headroom() {
    let f = fixture();
    f.launcher
        .prepare(&f.group, &f.config, &HashMap::new(), Vec::new())
        .unwrap(); // ativa `a`

    let plan = f
        .launcher
        .prepare(
            &f.group,
            &f.config,
            &usage(&[(f.a.id, 0.95), (f.b.id, 0.97)]),
            Vec::new(),
        )
        .unwrap();
    assert_eq!(plan.account.id, f.a.id);
}

#[test]
fn an_account_that_cannot_be_activated_is_no_usable_account() {
    let launcher = SessionLauncher::new(
        Arc::new(FakeStore::new()),
        vec![Arc::new(FakeAdapter::new()) as Arc<dyn ProviderAdapter>],
    );
    let orphan = account("sem-credencial@exemplo.com", "C:/Users/exemplo/.claude-z");
    let mut group = AccountGroup::new("g", ConfigDir::standard("C:/Users/exemplo"));
    group.account_ids = vec![orphan.id];
    let config = RouterConfig {
        accounts: vec![orphan],
        groups: vec![group.clone()],
        ..Default::default()
    };

    let err = launcher
        .prepare(&group, &config, &HashMap::new(), Vec::new())
        .unwrap_err();
    assert_eq!(err, LaunchError::NoUsableAccount("g".into()));
}
