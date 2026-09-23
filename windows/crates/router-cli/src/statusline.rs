//! O sensor: lê `rate_limits` do stdin, grava a amostra por conta e imprime a
//! linha colorida. Nenhuma chamada de rede, nenhum token nosso.
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

use chrono::{DateTime, TimeZone, Utc};
use serde::Deserialize;
use serde_json::{Map, Value};

use router_core::engine::anthropic_adapter::AnthropicAdapter;
use router_core::engine::config_dir::ConfigDir;
use router_core::engine::engine_lock::EngineLock;
use router_core::engine::group_usage::{GroupUsageSample, GroupUsageStore, UsageOrigin};
use router_core::engine::provider::ProviderAdapter;
use router_core::engine::router_paths::RouterPaths;
use router_core::usage::usage_percent::UsagePercent;

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
            let paths = RouterPaths::new();
            let _lock = EngineLock::acquire(&paths.base, LOCK_WAIT);
            let _ = GroupUsageStore::write(&sample, email, &paths.usage_dir());
        }
    }

    let line = render_line(email.as_deref(), five_pct, seven_pct);
    // `write!` e ignora erro: o Claude Code cancela o script no meio quando chega
    // um novo update, fechando o pipe — um panic aqui poluiria o terminal.
    let _ = writeln!(std::io::stdout(), "{line}");
    std::process::exit(0);
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

/// Extrai `(fração 0–1, reset)` de uma janela do `rate_limits`. `used_percentage`
/// pode ser inteiro ou decimal; `resets_at` é epoch em segundos.
fn window(limits: Option<&Map<String, Value>>, key: &str) -> (Option<f64>, Option<DateTime<Utc>>) {
    let Some(w) = limits.and_then(|l| l.get(key)).and_then(Value::as_object) else {
        return (None, None);
    };
    let pct = w
        .get("used_percentage")
        .and_then(Value::as_f64)
        .map(|p| p / 100.0);
    let reset = w
        .get("resets_at")
        .and_then(Value::as_f64)
        .and_then(|s| Utc.timestamp_opt(s as i64, 0).single());
    (pct, reset)
}

/// A linha colorida: `<label negrito>  5h <barra> N%  7d N%` — ou "sem uso ainda".
fn render_line(email: Option<&str>, five: Option<f64>, seven: Option<f64>) -> String {
    let mut parts: Vec<String> = Vec::new();
    let label = email
        .map(|e| e.split('@').next().unwrap_or(e))
        .unwrap_or("?");
    parts.push(format!("\x1b[1m{label}\x1b[0m"));
    if let Some(f) = five {
        parts.push(colored(
            f,
            &format!("5h {} {}%", bar(Some(f), 8), UsagePercent::value(f)),
        ));
    }
    if let Some(s) = seven {
        parts.push(colored(s, &format!("7d {}%", UsagePercent::value(s))));
    }
    if parts.len() == 1 {
        parts.push("\x1b[90msem uso ainda\x1b[0m".to_string());
    }
    parts.join("  ")
}

/// Barra de 8 células: `min(8, round(f*8))` de `█`, resto `░`.
fn bar(f: Option<f64>, width: usize) -> String {
    match f {
        None => "?".repeat(width),
        Some(f) => {
            let filled = ((f * width as f64).round() as i64).clamp(0, width as i64) as usize;
            "█".repeat(filled) + &"░".repeat(width - filled)
        }
    }
}

/// Cor por severidade: ≥0,90 vermelho (31), ≥0,70 amarelo (33), senão verde (32).
fn colored(f: f64, text: &str) -> String {
    let code = if f >= 0.90 {
        31
    } else if f >= 0.70 {
        33
    } else {
        32
    };
    format!("\x1b[{code}m{text}\x1b[0m")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn limits_of(v: &Value) -> Option<&Map<String, Value>> {
        v.get("rate_limits").and_then(Value::as_object)
    }

    #[test]
    fn window_reads_int_and_float_percentages() {
        let v = json!({"rate_limits": {
            "five_hour": {"used_percentage": 42, "resets_at": 1790200000},
            "seven_day": {"used_percentage": 7.5, "resets_at": 1790500000}
        }});
        let (f, fr) = window(limits_of(&v), "five_hour");
        let (s, _) = window(limits_of(&v), "seven_day");
        assert_eq!(f, Some(0.42));
        assert_eq!(s, Some(0.075));
        assert_eq!(fr, Utc.timestamp_opt(1790200000, 0).single());
    }

    #[test]
    fn window_absent_is_none() {
        let v = json!({"rate_limits": {}});
        assert_eq!(window(limits_of(&v), "five_hour"), (None, None));
        // rate_limits ausente por completo
        let empty = json!({});
        assert_eq!(window(limits_of(&empty), "five_hour"), (None, None));
    }

    #[test]
    fn render_no_windows_says_ready() {
        let line = render_line(Some("conta1@exemplo.com"), None, None);
        assert!(line.contains("conta1"));
        assert!(line.contains("sem uso ainda"));
        assert!(!line.contains("5h"));
    }

    #[test]
    fn render_colors_by_severity() {
        // 0,95 → vermelho (31) na barra de 5h
        assert!(render_line(Some("a@b"), Some(0.95), None).contains("\x1b[31m"));
        // 0,80 → amarelo (33)
        assert!(render_line(Some("a@b"), Some(0.80), None).contains("\x1b[33m"));
        // 0,10 → verde (32)
        assert!(render_line(Some("a@b"), Some(0.10), None).contains("\x1b[32m"));
    }

    #[test]
    fn bar_fills_proportionally() {
        assert_eq!(bar(Some(0.0), 8), "░░░░░░░░");
        assert_eq!(bar(Some(1.0), 8), "████████");
        // 0,5 * 8 = 4
        assert_eq!(bar(Some(0.5), 8), "████░░░░");
        // arredonda: 0,44 * 8 = 3,52 → 4
        assert_eq!(bar(Some(0.44), 8), "████░░░░");
    }

    #[test]
    fn label_is_local_part_or_question_mark() {
        assert!(render_line(Some("equipe1@exemplo.com"), None, None).contains("equipe1"));
        assert!(render_line(None, None, None).contains('?'));
    }

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
