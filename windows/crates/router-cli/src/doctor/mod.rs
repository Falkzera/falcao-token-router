//! `router doctor` — confere a instalação e nomeia o que está torto, em vez de
//! deixar o usuário descobrir por um sintoma que não parece com a causa.
//!
//! Os modos de falha daqui são SILENCIOSOS: uma função de shell que não carrega
//! (política de execução, perfil errado, app movido) não dá erro nenhum —
//! `claude trabalho` simplesmente abre no `~\.claude`, na conta errada. As
//! checagens do macOS vêm primeiro no espírito; as do Windows somam: os dois
//! `$PROFILE` e o `.bashrc`, a política de execução por edição, a status line de
//! cada grupo rodando DE VERDADE pelo shell que o Claude Code vai usar, uma
//! statusLine de projeto que vence a do grupo, os links, as variáveis que
//! desviam a sessão, e o `claude` com a versão.

// Cada família de checagem tem seu arquivo; o `run` abaixo é a ordem em que
// elas correm e o único lugar que decide o código de saída.
mod accounts;
mod environment;
mod status_line;
mod terminal;

use accounts::*;
use environment::*;
use status_line::*;
use terminal::*;

use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::time::Duration;

use chrono::Utc;

use router_core::engine::group_usage::UsageOrigin;
use router_core::engine::group_usage_reader::GroupUsageReader;
use router_core::engine::profile_sharing::ProfileSharing;
use router_core::engine::provider_env::ProviderEnv;
use router_core::engine::rotation_engine::RotationEngine;
use router_core::engine::router_paths::RouterPaths;
use router_core::engine::session_registry::{SessionRegistry, SessionStatus};
use router_core::engine::shell_integration::{ShellIntegration, ShellTargets, StatusShell};
use router_core::engine::terminal_report::{
    bash_login_profile, defines_claude_function, effective_policy, policy_blocks_profiles,
    powershell_editions, BashLogin, EditionEnv,
};
use router_core::platform::atomic_write::read_retrying;
use router_core::platform::git_bash::{find_git_bash, GitBashEnv};
use router_core::statusline::choice::{Item, Mode, StatusLineChoice};
use router_core::statusline::command::{self, Outcome, Shell};
use router_core::statusline::session;
use router_core::usage::claude_binary::ClaudeBinary;
use router_core::usage::usage_percent::UsagePercent;
use router_core::{ConfigDir, Id, RouterConfig};

use crate::shared;

/// Amostra mais velha que isto já não diz como a conta está.
const STALE_SAMPLE_MINUTES: i64 = 720;

struct Report {
    ok: bool,
}

impl Report {
    fn check(&mut self, good: bool, text: impl AsRef<str>) {
        println!(
            "{}{}",
            if good { "  ok  " } else { "  !!  " },
            text.as_ref()
        );
        if !good {
            self.ok = false;
        }
    }

    fn info(&self, text: impl AsRef<str>) {
        println!("  --  {}", text.as_ref());
    }

    fn finish(self) -> bool {
        println!(
            "{}",
            if self.ok {
                "\ntudo certo."
            } else {
                "\nhá problemas acima."
            }
        );
        self.ok
    }
}

fn describe(status: &SessionStatus) -> &str {
    match status {
        SessionStatus::Busy => "trabalhando",
        SessionStatus::Waiting => "esperando você",
        SessionStatus::Idle => "ociosa",
        SessionStatus::Shell => "shell",
        SessionStatus::Other(raw) if raw.is_empty() => "?",
        SessionStatus::Other(raw) => raw,
    }
}

pub fn run() -> bool {
    let mut report = Report { ok: true };
    let paths = RouterPaths::new();
    println!("router doctor");
    println!("  base: {}", paths.base.display());

    // O binário que a integração DEVERIA citar é este que está rodando.
    let Some(me) = shared::self_path() else {
        report.check(false, "não sei o meu próprio caminho");
        return report.finish();
    };
    println!("  binário: {}", me.display());

    let Some(config) = shared::load_config(&paths) else {
        report.check(false, "sem config.json — crie um grupo no app");
        return report.finish();
    };
    report.check(
        true,
        format!(
            "config: {} grupo(s), {} conta(s)",
            config.groups.len(),
            config.accounts.len()
        ),
    );

    let home = shared::home();
    let git_bash = find_git_bash(&GitBashEnv::from_process());
    let shell = if git_bash.is_some() {
        StatusShell::Bash
    } else {
        StatusShell::PowerShell
    };
    let ps1 = paths.base.join("shell.ps1");
    let sh = paths.base.join("shell.sh");

    check_scripts(&mut report, &ps1, &sh, &me, git_bash.is_some());
    let targets = ShellTargets::for_user(Path::new(&home));
    check_profiles(&mut report, &targets, &ps1, &sh, git_bash.is_some());
    let runner = Shell::detect();
    check_status_lines(&mut report, &config, &me, shell, runner.as_ref());
    check_status_line_choice(&mut report, &paths, runner.as_ref());
    check_project_status_line(&mut report, &me);

    let engine = RotationEngine::new(shared::credentials(&paths, &home), shared::adapters());
    check_active_accounts(&mut report, &config, &engine, &paths);
    list_sessions(&report, &config, &engine);
    check_active_in_two_groups(&mut report, &config, &engine);
    check_links(&mut report, &config);
    check_environment(&mut report);
    check_claude(&mut report);

    report.finish()
}
