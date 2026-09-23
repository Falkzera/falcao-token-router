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

use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::process::Command;
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
    check_status_lines(&mut report, &config, &me, shell, git_bash.as_deref());
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

/// Os scripts existem e citam ESTE binário?
fn check_scripts(report: &mut Report, ps1: &Path, sh: &Path, me: &Path, with_bash: bool) {
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
fn check_profiles(
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
fn check_bash_profile(report: &mut Report, home: &Path) {
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

/// O sensor de cada grupo: instalado com o comando certo — e rodando de verdade
/// pelo shell que o Claude Code vai usar (o que pega aspas e caminhos errados).
fn check_status_lines(
    report: &mut Report,
    config: &RouterConfig,
    me: &Path,
    shell: StatusShell,
    git_bash: Option<&Path>,
) {
    let shell_name = match shell {
        StatusShell::Bash => "Git Bash",
        StatusShell::PowerShell => "PowerShell",
    };
    for group in &config.groups {
        let expected = ShellIntegration::status_line_for_profile(me, &group.config_dir, shell);
        if ShellIntegration::status_line_is_stale(&expected, &group.config_dir) {
            report.check(
                false,
                format!(
                    "sensor no grupo {}: ausente ou apontando para outro binário (o próximo `claude {}` o instala)",
                    group.name, group.name
                ),
            );
            continue;
        }
        match run_status_line(&expected, git_bash, &group.config_dir) {
            Ok(()) => report.check(
                true,
                format!(
                    "sensor no grupo {}: instalado e rodando pelo {shell_name}",
                    group.name
                ),
            ),
            Err(why) => report.check(
                false,
                format!(
                    "sensor no grupo {}: instalado, mas não roda pelo {shell_name}: {why}",
                    group.name
                ),
            ),
        }
    }
}

fn run_status_line(command: &str, git_bash: Option<&Path>, dir: &ConfigDir) -> Result<(), String> {
    let mut process = match git_bash {
        Some(bash) => {
            let mut c = Command::new(bash);
            c.arg("-c").arg(command);
            c
        }
        None => {
            let mut c = Command::new("powershell.exe");
            c.args(["-NoProfile", "-NonInteractive", "-Command", command]);
            c
        }
    };
    match dir.environment_value() {
        Some(value) => process.env("CLAUDE_CONFIG_DIR", value),
        None => process.env_remove("CLAUDE_CONFIG_DIR"),
    };
    // `{}`: sem `rate_limits`, o sensor imprime a linha e NÃO grava amostra.
    match shared::run_with_timeout(process, Some(b"{}"), Duration::from_secs(20)) {
        Some((Some(0), out)) if !out.trim().is_empty() => Ok(()),
        Some((code, _)) => Err(format!("saiu com o código {code:?} ou sem imprimir nada")),
        None => Err("não respondeu em 20 s".to_string()),
    }
}

/// Uma `statusLine` de PROJETO vence a do grupo (visto no spike: rodando da
/// home, o `~\.claude\settings.json` conta como settings de projeto) — e as
/// sessões abertas ali não são medidas pelo sensor.
fn check_project_status_line(report: &mut Report, me: &Path) {
    let Ok(cwd) = std::env::current_dir() else {
        return;
    };
    let me_text = me.to_string_lossy().replace('\\', "/").to_lowercase();
    for name in ["settings.json", "settings.local.json"] {
        let path = cwd.join(".claude").join(name);
        let Some(command) = read_retrying(&path)
            .ok()
            .and_then(|b| serde_json::from_slice::<serde_json::Value>(&b).ok())
            .and_then(|v| {
                v.get("statusLine")?
                    .get("command")?
                    .as_str()
                    .map(String::from)
            })
        else {
            continue;
        };
        if command.to_lowercase().contains(&me_text) {
            continue; // é a do próprio router (grupo padrão)
        }
        report.check(
            false,
            format!(
                "uma statusLine de projeto em {} vence a do grupo quando o claude roda nesta pasta — as sessões daqui não são medidas pelo sensor",
                path.display()
            ),
        );
    }
}

/// Quem serve cada grupo, e há quanto tempo foi medida.
fn check_active_accounts(
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
fn list_sessions(report: &Report, config: &RouterConfig, engine: &RotationEngine) {
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
fn check_active_in_two_groups(report: &mut Report, config: &RouterConfig, engine: &RotationEngine) {
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
fn check_links(report: &mut Report, config: &RouterConfig) {
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

/// Variáveis que desviam a sessão da conta do grupo. As sessões de grupo as
/// removem; o `claude` puro (grupo padrão) não. Só os NOMES são mostrados.
fn check_environment(report: &mut Report) {
    let mut names: Vec<String> = std::env::vars_os()
        .map(|(k, _)| k)
        .filter(|k| ProviderEnv::redirects(k))
        .map(|k| k.to_string_lossy().into_owned())
        .collect();
    names.sort();
    if names.is_empty() {
        report.check(true, "nenhuma variável de proxy ou credencial no ambiente");
    } else {
        report.check(
            false,
            format!(
                "variáveis que desviam a sessão estão no ambiente: {} — as sessões de grupo as removem, mas o `claude` puro não",
                names.join(", ")
            ),
        );
    }
}

fn check_claude(report: &mut Report) {
    let Some(claude) = ClaudeBinary::locate() else {
        report.check(
            false,
            "binário `claude` não encontrado — o `launch` e a sonda precisam dele",
        );
        return;
    };
    let mut command = claude.command();
    command.arg("--version");
    let version = shared::run_with_timeout(command, None, Duration::from_secs(30))
        .and_then(|(code, out)| (code == Some(0)).then_some(out))
        .and_then(|out| out.lines().next().map(|l| l.trim().to_string()))
        .filter(|l| !l.is_empty())
        .unwrap_or_else(|| "versão desconhecida".to_string());
    report.check(true, format!("claude: {} ({version})", claude.describe()));
}
