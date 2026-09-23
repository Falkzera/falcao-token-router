//! A status line nos Ajustes: a escolha do usuário (`<base>\statusline.json`,
//! que o `router statusline` lê a cada render), a PRÉVIA — desenhada pelo
//! mesmo código da CLI (`router_core::statusline`), com a sessão de exemplo — e
//! o "Testar" do modo comando, pelo mesmo executor da CLI.

use std::collections::HashMap;
use std::path::Path;
use std::time::Instant;

use chrono::{DateTime, Local, TimeZone, Utc};
use router_core::statusline::choice::StatusLineChoice;
use router_core::statusline::command::{self, Outcome, Shell};
use router_core::statusline::session;
use router_core::statusline::view::{self, Label, Style};
use router_core::{ConfigDir, Id, RouterConfig};
use serde::Serialize;
use tauri::{AppHandle, Manager, State};

use crate::i18n;
use crate::locale::Locale;
use crate::state::AppState;

/// Quem rodaria o comando do usuário (nomes de produto: a tela não traduz).
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Runner {
    GitBash,
    PowerShell,
}

/// A seção da status line nos Ajustes.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StatusLineView {
    pub choice: StatusLineChoice,
    pub preview: Vec<Span>,
    pub runner: Option<Runner>,
    /// O prazo do comando do usuário.
    pub deadline_seconds: u64,
}

fn view_for(state: &AppState) -> StatusLineView {
    let store = state.store();
    let choice = StatusLineChoice::load(&store.paths().status_line_file());
    StatusLineView {
        preview: preview(
            &choice,
            store.config(),
            store.active_by_group(),
            &state.home,
            state.locale,
            Utc::now(),
            &Local,
        ),
        choice,
        runner: Shell::detect().map(|shell| match shell {
            Shell::Bash(_) => Runner::GitBash,
            Shell::PowerShell(_) => Runner::PowerShell,
        }),
        deadline_seconds: command::DEADLINE.as_secs(),
    }
}

#[tauri::command]
pub fn get_status_line(state: State<'_, AppState>) -> StatusLineView {
    view_for(&state)
}

/// Grava a escolha — atômica: a CLI a lê a cada render, e vale na próxima
/// atualização das sessões abertas — e devolve a prévia nova.
#[tauri::command]
pub fn set_status_line(
    state: State<'_, AppState>,
    choice: StatusLineChoice,
) -> Result<StatusLineView, String> {
    let path = state.store().paths().status_line_file();
    choice.save(&path).map_err(|e| e.to_string())?;
    Ok(view_for(&state))
}

/// O "Testar" do modo comando, fora da thread da interface (até o prazo).
#[tauri::command]
pub async fn test_status_line(app: AppHandle, command: String) -> Result<StatusLineTest, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let home = app.state::<AppState>().home.clone();
        test_command(&command, &home)
    })
    .await
    .map_err(|e| e.to_string())
}

/// Um trecho da linha, na cor que o terminal daria.
#[derive(Clone, PartialEq, Eq, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Span {
    pub text: String,
    /// `#rrggbb`; `None` = a cor padrão do texto do terminal.
    pub color: Option<String>,
    pub bold: bool,
}

/// As 16 cores do ANSI na paleta padrão do Windows Terminal (Campbell) — a
/// prévia imita o terminal onde a linha aparece.
const CAMPBELL: [&str; 16] = [
    "#0c0c0c", "#c50f1f", "#13a10e", "#c19c00", "#0037da", "#881798", "#3a96dd", "#cccccc",
    "#767676", "#e74856", "#16c60c", "#f9f1a5", "#3b78ff", "#b4009e", "#61d6d6", "#f2f2f2",
];

/// A prévia: a sessão de EXEMPLO pelo mesmo caminho da CLI (`view_from` →
/// escolha → `render`), com o nome e a conta ativa do 1º grupo do usuário — a
/// linha que ele vai ver, com números de exemplo. O terminal é o Windows
/// Terminal (truecolor) e os dias saem no idioma do app, que é o do Windows,
/// o mesmo critério da CLI.
pub fn preview<Tz: TimeZone>(
    choice: &StatusLineChoice,
    config: &RouterConfig,
    active: &HashMap<Id, Id>,
    home: &str,
    locale: Locale,
    now: DateTime<Utc>,
    tz: &Tz,
) -> Vec<Span> {
    let cwd = Path::new(home).join("app").to_string_lossy().into_owned();
    let sample = session::sample(&cwd, now);
    let first = config.groups.first();
    let email = first
        .and_then(|group| active.get(&group.id))
        .and_then(|&id| config.account(id))
        .map(|account| account.identity.email.clone())
        .unwrap_or_else(|| i18n::t(locale, "settings.statusLine.sample.email", &[]));
    let dir = first.map_or_else(
        || ConfigDir::dedicated(cwd.clone()),
        |g| g.config_dir.clone(),
    );
    let mut line = session::view_from(&sample, &dir, Some(config), Some(&email), home, tz);
    if first.is_none() {
        line.label = Some(Label::Group {
            name: i18n::t(locale, "settings.statusLine.sample.group", &[]),
            index: 0,
        });
    }
    let style = Style {
        truecolor: true,
        portuguese: locale == Locale::PtBr,
        phase: 0,
    };
    spans(&view::render(&choice.apply(line), &style))
}

/// O que o "Testar" viu.
#[derive(Clone, PartialEq, Eq, Debug, Serialize)]
#[serde(
    tag = "outcome",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum StatusLineTest {
    /// A linha que as sessões mostrariam, e quanto o comando levou.
    Printed {
        spans: Vec<Span>,
        elapsed_ms: u64,
    },
    /// Rodou, mas o Claude Code não mostraria nada (código ≠ 0 ou saída vazia).
    Failed {
        code: Option<i32>,
        detail: String,
    },
    NotStarted {
        detail: String,
    },
    TimedOut {
        seconds: u64,
    },
    /// Nem Git Bash nem PowerShell nesta máquina.
    NoShell,
}

/// Roda o comando como as sessões o rodariam — mesmo executor da CLI, mesmo
/// shell — com a sessão de exemplo, na pasta da home (a de uma sessão).
pub fn test_command(command: &str, home: &str) -> StatusLineTest {
    let Some(shell) = Shell::detect() else {
        return StatusLineTest::NoShell;
    };
    let input = session::sample(home, Utc::now()).to_string();
    let start = Instant::now();
    match command::run_in(
        Some(Path::new(home)),
        &shell,
        command.trim(),
        input.as_bytes(),
        command::DEADLINE,
    ) {
        Outcome::Printed(bytes) => StatusLineTest::Printed {
            spans: spans(&String::from_utf8_lossy(&bytes)),
            elapsed_ms: start.elapsed().as_millis() as u64,
        },
        Outcome::Failed { code, stderr } => StatusLineTest::Failed {
            code,
            detail: stderr,
        },
        Outcome::NotStarted(detail) => StatusLineTest::NotStarted { detail },
        Outcome::TimedOut => StatusLineTest::TimedOut {
            seconds: command::DEADLINE.as_secs(),
        },
    }
}

/// A linha com as cores do terminal (SGR) → trechos para a tela. Só a cor do
/// texto e o negrito contam; o resto — fundo, link (OSC 8), cursor, título —
/// some, e o texto fica. `\r` sai (a tela quebra linha com `\n`).
pub fn spans(ansi: &str) -> Vec<Span> {
    let mut out: Vec<Span> = Vec::new();
    let (mut color, mut bold) = (None::<String>, false);
    let mut chars = ansi.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\x1b' => match chars.next() {
                // CSI: parâmetros até o byte final (@ a ~); só o `m` pinta.
                Some('[') => {
                    let mut params = String::new();
                    for d in chars.by_ref() {
                        if ('\x40'..='\x7e').contains(&d) {
                            if d == 'm' {
                                sgr(&params, &mut color, &mut bold);
                            }
                            break;
                        }
                        params.push(d);
                    }
                }
                // OSC (link, título): até o BEL ou o `ESC \`.
                Some(']') => {
                    while let Some(d) = chars.next() {
                        if d == '\x07' {
                            break;
                        }
                        if d == '\x1b' {
                            chars.next_if_eq(&'\\');
                            break;
                        }
                    }
                }
                _ => {}
            },
            '\r' => {}
            c if c.is_control() && c != '\n' && c != '\t' => {}
            c => match out.last_mut() {
                Some(last) if last.color == color && last.bold == bold => last.text.push(c),
                _ => out.push(Span {
                    text: c.to_string(),
                    color: color.clone(),
                    bold,
                }),
            },
        }
    }
    out
}

/// Um `ESC [ … m`: o estado de cor e negrito depois dele.
fn sgr(params: &str, color: &mut Option<String>, bold: &mut bool) {
    let codes: Vec<u32> = if params.is_empty() {
        vec![0]
    } else {
        params.split(';').map(|p| p.parse().unwrap_or(0)).collect()
    };
    let mut i = 0;
    while i < codes.len() {
        match codes[i] {
            0 => (*color, *bold) = (None, false),
            1 => *bold = true,
            22 => *bold = false,
            code @ 30..=37 => *color = Some(CAMPBELL[(code - 30) as usize].to_string()),
            code @ 90..=97 => *color = Some(CAMPBELL[(code - 90 + 8) as usize].to_string()),
            39 => *color = None,
            // 38 (texto) e 48 (fundo) levam parâmetros: `5;n` ou `2;r;g;b`.
            code @ (38 | 48) => {
                let taken = match codes.get(i + 1) {
                    Some(5) => {
                        if code == 38 {
                            if let Some(&n) = codes.get(i + 2) {
                                *color = Some(xterm_256(n));
                            }
                        }
                        2
                    }
                    Some(2) => {
                        if let (38, Some(&r), Some(&g), Some(&b)) =
                            (code, codes.get(i + 2), codes.get(i + 3), codes.get(i + 4))
                        {
                            *color = Some(hex([r, g, b]));
                        }
                        4
                    }
                    _ => 0,
                };
                i += taken;
            }
            _ => {}
        }
        i += 1;
    }
}

fn hex(rgb: [u32; 3]) -> String {
    let [r, g, b] = rgb.map(|v| v.min(255));
    format!("#{r:02x}{g:02x}{b:02x}")
}

/// As 256 cores do xterm: as 16 da paleta, o cubo 6×6×6 e os 24 cinzas.
fn xterm_256(n: u32) -> String {
    match n {
        0..=15 => CAMPBELL[n as usize].to_string(),
        16..=231 => {
            let level = |v: u32| if v == 0 { 0 } else { 55 + v * 40 };
            let n = n - 16;
            hex([level(n / 36), level(n / 6 % 6), level(n % 6)])
        }
        _ => {
            let v = 8 + (n.min(255) - 232) * 10;
            hex([v, v, v])
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn span(text: &str, color: Option<&str>, bold: bool) -> Span {
        Span {
            text: text.into(),
            color: color.map(String::from),
            bold,
        }
    }

    #[test]
    fn plain_text_is_one_span_in_the_default_color() {
        assert_eq!(spans("linha"), [span("linha", None, false)]);
        assert_eq!(spans(""), []);
    }

    /// A 1ª parte da linha do app: `●` em negrito e o nome, na cor do grupo.
    #[test]
    fn ansi_colors_become_the_terminal_palette() {
        assert_eq!(
            spans("\x1b[36m\x1b[1m●\x1b[0m \x1b[36mTrabalho\x1b[0m"),
            [
                span("●", Some("#3a96dd"), true),
                span(" ", None, false),
                span("Trabalho", Some("#3a96dd"), false),
            ]
        );
        assert_eq!(spans("\x1b[94mx"), [span("x", Some("#3b78ff"), false)]);
        assert_eq!(spans("\x1b[90mx"), [span("x", Some("#767676"), false)]);
    }

    #[test]
    fn truecolor_and_256_colors_are_kept() {
        assert_eq!(
            spans("\x1b[38;2;153;153;153mcinza\x1b[39m fim"),
            [
                span("cinza", Some("#999999"), false),
                span(" fim", None, false)
            ]
        );
        // 256 cores: as 16 primeiras são a paleta; depois o cubo e os cinzas.
        assert_eq!(spans("\x1b[38;5;9mx"), [span("x", Some("#e74856"), false)]);
        assert_eq!(
            spans("\x1b[38;5;196mx"),
            [span("x", Some("#ff0000"), false)]
        );
        assert_eq!(
            spans("\x1b[38;5;244mx"),
            [span("x", Some("#808080"), false)]
        );
    }

    /// Vários parâmetros num SGR só, e o negrito que desliga sem mudar a cor.
    #[test]
    fn combined_parameters_and_normal_intensity() {
        assert_eq!(
            spans("\x1b[1;32mok\x1b[22m!"),
            [
                span("ok", Some("#13a10e"), true),
                span("!", Some("#13a10e"), false)
            ]
        );
    }

    /// O comando do usuário pode mandar mais que cor: link (OSC 8), cursor,
    /// fundo. O texto fica; o resto some.
    #[test]
    fn other_escape_sequences_are_dropped() {
        assert_eq!(
            spans("\x1b]8;;https://exemplo.com\x1b\\link\x1b]8;;\x1b\\ \x1b[2K\x1b[41mfundo"),
            [span("link fundo", None, false)]
        );
        assert_eq!(spans("a\x1b]0;titulo\x07b"), [span("ab", None, false)]);
    }

    // A prévia.

    use router_core::statusline::choice::Item;
    use router_core::{Account, AccountGroup, AccountIdentity, ConfigDir, Provider};

    fn now() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 9, 23, 12, 0, 0).unwrap()
    }

    fn text(spans: &[Span]) -> String {
        spans.iter().map(|s| s.text.as_str()).collect()
    }

    /// Dois grupos; o 1º ("Trabalho") com a conta1 ativa.
    fn world() -> (RouterConfig, HashMap<Id, Id>) {
        let mut config = RouterConfig::default();
        let account = Account {
            id: Id::new(),
            provider: Provider::Anthropic,
            identity: AccountIdentity {
                email: "conta1@exemplo.com".into(),
                organization_name: None,
                rate_limit_tier: None,
                raw: Default::default(),
            },
            home: ConfigDir::dedicated(r"C:\Users\exemplo\casa"),
            nickname: None,
        };
        let mut work = AccountGroup::new("Trabalho", ConfigDir::dedicated(r"C:\Users\exemplo\g1"));
        work.account_ids.push(account.id);
        let active = HashMap::from([(work.id, account.id)]);
        config.accounts.push(account);
        config.groups.push(work);
        config.groups.push(AccountGroup::new(
            "Pessoal",
            ConfigDir::dedicated(r"C:\Users\exemplo\g2"),
        ));
        (config, active)
    }

    fn preview_of(choice: &StatusLineChoice, locale: Locale) -> Vec<Span> {
        let (config, active) = world();
        let home = tempfile::tempdir().unwrap();
        preview(
            choice,
            &config,
            &active,
            &home.path().to_string_lossy(),
            locale,
            now(),
            &Utc,
        )
    }

    /// A sessão de exemplo pelo caminho da CLI, com o grupo e a conta de verdade
    /// — e a cor do grupo (o 1º é ciano).
    #[test]
    fn the_preview_is_the_sample_session_with_the_first_group_and_its_account() {
        let spans = preview_of(&StatusLineChoice::default(), Locale::PtBr);
        let line = text(&spans);
        assert!(line.starts_with("● Trabalho │ Opus 5.5 high │ "), "{line}");
        for part in [
            "26% 51k/200k",
            "5h █░░░░ 29% ↻ 14:13",
            // 27/09/2026 é um domingo; o dia no idioma do app (o do Windows).
            "7d ██░░░ 33% ↻ dom (27) 17:00",
            "$1.87",
            "conta1@exemplo.com",
        ] {
            assert!(line.contains(part), "{part} em {line}");
        }
        assert_eq!(spans[0], span("●", Some("#3a96dd"), true));
    }

    #[test]
    fn the_preview_follows_the_choice() {
        let mut choice = StatusLineChoice::default();
        choice.set_shown(Item::Context, false);
        choice.set_shown(Item::Email, false);
        let line = text(&preview_of(&choice, Locale::En));
        assert!(line.contains("7d ██░░░ 33% ↻ Sun (27) 17:00"), "{line}");
        assert!(
            !line.contains("51k/200k") && !line.contains("conta1"),
            "{line}"
        );

        for item in Item::ALL {
            choice.set_shown(item, false);
        }
        assert!(preview_of(&choice, Locale::En).is_empty());
    }

    /// Sem grupo (ou sem conta ativa), nomes de exemplo no idioma do app.
    #[test]
    fn without_a_group_the_preview_uses_example_names() {
        let home = tempfile::tempdir().unwrap();
        let home = home.path().to_string_lossy();
        let empty = RouterConfig::default();
        let choice = StatusLineChoice::default();
        let line = |locale| {
            text(&preview(
                &choice,
                &empty,
                &HashMap::new(),
                &home,
                locale,
                now(),
                &Utc,
            ))
        };
        let en = line(Locale::En);
        assert!(
            en.starts_with("● Work │ ") && en.ends_with("you@example.com"),
            "{en}"
        );
        let pt = line(Locale::PtBr);
        assert!(
            pt.starts_with("● Trabalho │ ") && pt.ends_with("voce@exemplo.com"),
            "{pt}"
        );

        let (config, _) = world();
        let no_active = text(&preview(
            &choice,
            &config,
            &HashMap::new(),
            &home,
            Locale::PtBr,
            now(),
            &Utc,
        ));
        assert!(no_active.ends_with("voce@exemplo.com"), "{no_active}");
    }

    // O "Testar" (pelo shell desta máquina: o comando serve no bash e no PowerShell).

    #[test]
    fn a_command_that_prints_is_shown_as_the_line() {
        let home = tempfile::tempdir().unwrap();
        let StatusLineTest::Printed { spans, .. } =
            test_command("echo linha", &home.path().to_string_lossy())
        else {
            panic!("o echo devia imprimir");
        };
        assert_eq!(text(&spans).trim(), "linha");
    }

    #[test]
    fn a_command_that_fails_says_how() {
        let home = tempfile::tempdir().unwrap();
        let outcome = test_command("C:/nao/existe/linha.exe", &home.path().to_string_lossy());
        assert!(
            matches!(outcome, StatusLineTest::Failed { code: Some(code), .. } if code != 0),
            "{outcome:?}"
        );
    }

    /// O formato que a tela lê.
    #[test]
    fn the_outcome_is_tagged_for_the_screen() {
        let printed = StatusLineTest::Printed {
            spans: vec![span("x", None, false)],
            elapsed_ms: 12,
        };
        assert_eq!(
            serde_json::to_value(printed).unwrap(),
            serde_json::json!({
                "outcome": "printed",
                "spans": [{"text": "x", "color": null, "bold": false}],
                "elapsedMs": 12
            })
        );
        assert_eq!(
            serde_json::to_value(StatusLineTest::TimedOut { seconds: 5 }).unwrap(),
            serde_json::json!({"outcome": "timedOut", "seconds": 5})
        );
        assert_eq!(
            serde_json::to_value(StatusLineTest::NoShell).unwrap(),
            serde_json::json!({"outcome": "noShell"})
        );
    }

    /// Uma status line de várias linhas continua com as quebras.
    #[test]
    fn line_breaks_stay() {
        assert_eq!(
            spans("um\r\n\x1b[31mdois\x1b[0m\n"),
            [
                span("um\n", None, false),
                span("dois", Some("#c50f1f"), false),
                span("\n", None, false),
            ]
        );
    }
}
