//! `router measure [grupo]` — a SONDA: mede as contas perguntando ao binário
//! oficial (`claude /usage`).
//!
//! O sensor passivo só enxerga quem está servindo, e só as janelas de 5h e 7
//! dias. A sonda mede conta OCIOSA e traz o limite POR MODELO. Custa um processo
//! e uma requisição por conta, então é comando, não laço.

use chrono::Utc;

use router_core::engine::rotation_engine::RotationEngine;
use router_core::engine::router_paths::RouterPaths;
use router_core::engine::session_launcher::SessionLauncher;
use router_core::usage::claude_binary::ClaudeBinary;
use router_core::usage::claude_usage_probe::{ClaudeUsageProbe, ProbeError, ProbeTarget, Reading};
use router_core::usage::usage_percent::UsagePercent;
use router_core::{Account, AccountGroup};

use crate::shared;

/// `true` se todas as contas responderam.
pub fn run(argv: &[String]) -> bool {
    let paths = RouterPaths::new();
    let Some(config) = shared::load_config(&paths) else {
        eprintln!("router: configuração não encontrada");
        return false;
    };
    let Some(claude) = ClaudeBinary::locate() else {
        eprintln!("router: binário `claude` não encontrado — a sonda precisa dele");
        return false;
    };
    let groups: Vec<&AccountGroup> = match argv.first() {
        Some(name) => match SessionLauncher::group_named(name, &config) {
            Some(group) => vec![group],
            None => {
                eprintln!("router: grupo desconhecido: {name}");
                return false;
            }
        },
        None => config.groups.iter().collect(),
    };
    // Contas dos grupos pedidos, sem repetir (uma conta pode estar em dois).
    let mut accounts: Vec<&Account> = Vec::new();
    for group in groups {
        for account in config.accounts_in(group) {
            if !accounts.iter().any(|a| a.id == account.id) {
                accounts.push(account);
            }
        }
    }
    if accounts.is_empty() {
        println!("nenhuma conta para medir.");
        return true;
    }

    let home = shared::home();
    let engine = RotationEngine::new(shared::credentials(&paths, &home), shared::adapters());
    let probe = ClaudeUsageProbe::system(claude, paths.base.clone());
    let usage_dir = paths.usage_dir();
    let mut failures = 0;
    for account in accounts {
        // A regra que não pode ser quebrada: conta ativa é sondada pelo perfil
        // do GRUPO, nunca pela casa.
        let target = ProbeTarget {
            account_id: account.id,
            label: account.label().to_string(),
            email: account.identity.email.clone(),
            dir: engine.probe_config_dir(account, &config),
        };
        match probe.measure_into(&target, &usage_dir, &paths.base, Utc::now()) {
            Ok(reading) => println!("  {}: {}", target.label, describe(&reading)),
            Err(ProbeError::NotSignedIn) => {
                println!(
                    "  {}: sem login neste perfil — use Relogar no app",
                    target.label
                );
                failures += 1;
            }
            Err(e) => {
                println!("  {}: falhou ({e})", target.label);
                failures += 1;
            }
        }
    }
    failures == 0
}

fn describe(reading: &Reading) -> String {
    let mut parts: Vec<String> = Vec::new();
    if let Some(s) = reading.session {
        parts.push(format!("5h {}", UsagePercent::text(s)));
    }
    if let Some(w) = reading.weekly_all {
        parts.push(format!("7d {}", UsagePercent::text(w)));
    }
    for model in &reading.models {
        parts.push(format!(
            "{} {}",
            model.name,
            UsagePercent::text(model.percent)
        ));
    }
    if parts.is_empty() {
        "sem janelas".to_string()
    } else {
        parts.join("  ")
    }
}
