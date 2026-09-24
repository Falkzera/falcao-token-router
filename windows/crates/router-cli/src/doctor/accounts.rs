//! Quem serve cada grupo, as sessões vivas, conta em dois grupos e os links.
//!
//! Checagens do `router doctor` — a ordem está no `mod.rs` ao lado.

use super::*;

/// Quem serve cada grupo, e há quanto tempo foi medida.
pub(super) fn check_active_accounts(
    report: &mut Report,
    config: &RouterConfig,
    engine: &RotationEngine,
    paths: &RouterPaths,
) {
    let detail = GroupUsageReader::new(paths.usage_dir()).detail_by_account(config, Utc::now());
    for group in &config.groups {
        let Some(active) = engine.active_account(group, config) else {
            report.check(false, format!("grupo {}: nenhuma conta ativa", group.name));
            continue;
        };
        match detail.get(&active.id) {
            Some(usage) => {
                let minutes = (Utc::now() - usage.sampled_at).num_minutes();
                let source = match usage.origin {
                    UsageOrigin::Probe => "sondado",
                    UsageOrigin::Sensor => "sensor",
                };
                report.check(
                    minutes < STALE_SAMPLE_MINUTES,
                    format!(
                        "grupo {}: {} — {} ({source}, {minutes} min)",
                        group.name,
                        active.label(),
                        UsagePercent::text(usage.fraction)
                    ),
                );
            }
            None => report.check(
                true,
                format!(
                    "grupo {}: {} — sem amostra ainda (pronta)",
                    group.name,
                    active.label()
                ),
            ),
        }
    }
}

/// As sessões vivas, por grupo: é onde o modo de falha silencioso aparece —
/// sessão que devia estar num grupo e subiu no perfil padrão fica do lado errado.
pub(super) fn list_sessions(report: &Report, config: &RouterConfig, engine: &RotationEngine) {
    let _ = report;
    for group in &config.groups {
        let live = SessionRegistry::live_sessions(&group.config_dir);
        if live.is_empty() {
            continue;
        }
        let account = engine
            .active_account(group, config)
            .map_or("?", |a| a.label());
        println!(
            "  --    {} sessão(ões) em {} → {account}",
            live.len(),
            group.name
        );
        for session in live.iter().take(8) {
            println!(
                "          pid {}  {}  [{}]",
                session.pid,
                session.label(),
                describe(&session.status)
            );
        }
        if live.len() > 8 {
            println!("          … e mais {}", live.len() - 8);
        }
    }
}

/// Conta ativa em mais de um grupo: duas cópias de um refresh token que gira.
pub(super) fn check_active_in_two_groups(
    report: &mut Report,
    config: &RouterConfig,
    engine: &RotationEngine,
) {
    let mut seen: HashMap<Id, &str> = HashMap::new();
    for group in &config.groups {
        let Some(active) = engine.active_account(group, config) else {
            continue;
        };
        if let Some(other) = seen.get(&active.id) {
            report.check(
                false,
                format!(
                    "{} está ativa em DOIS grupos ({other} e {}) — risco de matar a credencial",
                    active.label(),
                    group.name
                ),
            );
        }
        seen.insert(active.id, &group.name);
    }
}

/// Links do compartilhamento quebrados (alvo sumiu). Faltando não é problema:
/// o próximo `claude <grupo>` liga o que faltar.
pub(super) fn check_links(report: &mut Report, config: &RouterConfig) {
    for group in config.groups.iter().filter(|g| !g.config_dir.is_default) {
        let dir = group.config_dir.path();
        let mut names: Vec<&str> = ProfileSharing::SHARED_DIRS.to_vec();
        names.extend(ProfileSharing::SHARED_FILES);
        if config.share_history {
            names.extend(ProfileSharing::HISTORY_DIRS);
            names.extend(ProfileSharing::HISTORY_FILES);
        }
        let broken: Vec<&str> = names
            .into_iter()
            .filter(|n| {
                let p = dir.join(n);
                fs::symlink_metadata(&p).is_ok() && fs::metadata(&p).is_err()
            })
            .collect();
        if !broken.is_empty() {
            report.check(
                false,
                format!(
                    "link quebrado no grupo {}: {} — apague-o; o próximo `claude {}` o refaz",
                    group.name,
                    broken.join(", "),
                    group.name
                ),
            );
        }
    }
}
