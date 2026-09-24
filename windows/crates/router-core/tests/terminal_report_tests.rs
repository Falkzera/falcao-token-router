//! O estado da integração de terminal, por shell — o que a tela de Grupos
//! mostra e o `router doctor` confere. Tudo com caminhos e processos injetados:
//! nenhum teste consulta o PowerShell de verdade nem lê o perfil do usuário.

use std::fs;
use std::path::{Path, PathBuf};

use router_core::engine::shell_integration::{ShellIntegration, ShellTargets};
use router_core::engine::terminal_report::{
    bash_login_profile, defines_claude_function, parse_policy_list, policy_blocks_profiles,
    powershell_editions, scripts_state, BashLogin, EditionEnv, ScriptsState, ShellKind,
    TerminalReport,
};

fn touch(path: &Path) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, b"").unwrap();
}

/// Uma máquina de mentira: `SystemRoot` e `ProgramFiles` num temporário.
fn machine(with_ps51: bool, with_pwsh: bool) -> (tempfile::TempDir, EditionEnv) {
    let tmp = tempfile::tempdir().unwrap();
    let system_root = tmp.path().join("Windows");
    let program_files = tmp.path().join("Program Files");
    if with_ps51 {
        touch(&system_root.join(r"System32\WindowsPowerShell\v1.0\powershell.exe"));
    }
    if with_pwsh {
        touch(&program_files.join(r"PowerShell\7\pwsh.exe"));
    }
    let env = EditionEnv {
        system_root: Some(system_root),
        program_files: Some(program_files),
        path: None,
    };
    (tmp, env)
}

#[test]
fn both_editions_are_found_with_their_profile_folder_and_default_policy() {
    let (_tmp, env) = machine(true, true);
    let editions = powershell_editions(&env);

    assert_eq!(editions.len(), 2);
    let ps51 = editions
        .iter()
        .find(|e| e.kind == ShellKind::WindowsPowerShell)
        .unwrap();
    assert_eq!(ps51.profile_dir, "WindowsPowerShell");
    // O padrão do Windows PowerShell 5.1 nas edições cliente: o perfil não roda.
    assert_eq!(ps51.default_policy, "Restricted");
    let pwsh = editions
        .iter()
        .find(|e| e.kind == ShellKind::PowerShell7)
        .unwrap();
    assert_eq!(pwsh.profile_dir, "PowerShell");
    assert_eq!(pwsh.default_policy, "RemoteSigned");
}

/// Uma edição que não está instalada não tem terminal para quebrar.
#[test]
fn a_missing_edition_is_not_reported() {
    let (_tmp, env) = machine(true, false);
    let editions = powershell_editions(&env);
    assert_eq!(editions.len(), 1);
    assert_eq!(editions[0].kind, ShellKind::WindowsPowerShell);
}

/// O `pwsh` da Microsoft Store não mora no `ProgramFiles`: é achado pelo PATH
/// (o alias do WindowsApps — é o caso da máquina do spike).
#[test]
fn pwsh_is_also_found_on_the_path() {
    let (tmp, mut env) = machine(false, false);
    let apps = tmp.path().join("WindowsApps");
    touch(&apps.join("pwsh.exe"));
    env.path = Some(std::env::join_paths([apps.clone()]).unwrap());

    let editions = powershell_editions(&env);

    assert_eq!(editions.len(), 1);
    assert_eq!(editions[0].kind, ShellKind::PowerShell7);
    assert_eq!(editions[0].exe, apps.join("pwsh.exe"));
}

/// A política efetiva IGNORA o escopo Process: o shell do Claude Code herda
/// `Bypass`, e o que decide se o `$PROFILE` roda num terminal novo são os
/// outros escopos, na ordem de precedência.
#[test]
fn the_effective_policy_ignores_the_process_scope() {
    let listed = "MachinePolicy=Undefined\r\nUserPolicy=Undefined\r\nProcess=Bypass\r\nCurrentUser=Undefined\r\nLocalMachine=Undefined\r\n";
    assert_eq!(parse_policy_list(listed, "Restricted"), "Restricted");

    let user_set = "MachinePolicy=Undefined\nUserPolicy=Undefined\nProcess=Bypass\nCurrentUser=RemoteSigned\nLocalMachine=Undefined\n";
    assert_eq!(parse_policy_list(user_set, "Restricted"), "RemoteSigned");

    // Diretiva de grupo vence o que o usuário escolheu.
    let gpo = "MachinePolicy=AllSigned\nUserPolicy=Undefined\nProcess=Undefined\nCurrentUser=RemoteSigned\nLocalMachine=Undefined\n";
    assert_eq!(parse_policy_list(gpo, "Restricted"), "AllSigned");
}

#[test]
fn restricted_and_all_signed_block_the_profile() {
    assert!(policy_blocks_profiles("Restricted"));
    assert!(policy_blocks_profiles("AllSigned"));
    assert!(!policy_blocks_profiles("RemoteSigned"));
    assert!(!policy_blocks_profiles("Unrestricted"));
    assert!(!policy_blocks_profiles("Bypass"));
}

/// O perfil do usuário pode já definir `function claude` — em UTF-8 ou no
/// UTF-16LE que o Windows PowerShell 5.1 usa ao salvar.
#[test]
fn a_user_function_claude_is_detected_in_utf8_and_utf16() {
    let tmp = tempfile::tempdir().unwrap();
    let utf8 = tmp.path().join("utf8.ps1");
    fs::write(
        &utf8,
        "Set-Alias ll ls\r\nfunction claude { & claude.exe @args }\r\n",
    )
    .unwrap();
    assert!(defines_claude_function(&utf8));

    let utf16 = tmp.path().join("utf16.ps1");
    let mut bytes = vec![0xFF, 0xFE];
    for unit in "  function global:Claude {\r\n}\r\n".encode_utf16() {
        bytes.extend_from_slice(&unit.to_le_bytes());
    }
    fs::write(&utf16, bytes).unwrap();
    assert!(defines_claude_function(&utf16));

    let other = tmp.path().join("other.ps1");
    fs::write(&other, "function claude-gov { }\r\n# function claude\r\n").unwrap();
    assert!(!defines_claude_function(&other));
    assert!(!defines_claude_function(&tmp.path().join("ausente.ps1")));
}

/// O Git Bash só lê o `.bashrc` se um perfil de login o carregar.
#[test]
fn the_bash_login_profile_is_classified() {
    let tmp = tempfile::tempdir().unwrap();
    assert_eq!(bash_login_profile(tmp.path()), BashLogin::Missing);

    let profile = tmp.path().join(".bash_profile");
    fs::write(&profile, "export PATH=$PATH:~/bin\n").unwrap();
    assert_eq!(
        bash_login_profile(tmp.path()),
        BashLogin::Ignores(profile.clone())
    );

    fs::write(&profile, "test -f ~/.bashrc && . ~/.bashrc\n").unwrap();
    assert_eq!(bash_login_profile(tmp.path()), BashLogin::Loads(profile));
}

/// Os scripts citam ESTE binário? Senão o app foi movido e a integração aponta
/// para o nada — o modo de falha silencioso que a cura na subida conserta.
#[test]
fn scripts_are_missing_current_or_stale() {
    let tmp = tempfile::tempdir().unwrap();
    let ps1 = tmp.path().join("shell.ps1");
    let sh = tmp.path().join("shell.sh");
    let router = PathBuf::from(r"C:\Users\exemplo\AppData\Local\FalcaoTokenRouter\router.exe");
    let moved = PathBuf::from(r"D:\Apps\Falcao\router.exe");

    assert_eq!(
        scripts_state(&ps1, &sh, &router, true),
        ScriptsState::Missing
    );

    ShellIntegration::write_scripts(&router, &ps1, &sh).unwrap();
    assert_eq!(
        scripts_state(&ps1, &sh, &router, true),
        ScriptsState::Current
    );
    assert_eq!(scripts_state(&ps1, &sh, &moved, true), ScriptsState::Stale);

    // Sem Git Bash, só o shell.ps1 conta.
    fs::remove_file(&sh).unwrap();
    assert_eq!(
        scripts_state(&ps1, &sh, &router, false),
        ScriptsState::Current
    );
    assert_eq!(
        scripts_state(&ps1, &sh, &router, true),
        ScriptsState::Missing
    );
}

/// O quadro que a tela de Grupos mostra: por shell presente, se o perfil
/// carrega a integração, se a política deixa o perfil rodar e se há uma função
/// `claude` do usuário encadeada. A política só é consultada onde importa.
#[test]
fn the_report_covers_each_shell_present() {
    let (tmp, env) = machine(true, true);
    let home = tmp.path().join("home");
    let documents = home.join("Documents");
    let targets = ShellTargets::for_home(&home, Some(&documents));
    let base = tmp.path().join("app");
    let (ps1, sh) = (base.join("shell.ps1"), base.join("shell.sh"));
    let router = PathBuf::from(r"C:\Users\exemplo\AppData\Local\FalcaoTokenRouter\router.exe");
    let git_bash = PathBuf::from(r"C:\Program Files\Git\bin\bash.exe");

    ShellIntegration::write_scripts(&router, &ps1, &sh).unwrap();
    // Só o perfil do 5.1 recebeu a linha — e ele já tinha uma `function claude`.
    let ps51_profile = documents.join(r"WindowsPowerShell\Microsoft.PowerShell_profile.ps1");
    fs::create_dir_all(ps51_profile.parent().unwrap()).unwrap();
    fs::write(&ps51_profile, "function claude { claude.exe @args }\r\n").unwrap();
    ShellIntegration::ensure_in_profile(
        &ps51_profile,
        &ShellIntegration::powershell_source_line(&ps1),
        "\r\n",
    )
    .unwrap();

    let mut asked = Vec::new();
    let report = TerminalReport::build(
        &targets,
        &ps1,
        &sh,
        &router,
        &powershell_editions(&env),
        Some(&git_bash),
        false,
        |edition| {
            asked.push(edition.kind);
            Some("Restricted".to_string())
        },
    );

    assert_eq!(report.scripts, ScriptsState::Current);
    assert!(!report.developer_mode);
    assert_eq!(report.shells.len(), 3, "5.1, 7 e Git Bash");

    let ps51 = report.shell(ShellKind::WindowsPowerShell).unwrap();
    assert!(ps51.loads_integration);
    assert!(ps51.chains_user_function);
    assert_eq!(ps51.policy.as_deref(), Some("Restricted"));
    assert!(ps51.policy_blocks);
    assert_eq!(ps51.profile, ps51_profile);

    let pwsh = report.shell(ShellKind::PowerShell7).unwrap();
    assert!(!pwsh.loads_integration);
    assert_eq!(pwsh.policy, None, "sem a linha, a política não importa");
    assert!(!pwsh.policy_blocks);

    let bash = report.shell(ShellKind::GitBash).unwrap();
    assert!(!bash.loads_integration);
    assert_eq!(bash.profile, home.join(".bashrc"));
    assert_eq!(bash.bash_login, Some(BashLogin::Missing));

    assert_eq!(
        asked,
        vec![ShellKind::WindowsPowerShell],
        "a política só é consultada onde a linha está"
    );
    // Uma peça faltando (o perfil do 7, o .bashrc): não está tudo pronto.
    assert!(!report.fully_installed());
    // E há um aviso a dar: a política do 5.1 impede o perfil de rodar.
    assert!(report.blocked_by_policy());
}

/// Sem Git Bash, ele não entra no quadro; com tudo no lugar, está pronto.
#[test]
fn without_git_bash_the_report_has_only_powershell() {
    let (tmp, env) = machine(false, true);
    let home = tmp.path().join("home");
    let documents = home.join("Documents");
    let targets = ShellTargets::for_home(&home, Some(&documents));
    let base = tmp.path().join("app");
    let (ps1, sh) = (base.join("shell.ps1"), base.join("shell.sh"));
    let router = PathBuf::from(r"C:\Users\exemplo\AppData\Local\FalcaoTokenRouter\router.exe");

    ShellIntegration::write_scripts(&router, &ps1, &sh).unwrap();
    for profile in &targets.powershell_profiles {
        ShellIntegration::ensure_in_profile(
            profile,
            &ShellIntegration::powershell_source_line(&ps1),
            "\r\n",
        )
        .unwrap();
    }

    let report = TerminalReport::build(
        &targets,
        &ps1,
        &sh,
        &router,
        &powershell_editions(&env),
        None,
        true,
        |_| Some("RemoteSigned".to_string()),
    );

    assert_eq!(report.shells.len(), 1);
    assert_eq!(report.shells[0].kind, ShellKind::PowerShell7);
    assert!(report.developer_mode);
    assert!(report.fully_installed());
    assert!(!report.blocked_by_policy());
}
