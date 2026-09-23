//! A CLI `router` ponta a ponta — `is-group`, `launch`, `rotate`, `measure`,
//! `doctor` e o `--profile` do sensor —, com um `claude` de mentira
//! (`fake-claude`, achado por `ROUTER_CLAUDE_BIN`) e tudo num sandbox: base do
//! router (`ROUTER_APP_SUPPORT`), home (`USERPROFILE`), `APPDATA` e um `PATH`
//! mínimo, para o `claude` de verdade desta máquina nunca ser achado. Nenhuma
//! conta real; nenhum toque em `%USERPROFILE%\.claude`.

mod common;
use common::*;

use std::fs;

use router_core::engine::group_usage::{GroupUsageStore, UsageOrigin};
use router_core::platform::links::is_junction;
use serde_json::Value;

// --- is-group ---

/// A função de shell só quer o código: nada no stdout nem no stderr.
#[test]
fn is_group_answers_with_the_exit_code_only() {
    let w = world(1);
    for (name, expected) in [(" TRABALHO ", Some(0)), ("pessoal", Some(1))] {
        let out = w.sandbox.run(&["is-group", name]);
        assert_eq!(out.status.code(), expected, "{name}");
        assert!(out.stdout.is_empty() && out.stderr.is_empty());
    }
    let empty = Sandbox::new();
    assert_eq!(empty.run(&["is-group", "trabalho"]).status.code(), Some(1));
}

// --- launch ---

#[test]
fn launch_activates_the_account_and_runs_claude_in_the_group_profile() {
    let w = world(2);
    fs::create_dir_all(w.sandbox.home.join(".claude").join("projects")).unwrap();

    let out = w
        .sandbox
        .router(&["launch", "trabalho", "--", "--resume", "abc"])
        .env("FAKE_CLAUDE_EXIT", "7")
        .env("HTTPS_PROXY", "http://127.0.0.1:3456")
        .env("anthropic_api_key", "sk-falsa")
        .env("CLAUDE_SECURESTORAGE_CONFIG_DIR", r"C:\Users\exemplo\outra")
        .env("ANTHROPIC_MODEL", "claude-opus-5")
        .output()
        .unwrap();

    // O código de saída do claude é repassado.
    assert_eq!(out.status.code(), Some(7), "{}", stderr(&out));
    assert!(
        stderr(&out).contains("→ Trabalho: conta1"),
        "{}",
        stderr(&out)
    );

    let runs = w.sandbox.records();
    assert_eq!(runs.len(), 1);
    assert_eq!(runs[0]["args"], serde_json::json!(["--resume", "abc"]));
    assert_eq!(
        runs[0]["claudeConfigDir"].as_str(),
        Some(w.group.config_dir.raw.as_str())
    );
    let present: Vec<&str> = runs[0]["present"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(Value::as_str)
        .collect();
    for gone in [
        "HTTPS_PROXY",
        "ANTHROPIC_API_KEY",
        "CLAUDE_SECURESTORAGE_CONFIG_DIR",
    ] {
        assert!(!present.contains(&gone), "{gone} chegou ao claude");
    }
    assert!(
        present.contains(&"ANTHROPIC_MODEL"),
        "a escolha de modelo sumiu"
    );

    // A troca: credencial da casa no grupo, identidade e onboarding.
    assert_eq!(
        fs::read_to_string(w.group_dir().join(".credentials.json")).unwrap(),
        blob("1")
    );
    assert_eq!(w.group_identity().as_deref(), Some("conta1@exemplo.com"));
    // O sensor plantado no perfil do grupo, com o perfil embutido.
    let settings: Value =
        serde_json::from_slice(&fs::read(w.group_dir().join("settings.json")).unwrap()).unwrap();
    let command = settings["statusLine"]["command"].as_str().unwrap();
    assert!(command.contains("statusline --profile"), "{command}");
    // E o histórico compartilhado com o `~\.claude` do sandbox.
    assert!(is_junction(&w.group_dir().join("projects")));
}

/// Uma função do PowerShell engole o `--` do `$args`: sem ele, tudo depois do
/// grupo vai para o claude.
#[test]
fn launch_accepts_arguments_without_the_double_dash() {
    let w = world(1);
    let out = w.sandbox.run(&["launch", "trabalho", "--resume", "abc"]);
    assert_eq!(out.status.code(), Some(0), "{}", stderr(&out));
    assert_eq!(
        w.sandbox.records()[0]["args"],
        serde_json::json!(["--resume", "abc"])
    );
}

#[test]
fn launch_names_an_unknown_group_and_a_missing_config() {
    let w = world(1);
    let out = w.sandbox.run(&["launch", "pessoal"]);
    assert_eq!(out.status.code(), Some(1));
    assert!(stderr(&out).contains("router: grupo desconhecido: pessoal"));

    let empty = Sandbox::new();
    let out = empty.run(&["launch", "trabalho"]);
    assert_eq!(out.status.code(), Some(1));
    assert!(stderr(&out).contains("configuração não encontrada — crie um grupo no app"));
    assert!(empty.records().is_empty(), "rodou o claude sem config");
}

/// A ativa passou do limiar: o próximo `claude trabalho` sobe na seguinte.
#[test]
fn launch_moves_away_from_a_full_account() {
    let w = world(2);
    w.sandbox.run(&["launch", "trabalho"]);
    assert_eq!(w.group_identity().as_deref(), Some("conta1@exemplo.com"));
    w.write_sample("conta1@exemplo.com", 0.95);

    let out = w.sandbox.run(&["launch", "trabalho"]);

    assert!(
        stderr(&out).contains("→ Trabalho: conta2"),
        "{}",
        stderr(&out)
    );
    assert_eq!(w.group_identity().as_deref(), Some("conta2@exemplo.com"));
    assert_eq!(
        fs::read_to_string(w.group_dir().join(".credentials.json")).unwrap(),
        blob("2")
    );
}

// --- rotate ---

#[test]
fn rotate_switches_a_group_past_its_threshold_silently() {
    let w = world(2);
    w.sandbox.run(&["launch", "trabalho"]);
    // O Claude Code renova o token no grupo enquanto a conta 1 serve.
    fs::write(w.group_dir().join(".credentials.json"), blob("1-renovado")).unwrap();
    w.write_sample("conta1@exemplo.com", 0.95);

    let out = w.sandbox.run(&["rotate"]);

    assert_eq!(out.status.code(), Some(0));
    assert!(out.stdout.is_empty() && out.stderr.is_empty());
    assert_eq!(w.group_identity().as_deref(), Some("conta2@exemplo.com"));
    // Espelhou antes de trocar: a casa da 1 tem o token renovado.
    assert_eq!(
        fs::read_to_string(w.accounts[0].home.path().join(".credentials.json")).unwrap(),
        blob("1-renovado")
    );
}

// --- measure ---

#[test]
fn measure_reports_each_account_and_records_probe_samples() {
    let w = world(2);
    // A conta 2 perdeu o login (o fake responde "deslogado" sem credencial).
    fs::remove_file(w.accounts[1].home.path().join(".credentials.json")).unwrap();

    let out = w
        .sandbox
        .router(&["measure"])
        .env("CLAUDECODE", "1")
        .env("CLAUDE_CODE_ENTRYPOINT", "cli")
        .output()
        .unwrap();

    assert_eq!(out.status.code(), Some(1), "{}", stdout(&out));
    let text = stdout(&out);
    assert!(
        text.contains("  conta1: 5h 12%  7d 34%  Fable 56%"),
        "{text}"
    );
    assert!(
        text.contains("  conta2: sem login neste perfil — use Relogar no app"),
        "{text}"
    );

    let sample =
        GroupUsageStore::read("conta1@exemplo.com", &w.sandbox.paths().usage_dir()).unwrap();
    assert_eq!(sample.origin(), UsageOrigin::Probe);
    assert_eq!(sample.models.unwrap().windows[0].name, "Fable");

    let runs = w.sandbox.records();
    assert_eq!(
        runs[0]["args"],
        serde_json::json!([
            "--print",
            "--no-session-persistence",
            "--strict-mcp-config",
            "/usage"
        ])
    );
    assert!(runs[0]["cwd"].as_str().unwrap().ends_with("probe-scratch"));
    let present = runs[0]["present"].to_string();
    assert!(!present.contains("CLAUDECODE") && !present.contains("CLAUDE_CODE_ENTRYPOINT"));
}

/// A regra que protege a sessão viva: a conta ATIVA é medida pelo perfil do
/// grupo; a ociosa, pela casa.
#[test]
fn measure_probes_the_active_account_through_the_group() {
    let w = world(2);
    w.sandbox.run(&["launch", "trabalho"]);
    fs::remove_file(&w.sandbox.record).unwrap();

    w.sandbox.run(&["measure", "trabalho"]);

    let dirs: Vec<String> = w
        .sandbox
        .records()
        .iter()
        .map(|r| r["claudeConfigDir"].as_str().unwrap().to_string())
        .collect();
    assert_eq!(dirs.len(), 2);
    assert!(dirs.contains(&w.group.config_dir.raw), "{dirs:?}");
    assert!(dirs.contains(&w.accounts[1].home.raw), "{dirs:?}");
    assert!(
        !dirs.contains(&w.accounts[0].home.raw),
        "sondou a casa da ativa"
    );
}

#[test]
fn measure_names_an_unknown_group_and_an_empty_one() {
    let w = world(1);
    let out = w.sandbox.run(&["measure", "pessoal"]);
    assert_eq!(out.status.code(), Some(1));
    assert!(stderr(&out).contains("router: grupo desconhecido: pessoal"));

    let empty = world(0);
    let out = empty.sandbox.run(&["measure"]);
    assert_eq!(out.status.code(), Some(0));
    assert!(stdout(&out).contains("nenhuma conta para medir."));
}

// --- statusline ---

/// O sensor cai no `--profile` embutido quando o `CLAUDE_CONFIG_DIR` não chega
/// (subprocesso com o ambiente raspado).
#[test]
fn the_sensor_falls_back_to_the_embedded_profile() {
    let sandbox = Sandbox::new();
    let profile = sandbox.home.join("grupo");
    fs::create_dir_all(&profile).unwrap();
    fs::write(
        profile.join(".claude.json"),
        r#"{"oauthAccount":{"emailAddress":"conta1@exemplo.com"}}"#,
    )
    .unwrap();

    let mut child = sandbox
        .router(&["statusline", "--profile", &profile.to_string_lossy()])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    {
        use std::io::Write;
        let mut stdin = child.stdin.take().unwrap();
        stdin
            .write_all(
                br#"{"rate_limits":{"five_hour":{"used_percentage":42,"resets_at":4102444800}}}"#,
            )
            .unwrap();
    }
    assert!(child.wait().unwrap().success());

    let sample = GroupUsageStore::read("conta1@exemplo.com", &sandbox.paths().usage_dir()).unwrap();
    assert_eq!(sample.config_dir_raw, profile.to_string_lossy());
}

/// A linha do grupo, do `router.exe` de verdade: o grupo dono do perfil (do
/// `config.json`), modelo e esforço, o branch da pasta da sessão, o contexto, a
/// janela, o custo e o e-mail da conta ativa — e a amostra gravada como antes.
#[test]
fn the_status_line_names_the_group_and_shows_the_session() {
    let w = world(1);
    let group_dir = w.group_dir();
    fs::create_dir_all(&group_dir).unwrap();
    fs::write(
        w.group.config_dir.global_config_path(),
        r#"{"oauthAccount":{"emailAddress":"conta1@exemplo.com"}}"#,
    )
    .unwrap();
    let repo = w.sandbox.cwd.join("repo");
    fs::create_dir_all(repo.join(".git")).unwrap();
    fs::write(repo.join(".git").join("HEAD"), "ref: refs/heads/feat/x\n").unwrap();
    let input = serde_json::json!({
        "model": {"id": "claude-opus-5-5", "display_name": "Opus 5.5 (1M context)"},
        "effort": {"level": "high"},
        "workspace": {"current_dir": repo.to_string_lossy()},
        "context_window": {
            "total_input_tokens": 20000, "context_window_size": 200000, "used_percentage": 10
        },
        "cost": {"total_cost_usd": 0.5},
        "rate_limits": {"five_hour": {"used_percentage": 42, "resets_at": 4102444800u64}}
    });

    let mut child = w
        .sandbox
        .router(&["statusline"])
        .env("CLAUDE_CONFIG_DIR", &group_dir)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    {
        use std::io::Write;
        let mut stdin = child.stdin.take().unwrap();
        stdin.write_all(input.to_string().as_bytes()).unwrap();
    }
    let output = child.wait_with_output().unwrap();
    assert!(output.status.success());

    let ansi = regex::Regex::new("\x1b\\[[0-9;]*m").unwrap();
    let line = ansi.replace_all(&stdout(&output), "").into_owned();
    for part in [
        "● Trabalho │ ",
        "Opus 5.5 high",
        " feat/x ",
        "10% 20k/200k",
        "5h ",
        " 42% ↻ ",
        "$0.50",
        "conta1@exemplo.com",
    ] {
        assert!(line.contains(part), "{part:?} em {line:?}");
    }
    let sample =
        GroupUsageStore::read("conta1@exemplo.com", &w.sandbox.paths().usage_dir()).unwrap();
    assert_eq!(sample.five_hour_percent, Some(0.42));
}

// --- doctor ---

/// O `doctor` diz o estado sem mexer em nada — e o sensor do grupo roda DE
/// VERDADE pelo shell que o Claude Code usaria (aspas e caminhos à prova).
#[test]
fn doctor_reports_the_state_of_the_installation() {
    let w = world(1);
    w.sandbox.run(&["launch", "trabalho"]);

    let out = w.sandbox.run(&["doctor"]);
    let text = stdout(&out);
    println!("{text}"); // capturado quando passa; aparece quando falha

    assert!(text.starts_with("router doctor"), "{text}");
    assert!(
        text.contains("ok  config: 1 grupo(s), 1 conta(s)"),
        "{text}"
    );
    assert!(text.contains("!!  shell.ps1 ausente"), "{text}");
    assert!(
        text.contains("sensor no grupo Trabalho: instalado e rodando pelo"),
        "{text}"
    );
    assert!(
        text.contains("grupo Trabalho: conta1 — sem amostra ainda (pronta)"),
        "{text}"
    );
    assert!(text.contains("(2.1.280 (Claude Code))"), "{text}");
    assert!(text.trim_end().ends_with("há problemas acima."), "{text}");
    assert_eq!(out.status.code(), Some(1));
}

fn write_choice(w: &World, choice: serde_json::Value) {
    let base = w.sandbox.paths().base;
    fs::create_dir_all(&base).unwrap();
    fs::write(base.join("statusline.json"), choice.to_string()).unwrap();
}

/// Com todos os itens tirados a linha é vazia — e o sensor continua rodando:
/// o `doctor` confere que ele roda, não o que ele desenha.
#[test]
fn doctor_accepts_an_empty_line_and_names_the_choice() {
    let w = world(1);
    w.sandbox.run(&["launch", "trabalho"]);

    let text = stdout(&w.sandbox.run(&["doctor"]));
    assert!(
        text.contains("ok  status line: a linha do app, completa"),
        "{text}"
    );

    let all = [
        "group", "model", "effort", "place", "context", "fiveHour", "sevenDay", "resets", "cost",
        "email",
    ];
    write_choice(&w, serde_json::json!({"hidden": all}));
    let text = stdout(&w.sandbox.run(&["doctor"]));
    assert!(
        text.contains("ok  sensor no grupo Trabalho: instalado e rodando pelo"),
        "{text}"
    );
    assert!(
        text.contains("ok  status line: a linha do app, sem nenhum item (vazia)"),
        "{text}"
    );

    write_choice(&w, serde_json::json!({"hidden": ["context", "cost"]}));
    let text = stdout(&w.sandbox.run(&["doctor"]));
    assert!(
        text.contains("ok  status line: a linha do app, sem contexto, custo"),
        "{text}"
    );
}

/// No modo comando, o `doctor` roda o comando do usuário com a sessão de
/// exemplo — e diz quando ele não imprime (as sessões mostram a linha do app).
#[test]
fn doctor_tries_the_users_command() {
    let w = world(1);
    w.sandbox.run(&["launch", "trabalho"]);
    let echo = format!(
        "{} statusline-echo",
        fake_claude().to_string_lossy().replace('\\', "/")
    );
    write_choice(&w, serde_json::json!({"mode": "command", "command": echo}));
    let text = stdout(&w.sandbox.run(&["doctor"]));
    assert!(
        text.contains("ok  sensor no grupo Trabalho: instalado e rodando pelo"),
        "{text}"
    );
    assert!(
        text.contains("ok  status line: o seu comando imprime"),
        "{text}"
    );

    write_choice(
        &w,
        serde_json::json!({"mode": "command", "command": "C:/nao/existe/linha.exe"}),
    );
    let text = stdout(&w.sandbox.run(&["doctor"]));
    assert!(text.contains("!!  status line: o seu comando"), "{text}");
    assert!(text.contains("as sessões mostram a linha do app"), "{text}");
}

#[test]
fn an_unknown_command_prints_the_usage() {
    let out = Sandbox::new().run(&["xyz"]);
    assert_eq!(out.status.code(), Some(2));
    assert!(stderr(&out).contains("uso: router [statusline|launch <grupo>"));
}

/// Não sobra nada fora do sandbox: a guarda do perfil padrão não foi acionada
/// (nenhum grupo padrão) e nenhum `.claude` foi criado na home de mentira.
#[test]
fn a_dedicated_group_never_touches_the_default_profile() {
    let w = world(1);
    w.sandbox.run(&["launch", "trabalho"]);
    assert!(!w
        .sandbox
        .home
        .join(".claude")
        .join(".credentials.json")
        .exists());
    assert!(!w.sandbox.home.join(".claude.json").exists());
    assert!(!w.sandbox.paths().base.join("backups").exists());
}
