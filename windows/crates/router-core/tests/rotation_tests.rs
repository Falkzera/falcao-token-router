//! `RotationEngine`: um teste por regra.
//!
//! Portados de `EngineTests.swift` (suítes "RotationEngine" e "Decisão de
//! rotação") e de `ProbeTests.swift` ("Sonda: por qual perfil medir"), com os
//! mesmos cenários e fixtures anonimizadas. Acrescidos de um teste para cada
//! regra do motor que o Swift não fixava (conta ativa por identidade, falha de
//! escrita nomeada, espelho que só escreve se mudou, empurrão pós-relogin, limiar
//! exato, ordem de preferência) e de dois de ponta a ponta com arquivos reais do
//! Windows (mtime novo na troca; arquivo incompleto não se espalha).

mod common;
use common::*;

use std::collections::HashMap;
use std::fs::{self, OpenOptions};
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use router_core::engine::anthropic_adapter::AnthropicAdapter;
use router_core::engine::credential_store::{CredentialStore, FileCredentialStore};
use router_core::engine::provider::ProviderAdapter;
use router_core::engine::rotation_engine::{RotationEngine, RotationError};
use router_core::{Account, AccountGroup, ConfigDir, Id, RouterConfig};

struct Setup {
    engine: RotationEngine,
    store: Arc<FakeStore>,
    adapter: Arc<FakeAdapter>,
    config: RouterConfig,
    a: Account,
    b: Account,
    group: AccountGroup,
}

fn engine_with(store: &Arc<FakeStore>, adapter: &Arc<FakeAdapter>) -> RotationEngine {
    RotationEngine::new(
        store.clone(),
        vec![adapter.clone() as Arc<dyn ProviderAdapter>],
    )
}

fn config_of(accounts: &[&Account], groups: &[&AccountGroup]) -> RouterConfig {
    RouterConfig {
        accounts: accounts.iter().map(|a| (*a).clone()).collect(),
        groups: groups.iter().map(|g| (*g).clone()).collect(),
        ..Default::default()
    }
}

fn group_of(name: &str, dir: ConfigDir, accounts: &[&Account]) -> AccountGroup {
    let mut g = AccountGroup::new(name, dir);
    g.account_ids = accounts.iter().map(|a| a.id).collect();
    g
}

/// Duas contas com credencial na casa, num grupo padrão (≙ `setup()` do Swift).
fn setup() -> Setup {
    let store = Arc::new(FakeStore::new());
    let adapter = Arc::new(FakeAdapter::new());
    let engine = engine_with(&store, &adapter);
    let a = account("conta-a@exemplo.com", "C:/Users/exemplo/.claude-pessoal");
    let b = account("conta-b@exemplo.com", "C:/Users/exemplo/.claude-trabalho");
    store
        .write(&cred("cred-a"), &adapter.credential_location(&a.home))
        .unwrap();
    store
        .write(&cred("cred-b"), &adapter.credential_location(&b.home))
        .unwrap();
    let group = group_of(
        "trabalho",
        ConfigDir::standard("C:/Users/exemplo"),
        &[&a, &b],
    );
    let config = config_of(&[&a, &b], &[&group]);
    Setup {
        engine,
        store,
        adapter,
        config,
        a,
        b,
        group,
    }
}

impl Setup {
    fn group_location(&self) -> std::path::PathBuf {
        self.adapter.credential_location(&self.group.config_dir)
    }

    fn home_location(&self, account: &Account) -> std::path::PathBuf {
        self.adapter.credential_location(&account.home)
    }

    fn active_id(&self) -> Option<Id> {
        self.engine
            .active_account(&self.group, &self.config)
            .map(|a| a.id)
    }
}

// --- A troca em si (suíte "RotationEngine" do Swift) ---

#[test]
fn activate_copies_the_home_secret_into_the_group_and_writes_identity() {
    let s = setup();
    s.engine.activate(&s.a, &s.group, &s.config).unwrap();

    assert_eq!(s.store.read(&s.group_location()), Some(cred("cred-a")));
    assert_eq!(
        s.adapter
            .identity(&s.group.config_dir)
            .map(|i| i.email)
            .as_deref(),
        Some("conta-a@exemplo.com")
    );
    assert_eq!(s.active_id(), Some(s.a.id));
}

/// A regra que evita a cópia vencida: antes de trocar, o token vivo do grupo
/// volta para a casa da conta que sai.
#[test]
fn switching_mirrors_the_group_token_back_to_the_leaving_home() {
    let s = setup();
    s.engine.activate(&s.a, &s.group, &s.config).unwrap();
    // O Claude Code renova o token no item do grupo.
    s.store
        .write(&cred("cred-a-RENOVADO"), &s.group_location())
        .unwrap();

    s.engine.activate(&s.b, &s.group, &s.config).unwrap();

    assert_eq!(
        s.store.read(&s.home_location(&s.a)),
        Some(cred("cred-a-RENOVADO"))
    );
    assert_eq!(s.store.read(&s.group_location()), Some(cred("cred-b")));
    assert_eq!(s.active_id(), Some(s.b.id));
}

/// Regressão do "Login expired" de 26/ago (macOS): relançar com a mesma conta
/// ativa reescrevia o item do grupo com a cópia velha da casa — e o refresh
/// token gira, então a cópia velha está morta. Reativar a ativa preserva o token
/// vivo do grupo e o espelha para a casa.
#[test]
fn reactivating_the_active_account_keeps_the_live_token_and_updates_home() {
    let s = setup();
    s.engine.activate(&s.a, &s.group, &s.config).unwrap();
    s.store
        .write(&cred("cred-a-RENOVADO"), &s.group_location())
        .unwrap();

    s.engine.activate(&s.a, &s.group, &s.config).unwrap(); // relançamento

    assert_eq!(
        s.store.read(&s.group_location()),
        Some(cred("cred-a-RENOVADO"))
    );
    assert_eq!(
        s.store.read(&s.home_location(&s.a)),
        Some(cred("cred-a-RENOVADO"))
    );
}

/// O ciclo periódico que mantém a casa viva enquanto a conta está ativa.
#[test]
fn mirror_active_keeps_the_home_fresh() {
    let s = setup();
    s.engine.activate(&s.a, &s.group, &s.config).unwrap();
    s.store
        .write(&cred("cred-a-RENOVADO"), &s.group_location())
        .unwrap();

    s.engine.mirror_active(&s.group, &s.config);

    assert_eq!(
        s.store.read(&s.home_location(&s.a)),
        Some(cred("cred-a-RENOVADO"))
    );
}

/// A falha que já matou contas: a mesma conta ativa em dois grupos — duas
/// cópias de um refresh token que gira.
#[test]
fn refuses_an_account_that_already_serves_another_group() {
    let store = Arc::new(FakeStore::new());
    let adapter = Arc::new(FakeAdapter::new());
    let engine = engine_with(&store, &adapter);
    let shared = account("x@exemplo.com", "C:/Users/exemplo/.claude-x");
    store
        .write(&cred("cred-x"), &adapter.credential_location(&shared.home))
        .unwrap();
    let g1 = group_of(
        "trabalho",
        ConfigDir::standard("C:/Users/exemplo"),
        &[&shared],
    );
    let g2 = group_of(
        "pessoal",
        ConfigDir::dedicated("C:/Users/exemplo/.claude-pessoal"),
        &[&shared],
    );
    let config = config_of(&[&shared], &[&g1, &g2]);

    engine.activate(&shared, &g1, &config).unwrap();
    let err = engine.activate(&shared, &g2, &config).unwrap_err();

    assert_eq!(
        err,
        RotationError::AccountBusyElsewhere {
            account_id: shared.id,
            group_id: g1.id
        }
    );
    // E o grupo 2 não recebeu cópia nenhuma.
    assert_eq!(
        store.read(&adapter.credential_location(&g2.config_dir)),
        None
    );
}

#[test]
fn an_account_without_a_home_credential_is_a_named_error() {
    let store = Arc::new(FakeStore::new());
    let adapter = Arc::new(FakeAdapter::new());
    let engine = engine_with(&store, &adapter);
    let orphan = account("z@exemplo.com", "C:/Users/exemplo/.claude-z");
    let group = group_of("g", ConfigDir::standard("C:/Users/exemplo"), &[&orphan]);
    let config = config_of(&[&orphan], &[&group]);

    assert_eq!(
        engine.activate(&orphan, &group, &config).unwrap_err(),
        RotationError::NoCredential {
            account_id: orphan.id
        }
    );
}

// --- Regras do motor que o Swift não fixava ---

/// A conta ativa é a do grupo cuja identidade está gravada no perfil — nunca uma
/// conta de fora do grupo, mesmo que o perfil mostre o e-mail dela.
#[test]
fn the_active_account_is_read_from_the_profile_identity() {
    let s = setup();
    assert_eq!(s.active_id(), None, "grupo nunca ativado não tem ativa");

    let outsider = ident("de-fora@exemplo.com");
    s.adapter
        .write_identity(&outsider, &s.group.config_dir)
        .unwrap();
    assert_eq!(s.active_id(), None, "identidade de fora do grupo não conta");

    s.adapter
        .write_identity(&s.b.identity, &s.group.config_dir)
        .unwrap();
    assert_eq!(s.active_id(), Some(s.b.id));
}

#[test]
fn a_failed_identity_write_is_write_failed() {
    let s = setup();
    s.adapter.fail_identity_writes();

    let err = s.engine.activate(&s.a, &s.group, &s.config).unwrap_err();

    assert!(matches!(err, RotationError::WriteFailed(_)), "{err:?}");
}

/// O espelho só escreve na casa quando o token do grupo mudou — no Windows cada
/// escrita é um mtime novo, e escrita à toa faz sessão viva reler à toa.
#[test]
fn the_mirror_writes_the_home_only_when_it_changed() {
    let s = setup();
    s.engine.activate(&s.a, &s.group, &s.config).unwrap();
    let before = s.store.writes_to(&s.home_location(&s.a));

    s.engine.mirror_active(&s.group, &s.config);
    s.engine.mirror_active(&s.group, &s.config);
    assert_eq!(s.store.writes_to(&s.home_location(&s.a)), before);

    s.store
        .write(&cred("cred-a-2"), &s.group_location())
        .unwrap();
    s.engine.mirror_active(&s.group, &s.config);
    assert_eq!(s.store.writes_to(&s.home_location(&s.a)), before + 1);
}

/// Depois de um RELOGIN de conta ativa: a casa tem a credencial nova e o grupo
/// guarda a morta que motivou o relogin — o único caso em que casa→grupo com a
/// conta já ativa é o movimento certo.
#[test]
fn push_home_to_group_replaces_the_dead_copy_after_a_relogin() {
    let s = setup();
    s.engine.activate(&s.a, &s.group, &s.config).unwrap();
    s.store
        .write(&cred("cred-a-NOVA"), &s.home_location(&s.a))
        .unwrap();

    s.engine.push_home_to_group(&s.a, &s.group);

    assert_eq!(s.store.read(&s.group_location()), Some(cred("cred-a-NOVA")));
    assert_eq!(s.active_id(), Some(s.a.id));
}

// --- Por qual perfil sondar (ProbeTests.swift → "Sonda: por qual perfil medir") ---

/// A regra que protege a sessão viva: sondar a casa de uma conta ATIVA faria o
/// `claude` renovar com o refresh token velho, girar a cadeia e derrubar a
/// sessão do usuário em "Login expired" no meio do trabalho.
#[test]
fn an_active_account_is_probed_through_the_group_never_the_home() {
    let store = Arc::new(FakeStore::new());
    let adapter = Arc::new(FakeAdapter::new());
    let engine = engine_with(&store, &adapter);
    let home = ConfigDir::dedicated("C:/Users/exemplo/casa-a");
    let mut conta = account("conta2@exemplo.com", "x");
    conta.home = home.clone();
    let group_dir = ConfigDir::dedicated("C:/Users/exemplo/grupo-trabalho");
    let group = group_of("trabalho", group_dir.clone(), &[&conta]);
    let config = config_of(&[&conta], &[&group]);

    // Ainda não ativou ninguém: a casa é o alvo certo.
    assert_eq!(engine.probe_config_dir(&conta, &config), home);

    store
        .write(&cred("cred"), &adapter.credential_location(&home))
        .unwrap();
    engine.activate(&conta, &group, &config).unwrap();
    assert_eq!(engine.probe_config_dir(&conta, &config), group_dir);
}

// --- A decisão de quando trocar (suíte "Decisão de rotação" do Swift) ---

/// Duas contas com credencial, limiar 90, `a` ativa.
fn decision() -> Setup {
    let s = setup();
    s.engine.activate(&s.a, &s.group, &s.config).unwrap();
    s
}

fn usage(pairs: &[(Id, f64)]) -> HashMap<Id, f64> {
    pairs.iter().copied().collect()
}

#[test]
fn stays_while_the_active_account_has_headroom() {
    let s = decision();
    let target =
        s.engine
            .rotation_target(&s.group, &s.config, &usage(&[(s.a.id, 0.5), (s.b.id, 0.1)]));
    assert!(target.is_none());
}

#[test]
fn rotates_to_the_next_with_headroom_when_the_active_is_over() {
    let s = decision();
    let target = s.engine.rotation_target(
        &s.group,
        &s.config,
        &usage(&[(s.a.id, 0.95), (s.b.id, 0.1)]),
    );
    assert_eq!(target.map(|t| t.id), Some(s.b.id));
}

/// Fail-safe: ativa estourada mas nenhuma outra qualifica → mantém, não trava.
#[test]
fn keeps_the_current_account_when_no_candidate_qualifies() {
    let s = decision();
    let target = s.engine.rotation_target(
        &s.group,
        &s.config,
        &usage(&[(s.a.id, 0.95), (s.b.id, 0.97)]),
    );
    assert!(target.is_none());
}

/// Regressão de um deadlock real: no sensor passivo, conta nunca usada não tem
/// amostra — exigi-la impedia a rotação para sempre (só mede quem serve; só
/// serve quem é escolhida). Sem medição = presumida fresca.
#[test]
fn an_unmeasured_account_is_presumed_fresh_and_becomes_the_target() {
    let s = decision();
    let target = s
        .engine
        .rotation_target(&s.group, &s.config, &usage(&[(s.a.id, 0.95)]));
    assert_eq!(target.map(|t| t.id), Some(s.b.id));
}

#[test]
fn auto_rotate_off_never_rotates_on_its_own() {
    let s = decision();
    let mut off = s.group.clone();
    off.auto_rotate = false;
    let config = config_of(&[&s.a, &s.b], &[&off]);
    let target = s
        .engine
        .rotation_target(&off, &config, &usage(&[(s.a.id, 0.99), (s.b.id, 0.0)]));
    assert!(target.is_none());
}

/// Comparação estrita: bater o limiar exato já conta como passar dele.
#[test]
fn exactly_at_the_threshold_counts_as_over() {
    let s = decision();
    let target = s.engine.rotation_target(
        &s.group,
        &s.config,
        &usage(&[(s.a.id, 0.90), (s.b.id, 0.1)]),
    );
    assert_eq!(target.map(|t| t.id), Some(s.b.id));
}

/// A ativa sem amostra é presumida fresca também — o grupo fica onde está.
#[test]
fn an_active_account_without_a_sample_stays_put() {
    let s = decision();
    let target = s
        .engine
        .rotation_target(&s.group, &s.config, &usage(&[(s.b.id, 0.1)]));
    assert!(target.is_none());
}

/// A preferência é a ordem do grupo: a primeira com folga, não a mais folgada.
#[test]
fn the_next_account_follows_the_preference_order() {
    let s = setup();
    let c = account("conta-c@exemplo.com", "C:/Users/exemplo/.claude-c");
    let d = account("conta-d@exemplo.com", "C:/Users/exemplo/.claude-d");
    let group = group_of(
        "g",
        ConfigDir::standard("C:/Users/exemplo"),
        &[&s.a, &s.b, &c, &d],
    );
    let config = config_of(&[&s.a, &s.b, &c, &d], &[&group]);

    let next = s.engine.next_account(
        &group,
        &config,
        &usage(&[(s.a.id, 0.95), (s.b.id, 0.99), (c.id, 0.60), (d.id, 0.0)]),
    );
    assert_eq!(next.map(|n| n.id), Some(c.id));
}

// --- Ponta a ponta com arquivos reais do Windows ---

fn long_ago() -> SystemTime {
    SystemTime::UNIX_EPOCH + Duration::from_secs(1_000_000_000)
}

fn real_blob(tag: &str) -> Vec<u8> {
    format!(r#"{{"claudeAiOauth":{{"accessToken":"falso-{tag}"}},"mcpOAuth":{{}}}}"#).into_bytes()
}

fn seed_home(dir: &ConfigDir, email: &str, tag: &str) {
    let path = AnthropicAdapter.credential_location(dir);
    fs::create_dir_all(dir.path()).unwrap();
    fs::write(&path, real_blob(tag)).unwrap();
    OpenOptions::new()
        .write(true)
        .open(&path)
        .unwrap()
        .set_modified(long_ago())
        .unwrap();
    AnthropicAdapter.write_identity(&ident(email), dir).unwrap();
}

/// A troca a quente do Windows, com o store e o adapter de verdade: a credencial
/// do grupo sai com mtime novo (é o que faz a sessão viva reler) mesmo vindo de
/// uma casa com mtime antigo; a identidade e o onboarding vão para o grupo; e na
/// troca seguinte o token vivo do grupo volta para a casa que sai.
#[test]
fn a_real_hot_swap_leaves_a_fresh_mtime_and_mirrors_back() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = |name: &str| ConfigDir::dedicated(tmp.path().join(name).to_string_lossy());
    let (home_a, home_b, group_dir) = (dir("casa-a"), dir("casa-b"), dir("grupo"));
    seed_home(&home_a, "conta1@exemplo.com", "a");
    seed_home(&home_b, "conta2@exemplo.com", "b");
    let mut a = account("conta1@exemplo.com", "x");
    a.home = home_a.clone();
    let mut b = account("conta2@exemplo.com", "x");
    b.home = home_b.clone();
    let group = group_of("trabalho", group_dir.clone(), &[&a, &b]);
    let config = config_of(&[&a, &b], &[&group]);
    let engine = RotationEngine::new(
        Arc::new(FileCredentialStore::new()),
        vec![Arc::new(AnthropicAdapter) as Arc<dyn ProviderAdapter>],
    );
    let group_cred = AnthropicAdapter.credential_location(&group_dir);

    engine.activate(&a, &group, &config).unwrap();
    let fresh = fs::metadata(&group_cred).unwrap().modified().unwrap();
    assert!(
        fresh > long_ago() + Duration::from_secs(3600),
        "mtime herdado"
    );
    assert_eq!(fs::read(&group_cred).unwrap(), real_blob("a"));
    let root: serde_json::Value =
        serde_json::from_slice(&fs::read(group_dir.global_config_path()).unwrap()).unwrap();
    assert_eq!(root["oauthAccount"]["emailAddress"], "conta1@exemplo.com");
    assert_eq!(root["hasCompletedOnboarding"], true);

    // O Claude Code renova o token no grupo; depois, a troca para B.
    fs::write(&group_cred, real_blob("a-renovado")).unwrap();
    engine.activate(&b, &group, &config).unwrap();

    assert_eq!(
        fs::read(AnthropicAdapter.credential_location(&home_a)).unwrap(),
        real_blob("a-renovado")
    );
    assert_eq!(fs::read(&group_cred).unwrap(), real_blob("b"));
    assert_eq!(
        engine.active_account(&group, &config).map(|x| x.id),
        Some(b.id)
    );
}

/// Um `.credentials.json` do grupo pego no meio de uma escrita não é espalhado
/// para a casa: o espelho não acontece e a casa fica como estava.
#[test]
fn the_mirror_never_spreads_an_incomplete_group_file() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = |name: &str| ConfigDir::dedicated(tmp.path().join(name).to_string_lossy());
    let (home_a, group_dir) = (dir("casa-a"), dir("grupo"));
    seed_home(&home_a, "conta1@exemplo.com", "a");
    let mut a = account("conta1@exemplo.com", "x");
    a.home = home_a.clone();
    let group = group_of("trabalho", group_dir.clone(), &[&a]);
    let config = config_of(&[&a], &[&group]);
    let engine = RotationEngine::new(
        Arc::new(FileCredentialStore::new()),
        vec![Arc::new(AnthropicAdapter) as Arc<dyn ProviderAdapter>],
    );
    engine.activate(&a, &group, &config).unwrap();

    fs::write(
        AnthropicAdapter.credential_location(&group_dir),
        br#"{"claudeAiOauth":{"accessT"#,
    )
    .unwrap();
    engine.mirror_active(&group, &config);

    assert_eq!(
        fs::read(AnthropicAdapter.credential_location(&home_a)).unwrap(),
        real_blob("a")
    );
}
