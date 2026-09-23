//! `RouterConfigStore`: a API que a UI de grupos chama, ponta a ponta, com
//! credencial e identidade em memória.
//!
//! Portados de `StoreTests.swift` (16), com uma diferença de montagem: no macOS o
//! grupo padrão apontava para a home REAL (`NSHomeDirectory()`); aqui a home é
//! injetada e mora num diretório temporário, então nenhum teste toca o
//! `%USERPROFILE%\.claude`. Acrescidos das regressões do porte: limiar com
//! limites, reordenar sem perder conta, `config.json` ilegível guardado de lado
//! (nunca sobrescrito), remoção que não apaga pasta fora da base e a volta de
//! rotação completa.

mod common;
use common::*;

use std::path::{Path, PathBuf};
use std::sync::Arc;

use chrono::Utc;
use router_core::engine::credential_store::CredentialStore;
use router_core::engine::group_usage::{GroupUsageSample, GroupUsageStore, UsageOrigin};
use router_core::engine::provider::ProviderAdapter;
use router_core::engine::router_config_store::{LoginOutcome, ReloginOutcome, RouterConfigStore};
use router_core::engine::router_paths::RouterPaths;
use router_core::{Account, ConfigDir, Id};

struct Env {
    store: RouterConfigStore,
    creds: Arc<FakeStore>,
    adapter: Arc<FakeAdapter>,
    paths: RouterPaths,
    tmp: tempfile::TempDir,
}

fn open(tmp: &Path, creds: &Arc<FakeStore>, adapter: &Arc<FakeAdapter>) -> RouterConfigStore {
    RouterConfigStore::new(
        RouterPaths::with_app_support(Some(tmp.join("app"))),
        tmp.join("home").to_string_lossy().into_owned(),
        creds.clone(),
        vec![adapter.clone() as Arc<dyn ProviderAdapter>],
    )
}

/// Um store com caminhos temporários e adapter/credencial falsos.
fn make_store() -> Env {
    let tmp = tempfile::tempdir().unwrap();
    let creds = Arc::new(FakeStore::new());
    let adapter = Arc::new(FakeAdapter::new());
    let store = open(tmp.path(), &creds, &adapter);
    Env {
        store,
        creds,
        adapter,
        paths: RouterPaths::with_app_support(Some(tmp.path().join("app"))),
        tmp,
    }
}

impl Env {
    /// Simula um login concluído: identidade no perfil + credencial.
    fn seed_login(&self, email: &str, home: &ConfigDir) {
        self.adapter.write_identity(&ident(email), home).unwrap();
        self.creds
            .write(
                &cred(&format!("cred-{email}")),
                &self.adapter.credential_location(home),
            )
            .unwrap();
    }

    /// Login completo numa casa nova, adicionado ao grupo.
    fn add_account(&mut self, email: &str, group: Id) -> Account {
        let (home, id) = self.store.new_account_home();
        self.seed_login(email, &home);
        match self.store.finish_pending_login(&home, id, group) {
            LoginOutcome::Added(account) => account,
            other => panic!("login não entrou: {other:?}"),
        }
    }

    fn group(&self, id: Id) -> router_core::AccountGroup {
        self.store
            .config()
            .groups
            .iter()
            .find(|g| g.id == id)
            .cloned()
            .unwrap()
    }
}

#[test]
fn the_first_group_is_the_default_and_the_second_is_dedicated() {
    let mut env = make_store();
    let work = env.store.add_group("trabalho");
    let personal = env.store.add_group("pessoal");

    assert!(work.config_dir.is_default);
    assert!(!personal.config_dir.is_default);
    assert_eq!(
        env.store.config().default_group().map(|g| g.id),
        Some(work.id)
    );
}

/// "Remover a conta" tem de remover a conta. Até 18/09/2026 (macOS) só o registro
/// saía: a credencial e a pasta da casa ficavam para trás com um refresh token
/// vivo, para sempre.
#[test]
fn removing_an_account_deletes_its_credential_and_home() {
    let mut env = make_store();
    let group = env.store.add_group("trabalho");
    let account = env.add_account("conta9@exemplo.com", group.id);
    let location = env.adapter.credential_location(&account.home);
    assert!(env.creds.exists(&location));
    assert!(account.home.path().exists());

    env.store.remove_account(account.id);

    assert!(env.store.config().account(account.id).is_none());
    assert!(!env.creds.exists(&location), "a credencial ficou");
    assert!(!account.home.path().exists(), "a casa ficou no disco");
}

/// Apagar o grupo leva junto as contas que só existiam nele — e, por tabela, as
/// credenciais delas.
#[test]
fn removing_a_group_deletes_the_credentials_of_its_exclusive_accounts() {
    let mut env = make_store();
    let group = env.store.add_group("temporario");
    let account = env.add_account("so-aqui@exemplo.com", group.id);
    let location = env.adapter.credential_location(&account.home);

    env.store.remove_group(group.id);

    assert!(env.store.config().accounts.is_empty());
    assert!(!env.creds.exists(&location));
}

/// Relogin renova a identidade no registro e, se a conta está ATIVA num grupo,
/// empurra a credencial nova da casa para o grupo — que guardava a morta que
/// motivou o relogin (o caso real de 26/ago no macOS).
#[test]
fn a_relogin_renews_the_account_and_updates_the_group_when_active() {
    let mut env = make_store();
    let group = env.store.add_group("trabalho");
    let account = env.add_account("conta2@exemplo.com", group.id);
    env.store.activate(&account, &env.group(group.id));

    // O relogin oficial escreve credencial NOVA na casa da conta.
    env.creds
        .write(
            &cred("cred-NOVA"),
            &env.adapter.credential_location(&account.home),
        )
        .unwrap();
    let outcome = env.store.finish_relogin(account.id);

    assert!(matches!(outcome, ReloginOutcome::Renewed(_)), "{outcome:?}");
    let group_location = env
        .adapter
        .credential_location(&env.group(group.id).config_dir);
    assert_eq!(env.creds.read(&group_location), Some(cred("cred-NOVA")));
}

/// Relogin que autenticou OUTRO e-mail não toca no registro — a UI explica.
#[test]
fn a_relogin_into_another_account_is_wrong_account_and_changes_nothing() {
    let mut env = make_store();
    let group = env.store.add_group("trabalho");
    let account = env.add_account("conta2@exemplo.com", group.id);

    // O navegador estava logado em outra conta: a casa recebe outra identidade.
    env.seed_login("intrusa@exemplo.com", &account.home);
    let outcome = env.store.finish_relogin(account.id);

    assert_eq!(
        outcome,
        ReloginOutcome::WrongAccount {
            expected: "conta2@exemplo.com".into(),
            got: "intrusa@exemplo.com".into()
        }
    );
    assert_eq!(
        env.store
            .config()
            .account(account.id)
            .map(|a| a.identity.email.as_str()),
        Some("conta2@exemplo.com")
    );
}

/// Órfãs não aparecem em tela nenhuma. As contas dos outros grupos ficam.
#[test]
fn removing_a_group_drops_its_exclusive_accounts_and_keeps_the_others() {
    let mut env = make_store();
    let work = env.store.add_group("trabalho");
    let personal = env.store.add_group("pessoal");
    let work_acc = env.add_account("trabalho@exemplo.com", work.id);
    let personal_acc = env.add_account("eu@exemplo.com", personal.id);

    env.store.remove_group(work.id);

    assert_eq!(env.store.config().groups.len(), 1);
    assert!(env.store.config().account(work_acc.id).is_none());
    assert!(env.store.config().account(personal_acc.id).is_some());
}

#[test]
fn making_another_group_the_default_takes_it_from_the_previous_one() {
    let mut env = make_store();
    let work = env.store.add_group("trabalho");
    let personal = env.store.add_group("pessoal");

    env.store.make_default(personal.id);

    assert!(env.group(personal.id).config_dir.is_default);
    assert!(!env.group(work.id).config_dir.is_default);
    let defaults = env
        .store
        .config()
        .groups
        .iter()
        .filter(|g| g.config_dir.is_default)
        .count();
    assert_eq!(defaults, 1, "no máximo um padrão");
}

#[test]
fn a_finished_login_becomes_an_account_in_the_group() {
    let mut env = make_store();
    let group = env.store.add_group("trabalho");
    let id = Id::new();
    let home = env.paths.account_home(id);
    env.seed_login("conta1@exemplo.com", &home);

    let outcome = env.store.finish_pending_login(&home, id, group.id);

    let LoginOutcome::Added(account) = outcome else {
        panic!("esperava Added, veio {outcome:?}");
    };
    assert_eq!(account.identity.email, "conta1@exemplo.com");
    assert_eq!(env.store.config().accounts.len(), 1);
    assert_eq!(env.group(group.id).account_ids, vec![id]);
}

/// Quase sempre porque o navegador ainda estava logado na mesma conta.
#[test]
fn the_same_account_again_is_a_duplicate_not_another_account() {
    let mut env = make_store();
    let group = env.store.add_group("pessoal");
    let (id1, id2) = (Id::new(), Id::new());
    let (home1, home2) = (env.paths.account_home(id1), env.paths.account_home(id2));
    env.seed_login("pessoal@exemplo.com", &home1);
    env.store.finish_pending_login(&home1, id1, group.id);

    env.seed_login("pessoal@exemplo.com", &home2);
    let outcome = env.store.finish_pending_login(&home2, id2, group.id);

    assert_eq!(
        outcome,
        LoginOutcome::Duplicate {
            email: "pessoal@exemplo.com".into()
        }
    );
    assert_eq!(env.store.config().accounts.len(), 1);
}

#[test]
fn an_unfinished_login_adds_nothing() {
    let mut env = make_store();
    let group = env.store.add_group("g");
    let id = Id::new();

    let outcome = env
        .store
        .finish_pending_login(&env.paths.account_home(id), id, group.id);

    assert_eq!(outcome, LoginOutcome::Pending);
    assert!(env.store.config().accounts.is_empty());
}

#[test]
fn reordering_rewrites_the_preference_order() {
    let mut env = make_store();
    let group = env.store.add_group("g");
    let ids: Vec<Id> = ["a@exemplo.com", "b@exemplo.com", "c@exemplo.com"]
        .iter()
        .map(|e| env.add_account(e, group.id).id)
        .collect();

    let reversed: Vec<Id> = ids.iter().rev().copied().collect();
    env.store.reorder_accounts(group.id, &reversed);

    assert_eq!(env.group(group.id).account_ids, reversed);
}

#[test]
fn activating_switches_the_account_that_serves_the_group() {
    let mut env = make_store();
    let group = env.store.add_group("trabalho");
    let first = env.add_account("conta1@exemplo.com", group.id);
    let second = env.add_account("conta2@exemplo.com", group.id);
    let _ = first;

    env.store.activate(&second, &env.group(group.id));

    assert!(
        env.store.last_error().is_none(),
        "{:?}",
        env.store.last_error()
    );
    assert_eq!(
        env.store.active_account(&env.group(group.id)).map(|a| a.id),
        Some(second.id)
    );
    assert_eq!(
        env.store.active_by_group().get(&group.id),
        Some(&second.id),
        "o quadro publicado não acompanhou"
    );
}

#[test]
fn removing_an_account_drops_it_from_the_group_lists() {
    let mut env = make_store();
    let group = env.store.add_group("g");
    let account = env.add_account("x@exemplo.com", group.id);

    env.store.remove_account(account.id);

    assert!(env.store.config().accounts.is_empty());
    assert!(env.group(group.id).account_ids.is_empty());
}

#[test]
fn the_configuration_persists_between_instances() {
    let tmp = tempfile::tempdir().unwrap();
    let creds = Arc::new(FakeStore::new());
    let adapter = Arc::new(FakeAdapter::new());

    let mut first = open(tmp.path(), &creds, &adapter);
    let group = first.add_group("trabalho");
    first.set_threshold(group.id, 85.0);

    let second = open(tmp.path(), &creds, &adapter);
    let g = &second.config().groups[0];
    assert_eq!(g.name, "trabalho");
    assert_eq!(g.threshold_percent, 85.0);
}

#[test]
fn sensor_usage_becomes_usage_per_account() {
    let mut env = make_store();
    let group = env.store.add_group("g");
    let account = env.add_account("m@exemplo.com", group.id);

    // O sensor gravou uma amostra dessa conta.
    let sample = GroupUsageSample::new(
        group.config_dir.raw.clone(),
        Some("m@exemplo.com".into()),
        Some(0.4),
        None,
        Some(0.72),
        None,
        Utc::now(),
        None,
        UsageOrigin::Sensor,
    );
    GroupUsageStore::write(&sample, "m@exemplo.com", &env.paths.usage_dir()).unwrap();

    env.store.refresh_usage();

    // Liga no maior dos dois: 72% > 40%.
    assert_eq!(env.store.usage_snapshot().get(&account.id), Some(&0.72));
}

#[test]
fn clear_default_leaves_no_group_in_dot_claude() {
    let mut env = make_store();
    let work = env.store.add_group("trabalho"); // vira padrão
    env.store.add_group("pessoal");
    assert_eq!(
        env.store.config().default_group().map(|g| g.id),
        Some(work.id)
    );

    env.store.clear_default();

    assert!(env.store.config().default_group().is_none());
    assert!(env
        .store
        .config()
        .groups
        .iter()
        .all(|g| !g.config_dir.is_default));
}

// --- Regressões do porte ---

/// O limiar vive entre 50% e 100%: abaixo disso a conta troca antes de servir
/// qualquer coisa útil; acima, nunca troca.
#[test]
fn the_threshold_is_clamped_between_50_and_100() {
    let mut env = make_store();
    let group = env.store.add_group("g");

    env.store.set_threshold(group.id, 30.0);
    assert_eq!(env.group(group.id).threshold_percent, 50.0);
    env.store.set_threshold(group.id, 120.0);
    assert_eq!(env.group(group.id).threshold_percent, 100.0);
    env.store.set_threshold(group.id, 85.0);
    assert_eq!(env.group(group.id).threshold_percent, 85.0);
}

/// Reordenar ignora id estranho (como no macOS) e — diferente do macOS — não
/// derruba do grupo uma conta que a lista pedida esqueceu: ela vai para o fim.
#[test]
fn reordering_ignores_unknown_ids_and_never_drops_an_account() {
    let mut env = make_store();
    let group = env.store.add_group("g");
    let a = env.add_account("a@exemplo.com", group.id).id;
    let b = env.add_account("b@exemplo.com", group.id).id;
    let c = env.add_account("c@exemplo.com", group.id).id;

    env.store.reorder_accounts(group.id, &[c, Id::new(), a]);

    assert_eq!(env.group(group.id).account_ids, vec![c, a, b]);
}

/// Um `config.json` ilegível não pode ser sobrescrito pelo primeiro `save`: a
/// configuração do usuário (grupos, contas) sumiria sem aviso. O porte começa
/// vazio em memória e, ao salvar, guarda o arquivo ilegível de lado.
#[test]
fn an_unreadable_config_is_set_aside_not_overwritten() {
    let tmp = tempfile::tempdir().unwrap();
    let paths = RouterPaths::with_app_support(Some(tmp.path().join("app")));
    std::fs::create_dir_all(&paths.base).unwrap();
    std::fs::write(paths.config_file(), b"{\"groups\": [ quebrado").unwrap();
    let creds = Arc::new(FakeStore::new());
    let adapter = Arc::new(FakeAdapter::new());

    let mut store = open(tmp.path(), &creds, &adapter);
    assert!(store.config().groups.is_empty());
    store.add_group("novo");

    let set_aside: Vec<PathBuf> = std::fs::read_dir(&paths.base)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.to_string_lossy().contains("config.unreadable-"))
        .collect();
    assert_eq!(set_aside.len(), 1, "{set_aside:?}");
    assert_eq!(
        std::fs::read(&set_aside[0]).unwrap(),
        b"{\"groups\": [ quebrado"
    );
    let reopened = open(tmp.path(), &creds, &adapter);
    assert_eq!(reopened.config().groups[0].name, "novo");
}

/// A casa que o router apaga é a que ele criou, sob `<base>\accounts\`. Um
/// `config.json` editado à mão apontando uma conta para outra pasta nunca faz o
/// router apagar essa pasta — só a credencial dela.
#[test]
fn removing_an_account_never_deletes_a_folder_outside_the_base() {
    let mut env = make_store();
    let group = env.store.add_group("g");
    let outside_dir = env.tmp.path().join("fora-da-base");
    std::fs::create_dir_all(&outside_dir).unwrap();
    let outside = ConfigDir::dedicated(outside_dir.to_string_lossy());
    let id = Id::new();
    env.seed_login("fora@exemplo.com", &outside);
    env.store.finish_pending_login(&outside, id, group.id);

    env.store.remove_account(id);

    assert!(outside_dir.exists(), "apagou pasta fora da base");
    assert!(env
        .creds
        .deleted(&env.adapter.credential_location(&outside)));
}

/// As sessões vivas são publicadas POR GRUPO — é o que mostra uma sessão que
/// devia estar num grupo e subiu no perfil errado. O leitor é injetado: no macOS
/// este caminho lia o `~/.claude/sessions` real durante os testes.
#[test]
fn live_sessions_are_published_per_group() {
    use router_core::engine::session_registry::{LiveSession, SessionStatus};

    let env = make_store();
    let tmp = env.tmp;
    let creds = env.creds;
    let adapter = env.adapter;
    let mut first = open(tmp.path(), &creds, &adapter);
    let busy = first.add_group("trabalho");
    let quiet = first.add_group("pessoal");
    drop(first);

    let busy_dir = busy.config_dir.clone();
    let store = open(tmp.path(), &creds, &adapter).with_session_reader(move |dir| {
        if *dir == busy_dir {
            vec![LiveSession {
                pid: 4242,
                session_id: None,
                cwd: "C:\\Users\\exemplo\\Projects\\app".into(),
                name: None,
                started_at: None,
                status: SessionStatus::Busy,
                status_updated_at: None,
                pid_domain: None,
            }]
        } else {
            Vec::new()
        }
    });

    assert_eq!(store.session_count(busy.id), 1);
    assert_eq!(store.session_count(quiet.id), 0);
    assert_eq!(store.live_sessions()[&busy.id][0].label(), "app");
}

/// Ativar a integração: scripts, status line em cada perfil de grupo e a linha
/// nos dois `$PROFILE` e no `.bashrc` — sem tocar em nada fora das pastas de
/// teste (a home e a Documentos são injetadas).
#[test]
fn installing_the_integration_writes_scripts_status_lines_and_profile_lines() {
    use router_core::engine::shell_integration::{ShellIntegration, ShellTargets, StatusShell};

    let mut env = make_store();
    let default_group = env.store.add_group("trabalho");
    let dedicated = env.store.add_group("pessoal");
    let router = env.tmp.path().join("app-instalado").join("router.exe");
    std::fs::create_dir_all(router.parent().unwrap()).unwrap();
    std::fs::write(&router, b"").unwrap();
    env.store.set_router_path(Some(router.clone()));
    let home = env.tmp.path().join("home");
    let targets = ShellTargets::for_home(&home, Some(&env.tmp.path().join("Documentos")));

    env.store
        .install_shell_integration(&targets, StatusShell::Bash)
        .unwrap();

    assert!(env.store.powershell_script_path().exists());
    assert!(env.store.bash_script_path().exists());
    for group in [&default_group, &dedicated] {
        let expected = env
            .store
            .expected_status_line(group, StatusShell::Bash)
            .unwrap();
        assert!(
            !ShellIntegration::status_line_is_stale(&expected, &group.config_dir),
            "status line faltando em {}",
            group.name
        );
    }
    let ps_line = ShellIntegration::powershell_source_line(&env.store.powershell_script_path());
    for profile in &targets.powershell_profiles {
        assert!(ShellIntegration::profile_has_line(profile, &ps_line));
    }
    let sh_line = ShellIntegration::bash_source_line(&env.store.bash_script_path());
    assert!(ShellIntegration::profile_has_line(
        &targets.bashrc,
        &sh_line
    ));
    assert!(home.join(".bash_profile").exists());
    assert!(!env.store.integration_is_stale(StatusShell::Bash));
}

/// O app reinstalado noutro lugar deixa script e status lines apontando para o
/// caminho morto — e `claude trabalho` cairia no `claude` puro, em silêncio. A
/// cura (na subida do app) reescreve tudo para o binário atual.
#[test]
fn a_moved_router_makes_the_integration_stale_and_healing_fixes_it() {
    use router_core::engine::shell_integration::{ShellTargets, StatusShell};

    let mut env = make_store();
    env.store.add_group("pessoal");
    let targets = ShellTargets::for_home(&env.tmp.path().join("home"), None);
    let old = env.tmp.path().join("antigo").join("router.exe");
    let new = env.tmp.path().join("novo").join("router.exe");
    env.store.set_router_path(Some(old));
    env.store
        .install_shell_integration(&targets, StatusShell::PowerShell)
        .unwrap();
    assert!(!env.store.integration_is_stale(StatusShell::PowerShell));

    env.store.set_router_path(Some(new.clone()));
    assert!(env.store.integration_is_stale(StatusShell::PowerShell));

    assert!(env
        .store
        .heal_shell_integration(&targets, StatusShell::PowerShell));
    assert!(!env.store.integration_is_stale(StatusShell::PowerShell));
    let script = std::fs::read_to_string(env.store.powershell_script_path()).unwrap();
    assert!(script.contains(&*new.to_string_lossy()));
}

/// A volta de rotação: primeiro o espelho (a casa da ativa recebe o token vivo
/// do grupo), depois a troca se a ativa passou do limiar.
#[test]
fn rotate_all_mirrors_then_rotates() {
    let mut env = make_store();
    let group = env.store.add_group("trabalho");
    let a = env.add_account("conta1@exemplo.com", group.id);
    let b = env.add_account("conta2@exemplo.com", group.id);
    env.store.activate(&a, &env.group(group.id));
    let group_location = env
        .adapter
        .credential_location(&env.group(group.id).config_dir);
    env.creds
        .write(&cred("cred-a-RENOVADO"), &group_location)
        .unwrap();
    let sample = GroupUsageSample::new(
        group.config_dir.raw.clone(),
        Some("conta1@exemplo.com".into()),
        Some(0.95),
        None,
        None,
        None,
        Utc::now(),
        None,
        UsageOrigin::Sensor,
    );
    GroupUsageStore::write(&sample, "conta1@exemplo.com", &env.paths.usage_dir()).unwrap();
    env.store.refresh_usage();

    env.store.rotate_all();

    assert_eq!(
        env.creds.read(&env.adapter.credential_location(&a.home)),
        Some(cred("cred-a-RENOVADO"))
    );
    assert_eq!(
        env.store.active_account(&env.group(group.id)).map(|x| x.id),
        Some(b.id)
    );
}
