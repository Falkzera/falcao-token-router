//! A status line — o sensor — rodando de verdade pelo shell que o Claude Code usa.
//!
//! Checagens do `router doctor` — a ordem está no `mod.rs` ao lado.

use super::*;

/// O sensor de cada grupo: instalado com o comando certo — e rodando de verdade
/// pelo shell que o Claude Code vai usar (o que pega aspas e caminhos errados).
pub(super) fn check_status_lines(
    report: &mut Report,
    config: &RouterConfig,
    me: &Path,
    shell: StatusShell,
    runner: Option<&Shell>,
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
        let Some(runner) = runner else {
            report.check(
                false,
                format!(
                    "sensor no grupo {}: nem Git Bash nem PowerShell para rodá-lo",
                    group.name
                ),
            );
            continue;
        };
        match run_status_line(&expected, runner, &group.config_dir) {
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

/// Roda o sensor pelo shell e do jeito que o Claude Code o roda.
pub(super) fn run_status_line(
    command: &str,
    runner: &Shell,
    dir: &ConfigDir,
) -> Result<(), String> {
    let mut process = runner.process(command, |key| std::env::var(key).ok());
    // Só o sensor: com a marca de encadeado, o router nunca roda o comando do
    // usuário (ele tem linha própria, `check_status_line_choice`).
    process.env(command::CHAINED_ENV, "1");
    match dir.environment_value() {
        Some(value) => process.env("CLAUDE_CONFIG_DIR", value),
        None => process.env_remove("CLAUDE_CONFIG_DIR"),
    };
    // `{}`: sem `rate_limits`, o sensor imprime a linha e NÃO grava amostra. Vale
    // o código 0 — a linha pode sair vazia (o usuário tirou todos os itens).
    match shared::run_with_timeout(process, Some(b"{}"), Duration::from_secs(20)) {
        Some((Some(0), _)) => Ok(()),
        Some((code, _)) => Err(format!("saiu com o código {code:?}")),
        None => Err("não respondeu em 20 s".to_string()),
    }
}

/// O que as sessões dos grupos mostram (a escolha da aba Ajustes) — e, no modo
/// comando, se o comando do usuário imprime algo com uma sessão de exemplo.
pub(super) fn check_status_line_choice(
    report: &mut Report,
    paths: &RouterPaths,
    runner: Option<&Shell>,
) {
    let choice = StatusLineChoice::load(&paths.status_line_file());
    let command = match (choice.mode, choice.command_to_run()) {
        (Mode::App, _) => {
            report.check(
                true,
                format!("status line: a linha do app, {}", items_text(&choice)),
            );
            return;
        }
        (Mode::Command, None) => {
            report.check(
                true,
                "status line: modo comando sem comando — vale a linha do app",
            );
            return;
        }
        (Mode::Command, Some(command)) => command.to_string(),
    };
    let Some(runner) = runner else {
        report.check(
            false,
            "status line: nem Git Bash nem PowerShell para rodar o seu comando — as sessões mostram a linha do app",
        );
        return;
    };
    let cwd = std::env::current_dir()
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_default();
    let sample = session::sample(&cwd, Utc::now()).to_string();
    let why = match command::run(runner, &command, sample.as_bytes(), command::DEADLINE) {
        Outcome::Printed(_) => {
            report.check(
                true,
                "status line: o seu comando imprime (numa sessão de exemplo)",
            );
            return;
        }
        Outcome::Failed { code: Some(0), .. } => "não imprimiu nada".to_string(),
        Outcome::Failed { code, stderr } => {
            let first = stderr.lines().next().unwrap_or_default();
            format!("saiu com o código {code:?} {first}")
                .trim_end()
                .to_string()
        }
        Outcome::NotStarted(e) => format!("não subiu ({e})"),
        Outcome::TimedOut => format!("passou de {} s", command::DEADLINE.as_secs()),
    };
    report.check(
        false,
        format!("status line: o seu comando {why} — as sessões mostram a linha do app"),
    );
}

/// "completa", "sem nenhum item (vazia)" ou "sem contexto, custo".
pub(super) fn items_text(choice: &StatusLineChoice) -> String {
    let hidden = choice.hidden();
    if hidden.is_empty() {
        return "completa".to_string();
    }
    if hidden.len() == Item::ALL.len() {
        return "sem nenhum item (vazia)".to_string();
    }
    let names: Vec<&str> = hidden
        .iter()
        .map(|item| match item {
            Item::Group => "grupo",
            Item::Model => "modelo",
            Item::Effort => "esforço",
            Item::Place => "branch/pasta",
            Item::Context => "contexto",
            Item::FiveHour => "janela de 5h",
            Item::SevenDay => "janela de 7d",
            Item::Resets => "horário do reset",
            Item::Cost => "custo",
            Item::Email => "e-mail",
        })
        .collect();
    format!("sem {}", names.join(", "))
}

/// Uma `statusLine` de PROJETO vence a do grupo (visto no spike: rodando da
/// home, o `~\.claude\settings.json` conta como settings de projeto) — e as
/// sessões abertas ali não são medidas pelo sensor.
pub(super) fn check_project_status_line(report: &mut Report, me: &Path) {
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
