//! A integração de terminal na tela (≙ TerminalIntegrationRow), com o quadro
//! POR SHELL que o macOS não tinha: no Windows os modos de falha são vários e
//! silenciosos (política de execução, perfil que não carrega, `.bash_profile`
//! que ignora o `.bashrc`) e todos terminam em `claude <grupo>` abrindo no
//! `claude` puro, na conta errada.
//!
//! O quadro consulta a política de cada PowerShell (abre um processo por
//! edição): roda fora da thread da interface.

use std::path::Path;

use router_core::engine::shell_integration::{ShellTargets, StatusShell};
use router_core::engine::terminal_report::{
    allow_profiles, effective_policy, powershell_editions, BashLogin, EditionEnv, ScriptsState,
    ShellKind, TerminalReport,
};
use router_core::platform::git_bash::{find_git_bash, GitBashEnv};
use router_core::platform::links::developer_mode_enabled;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

use crate::commands::publish;
use crate::snapshot::Snapshot;
use crate::state::AppState;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ShellName {
    PowerShell7,
    WindowsPowerShell,
    GitBash,
}

impl From<ShellKind> for ShellName {
    fn from(kind: ShellKind) -> Self {
        match kind {
            ShellKind::PowerShell7 => ShellName::PowerShell7,
            ShellKind::WindowsPowerShell => ShellName::WindowsPowerShell,
            ShellKind::GitBash => ShellName::GitBash,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum BashLoginView {
    Missing,
    Loads,
    Ignores,
}

#[derive(Clone, PartialEq, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShellView {
    pub shell: ShellName,
    /// O arquivo que a integração edita (para o usuário achar, se quiser).
    pub profile: String,
    pub loads_integration: bool,
    pub policy: Option<String>,
    pub policy_blocks: bool,
    pub chains_user_function: bool,
    pub bash_login: Option<BashLoginView>,
    /// O nome do perfil de login do Git Bash (`.bash_profile`, `.bash_login`
    /// ou `.profile`) — quando ele ignora o `.bashrc`, é nele que o usuário
    /// acrescenta a linha, e a tela precisa dizer qual.
    pub bash_login_file: Option<String>,
}

#[derive(Clone, PartialEq, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TerminalView {
    /// O app sabe onde está o `router.exe` (sem ele não há integração).
    pub router_found: bool,
    pub scripts: crate::snapshot::Scripts,
    pub shells: Vec<ShellView>,
    pub developer_mode: bool,
    pub fully_installed: bool,
    pub blocked_by_policy: bool,
    /// "Ativar" resolve algo: scripts ausentes ou citando outro router, ou um
    /// shell sem a linha no perfil. A política que bloqueia e o `.bash_profile`
    /// que ignora o `.bashrc` NÃO se resolvem instalando — têm correção
    /// própria, e o botão não pode prometer o que não faz.
    pub needs_install: bool,
}

/// O nome do arquivo de login do Git Bash que existe (o que ele lê ao abrir).
fn login_file_name(login: &BashLogin) -> Option<String> {
    match login {
        BashLogin::Missing => None,
        BashLogin::Loads(path) | BashLogin::Ignores(path) => path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned()),
    }
}

fn view(report: &TerminalReport, router_found: bool) -> TerminalView {
    TerminalView {
        router_found,
        scripts: match report.scripts {
            ScriptsState::Missing => crate::snapshot::Scripts::Missing,
            ScriptsState::Current => crate::snapshot::Scripts::Current,
            ScriptsState::Stale => crate::snapshot::Scripts::Stale,
        },
        shells: report
            .shells
            .iter()
            .map(|s| ShellView {
                shell: s.kind.into(),
                profile: s.profile.to_string_lossy().into_owned(),
                loads_integration: s.loads_integration,
                policy: s.policy.clone(),
                policy_blocks: s.policy_blocks,
                chains_user_function: s.chains_user_function,
                bash_login: s.bash_login.as_ref().map(|b| match b {
                    BashLogin::Missing => BashLoginView::Missing,
                    BashLogin::Loads(_) => BashLoginView::Loads,
                    BashLogin::Ignores(_) => BashLoginView::Ignores,
                }),
                bash_login_file: s.bash_login.as_ref().and_then(login_file_name),
            })
            .collect(),
        developer_mode: report.developer_mode,
        fully_installed: router_found && report.fully_installed(),
        blocked_by_policy: report.blocked_by_policy(),
        needs_install: report.scripts != ScriptsState::Current
            || report.shells.iter().any(|s| !s.loads_integration),
    }
}

/// Monta o quadro (lento: consulta a política de cada PowerShell presente).
fn build(app: &AppHandle) -> TerminalView {
    let state = app.state::<AppState>();
    let (router, ps1, sh) = {
        let store = state.store();
        (
            store.router_path().map(Path::to_path_buf),
            store.powershell_script_path(),
            store.bash_script_path(),
        )
    };
    let targets = ShellTargets::for_user(Path::new(&state.home));
    let git_bash = find_git_bash(&GitBashEnv::from_process());
    let report = TerminalReport::build(
        &targets,
        &ps1,
        &sh,
        router.as_deref().unwrap_or(Path::new("")),
        &powershell_editions(&EditionEnv::from_process()),
        git_bash.as_deref(),
        developer_mode_enabled(),
        effective_policy,
    );
    view(&report, router.is_some())
}

/// O quadro da integração, fora da thread da interface.
#[tauri::command]
pub async fn terminal_report(app: AppHandle) -> Result<TerminalView, String> {
    tauri::async_runtime::spawn_blocking(move || build(&app))
        .await
        .map_err(|e| e.to_string())
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallResult {
    /// Tudo gravado. "Instalada ✓" só aparece com isto — o macOS mostrava o
    /// ✓ mesmo quando a instalação falhava.
    pub ok: bool,
    pub snapshot: Snapshot,
    pub report: TerminalView,
}

/// "Ativar"/"Reinstalar": scripts, status line e compartilhamento em cada
/// grupo, e a linha nos perfis de shell. Idempotente.
#[tauri::command]
pub async fn install_integration(app: AppHandle) -> Result<InstallResult, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let ok = {
            let state = app.state::<AppState>();
            let targets = ShellTargets::for_user(Path::new(&state.home));
            let shell = if find_git_bash(&GitBashEnv::from_process()).is_some() {
                StatusShell::Bash
            } else {
                StatusShell::PowerShell
            };
            let mut store = state.store();
            store.install_shell_integration(&targets, shell).is_ok()
        };
        InstallResult {
            ok,
            snapshot: publish(&app),
            report: build(&app),
        }
    })
    .await
    .map_err(|e| e.to_string())
}

/// A correção consentida: `RemoteSigned` no escopo do usuário, na edição do
/// PowerShell que estava impedindo o perfil de rodar. Confere de novo depois
/// (diretiva de grupo continua vencendo) e devolve o quadro novo.
#[tauri::command]
pub async fn allow_profiles_for(app: AppHandle, shell: ShellName) -> Result<TerminalView, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let kind = match shell {
            ShellName::PowerShell7 => ShellKind::PowerShell7,
            ShellName::WindowsPowerShell => ShellKind::WindowsPowerShell,
            ShellName::GitBash => return build(&app),
        };
        if let Some(edition) = powershell_editions(&EditionEnv::from_process())
            .into_iter()
            .find(|e| e.kind == kind)
        {
            let _ = allow_profiles(&edition);
        }
        build(&app)
    })
    .await
    .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    use router_core::engine::terminal_report::ShellReport;

    fn shell(kind: ShellKind, loads: bool) -> ShellReport {
        ShellReport {
            kind,
            profile: PathBuf::from(
                r"C:\Users\exemplo\Documents\PowerShell\Microsoft.PowerShell_profile.ps1",
            ),
            loads_integration: loads,
            policy: None,
            policy_blocks: false,
            chains_user_function: false,
            bash_login: None,
        }
    }

    fn report(scripts: ScriptsState, shells: Vec<ShellReport>) -> TerminalReport {
        TerminalReport {
            scripts,
            shells,
            developer_mode: false,
        }
    }

    fn both_powershells(loads: bool) -> Vec<ShellReport> {
        vec![
            shell(ShellKind::PowerShell7, loads),
            shell(ShellKind::WindowsPowerShell, loads),
        ]
    }

    /// "Ativar" só é oferecido quando instalar resolve algo: scripts ausentes
    /// ou citando outro router, ou um shell sem a linha no perfil.
    #[test]
    fn activate_is_offered_when_installing_fixes_something() {
        assert!(
            view(
                &report(ScriptsState::Missing, both_powershells(false)),
                true
            )
            .needs_install
        );
        assert!(view(&report(ScriptsState::Stale, both_powershells(true)), true).needs_install);
        let one_missing = vec![
            shell(ShellKind::PowerShell7, true),
            shell(ShellKind::WindowsPowerShell, false),
        ];
        assert!(view(&report(ScriptsState::Current, one_missing), true).needs_install);

        let installed = view(&report(ScriptsState::Current, both_powershells(true)), true);
        assert!(!installed.needs_install);
        assert!(installed.fully_installed);
    }

    /// A política que bloqueia o perfil tem correção própria ("Permitir"):
    /// reinstalar não a resolve, e a integração não está pronta.
    #[test]
    fn a_blocking_policy_is_not_fixed_by_installing() {
        let mut shells = both_powershells(true);
        shells[1].policy = Some("Restricted".to_string());
        shells[1].policy_blocks = true;
        let v = view(&report(ScriptsState::Current, shells), true);
        assert!(!v.needs_install);
        assert!(!v.fully_installed);
        assert!(v.blocked_by_policy);
        assert_eq!(v.shells[1].policy.as_deref(), Some("Restricted"));
    }

    /// Git Bash cujo perfil de login ignora o `.bashrc`: a tela diz QUAL
    /// arquivo precisa da linha (pode ser `.bash_profile`, `.bash_login` ou
    /// `.profile`), e não conta como pronto.
    #[test]
    fn the_bash_login_file_is_named() {
        let mut bash = shell(ShellKind::GitBash, true);
        bash.profile = PathBuf::from(r"C:\Users\exemplo\.bashrc");
        bash.bash_login = Some(BashLogin::Ignores(PathBuf::from(
            r"C:\Users\exemplo\.bash_login",
        )));
        let v = view(&report(ScriptsState::Current, vec![bash.clone()]), true);
        assert_eq!(v.shells[0].bash_login, Some(BashLoginView::Ignores));
        assert_eq!(v.shells[0].bash_login_file.as_deref(), Some(".bash_login"));
        assert!(!v.fully_installed);
        assert!(!v.needs_install, "a linha no .bash_login é do usuário");

        // Sem perfil de login o próprio Git Bash cria um que carrega o
        // `.bashrc` (com um aviso) — funciona; não há o que nomear.
        bash.bash_login = Some(BashLogin::Missing);
        let v = view(&report(ScriptsState::Current, vec![bash]), true);
        assert_eq!(v.shells[0].bash_login_file, None);
        assert!(v.fully_installed);
    }

    /// Sem saber onde está o `router.exe` não há integração, diga o disco o que
    /// disser.
    #[test]
    fn without_the_router_nothing_counts_as_installed() {
        let v = view(
            &report(ScriptsState::Current, both_powershells(true)),
            false,
        );
        assert!(!v.router_found);
        assert!(!v.fully_installed);
    }
}
