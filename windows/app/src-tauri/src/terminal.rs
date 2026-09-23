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
            })
            .collect(),
        developer_mode: report.developer_mode,
        fully_installed: router_found && report.fully_installed(),
        blocked_by_policy: report.blocked_by_policy(),
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
