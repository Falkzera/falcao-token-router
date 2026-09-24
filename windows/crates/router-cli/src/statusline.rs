//! O sensor: lê `rate_limits` do stdin, grava a amostra por conta e imprime a
//! linha do grupo (o layout mora no núcleo, `router_core::statusline`).
//! Nenhuma chamada de rede, nenhum token nosso.
//!
//! Mudanças do Windows sobre o macOS:
//! - stdin **pode nunca fechar** (medido: processos pendurados), então o leitor
//!   pega o 1º JSON completo e tem prazo; nunca espera EOF.
//! - grava a amostra **só se houver ao menos uma janela** — no macOS uma amostra
//!   sem janela apagava a última leitura (a conta cheia virava "pronta").
//! - o perfil vem do `CLAUDE_CONFIG_DIR`; se a variável não chegar (subprocesso
//!   com ambiente raspado), do `--profile` que a status line do grupo embute.
//! - respeita o `ROUTER_APP_SUPPORT` (o sensor do macOS o ignora).

use std::io::Write;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use chrono::{Local, Utc};
use serde::Deserialize;
use serde_json::Value;

use router_core::engine::anthropic_adapter::AnthropicAdapter;
use router_core::engine::config_dir::ConfigDir;
use router_core::engine::engine_lock::EngineLock;
use router_core::engine::group_usage::{GroupUsageSample, GroupUsageStore, UsageOrigin};
use router_core::engine::provider::ProviderAdapter;
use router_core::engine::router_paths::RouterPaths;
use router_core::platform::ui_language;
use router_core::statusline::choice::StatusLineChoice;
use router_core::statusline::command::{self, Outcome, Shell};
use router_core::statusline::session::{view_from, window};
use router_core::statusline::view::{self as statusline_view, Style};

use crate::shared;

/// Prazo do leitor de stdin. O sensor real precisa ser rápido e nunca pendurar.
const DEADLINE_MS: u64 = 250;

/// Quanto esperar pela trava do motor antes de gravar mesmo assim: a status line
/// não pode esperar uma troca de conta terminar.
const LOCK_WAIT: Duration = Duration::from_millis(100);

pub fn run(args: &[String]) {
    let input = read_stdin_with_deadline();
    let dir = current_config_dir(profile_arg(args).as_deref());
    // Identidade do PERFIL (o `.claude.json`), nunca do stdin.
    let email = AnthropicAdapter.identity(&dir).map(|id| id.email);

    let limits = input.get("rate_limits").and_then(Value::as_object);
    let (five_pct, five_reset) = window(limits, "five_hour");
    let (seven_pct, seven_reset) = window(limits, "seven_day");

    // Grava a amostra SÓ se houver e-mail E ao menos uma janela.
    let paths = RouterPaths::new();
    if let Some(email) = &email {
        if five_pct.is_some() || seven_pct.is_some() {
            let sample = GroupUsageSample::new(
                dir.raw.clone(),
                Some(email.clone()),
                five_pct,
                five_reset,
                seven_pct,
                seven_reset,
                Utc::now(),
                None,
                UsageOrigin::Sensor,
            );
            let _lock = EngineLock::acquire(&paths.base, LOCK_WAIT);
            let _ = GroupUsageStore::write(&sample, email, &paths.usage_dir());
        }
    }

    // A linha, pela escolha do usuário (lida a cada render: a mudança vale na
    // próxima atualização das sessões abertas). No modo comando, a dele.
    let choice = StatusLineChoice::load(&paths.status_line_file());
    if let Some(line) = users_line(&choice, &input) {
        let _ = std::io::stdout().write_all(&line);
        std::process::exit(0);
    }
    // A do app: o grupo sai do `config.json`; o resto, do JSON do Claude Code.
    let config = shared::load_config(&paths);
    let view = choice.apply(view_from(
        &input,
        &dir,
        config.as_ref(),
        email.as_deref(),
        &home_dir(),
        &Local,
    ));
    let style = Style {
        truecolor: statusline_view::truecolor(|key| std::env::var(key).ok()),
        portuguese: ui_language::portuguese_ui(),
        phase: Utc::now().timestamp().max(0) as u64,
    };
    let line = statusline_view::render(&view, &style);
    // `write!` e ignora erro: o Claude Code cancela o script no meio quando chega
    // um novo update, fechando o pipe — um panic aqui poluiria o terminal.
    let _ = writeln!(std::io::stdout(), "{line}");
    std::process::exit(0);
}

/// O modo "meu comando": a linha do comando do usuário, rodado com o MESMO
/// JSON pelo shell que o Claude Code usaria. `None` — vale a linha do app —
/// fora desse modo, sem o JSON (o prazo do stdin venceu), dentro do próprio
/// comando (um router chamado por ele não o roda de novo) e quando ele falha.
fn users_line(choice: &StatusLineChoice, input: &Value) -> Option<Vec<u8>> {
    let command = choice.command_to_run()?;
    if input.is_null() || std::env::var_os(command::CHAINED_ENV).is_some() {
        return None;
    }
    let shell = Shell::detect()?;
    let json = serde_json::to_vec(input).ok()?;
    match command::run(&shell, command, &json, command::DEADLINE) {
        Outcome::Printed(line) => Some(line),
        _ => None,
    }
}

/// O valor de `--profile <perfil>`, se veio.
fn profile_arg(args: &[String]) -> Option<String> {
    let at = args.iter().position(|a| a == "--profile")?;
    args.get(at + 1).filter(|p| !p.is_empty()).cloned()
}

/// O perfil de onde a amostra vem: `CLAUDE_CONFIG_DIR` não-vazio → dedicado;
/// senão o `--profile` embutido; senão o padrão (`<home>\.claude`).
fn current_config_dir(profile: Option<&str>) -> ConfigDir {
    match std::env::var("CLAUDE_CONFIG_DIR") {
        Ok(raw) if !raw.is_empty() => ConfigDir::dedicated(raw),
        _ => match profile {
            Some(p) => ConfigDir::dedicated(p),
            None => ConfigDir::standard(&home_dir()),
        },
    }
}

fn home_dir() -> String {
    std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .unwrap_or_default()
}

/// Lê o 1º JSON completo do stdin, com prazo. Não espera EOF — o `serde_json`
/// devolve assim que o valor fecha, e um cão de guarda cobre o stdin que nunca
/// fecha (medido no Windows).
fn read_stdin_with_deadline() -> Value {
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let stdin = std::io::stdin();
        let mut de = serde_json::Deserializer::from_reader(stdin.lock());
        let value = Value::deserialize(&mut de).ok();
        let _ = tx.send(value);
    });
    match rx.recv_timeout(Duration::from_millis(DEADLINE_MS)) {
        Ok(Some(v)) => v,
        _ => Value::Null,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_profile_argument_is_read_after_the_flag() {
        let args = |v: &[&str]| v.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        assert_eq!(
            profile_arg(&args(&["--profile", "C:/Users/exemplo/g"])).as_deref(),
            Some("C:/Users/exemplo/g")
        );
        assert_eq!(profile_arg(&args(&["--profile"])), None);
        assert_eq!(profile_arg(&args(&[])), None);
    }
}
