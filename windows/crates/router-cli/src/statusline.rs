//! O sensor: lê `rate_limits` do stdin, grava a amostra por conta e imprime a
//! linha do grupo (o layout mora no núcleo, `router_core::statusline::view`).
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
use std::path::Path;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use chrono::{DateTime, Local, TimeZone, Utc};
use serde::Deserialize;
use serde_json::{Map, Value};

use router_core::engine::anthropic_adapter::AnthropicAdapter;
use router_core::engine::config_dir::ConfigDir;
use router_core::engine::engine_lock::EngineLock;
use router_core::engine::group_usage::{GroupUsageSample, GroupUsageStore, UsageOrigin};
use router_core::engine::provider::ProviderAdapter;
use router_core::engine::router_paths::RouterPaths;
use router_core::platform::{paths, ui_language};
use router_core::statusline::view::{self as statusline_view, Context, Label, Style, View, Window};
use router_core::RouterConfig;

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

    // A linha: o grupo sai do `config.json`; o resto, do JSON do Claude Code.
    let config = shared::load_config(&paths);
    let view = view_from(
        &input,
        &dir,
        config.as_ref(),
        email.as_deref(),
        &home_dir(),
        &Local,
    );
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

/// Quem a linha nomeia: o grupo dono deste perfil (comparação de caminho sem
/// caixa nem estilo de barra; o grupo padrão casa pelo `~\.claude`). Fora de
/// um grupo, a conta — ou, sem conta, a pasta do perfil.
fn label_for(config: Option<&RouterConfig>, dir: &ConfigDir, email: Option<&str>) -> Label {
    let mine = paths::normalized(&dir.path());
    let owner = config.and_then(|c| {
        c.groups
            .iter()
            .enumerate()
            .find(|(_, g)| paths::normalized(&g.config_dir.path()) == mine)
    });
    if let Some((index, group)) = owner {
        return Label::Group {
            name: group.name.clone(),
            index,
        };
    }
    Label::Account(match email {
        Some(email) => email.split('@').next().unwrap_or(email).to_string(),
        None => dir
            .path()
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| "?".to_string()),
    })
}

/// O que a linha mostra, tirado do JSON do Claude Code (esquema da doc oficial
/// da status line: `model`, `effort.level`, `workspace`, `context_window`,
/// `rate_limits`, `cost`). O que não veio fica `None`.
fn view_from<Tz: TimeZone>(
    input: &Value,
    dir: &ConfigDir,
    config: Option<&RouterConfig>,
    email: Option<&str>,
    home: &str,
    tz: &Tz,
) -> View {
    let text = |pointer: &str| {
        input
            .pointer(pointer)
            .and_then(Value::as_str)
            .filter(|s| !s.is_empty())
            .map(String::from)
    };
    let count = |value: &Value, key: &str| value.get(key).and_then(Value::as_f64);
    let limits = input.get("rate_limits").and_then(Value::as_object);
    let window_of = |key: &str| {
        let (fraction, reset) = window(limits, key);
        fraction.map(|fraction| Window {
            fraction,
            resets_local: reset.map(|r| r.with_timezone(tz).naive_local()),
        })
    };
    // A pasta da sessão; sem ela (o prazo do stdin venceu), a do processo, que
    // o Claude Code abre na pasta da sessão.
    let cwd = text("/workspace/current_dir")
        .or_else(|| text("/workspace/project_dir"))
        .or_else(|| text("/cwd"))
        .or_else(|| {
            std::env::current_dir()
                .ok()
                .map(|p| p.to_string_lossy().into_owned())
        });
    let context = input.get("context_window").and_then(|c| {
        let size = count(c, "context_window_size").filter(|s| *s > 0.0)?;
        Some(Context {
            used_percent: count(c, "used_percentage").unwrap_or(0.0),
            input_tokens: count(c, "total_input_tokens").unwrap_or(0.0) as u64,
            window_size: size as u64,
        })
    });
    View {
        label: label_for(config, dir, email),
        model: text("/model/display_name").or_else(|| text("/model/id")),
        model_id: text("/model/id"),
        effort: text("/effort/level"),
        place: cwd.map(|cwd| {
            statusline_view::git_branch(Path::new(&cwd))
                .unwrap_or_else(|| statusline_view::shorten_path(&cwd, home))
        }),
        context,
        five_hour: window_of("five_hour"),
        seven_day: window_of("seven_day"),
        cost_usd: input
            .pointer("/cost/total_cost_usd")
            .and_then(Value::as_f64),
        email: email.map(String::from),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use router_core::engine::group_model::AccountGroup;
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

    fn config_with(groups: &[(&str, ConfigDir)]) -> RouterConfig {
        let mut config = RouterConfig::default();
        for (name, dir) in groups {
            config.groups.push(AccountGroup::new(*name, dir.clone()));
        }
        config
    }

    #[test]
    fn the_label_is_the_group_that_owns_the_profile() {
        let home = r"C:\Users\exemplo";
        let base = r"C:\Users\exemplo\AppData\Local\com.synqo.falcao-router\groups";
        let config = config_with(&[
            ("Pessoal", ConfigDir::dedicated(format!(r"{base}\A"))),
            ("Trabalho", ConfigDir::dedicated(format!(r"{base}\B"))),
            ("Principal", ConfigDir::standard(home)),
        ]);

        // Caixa e barras diferentes: é o mesmo perfil.
        let same = ConfigDir::dedicated(
            "c:/users/exemplo/appdata/local/com.synqo.falcao-router/groups/b/",
        );
        assert!(matches!(
            label_for(Some(&config), &same, Some("conta1@exemplo.com")),
            Label::Group { ref name, index: 1 } if name == "Trabalho"
        ));
        // O grupo padrão roda no `~\.claude`, sem variável.
        assert!(matches!(
            label_for(Some(&config), &ConfigDir::standard(home), None),
            Label::Group { index: 2, .. }
        ));
        // Fora de um grupo: a conta; sem conta, a pasta do perfil.
        let other = ConfigDir::dedicated(r"D:\outro\perfil");
        assert!(matches!(
            label_for(Some(&config), &other, Some("conta1@exemplo.com")),
            Label::Account(ref name) if name == "conta1"
        ));
        assert!(matches!(
            label_for(None, &other, None),
            Label::Account(ref name) if name == "perfil"
        ));
    }

    #[test]
    fn the_view_reads_what_claude_code_sends() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(tmp.path().join(".git")).unwrap();
        std::fs::write(
            tmp.path().join(".git").join("HEAD"),
            "ref: refs/heads/feat/x\n",
        )
        .unwrap();
        let input = json!({
            "model": {"id": "claude-opus-5-5", "display_name": "Opus 5.5 (1M context)"},
            "effort": {"level": "max"},
            "workspace": {"current_dir": tmp.path().to_string_lossy()},
            // `used_percentage` pode vir null no começo da sessão (doc oficial).
            "context_window": {
                "total_input_tokens": 511000, "context_window_size": 1000000,
                "used_percentage": null
            },
            "cost": {"total_cost_usd": 1.5},
            "rate_limits": {"seven_day": {"used_percentage": 41, "resets_at": 1790500000}}
        });
        let dir = ConfigDir::dedicated(r"D:\outro\perfil");
        let view = view_from(&input, &dir, None, Some("conta1@exemplo.com"), "", &Utc);

        assert_eq!(view.model.as_deref(), Some("Opus 5.5 (1M context)"));
        assert_eq!(view.model_id.as_deref(), Some("claude-opus-5-5"));
        assert_eq!(view.effort.as_deref(), Some("max"));
        assert_eq!(view.place.as_deref(), Some("feat/x"));
        let context = view.context.expect("contexto");
        assert_eq!(context.used_percent, 0.0);
        assert_eq!(
            (context.input_tokens, context.window_size),
            (511_000, 1_000_000)
        );
        assert_eq!(view.cost_usd, Some(1.5));
        assert!(view.five_hour.is_none());
        let seven = view.seven_day.expect("7d");
        assert_eq!(seven.fraction, 0.41);
        assert_eq!(
            seven.resets_local,
            Utc.timestamp_opt(1790500000, 0)
                .single()
                .map(|d| d.naive_utc())
        );
        assert_eq!(view.email.as_deref(), Some("conta1@exemplo.com"));
    }

    #[test]
    fn the_view_skips_what_did_not_come() {
        // Sem tamanho de janela, não há medidor de contexto; sem nada, só o rótulo.
        let input = json!({"context_window": {"used_percentage": 12}});
        let dir = ConfigDir::dedicated(r"D:\outro\perfil");
        let view = view_from(&input, &dir, None, None, "", &Utc);
        assert!(view.context.is_none());
        assert!(view.model.is_none() && view.effort.is_none() && view.cost_usd.is_none());
        assert!(view.five_hour.is_none() && view.seven_day.is_none());
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
