//! Onde está o binário oficial `claude` — UM resolvedor só (o macOS tem três
//! buscas diferentes espalhadas).
//!
//! Ordem: `ROUTER_CLAUDE_BIN` (override explícito) → o instalador nativo
//! (`%USERPROFILE%\.local\bin\claude.exe`) → o `PATH` → o npm global
//! (`%APPDATA%\npm`). O shim `claude.cmd` do npm é lido e vira `node` + `cli.js`
//! (ou o `.exe` do pacote), para nunca passar pelo `cmd.exe`. A cópia do Claude
//! Desktop e os aliases do WindowsApps nunca são escolhidos: não usam a
//! credencial do perfil que o router gerencia.

use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};

use router_core::usage::claude_binary::{ClaudeBinary, LocateEnv};

fn touch(path: &Path) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, b"").unwrap();
}

fn path_of(dirs: &[&Path]) -> OsString {
    std::env::join_paths(dirs).unwrap()
}

/// O shim que o `npm install -g` gera no Windows (cmd-shim), com o alvo dado.
fn npm_shim(target: &str) -> String {
    format!(
        "@ECHO off\r\nGOTO start\r\n:find_dp0\r\nSET dp0=%~dp0\r\nEXIT /b\r\n:start\r\nSETLOCAL\r\nCALL :find_dp0\r\n\r\nIF EXIST \"%dp0%\\node.exe\" (\r\n  SET \"_prog=%dp0%\\node.exe\"\r\n) ELSE (\r\n  SET \"_prog=node\"\r\n  SET PATHEXT=%PATHEXT:;.JS;=;%\r\n)\r\n\r\nendLocal & goto #_undefined_# 2>NUL || title %COMSPEC% & \"%_prog%\"  \"%dp0%\\{target}\" %*\r\n"
    )
}

fn env_with(user_profile: &Path, path: OsString) -> LocateEnv {
    LocateEnv {
        override_bin: None,
        user_profile: Some(user_profile.to_path_buf()),
        path: Some(path),
        appdata: None,
    }
}

#[test]
fn an_explicit_override_wins() {
    let tmp = tempfile::tempdir().unwrap();
    let fake = tmp.path().join("teste").join("fake-claude.exe");
    touch(&fake);
    touch(&tmp.path().join(".local").join("bin").join("claude.exe"));
    let mut env = env_with(tmp.path(), OsString::new());
    env.override_bin = Some(fake.clone());

    let found = ClaudeBinary::locate_in(&env).unwrap();
    assert_eq!(found.program, fake);
    assert!(found.prefix_args.is_empty());
}

#[test]
fn the_native_installer_comes_before_the_path() {
    let tmp = tempfile::tempdir().unwrap();
    let native = tmp.path().join(".local").join("bin").join("claude.exe");
    touch(&native);
    let other = tmp.path().join("outro");
    touch(&other.join("claude.exe"));

    let found = ClaudeBinary::locate_in(&env_with(tmp.path(), path_of(&[&other]))).unwrap();
    assert_eq!(found.program, native);
}

#[test]
fn falls_back_to_the_path() {
    let tmp = tempfile::tempdir().unwrap();
    let bin = tmp.path().join("ferramentas");
    touch(&bin.join("claude.exe"));

    let found = ClaudeBinary::locate_in(&env_with(tmp.path(), path_of(&[&bin]))).unwrap();
    assert_eq!(found.program, bin.join("claude.exe"));
}

/// `claude.cmd` do npm: roda `node` + `cli.js` direto. Passar pelo `cmd.exe`
/// reinterpretaria `&`, `|` e `%` nos argumentos do usuário.
#[test]
fn an_npm_shim_runs_node_with_cli_js_without_cmd_exe() {
    let tmp = tempfile::tempdir().unwrap();
    let npm = tmp.path().join("npm");
    fs::create_dir_all(&npm).unwrap();
    fs::write(
        npm.join("claude.cmd"),
        npm_shim(r"node_modules\@anthropic-ai\claude-code\cli.js"),
    )
    .unwrap();
    touch(&npm.join("node.exe"));
    let cli = npm
        .join("node_modules")
        .join("@anthropic-ai")
        .join("claude-code")
        .join("cli.js");
    touch(&cli);

    let found = ClaudeBinary::locate_in(&env_with(tmp.path(), path_of(&[&npm]))).unwrap();
    assert_eq!(found.program, npm.join("node.exe"));
    assert_eq!(found.prefix_args, vec![PathBuf::from(&cli)]);
}

#[test]
fn an_npm_shim_without_a_bundled_node_uses_node_from_the_path() {
    let tmp = tempfile::tempdir().unwrap();
    let npm = tmp.path().join("npm");
    let nodejs = tmp.path().join("nodejs");
    fs::create_dir_all(&npm).unwrap();
    fs::write(
        npm.join("claude.cmd"),
        npm_shim(r"node_modules\@anthropic-ai\claude-code\cli.js"),
    )
    .unwrap();
    touch(
        &npm.join("node_modules")
            .join("@anthropic-ai")
            .join("claude-code")
            .join("cli.js"),
    );
    touch(&nodejs.join("node.exe"));

    let found = ClaudeBinary::locate_in(&env_with(tmp.path(), path_of(&[&npm, &nodejs]))).unwrap();
    assert_eq!(found.program, nodejs.join("node.exe"));
}

#[test]
fn an_npm_shim_that_points_at_an_exe_runs_the_exe() {
    let tmp = tempfile::tempdir().unwrap();
    let npm = tmp.path().join("npm");
    fs::create_dir_all(&npm).unwrap();
    fs::write(
        npm.join("claude.cmd"),
        npm_shim(r"node_modules\@anthropic-ai\claude-code\bin\claude.exe"),
    )
    .unwrap();
    let exe = npm
        .join("node_modules")
        .join("@anthropic-ai")
        .join("claude-code")
        .join("bin")
        .join("claude.exe");
    touch(&exe);

    let found = ClaudeBinary::locate_in(&env_with(tmp.path(), path_of(&[&npm]))).unwrap();
    assert_eq!(found.program, exe);
    assert!(found.prefix_args.is_empty());
}

/// O `%APPDATA%\npm` fora do PATH (app aberto com PATH mínimo) ainda é achado.
#[test]
fn the_npm_global_folder_is_searched_even_off_the_path() {
    let tmp = tempfile::tempdir().unwrap();
    let appdata = tmp.path().join("AppData").join("Roaming");
    let npm = appdata.join("npm");
    fs::create_dir_all(&npm).unwrap();
    fs::write(
        npm.join("claude.cmd"),
        npm_shim(r"node_modules\@anthropic-ai\claude-code\bin\claude.exe"),
    )
    .unwrap();
    touch(
        &npm.join("node_modules")
            .join("@anthropic-ai")
            .join("claude-code")
            .join("bin")
            .join("claude.exe"),
    );
    let mut env = env_with(tmp.path(), OsString::new());
    env.appdata = Some(appdata);

    assert!(ClaudeBinary::locate_in(&env).is_some());
}

/// A cópia do Claude Desktop guarda o token noutro lugar, e o alias do
/// WindowsApps abre o app da loja: perguntar o uso a eles responderia por outra
/// conta, ou por nenhuma.
#[test]
fn the_desktop_app_and_windowsapps_aliases_are_never_chosen() {
    let tmp = tempfile::tempdir().unwrap();
    let apps = tmp.path().join("Microsoft").join("WindowsApps");
    let desktop = tmp.path().join("AnthropicClaude").join("app-1.0.0");
    touch(&apps.join("claude.exe"));
    touch(&desktop.join("claude.exe"));
    assert!(ClaudeBinary::locate_in(&env_with(tmp.path(), path_of(&[&apps, &desktop]))).is_none());

    let real = tmp.path().join("ferramentas");
    touch(&real.join("claude.exe"));
    let found =
        ClaudeBinary::locate_in(&env_with(tmp.path(), path_of(&[&apps, &desktop, &real]))).unwrap();
    assert_eq!(found.program, real.join("claude.exe"));
}

#[test]
fn nothing_installed_is_none() {
    let tmp = tempfile::tempdir().unwrap();
    assert!(ClaudeBinary::locate_in(&env_with(tmp.path(), OsString::new())).is_none());
}
