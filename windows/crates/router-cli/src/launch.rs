//! `router launch`, `router is-group` e `router rotate`.
//!
//! O `launch` do macOS faz `execvp` e some; no Windows não há `exec`: o `router`
//! sobe o `claude` como filho herdando o console, ignora Ctrl+C enquanto ele roda
//! (o `claude` trata os dele) e sai com o código de saída do filho.

use std::ffi::OsStr;

use chrono::Utc;

use router_core::engine::engine_lock::EngineLock;
use router_core::engine::group_usage_reader::GroupUsageReader;
use router_core::engine::profile_sharing::ProfileSharing;
use router_core::engine::provider_env::ProviderEnv;
use router_core::engine::rotation_engine::RotationEngine;
use router_core::engine::router_paths::RouterPaths;
use router_core::engine::session_launcher::SessionLauncher;
use router_core::engine::shell_integration::ShellIntegration;
use router_core::platform::console;
use router_core::usage::claude_binary::ClaudeBinary;
use router_core::ConfigDir;

use crate::shared::{self, fail};

/// Código 0 se `<nome>` é um grupo (sem caixa, sem espaço nas pontas). Mudo:
/// a função de shell só quer o código.
pub fn is_group(name: Option<&str>) -> bool {
    let Some(name) = name else {
        return false;
    };
    let Some(config) = shared::load_config(&RouterPaths::new()) else {
        return false;
    };
    SessionLauncher::group_named(name, &config).is_some()
}

/// `router launch <grupo> [--] [args do claude]`. Aceita as duas formas porque
/// uma FUNÇÃO do PowerShell engole o `--` do `$args` (medido nas duas edições):
/// tudo depois de `--`, ou, sem ele, tudo depois do grupo.
pub fn run(argv: &[String]) {
    let Some(name) = argv.first() else {
        fail("uso: router launch <grupo>");
    };
    let paths = RouterPaths::new();
    let Some(config) = shared::load_config(&paths) else {
        fail("configuração não encontrada — crie um grupo no app");
    };
    let passthrough: Vec<String> = match argv.iter().position(|a| a == "--") {
        Some(dash) => argv[dash + 1..].to_vec(),
        None => argv[1..].to_vec(),
    };

    let home = shared::home();
    let launcher = SessionLauncher::new(shared::credentials(&paths, &home), shared::adapters());
    let Some(group) = SessionLauncher::group_named(name, &config).cloned() else {
        fail(&format!("grupo desconhecido: {name}"));
    };
    let usage = GroupUsageReader::new(paths.usage_dir()).usage_by_account(&config, Utc::now());

    // A ativação escreve credencial: sob a trava, que sai antes da sessão subir.
    let plan = {
        let _lock = EngineLock::acquire(&paths.base, EngineLock::WAIT);
        launcher
            .prepare(&group, &config, &usage, passthrough)
            .unwrap_or_else(|e| fail(&e.to_string()))
    };

    // Garante o sensor no perfil do grupo, para ESTA sessão já registrar o uso,
    // preservando o `settings.json` que houver.
    if let Some(me) = shared::self_path() {
        let command = ShellIntegration::status_line_for_profile(
            &me,
            &group.config_dir,
            shared::detected_shell(),
        );
        let _ = ShellIntegration::install_status_line(&command, &group.config_dir);
    }
    // Histórico e ferramentas do `~\.claude`, para o `--resume` enxergar tudo.
    ProfileSharing::link(
        &group.config_dir,
        &ConfigDir::standard(&home),
        config.share_history,
        &paths.base,
    );

    let Some(claude) = ClaudeBinary::locate() else {
        fail("não foi possível executar claude: binário não encontrado");
    };
    // O ambiente, SEM nada que desvie a sessão da conta do grupo; o perfil só no
    // dedicado (no padrão a variável SAI — setá-la no caminho padrão sobe
    // deslogado).
    let env = ProviderEnv::with_var(
        ProviderEnv::direct(std::env::vars_os()),
        "CLAUDE_CONFIG_DIR",
        plan.config_dir_env.as_deref().map(OsStr::new),
    );
    eprintln!("→ {}: {}", group.name, plan.account.label());

    console::ignore_interrupts_in_this_process();
    let mut command = claude.command();
    command.args(&plan.arguments).env_clear().envs(env);
    match command.status() {
        Ok(status) => std::process::exit(status.code().unwrap_or(1)),
        Err(e) => fail(&format!("não foi possível executar claude: {e}")),
    }
}

/// Uma volta da rotação em todos os grupos (para um agendador): espelha a
/// ativa e troca se ela passou do limiar e há destino. Mudo, código 0.
pub fn rotate() {
    let paths = RouterPaths::new();
    let Some(config) = shared::load_config(&paths) else {
        return;
    };
    let home = shared::home();
    let engine = RotationEngine::new(shared::credentials(&paths, &home), shared::adapters());
    let usage = GroupUsageReader::new(paths.usage_dir()).usage_by_account(&config, Utc::now());
    for group in &config.groups {
        let _lock = EngineLock::acquire(&paths.base, EngineLock::WAIT);
        engine.mirror_active(group, &config);
        if let Some(target) = engine.rotation_target(group, &config, &usage) {
            let _ = engine.activate(target, group, &config);
        }
    }
}
