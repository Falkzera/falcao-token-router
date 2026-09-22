//! O `router statusline` ponta a ponta, com perfil e base de rascunho (via
//! `CLAUDE_CONFIG_DIR` e `ROUTER_APP_SUPPORT`). Nenhuma conta real.
//!
//! Regressões do Windows: stdin que **nunca fecha** sai rápido (o leitor pega o
//! 1º JSON sem esperar EOF, e há prazo); amostra só é gravada se houver janela.

use std::io::Write;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

/// Escreve um `.claude.json` de perfil dedicado com a identidade dada.
fn write_identity(profile: &std::path::Path, email: &str) {
    std::fs::write(
        profile.join(".claude.json"),
        format!(r#"{{"oauthAccount":{{"emailAddress":"{email}"}}}}"#),
    )
    .unwrap();
}

fn usage_file(app: &std::path::Path, email: &str) -> std::path::PathBuf {
    app.join("com.synqo.falcao-router")
        .join("usage")
        .join(format!("{email}.json"))
}

#[test]
fn writes_sample_and_prints_line() {
    let profile = tempfile::tempdir().unwrap();
    let app = tempfile::tempdir().unwrap();
    write_identity(profile.path(), "conta1@exemplo.com");

    // resets_at em 2100 → janela válida; used_percentage inteiro.
    let json = r#"{"rate_limits":{"five_hour":{"used_percentage":42,"resets_at":4102444800},"seven_day":{"used_percentage":7,"resets_at":4102444800}}}"#;

    let assert = assert_cmd::Command::cargo_bin("router")
        .unwrap()
        .env("CLAUDE_CONFIG_DIR", profile.path())
        .env("ROUTER_APP_SUPPORT", app.path())
        .write_stdin(json)
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout).into_owned();
    assert!(stdout.contains("conta1"), "stdout: {stdout}");
    assert!(stdout.contains("5h"), "stdout: {stdout}");

    let data = std::fs::read_to_string(usage_file(app.path(), "conta1@exemplo.com")).unwrap();
    assert!(data.contains("\"fiveHourPercent\":0.42"), "{data}");
    assert!(data.contains("\"email\":\"conta1@exemplo.com\""), "{data}");
    assert!(data.contains("\"origin\":\"sensor\""), "{data}");
}

#[test]
fn no_rate_limits_writes_nothing() {
    let profile = tempfile::tempdir().unwrap();
    let app = tempfile::tempdir().unwrap();
    write_identity(profile.path(), "conta1@exemplo.com");

    assert_cmd::Command::cargo_bin("router")
        .unwrap()
        .env("CLAUDE_CONFIG_DIR", profile.path())
        .env("ROUTER_APP_SUPPORT", app.path())
        .write_stdin("{}")
        .assert()
        .success();

    let usage_dir = app.path().join("com.synqo.falcao-router").join("usage");
    let count = std::fs::read_dir(&usage_dir)
        .map(|d| d.count())
        .unwrap_or(0);
    assert_eq!(count, 0, "não devia gravar amostra sem janela");
}

/// A propriedade que o macOS nunca precisou: o stdin da status line pode nunca
/// fechar (medido: processos pendurados). O leitor pega o 1º JSON completo e sai
/// — nunca espera EOF.
#[test]
fn stdin_that_never_closes_exits_fast() {
    let profile = tempfile::tempdir().unwrap();
    let app = tempfile::tempdir().unwrap();
    write_identity(profile.path(), "conta1@exemplo.com");

    let bin = assert_cmd::cargo::cargo_bin("router");
    let mut child = Command::new(bin)
        .env("CLAUDE_CONFIG_DIR", profile.path())
        .env("ROUTER_APP_SUPPORT", app.path())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();

    let mut stdin = child.stdin.take().unwrap();
    let json = r#"{"rate_limits":{"five_hour":{"used_percentage":42,"resets_at":4102444800}}}"#;
    stdin.write_all(json.as_bytes()).unwrap();
    stdin.flush().unwrap();
    // NÃO fecha o stdin (não dropa): simula o pipe que nunca recebe EOF.

    let start = Instant::now();
    let status = loop {
        if let Some(s) = child.try_wait().unwrap() {
            break s;
        }
        if start.elapsed() > Duration::from_secs(3) {
            let _ = child.kill();
            panic!("statusline não saiu sem EOF");
        }
        std::thread::sleep(Duration::from_millis(20));
    };
    assert!(status.success());
    assert!(start.elapsed() < Duration::from_secs(3));
    drop(stdin);

    // E gravou a amostra mesmo sem EOF.
    assert!(usage_file(app.path(), "conta1@exemplo.com").exists());
}

/// Sem entrada nenhuma e stdin aberto: o prazo dispara e o processo sai (sem
/// gravar amostra, porque não houve janela).
#[test]
fn empty_stdin_that_never_closes_hits_deadline() {
    let profile = tempfile::tempdir().unwrap();
    let app = tempfile::tempdir().unwrap();
    write_identity(profile.path(), "conta1@exemplo.com");

    let bin = assert_cmd::cargo::cargo_bin("router");
    let mut child = Command::new(bin)
        .env("CLAUDE_CONFIG_DIR", profile.path())
        .env("ROUTER_APP_SUPPORT", app.path())
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();

    // Segura o stdin aberto e não escreve nada.
    let _stdin = child.stdin.take().unwrap();

    let start = Instant::now();
    let status = loop {
        if let Some(s) = child.try_wait().unwrap() {
            break s;
        }
        if start.elapsed() > Duration::from_secs(3) {
            let _ = child.kill();
            panic!("statusline não respeitou o prazo do stdin");
        }
        std::thread::sleep(Duration::from_millis(20));
    };
    assert!(status.success());
    let usage_dir = app.path().join("com.synqo.falcao-router").join("usage");
    let count = std::fs::read_dir(&usage_dir)
        .map(|d| d.count())
        .unwrap_or(0);
    assert_eq!(count, 0);
}
