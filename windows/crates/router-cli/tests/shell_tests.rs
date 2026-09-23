//! Os scripts de integração EXECUTADOS de verdade — não só o texto deles.
//!
//! `shell.ps1` no Windows PowerShell 5.1 (o mais exigente: lê arquivo sem BOM
//! como ANSI) e `shell.sh` no Git Bash com um `PATH` que só tem o System32 (prova
//! que o shim não depende de `grep`/`sed`). Em cada um: `claude <grupo>` chega ao
//! `router launch` com os argumentos certos — inclusive quando a função engole o
//! `--` —, o código de saída da sessão volta ao shell, o `claude` sem grupo cai
//! na função `claude` que o usuário JÁ tinha, e recarregar o script não entra em
//! recursão (a função do router não se guarda como "a anterior").

mod common;
use common::*;

use std::fs;
use std::path::PathBuf;

use router_core::engine::shell_integration::ShellIntegration;
use router_core::platform::git_bash::{find_git_bash, GitBashEnv};

fn write_scripts(w: &World) -> (PathBuf, PathBuf) {
    let base = w.sandbox.paths().base;
    let (ps1, sh) = (base.join("shell.ps1"), base.join("shell.sh"));
    ShellIntegration::write_scripts(&assert_cmd::cargo::cargo_bin("router"), &ps1, &sh).unwrap();
    (ps1, sh)
}

fn args_of_single_run(w: &World) -> serde_json::Value {
    let runs = w.sandbox.records();
    assert_eq!(runs.len(), 1, "o claude devia rodar uma vez só: {runs:?}");
    assert_eq!(
        runs[0]["claudeConfigDir"].as_str(),
        Some(w.group.config_dir.raw.as_str())
    );
    runs[0]["args"].clone()
}

#[test]
fn the_powershell_function_routes_groups_and_chains_the_previous_claude() {
    let w = world(1);
    let (ps1, _) = write_scripts(&w);
    let system = std::env::var("SystemRoot").unwrap_or_else(|_| r"C:\Windows".into());
    let powershell = PathBuf::from(system).join(r"System32\WindowsPowerShell\v1.0\powershell.exe");
    let test = w.sandbox.cwd.join("teste.ps1");
    fs::write(
        &test,
        format!(
            "function claude {{ 'anterior:' + ($args -join ',') }}\r\n\
             . '{ps1}'\r\n\
             claude trabalho -- --resume abc\r\n\
             $primeiro = $LASTEXITCODE\r\n\
             claude --version\r\n\
             . '{ps1}'\r\n\
             claude --help\r\n\
             exit $primeiro\r\n",
            ps1 = ps1.display()
        ),
    )
    .unwrap();

    let out = w
        .sandbox
        .command(&powershell)
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-ExecutionPolicy",
            "Bypass",
            "-File",
        ])
        .arg(&test)
        .env("FAKE_CLAUDE_EXIT", "3")
        .output()
        .unwrap();

    let text = stdout(&out);
    assert_eq!(out.status.code(), Some(3), "{text}\n{}", stderr(&out));
    assert!(text.contains("anterior:--version"), "{text}");
    assert!(
        text.contains("anterior:--help"),
        "recursão ao recarregar: {text}"
    );
    assert_eq!(
        args_of_single_run(&w),
        serde_json::json!(["--resume", "abc"])
    );
}

#[test]
fn the_bash_function_routes_groups_and_chains_the_previous_claude() {
    let Some(bash) = find_git_bash(&GitBashEnv::from_process()) else {
        eprintln!("sem Git Bash nesta máquina: teste pulado");
        return;
    };
    let w = world(1);
    let (_, sh) = write_scripts(&w);
    let sh = sh.to_string_lossy().replace('\\', "/");
    let test = w.sandbox.cwd.join("teste.sh");
    fs::write(
        &test,
        format!(
            "claude() {{ echo \"anterior:$*\"; }}\n\
             . '{sh}'\n\
             claude trabalho --resume abc\n\
             primeiro=$?\n\
             claude --version\n\
             . '{sh}'\n\
             claude --help\n\
             exit $primeiro\n"
        ),
    )
    .unwrap();

    let out = w
        .sandbox
        .command(&bash)
        .arg(test.to_string_lossy().replace('\\', "/"))
        .env("FAKE_CLAUDE_EXIT", "3")
        .output()
        .unwrap();

    let text = stdout(&out);
    assert_eq!(out.status.code(), Some(3), "{text}\n{}", stderr(&out));
    assert!(text.contains("anterior:--version"), "{text}");
    assert!(
        text.contains("anterior:--help"),
        "recursão ao recarregar: {text}"
    );
    assert_eq!(
        args_of_single_run(&w),
        serde_json::json!(["--resume", "abc"])
    );
}

/// O `router.exe` sumiu (app desinstalado ou movido): a função avisa ALTO e o
/// `claude` segue para a função anterior — em vez de cair em silêncio no
/// `claude` puro, na conta errada, como no macOS.
#[test]
fn a_missing_router_is_announced_loudly() {
    let w = world(1);
    let base = w.sandbox.paths().base;
    let (ps1, sh) = (base.join("shell.ps1"), base.join("shell.sh"));
    let gone = w.sandbox.cwd.join("sumiu").join("router.exe");
    ShellIntegration::write_scripts(&gone, &ps1, &sh).unwrap();
    let system = std::env::var("SystemRoot").unwrap_or_else(|_| r"C:\Windows".into());
    let powershell = PathBuf::from(system).join(r"System32\WindowsPowerShell\v1.0\powershell.exe");
    let test = w.sandbox.cwd.join("teste.ps1");
    fs::write(
        &test,
        format!(
            "function claude {{ 'anterior:' + ($args -join ',') }}\r\n. '{}'\r\nclaude trabalho 6>&1\r\n",
            ps1.display()
        ),
    )
    .unwrap();

    let out = w
        .sandbox
        .command(&powershell)
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-ExecutionPolicy",
            "Bypass",
            "-File",
        ])
        .arg(&test)
        .output()
        .unwrap();

    let text = stdout(&out);
    assert!(text.contains("router.exe não encontrado"), "{text}");
    assert!(text.contains("anterior:trabalho"), "{text}");
    assert!(
        w.sandbox.records().is_empty(),
        "rodou o claude pelo router sumido"
    );
}
