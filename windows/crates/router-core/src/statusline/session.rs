//! O que o Claude Code entrega à status line (o JSON do esquema da doc oficial)
//! e o que a linha tira dele. A CLI monta a `View` da sessão viva com isto; a
//! prévia dos Ajustes, com a sessão de EXEMPLO (`sample`) — o mesmo caminho.

use std::path::Path;

use chrono::{DateTime, TimeZone, Utc};
use serde_json::{Map, Value};

use super::view::{self, Context, Label, View, Window};
use crate::engine::config_dir::ConfigDir;
use crate::engine::group_model::RouterConfig;
use crate::platform::paths;

/// Extrai `(fração 0–1, reset)` de uma janela do `rate_limits`. `used_percentage`
/// pode ser inteiro ou decimal; `resets_at` é epoch em segundos.
pub fn window(
    limits: Option<&Map<String, Value>>,
    key: &str,
) -> (Option<f64>, Option<DateTime<Utc>>) {
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
pub fn label_for(config: Option<&RouterConfig>, dir: &ConfigDir, email: Option<&str>) -> Label {
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
pub fn view_from<Tz: TimeZone>(
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
        label: Some(label_for(config, dir, email)),
        model: text("/model/display_name").or_else(|| text("/model/id")),
        model_id: text("/model/id"),
        effort: text("/effort/level"),
        place: cwd.map(|cwd| {
            view::git_branch(Path::new(&cwd)).unwrap_or_else(|| view::shorten_path(&cwd, home))
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

/// Uma sessão de EXEMPLO no formato do JSON da status line, na pasta `cwd`: a
/// prévia dos Ajustes desenha com ela, e o "Testar" (e o `doctor`) a entrega ao
/// comando do usuário para ver se ele imprime algo. Números de exemplo; os
/// resets caem no futuro de `now`, dentro das janelas.
pub fn sample(cwd: &str, now: DateTime<Utc>) -> Value {
    let at = |seconds: i64| (now.timestamp() + seconds) / 60 * 60;
    serde_json::json!({
        "hook_event_name": "Status",
        "session_id": "00000000-0000-4000-8000-000000000000",
        "cwd": cwd,
        "model": {"id": "claude-opus-5-5", "display_name": "Opus 5.5"},
        "workspace": {"current_dir": cwd, "project_dir": cwd},
        "version": "2.1.280",
        "output_style": {"name": "default"},
        "cost": {
            "total_cost_usd": 1.87,
            "total_duration_ms": 1_380_000,
            "total_api_duration_ms": 312_000,
            "total_lines_added": 128,
            "total_lines_removed": 37
        },
        "context_window": {
            "total_input_tokens": 51_200,
            "total_output_tokens": 3_100,
            "context_window_size": 200_000,
            "used_percentage": 25.6,
            "remaining_percentage": 74.4
        },
        "exceeds_200k_tokens": false,
        "rate_limits": {
            "five_hour": {"used_percentage": 29, "resets_at": at(2 * 3600 + 13 * 60)},
            "seven_day": {"used_percentage": 33, "resets_at": at(4 * 86400 + 5 * 3600)}
        },
        "effort": {"level": "high"}
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::group_model::AccountGroup;
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

    // A sessão de exemplo (a prévia e o "Testar" dos Ajustes, o `doctor`).

    fn now() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 9, 23, 12, 0, 0).unwrap()
    }

    /// Tem o que a linha completa mostra — nenhum item fica de fora da prévia.
    #[test]
    fn the_sample_fills_every_item_of_the_line() {
        let tmp = tempfile::tempdir().unwrap();
        let cwd = tmp.path().join("projetos").join("app");
        let sample = sample(&cwd.to_string_lossy(), now());
        let dir = ConfigDir::dedicated(r"D:\outro\perfil");
        let view = view_from(&sample, &dir, None, Some("conta1@exemplo.com"), "", &Utc);
        assert_eq!(view.model.as_deref(), Some("Opus 5.5"));
        assert_eq!(view.effort.as_deref(), Some("high"));
        assert!(view.place.is_some());
        assert!(view.context.is_some() && view.cost_usd.is_some());
        let five = view.five_hour.expect("5h");
        let seven = view.seven_day.expect("7d");
        assert!(five.resets_local.is_some() && seven.resets_local.is_some());
    }

    /// Os resets caem no futuro da janela de cada um, como numa sessão de verdade.
    #[test]
    fn the_sample_resets_fall_inside_their_windows() {
        let sample = sample(r"C:\Users\exemplo\projetos\app", now());
        let at = |key: &str| sample["rate_limits"][key]["resets_at"].as_i64().expect(key);
        let five = at("five_hour") - now().timestamp();
        let seven = at("seven_day") - now().timestamp();
        assert!((1..=5 * 3600).contains(&five), "{five}");
        assert!((1..=7 * 86400).contains(&seven), "{seven}");
    }

    /// O formato documentado: o que um script de status line costuma ler.
    #[test]
    fn the_sample_speaks_the_documented_schema() {
        let cwd = r"C:\Users\exemplo\projetos\app";
        let sample = sample(cwd, now());
        for pointer in [
            "/session_id",
            "/model/id",
            "/model/display_name",
            "/version",
            "/output_style/name",
            "/cost/total_cost_usd",
            "/cost/total_lines_added",
            "/context_window/context_window_size",
            "/context_window/used_percentage",
            "/rate_limits/five_hour/used_percentage",
            "/rate_limits/seven_day/used_percentage",
            "/effort/level",
        ] {
            assert!(sample.pointer(pointer).is_some(), "{pointer}");
        }
        assert_eq!(sample["cwd"], cwd);
        assert_eq!(sample["workspace"]["current_dir"], cwd);
        assert_eq!(sample["workspace"]["project_dir"], cwd);
    }
}
