//! A função de shell: os scripts, os dois `$PROFILE`, o `.bashrc` e a política de execução.
//!
//! Checagens do `router doctor` — a ordem está no `mod.rs` ao lado.

use super::*;

/// Os scripts existem e citam ESTE binário?
pub(super) fn check_scripts(
    report: &mut Report,
    ps1: &Path,
    sh: &Path,
    me: &Path,
    with_bash: bool,
) {
    let scripts: Vec<(&Path, &str, String)> = {
        let mut v = vec![(ps1, "shell.ps1", me.to_string_lossy().replace('\'', "''"))];
        if with_bash {
            v.push((sh, "shell.sh", me.to_string_lossy().replace('\\', "/")));
        }
        v
    };
    for (path, name, needle) in scripts {
        match read_retrying(path) {
            Err(_) => report.check(
                false,
                format!("{name} ausente — Grupos → Integração com o terminal → Ativar"),
            ),
            Ok(bytes) if String::from_utf8_lossy(&bytes).contains(needle.as_str()) => {
                report.check(true, format!("{name} aponta para este binário"))
            }
            Ok(_) => report.check(
                false,
                format!(
                    "{name} aponta para OUTRO binário (app movido ou reinstalado) — abra o app para reparar"
                ),
            ),
        }
    }
}

/// A linha nos dois `$PROFILE` (e no `.bashrc`), a política de execução de cada
/// edição, e uma função `claude` do usuário que a integração encadeia.
pub(super) fn check_profiles(
    report: &mut Report,
    targets: &ShellTargets,
    ps1: &Path,
    sh: &Path,
    with_bash: bool,
) {
    let ps_line = ShellIntegration::powershell_source_line(ps1);
    let editions = powershell_editions(&EditionEnv::from_process());
    for profile in &targets.powershell_profiles {
        let folder = profile
            .parent()
            .and_then(Path::file_name)
            .map(|f| f.to_string_lossy().into_owned())
            .unwrap_or_default();
        let label = if folder.eq_ignore_ascii_case("WindowsPowerShell") {
            "Windows PowerShell 5.1"
        } else {
            "PowerShell 7"
        };
        let Some(edition) = editions
            .iter()
            .find(|e| folder.eq_ignore_ascii_case(e.profile_dir))
        else {
            // Uma edição que não existe aqui não tem terminal para quebrar.
            report.info(format!(
                "{label} não encontrado nesta máquina — o $PROFILE dele não é conferido"
            ));
            continue;
        };
        let has = ShellIntegration::profile_has_line(profile, &ps_line);
        if has {
            report.check(true, format!("$PROFILE do {label} carrega o shell.ps1"));
            if defines_claude_function(profile) {
                report.info(format!(
                    "o $PROFILE do {label} define uma função `claude`: a integração a encadeia — `claude` sem grupo continua passando por ela"
                ));
            }
        } else {
            report.check(
                false,
                format!(
                    "$PROFILE do {label} não carrega o shell.ps1 ({}) — Grupos → Integração com o terminal → Ativar",
                    profile.display()
                ),
            );
        }
        // A política só importa onde a integração deveria rodar (e consultá-la
        // abre um PowerShell, que não é de graça).
        if has {
            match effective_policy(edition) {
                Some(policy) if policy_blocks_profiles(&policy) => report.check(
                    false,
                    format!(
                        "política de execução do {label}: {policy} — o $PROFILE não roda e `claude <grupo>` cai no claude puro, na conta errada. Corrija: Set-ExecutionPolicy -Scope CurrentUser RemoteSigned"
                    ),
                ),
                Some(policy) => {
                    report.check(true, format!("política de execução do {label}: {policy}"))
                }
                None => report.info(format!(
                    "não foi possível consultar a política de execução do {label}"
                )),
            }
        }
    }
    if with_bash {
        let sh_line = ShellIntegration::bash_source_line(sh);
        let has = ShellIntegration::profile_has_line(&targets.bashrc, &sh_line);
        report.check(
            has,
            if has {
                "~/.bashrc carrega o shell.sh (Git Bash)".to_string()
            } else {
                "~/.bashrc não carrega o shell.sh (Git Bash) — Grupos → Integração com o terminal → Ativar".to_string()
            },
        );
        check_bash_profile(report, &targets.home);
    }
}

/// O Git Bash só lê o `.bashrc` se um perfil de login o carregar.
pub(super) fn check_bash_profile(report: &mut Report, home: &Path) {
    match bash_login_profile(home) {
        BashLogin::Missing => report.check(
            false,
            "sem ~/.bash_profile — o Git Bash não carrega o ~/.bashrc (e avisa em vermelho). Ativar a integração no app o cria",
        ),
        BashLogin::Loads(first) => {
            report.check(true, format!("{} carrega o ~/.bashrc", first.display()))
        }
        BashLogin::Ignores(first) => report.check(
            false,
            format!(
                "{} não carrega o ~/.bashrc — acrescente: test -f ~/.bashrc && . ~/.bashrc",
                first.display()
            ),
        ),
    }
}
