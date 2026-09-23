//! Integração de terminal: a `statusLine` de cada perfil de grupo, as funções
//! `claude` (PowerShell e Git Bash) e a linha que as carrega no perfil do shell.
//!
//! Portados de `LauncherTests.swift` (suítes "ShellIntegration", "ShellIntegration
//! profile" e "integridade do ~/.zshrc": 8), mais o que o Windows exige: o
//! caminho vai com `/` e sem aspas (funciona no Git Bash e no PowerShell — com
//! aspas quebra no PowerShell, medido); com espaço vira o nome 8.3; sem 8.3, as
//! aspas do shell que o Claude Code usa. O perfil do usuário recebe a linha em
//! BYTES, na codificação e no fim de linha que ele já tem (UTF-16, ANSI, LF).

use std::fs::{self, OpenOptions};
use std::os::windows::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};

use router_core::engine::shell_integration::{
    SettingsError, ShellIntegration, ShellTargets, StatusShell,
};
use router_core::platform::git_bash::{find_git_bash, GitBashEnv};
use router_core::platform::profile_append::AppendOutcome;
use router_core::ConfigDir;

fn no_short(_: &Path) -> Option<PathBuf> {
    None
}

fn command(router: &str, profile: Option<&str>, shell: StatusShell) -> String {
    ShellIntegration::status_line_command(
        Path::new(router),
        profile.map(Path::new),
        shell,
        &no_short,
    )
}

fn dedicated(path: &Path) -> ConfigDir {
    ConfigDir::dedicated(path.to_string_lossy().into_owned())
}

fn settings_of(dir: &ConfigDir) -> serde_json::Map<String, serde_json::Value> {
    let data = fs::read(ShellIntegration::settings_path(dir)).unwrap();
    serde_json::from_slice::<serde_json::Value>(&data)
        .unwrap()
        .as_object()
        .unwrap()
        .clone()
}

// --- A statusLine (portados) ---

#[test]
fn the_status_line_points_at_the_sensor() {
    let cmd = command(
        r"C:\Users\exemplo\AppData\Local\FalcaoTokenRouter\router.exe",
        None,
        StatusShell::Bash,
    );
    assert!(cmd.contains("statusline"), "{cmd}");
    assert!(
        cmd.contains("C:/Users/exemplo/AppData/Local/FalcaoTokenRouter/router.exe"),
        "{cmd}"
    );
}

/// O `settings.json` do perfil tem chaves do usuário que não podem sumir.
#[test]
fn installing_the_status_line_keeps_the_other_keys_in_place() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = dedicated(tmp.path());
    fs::write(
        ShellIntegration::settings_path(&dir),
        br#"{"model":"claude-fable-5[1m]","statusLine":{"type":"command","command":"x"},"theme":"dark"}"#,
    )
    .unwrap();

    ShellIntegration::install_status_line("C:/r/router.exe statusline", &dir).unwrap();

    let root = settings_of(&dir);
    assert_eq!(root["model"], "claude-fable-5[1m]");
    assert_eq!(root["theme"], "dark");
    assert_eq!(root["statusLine"]["command"], "C:/r/router.exe statusline");
    assert_eq!(root["statusLine"]["type"], "command");
    assert_eq!(root["statusLine"]["padding"], 0);
    let keys: Vec<&String> = root.keys().collect();
    assert_eq!(keys, ["model", "statusLine", "theme"]);
}

/// O `launch` planta a status line a cada sessão: quando ela já está certa, o
/// arquivo não é tocado (no grupo padrão ele é o `settings.json` do usuário).
#[test]
fn installing_the_same_status_line_again_does_not_touch_the_file() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = dedicated(tmp.path());
    ShellIntegration::install_status_line("C:/r/router.exe statusline", &dir).unwrap();
    let path = ShellIntegration::settings_path(&dir);
    let long_ago =
        std::time::SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1_000_000_000);
    OpenOptions::new()
        .write(true)
        .open(&path)
        .unwrap()
        .set_modified(long_ago)
        .unwrap();

    ShellIntegration::install_status_line("C:/r/router.exe statusline", &dir).unwrap();

    assert_eq!(fs::metadata(&path).unwrap().modified().unwrap(), long_ago);
}

/// O caso que motivou a checagem no macOS: o app mudou de nome e a status line
/// continuou apontando para o caminho morto. Aqui: app reinstalado noutro lugar.
#[test]
fn a_status_line_of_another_binary_is_stale_and_the_current_one_is_not() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = dedicated(tmp.path());
    let old = command(r"C:\Antigo\router.exe", None, StatusShell::Bash);
    let current = command(r"C:\Novo\router.exe", None, StatusShell::Bash);

    ShellIntegration::install_status_line(&old, &dir).unwrap();
    assert!(ShellIntegration::status_line_is_stale(&current, &dir));

    ShellIntegration::install_status_line(&current, &dir).unwrap();
    assert!(!ShellIntegration::status_line_is_stale(&current, &dir));
}

/// Sem `settings.json`: o sensor não está plantado — mesmo resultado prático de
/// apontar para o lugar errado (a conta fica sem medição).
#[test]
fn a_profile_without_a_status_line_is_stale() {
    let tmp = tempfile::tempdir().unwrap();
    assert!(ShellIntegration::status_line_is_stale(
        "C:/r/router.exe statusline",
        &dedicated(tmp.path())
    ));
}

#[test]
fn the_shell_functions_embed_the_router_path() {
    let router = Path::new(r"C:\Users\exemplo\AppData\Local\FalcaoTokenRouter\router.exe");
    let ps = ShellIntegration::powershell_script(router);
    assert!(ps.contains("function global:claude"), "{ps}");
    assert!(ps.contains("is-group") && ps.contains("launch"));
    assert!(ps.contains(r"C:\Users\exemplo\AppData\Local\FalcaoTokenRouter\router.exe"));

    let sh = ShellIntegration::bash_script(router);
    assert!(sh.contains("claude()"), "{sh}");
    assert!(sh.contains("is-group") && sh.contains("launch"));
    assert!(sh.contains("C:/Users/exemplo/AppData/Local/FalcaoTokenRouter/router.exe"));
}

// --- A linha no perfil do shell (portados) ---

#[test]
fn the_profile_line_is_added_once_and_the_original_stays() {
    let tmp = tempfile::tempdir().unwrap();
    let profile = tmp.path().join("Microsoft.PowerShell_profile.ps1");
    fs::write(&profile, "# meu perfil\r\nSet-Alias ll Get-ChildItem\r\n").unwrap();
    let line = ShellIntegration::powershell_source_line(Path::new(
        r"C:\Users\exemplo\AppData\Local\com.synqo.falcao-router\shell.ps1",
    ));

    let first = ShellIntegration::ensure_in_profile(&profile, &line, "\r\n").unwrap();
    let second = ShellIntegration::ensure_in_profile(&profile, &line, "\r\n").unwrap();

    assert_eq!(first, AppendOutcome::Added);
    assert_eq!(second, AppendOutcome::AlreadyPresent);
    let text = fs::read_to_string(&profile).unwrap();
    assert!(text.starts_with("# meu perfil\r\nSet-Alias ll Get-ChildItem\r\n"));
    assert_eq!(
        text.matches("shell.ps1").count(),
        2,
        "linha duplicada: {text}"
    );
}

#[test]
fn a_missing_profile_is_created_with_its_folder() {
    let tmp = tempfile::tempdir().unwrap();
    let profile = tmp
        .path()
        .join("Documents")
        .join("WindowsPowerShell")
        .join("Microsoft.PowerShell_profile.ps1");

    let out = ShellIntegration::ensure_in_profile(&profile, "# linha", "\r\n").unwrap();

    assert_eq!(out, AppendOutcome::Added);
    assert!(fs::read_to_string(&profile).unwrap().contains("# linha"));
}

/// Um perfil que não é UTF-8 (um alias com acento salvo em ANSI) não pode ser
/// reescrito: no macOS a leitura como texto falhava, virava "vazio" e o arquivo
/// era sobrescrito só com a linha nova. Aqui só se acrescenta bytes no fim.
#[test]
fn a_profile_that_is_not_utf8_keeps_every_original_byte() {
    let tmp = tempfile::tempdir().unwrap();
    let profile = tmp.path().join("perfil.ps1");
    let mut original = b"Set-Alias caf".to_vec();
    original.push(0xE9); // "é" em Windows-1252
    original.extend_from_slice(b" Get-Date\r\n");
    fs::write(&profile, &original).unwrap();

    ShellIntegration::ensure_in_profile(&profile, "# linha do router", "\r\n").unwrap();

    let after = fs::read(&profile).unwrap();
    assert!(after.starts_with(&original), "bytes originais alterados");
    assert!(after.ends_with(b"# linha do router\r\n"));
}

// --- Regressões do Windows: a statusLine ---

/// Sem espaço: `/` e sem aspas — a forma que funciona no Git Bash e no
/// PowerShell (medido no spike). Grupo dedicado leva o perfil junto.
#[test]
fn a_path_without_spaces_goes_plain_with_forward_slashes() {
    let cmd = command(
        r"C:\Users\exemplo\AppData\Local\FalcaoTokenRouter\router.exe",
        Some(r"C:\Users\exemplo\AppData\Local\com.synqo.falcao-router\groups\ABC"),
        StatusShell::PowerShell,
    );
    assert_eq!(
        cmd,
        "C:/Users/exemplo/AppData/Local/FalcaoTokenRouter/router.exe statusline --profile C:/Users/exemplo/AppData/Local/com.synqo.falcao-router/groups/ABC"
    );
}

/// Com espaço, o nome 8.3 (ativo no C: por padrão) mantém a forma sem aspas.
#[test]
fn a_path_with_spaces_uses_the_short_name() {
    let short = |p: &Path| {
        (p == Path::new(r"C:\Program Files\Falcao\router.exe"))
            .then(|| PathBuf::from(r"C:\PROGRA~1\Falcao\router.exe"))
    };
    let cmd = ShellIntegration::status_line_command(
        Path::new(r"C:\Program Files\Falcao\router.exe"),
        None,
        StatusShell::PowerShell,
        &short,
    );
    assert_eq!(cmd, "C:/PROGRA~1/Falcao/router.exe statusline");
}

/// Sem nome 8.3 (volume com 8.3 desligado), aspas — do jeito do shell que o
/// Claude Code vai usar: no PowerShell um caminho entre aspas seguido de
/// argumento é erro de sintaxe, então vai com `&`.
#[test]
fn a_path_with_spaces_and_no_short_name_is_quoted_for_the_shell() {
    let bash = command(
        r"C:\Program Files\Falcao\router.exe",
        Some(r"C:\Users\exemplo\Pasta Com Espaco\grupo"),
        StatusShell::Bash,
    );
    assert_eq!(
        bash,
        "'C:/Program Files/Falcao/router.exe' statusline --profile 'C:/Users/exemplo/Pasta Com Espaco/grupo'"
    );
    let ps = command(
        r"C:\Program Files\Falcao\router.exe",
        None,
        StatusShell::PowerShell,
    );
    assert_eq!(ps, "& 'C:/Program Files/Falcao/router.exe' statusline");
}

/// Defeito do macOS que o porte não copia: `settings.json` ilegível virava `{}`
/// e perdia as chaves do usuário. Aqui é recusado e fica intacto.
#[test]
fn an_unreadable_settings_file_is_refused_and_left_alone() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = dedicated(tmp.path());
    let broken = b"{\"model\": \"x\",, }";
    fs::write(ShellIntegration::settings_path(&dir), broken).unwrap();

    let err =
        ShellIntegration::install_status_line("C:/r/router.exe statusline", &dir).unwrap_err();

    assert!(matches!(err, SettingsError::Unreadable { .. }), "{err:?}");
    assert_eq!(
        fs::read(ShellIntegration::settings_path(&dir)).unwrap(),
        broken
    );
}

// --- Regressões do Windows: o perfil do shell ---

/// O Windows PowerShell 5.1 grava perfis em UTF-16LE com BOM ("Unicode"). A
/// linha tem de entrar em UTF-16 também, senão o arquivo vira lixo.
#[test]
fn a_utf16_profile_receives_the_line_in_utf16() {
    let tmp = tempfile::tempdir().unwrap();
    let profile = tmp.path().join("perfil.ps1");
    let mut original = vec![0xFF, 0xFE];
    for unit in "Set-Alias ll Get-ChildItem\r\n".encode_utf16() {
        original.extend_from_slice(&unit.to_le_bytes());
    }
    fs::write(&profile, &original).unwrap();

    ShellIntegration::ensure_in_profile(&profile, "# linha do router", "\r\n").unwrap();

    let after = fs::read(&profile).unwrap();
    assert!(after.starts_with(&original));
    let units: Vec<u16> = after[2..]
        .chunks(2)
        .map(|c| u16::from_le_bytes([c[0], c[1]]))
        .collect();
    let text = String::from_utf16(&units).unwrap();
    assert!(text.ends_with("# linha do router\r\n"), "{text:?}");
}

/// O fim de linha é o do arquivo: LF fica LF. E um arquivo sem quebra no fim
/// ganha uma antes do bloco, para a última linha do usuário não grudar.
#[test]
fn the_line_ending_follows_the_file_and_a_missing_final_newline_is_added() {
    let tmp = tempfile::tempdir().unwrap();
    let profile = tmp.path().join(".bashrc");
    fs::write(&profile, "alias ll='ls -l'\nexport X=1").unwrap();

    ShellIntegration::ensure_in_profile(&profile, "# linha", "\r\n").unwrap();

    let text = fs::read_to_string(&profile).unwrap();
    assert!(
        text.starts_with("alias ll='ls -l'\nexport X=1\n"),
        "{text:?}"
    );
    assert!(text.ends_with("# linha\n"), "{text:?}");
    assert!(!text.contains('\r'), "misturou CRLF num arquivo LF");
}

/// Um perfil preso por outro processo não é tocado: o erro sobe.
#[test]
fn a_locked_profile_is_an_error_and_stays_untouched() {
    let tmp = tempfile::tempdir().unwrap();
    let profile = tmp.path().join("perfil.ps1");
    fs::write(&profile, "# meu\r\n").unwrap();
    let held = OpenOptions::new()
        .read(true)
        .share_mode(0)
        .open(&profile)
        .unwrap();

    let result = ShellIntegration::ensure_in_profile(&profile, "# linha", "\r\n");
    drop(held);

    assert!(result.is_err());
    assert_eq!(fs::read_to_string(&profile).unwrap(), "# meu\r\n");
}

/// A linha é ASCII (entra em qualquer codificação) e só carrega o script se ele
/// existir — perfil de um app desinstalado não quebra o terminal.
#[test]
fn the_source_lines_are_ascii_and_guarded() {
    let ps = ShellIntegration::powershell_source_line(Path::new(
        r"C:\Users\exemplo\AppData\Local\com.synqo.falcao-router\shell.ps1",
    ));
    let sh = ShellIntegration::bash_source_line(Path::new(
        r"C:\Users\exemplo\AppData\Local\com.synqo.falcao-router\shell.sh",
    ));
    for line in [&ps, &sh] {
        assert!(line.is_ascii(), "{line}");
    }
    assert!(ps.starts_with("if (Test-Path"), "{ps}");
    assert!(sh.starts_with("[ -f "), "{sh}");
}

// --- Os scripts gravados ---

/// `shell.ps1` em UTF-8 COM BOM (o PowerShell 5.1 lê arquivo sem BOM como ANSI
/// e estragaria um caminho com acento); `shell.sh` sem BOM e só LF (o bash leria
/// o BOM como comando).
#[test]
fn the_scripts_are_written_in_each_shells_encoding() {
    let tmp = tempfile::tempdir().unwrap();
    let router = Path::new(r"C:\Users\exemplo\AppData\Local\FalcaoTokenRouter\router.exe");
    let (ps1, sh) = (tmp.path().join("shell.ps1"), tmp.path().join("shell.sh"));

    ShellIntegration::write_scripts(router, &ps1, &sh).unwrap();

    let ps_bytes = fs::read(&ps1).unwrap();
    assert!(ps_bytes.starts_with(&[0xEF, 0xBB, 0xBF]));
    let sh_bytes = fs::read(&sh).unwrap();
    assert!(!sh_bytes.starts_with(&[0xEF, 0xBB, 0xBF]));
    assert!(!sh_bytes.contains(&b'\r'), "CRLF no shell.sh");
}

/// O marcador do shim do bash sobrevive ao `declare -f` (que joga fora os
/// comentários): sem isso, recarregar o script guardaria a própria função como
/// "a anterior" e o `claude` sem grupo entraria em recursão.
#[test]
fn the_bash_shim_marker_is_a_command_not_a_comment() {
    let sh = ShellIntegration::bash_script(Path::new(r"C:\x\router.exe"));
    assert!(sh.contains(": falcao-router-shim"), "{sh}");
    assert!(sh.contains("falcao_claude_anterior"));
    let ps = ShellIntegration::powershell_script(Path::new(r"C:\x\router.exe"));
    assert!(ps.contains("falcao-router-shim"));
    assert!(ps.contains("falcao-claude-anterior"));
}

// --- Onde a integração mora ---

/// O Git Bash acusa um WARNING vermelho quando acha `.bashrc` sem
/// `.bash_profile`; o porte cria o `.bash_profile` padrão do Git for Windows.
#[test]
fn a_bash_profile_is_created_when_none_loads_the_bashrc() {
    let home = tempfile::tempdir().unwrap();
    let targets = ShellTargets::for_home(home.path(), None);

    ShellIntegration::ensure_bash_profile(&targets).unwrap();
    let created = fs::read_to_string(home.path().join(".bash_profile")).unwrap();
    assert!(created.contains(". ~/.bashrc"), "{created}");

    // Já existindo (qualquer um dos três), não mexe.
    fs::write(home.path().join(".bash_profile"), "# meu").unwrap();
    ShellIntegration::ensure_bash_profile(&targets).unwrap();
    assert_eq!(
        fs::read_to_string(home.path().join(".bash_profile")).unwrap(),
        "# meu"
    );
}

#[test]
fn the_two_powershell_profiles_live_under_documents() {
    let home = tempfile::tempdir().unwrap();
    let documents = home.path().join("OneDrive").join("Documentos");
    let targets = ShellTargets::for_home(home.path(), Some(&documents));
    assert_eq!(
        targets.powershell_profiles,
        vec![
            documents
                .join("PowerShell")
                .join("Microsoft.PowerShell_profile.ps1"),
            documents
                .join("WindowsPowerShell")
                .join("Microsoft.PowerShell_profile.ps1"),
        ]
    );
    assert_eq!(targets.bashrc, home.path().join(".bashrc"));
}

/// A mesma ordem do Claude Code para achar o bash da status line:
/// `CLAUDE_CODE_GIT_BASH_PATH` → Program Files → Program Files (x86) → o git do
/// PATH → nenhum (PowerShell).
#[test]
fn git_bash_is_found_in_the_order_claude_code_uses() {
    let tmp = tempfile::tempdir().unwrap();
    let touch = |p: &Path| {
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(p, b"").unwrap();
    };
    let pf = tmp.path().join("Program Files");
    let pf86 = tmp.path().join("Program Files (x86)");
    let mut env = GitBashEnv {
        override_path: None,
        program_files: Some(pf.clone()),
        program_files_x86: Some(pf86.clone()),
        path: None,
    };
    assert_eq!(find_git_bash(&env), None);

    let portable = tmp.path().join("PortableGit");
    touch(&portable.join("cmd").join("git.exe"));
    touch(&portable.join("bin").join("bash.exe"));
    env.path = Some(std::env::join_paths([portable.join("cmd")]).unwrap());
    assert_eq!(
        find_git_bash(&env),
        Some(portable.join("bin").join("bash.exe"))
    );

    touch(&pf86.join("Git").join("bin").join("bash.exe"));
    assert_eq!(
        find_git_bash(&env),
        Some(pf86.join("Git").join("bin").join("bash.exe"))
    );

    touch(&pf.join("Git").join("bin").join("bash.exe"));
    assert_eq!(
        find_git_bash(&env),
        Some(pf.join("Git").join("bin").join("bash.exe"))
    );

    let custom = tmp.path().join("meu").join("bash.exe");
    touch(&custom);
    env.override_path = Some(custom.clone());
    assert_eq!(find_git_bash(&env), Some(custom));
}
