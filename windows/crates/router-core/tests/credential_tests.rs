//! A credencial em arquivo (≙ `KeychainStore` do macOS).
//!
//! No Windows o Claude Code guarda a credencial de um perfil num ARQUIVO,
//! `<perfil>\.credentials.json` (nada no Credential Manager — conferido no spike
//! de 22/09/2026). O "chaveiro" do porte é uma cópia de arquivo, e as invariantes
//! não relaxam: o blob é opaco (nunca decodificado), só se propaga blob completo,
//! toda escrita sai com mtime novo, e o que não mudou não é regravado.
//!
//! Os quatro testes de hash de item de chaveiro do macOS (`ConfigDirTests`) não
//! se aplicam aqui — no Windows não há hash, a credencial mora dentro do perfil.
//! Em seu lugar, os testes de onde ela mora.

use std::fs::{self, OpenOptions};
use std::thread;
use std::time::{Duration, SystemTime};

use router_core::engine::anthropic_adapter::AnthropicAdapter;
use router_core::engine::credential_store::{
    CredentialBlob, CredentialError, CredentialStore, FileCredentialStore,
};
use router_core::engine::default_profile_guard::DefaultProfileGuard;
use router_core::engine::provider::ProviderAdapter;
use router_core::ConfigDir;

/// Um blob com a forma do real (`claudeAiOauth` + `mcpOAuth`), valores falsos.
fn blob(tag: &str) -> CredentialBlob {
    CredentialBlob::from_bytes(
        format!(
            r#"{{"claudeAiOauth":{{"accessToken":"falso-{tag}","refreshToken":"falso-{tag}","expiresAt":1,"scopes":["user:inference"]}},"mcpOAuth":{{}}}}"#
        )
        .into_bytes(),
    )
}

fn long_ago() -> SystemTime {
    SystemTime::UNIX_EPOCH + Duration::from_secs(1_000_000_000)
}

fn stamp_long_ago(path: &std::path::Path) {
    OpenOptions::new()
        .write(true)
        .open(path)
        .unwrap()
        .set_modified(long_ago())
        .unwrap();
}

fn mtime(path: &std::path::Path) -> SystemTime {
    fs::metadata(path).unwrap().modified().unwrap()
}

// --- Onde mora (substitui os testes de hash do macOS) ---

#[test]
fn default_profile_credential_lives_inside_dot_claude() {
    let dir = ConfigDir::standard("C:/Users/exemplo");
    assert_eq!(
        AnthropicAdapter.credential_location(&dir),
        std::path::Path::new("C:/Users/exemplo/.claude").join(".credentials.json")
    );
}

#[test]
fn dedicated_profile_credential_lives_inside_the_profile() {
    let dir = ConfigDir::dedicated("C:/Users/exemplo/AppData/Local/grupo");
    assert_eq!(
        AnthropicAdapter.credential_location(&dir),
        std::path::Path::new("C:/Users/exemplo/AppData/Local/grupo").join(".credentials.json")
    );
}

// --- O blob é opaco ---

/// Checagem ESTRUTURAL, sem ler valor nenhum: JSON completo cujo objeto raiz
/// tem `claudeAiOauth` como objeto. É o que impede propagar um arquivo pego no
/// meio de uma escrita (o Claude Code tem um braço de escrita in-place).
#[test]
fn blob_is_complete_only_with_a_claude_ai_oauth_object() {
    let complete = |s: &str| CredentialBlob::from_bytes(s.as_bytes().to_vec()).is_complete();
    assert!(complete(r#"{"claudeAiOauth":{"a":1},"mcpOAuth":{}}"#));
    assert!(complete("  {\"claudeAiOauth\":{}}\r\n"));
    // Truncado no meio de um valor.
    assert!(!complete(r#"{"claudeAiOauth":{"accessToken":"abc"#));
    // Lixo depois do JSON.
    assert!(!complete(r#"{"claudeAiOauth":{}} x"#));
    // Sem a conta, ou com ela em outro formato.
    assert!(!complete(r#"{"mcpOAuth":{}}"#));
    assert!(!complete(r#"{"claudeAiOauth":"texto"}"#));
    assert!(!complete(r#"{"claudeAiOauth":null}"#));
    assert!(!complete("[]"));
    assert!(!complete(""));
}

/// Um `{:?}` descuidado num log nunca pode imprimir a credencial.
#[test]
fn blob_debug_never_shows_the_bytes() {
    let shown = format!("{:?}", blob("segredo"));
    assert!(!shown.contains("segredo"), "{shown}");
    assert!(!shown.contains("claudeAiOauth"), "{shown}");
}

// --- O arquivo ---

#[test]
fn store_reads_back_what_it_wrote() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join(".credentials.json");
    let store = FileCredentialStore::new();

    store.write(&blob("a"), &path).unwrap();

    assert_eq!(store.read(&path), Some(blob("a")));
    assert!(store.exists(&path));
}

/// O gatilho da troca a quente: o Claude Code só relê a credencial quando o
/// mtime muda. Toda escrita tem de sair com mtime novo — nunca herdado.
#[test]
fn store_write_gives_a_fresh_mtime() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join(".credentials.json");
    fs::write(&path, blob("antigo").as_bytes()).unwrap();
    stamp_long_ago(&path);

    FileCredentialStore::new()
        .write(&blob("novo"), &path)
        .unwrap();

    assert!(mtime(&path) > long_ago() + Duration::from_secs(3600));
    assert_eq!(fs::read(&path).unwrap(), blob("novo").as_bytes());
}

/// O que não mudou não é regravado: um mtime novo sem motivo faria toda sessão
/// viva do perfil descartar o cache e reler à toa.
#[test]
fn store_does_not_rewrite_identical_bytes() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join(".credentials.json");
    fs::write(&path, blob("a").as_bytes()).unwrap();
    stamp_long_ago(&path);

    FileCredentialStore::new().write(&blob("a"), &path).unwrap();

    assert_eq!(mtime(&path), long_ago(), "regravou bytes idênticos");
}

#[test]
fn store_refuses_to_write_an_incomplete_blob() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join(".credentials.json");
    fs::write(&path, blob("bom").as_bytes()).unwrap();

    let meio = CredentialBlob::from_bytes(br#"{"claudeAiOauth":{"accessT"#.to_vec());
    let err = FileCredentialStore::new().write(&meio, &path).unwrap_err();

    assert!(matches!(err, CredentialError::Incomplete), "{err:?}");
    assert_eq!(fs::read(&path).unwrap(), blob("bom").as_bytes());
}

/// Um arquivo pego no meio de uma escrita não é credencial: a leitura não o
/// devolve (e o motor, sem leitura, não espelha nada).
#[test]
fn store_never_returns_an_incomplete_file() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join(".credentials.json");
    fs::write(&path, br#"{"claudeAiOauth":{"access"#).unwrap();

    let store = FileCredentialStore::new();
    assert_eq!(store.read(&path), None);
    assert!(!store.exists(&path));
}

/// ...mas espera um pouco: se a escrita do Claude Code termina logo, a leitura
/// pega o arquivo completo em vez de desistir.
#[test]
fn store_read_waits_for_a_write_in_progress() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join(".credentials.json");
    fs::write(&path, br#"{"claudeAiOauth":{"access"#).unwrap();

    let finisher = {
        let path = path.clone();
        thread::spawn(move || {
            thread::sleep(Duration::from_millis(60));
            fs::write(&path, blob("pronto").as_bytes()).unwrap();
        })
    };
    let read = FileCredentialStore::new().read(&path);
    finisher.join().unwrap();

    assert_eq!(read, Some(blob("pronto")));
}

#[test]
fn store_delete_removes_and_is_silent_when_absent() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join(".credentials.json");
    let store = FileCredentialStore::new();
    store.write(&blob("a"), &path).unwrap();

    store.delete(&path);
    assert!(!path.exists());
    assert_eq!(store.read(&path), None);
    store.delete(&path); // de novo: nada acontece
}

// --- A guarda do perfil padrão ---

/// A primeira vez que o router escreve no perfil padrão (`~\.claude`), guarda
/// antes o login que estava lá — credencial e `oauthAccount`. Um grupo padrão
/// pode ser criado numa máquina onde o `claude` puro já tem uma conta que o
/// router não conhece; sem esta cópia, a primeira ativação a perderia.
#[test]
fn guard_backs_up_the_default_profile_once_before_the_first_write() {
    let home = tempfile::tempdir().unwrap();
    let base = tempfile::tempdir().unwrap();
    let default = ConfigDir::standard(&home.path().to_string_lossy());
    let cred = AnthropicAdapter.credential_location(&default);
    fs::create_dir_all(cred.parent().unwrap()).unwrap();
    fs::write(&cred, blob("de-antes").as_bytes()).unwrap();
    fs::write(
        default.global_config_path(),
        br#"{"oauthAccount":{"emailAddress":"antes@exemplo.com"},"outra":1}"#,
    )
    .unwrap();

    let store = FileCredentialStore::with_guard(DefaultProfileGuard::new(
        cred.clone(),
        default.global_config_path(),
        base.path(),
    ));
    store.write(&blob("nova"), &cred).unwrap();
    store.write(&blob("mais-nova"), &cred).unwrap();

    let backups: Vec<_> = fs::read_dir(base.path().join("backups"))
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    assert_eq!(backups.len(), 1, "uma cópia só: {backups:?}");
    let saved = &backups[0];
    assert_eq!(
        fs::read(saved.join(".credentials.json")).unwrap(),
        blob("de-antes").as_bytes()
    );
    let account: serde_json::Value =
        serde_json::from_slice(&fs::read(saved.join("oauthAccount.json")).unwrap()).unwrap();
    assert_eq!(account["emailAddress"], "antes@exemplo.com");
    assert_eq!(fs::read(&cred).unwrap(), blob("mais-nova").as_bytes());
}

#[test]
fn guard_ignores_dedicated_profiles() {
    let home = tempfile::tempdir().unwrap();
    let base = tempfile::tempdir().unwrap();
    let default = ConfigDir::standard(&home.path().to_string_lossy());
    let store = FileCredentialStore::with_guard(DefaultProfileGuard::new(
        AnthropicAdapter.credential_location(&default),
        default.global_config_path(),
        base.path(),
    ));
    let group = base
        .path()
        .join("groups")
        .join("G")
        .join(".credentials.json");

    store.write(&blob("a"), &group).unwrap();

    assert!(!base.path().join("backups").exists());
}
