//! A linha que a status line de um grupo imprime — o layout padrão do app:
//!
//!   ● grupo │ Modelo effort │ branch │ contexto │ 5h … ↻ hh:mm  7d … ↻ dia (dd) h:mm │ $custo │ e-mail
//!
//! Pedido de 23/09/2026: a linha do router substituía uma status line completa
//! por `conta 5h 7d`, e quem usava a completa perdia modelo, esforço, branch,
//! contexto e custo nas sessões dos grupos. Agora a do router é a completa, com o
//! GRUPO no lugar do nome do perfil e o e-mail da conta ATIVA no fim — é ali que
//! se vê a troca. Tudo vem do JSON que o Claude Code entrega (esquema da doc
//! oficial da status line), do `.claude.json` do perfil e do `config.json`.
//!
//! Pura: o relógio (a fase das animações), o idioma e o suporte a cor vêm de
//! fora, e o sensor (a amostra) não passa por aqui. Mora no núcleo porque a
//! prévia dos Ajustes do app desenha com ESTE código — a prévia e a sessão
//! nunca discordam.

use std::path::{Path, PathBuf};

use chrono::{Datelike, NaiveDateTime, Timelike};

use crate::usage::usage_percent::UsagePercent;

const RESET: &str = "\x1b[0m";
const BOLD: &str = "\x1b[1m";
const RED: &str = "\x1b[31m";
const GREEN: &str = "\x1b[32m";
const YELLOW: &str = "\x1b[33m";
const MAGENTA: &str = "\x1b[35m";
const CYAN: &str = "\x1b[36m";
const BLUE: &str = "\x1b[94m";

/// Texto secundário. O "faint" (`ESC[2m`) do Windows Terminal escurece a cor
/// pela metade e some sobre fundo escuro: um cinza claro explícito (o mesmo tom
/// das dicas da TUI), com o bright-black do ANSI onde não há truecolor.
const GRAY_TRUECOLOR: &str = "\x1b[38;2;153;153;153m";
const GRAY_ANSI: &str = "\x1b[90m";

/// A cor de cada grupo pela posição na lista do usuário: estável entre sessões
/// e diferente entre vizinhos. Amarelo fica para a conta fora de um grupo.
const GROUP_COLORS: [&str; 4] = [CYAN, GREEN, MAGENTA, BLUE];

/// As cores do seletor do `/effort` no tema escuro do Claude Code; `xhigh` faz
/// um brilho passear e `max` gira um arco-íris, como na TUI.
const RAINBOW: [[u8; 3]; 7] = [
    [235, 95, 87],
    [245, 139, 87],
    [250, 195, 95],
    [145, 200, 130],
    [130, 170, 220],
    [155, 130, 200],
    [200, 130, 180],
];

enum EffortPaint {
    Flat([u8; 3]),
    Glow { base: [u8; 3], light: [u8; 3] },
    Rainbow,
}

fn effort_palette(level: &str) -> Option<(EffortPaint, &'static str)> {
    match level {
        "low" => Some((EffortPaint::Flat([255, 193, 7]), YELLOW)),
        "medium" => Some((EffortPaint::Flat([78, 186, 101]), GREEN)),
        "high" => Some((EffortPaint::Flat([177, 185, 249]), BLUE)),
        "xhigh" => Some((
            EffortPaint::Glow {
                base: [175, 135, 255],
                light: [208, 180, 255],
            },
            MAGENTA,
        )),
        "max" => Some((EffortPaint::Rainbow, RED)),
        _ => None,
    }
}

/// Quem a linha nomeia primeiro: o grupo dono do perfil, ou — fora de um grupo
/// conhecido — a conta.
pub enum Label {
    Group { name: String, index: usize },
    Account(String),
}

/// Uma janela do `rate_limits`: fração 0–1 e o reset já em hora local.
pub struct Window {
    pub fraction: f64,
    pub resets_local: Option<NaiveDateTime>,
}

/// A janela de contexto: `used_percentage` (0–100) e os tokens.
pub struct Context {
    pub used_percent: f64,
    pub input_tokens: u64,
    pub window_size: u64,
}

pub struct View {
    /// `None` quando o usuário tirou o grupo da linha.
    pub label: Option<Label>,
    pub model: Option<String>,
    pub model_id: Option<String>,
    pub effort: Option<String>,
    pub place: Option<String>,
    pub context: Option<Context>,
    pub five_hour: Option<Window>,
    pub seven_day: Option<Window>,
    pub cost_usd: Option<f64>,
    pub email: Option<String>,
}

pub struct Style {
    pub truecolor: bool,
    pub portuguese: bool,
    pub phase: u64,
}

/// A linha inteira. O que não veio (o 1º render não tem `rate_limits`; o
/// `effort` só vem com modelo que o aceita) fica de fora, sem marcador.
pub fn render(view: &View, style: &Style) -> String {
    let gray = if style.truecolor {
        GRAY_TRUECOLOR
    } else {
        GRAY_ANSI
    };
    let mut segments = Vec::new();

    if let Some(label) = &view.label {
        let (color, name) = match label {
            Label::Group { name, index } => (GROUP_COLORS[index % GROUP_COLORS.len()], name),
            Label::Account(name) => (YELLOW, name),
        };
        segments.push(format!("{color}{BOLD}●{RESET} {color}{name}{RESET}"));
    }

    // O esforço vai ao lado do modelo; sem o modelo (tirado pelo usuário), sozinho.
    let effort = view
        .effort
        .as_deref()
        .map(|level| paint_effort(level, style, gray));
    match (&view.model, effort) {
        (Some(display), effort) => {
            let color = model_color(display, view.model_id.as_deref());
            let mut segment = format!("{color}{BOLD}{}{RESET}", model_name(display));
            if let Some(effort) = effort {
                segment.push(' ');
                segment.push_str(&effort);
            }
            segments.push(segment);
        }
        (None, Some(effort)) => segments.push(effort),
        (None, None) => {}
    }

    if let Some(place) = &view.place {
        segments.push(format!("{gray}{place}{RESET}"));
    }

    if let Some(context) = &view.context {
        let fraction = context.used_percent / 100.0;
        let tone = tone(fraction);
        segments.push(format!(
            "{tone}{}{RESET} {tone}{}%{RESET} {gray}{}/{}{RESET}",
            bar(fraction, 10),
            UsagePercent::value(fraction),
            tokens(context.input_tokens),
            tokens(context.window_size)
        ));
    }

    let windows: Vec<String> = [
        ("5h", view.five_hour.as_ref(), false),
        ("7d", view.seven_day.as_ref(), true),
    ]
    .into_iter()
    .filter_map(|(label, window, with_day)| {
        let window = window?;
        let tone = tone(window.fraction);
        let mut text = format!(
            "{gray}{label}{RESET} {tone}{} {}%{RESET}",
            bar(window.fraction, 5),
            UsagePercent::value(window.fraction)
        );
        if let Some(at) = window.resets_local {
            // 5h: só a hora, o reset cai nas próximas horas; 7d: o dia também.
            let when = if with_day {
                day_and_time(at, style.portuguese)
            } else {
                format!("{:02}:{:02}", at.hour(), at.minute())
            };
            text.push_str(&format!(" {gray}↻ {when}{RESET}"));
        }
        Some(text)
    })
    .collect();
    if !windows.is_empty() {
        segments.push(windows.join("  "));
    }

    if let Some(cost) = view.cost_usd {
        segments.push(format!("{gray}${cost:.2}{RESET}"));
    }
    if let Some(email) = &view.email {
        segments.push(format!("{gray}{email}{RESET}"));
    }
    segments.join(&format!("{gray} │ {RESET}"))
}

/// "Opus 5 (1M context)" → "Opus 5": a janela já aparece no medidor de contexto.
pub fn model_name(display: &str) -> String {
    let trimmed = display.trim_end();
    if let Some(open) = trimmed.strip_suffix(')').and_then(|t| t.rfind('(')) {
        let inside = &trimmed[open + 1..trimmed.len() - 1];
        if !inside.contains(')') {
            return trimmed[..open].trim_end().to_string();
        }
    }
    trimmed.to_string()
}

/// Fable em vermelho, para não passar batido — é o limite por modelo, o que
/// costuma travar a conta antes das outras janelas; o resto em azul.
fn model_color(display: &str, id: Option<&str>) -> &'static str {
    let target = format!("{display} {}", id.unwrap_or_default()).to_lowercase();
    if target.contains("fable") {
        RED
    } else {
        BLUE
    }
}

fn paint_effort(level: &str, style: &Style, gray: &str) -> String {
    let Some((paint, ansi)) = effort_palette(level) else {
        return format!("{gray}{level}{RESET}");
    };
    if !style.truecolor {
        return format!("{ansi}{BOLD}{level}{RESET}");
    }
    match paint {
        EffortPaint::Flat(color) => format!("{}{BOLD}{level}{RESET}", rgb(color)),
        EffortPaint::Rainbow => {
            let body: String = level
                .chars()
                .enumerate()
                .map(|(i, c)| {
                    let color = RAINBOW[(i + style.phase as usize) % RAINBOW.len()];
                    format!("{}{c}", rgb(color))
                })
                .collect();
            format!("{BOLD}{body}{RESET}")
        }
        EffortPaint::Glow { base, light } => {
            // A faixa clara anda uma letra por segundo (a fase vem do relógio).
            let len = level.chars().count() as u64;
            let top = (style.phase % (len + 4)) as i64;
            let body: String = level
                .chars()
                .enumerate()
                .map(|(i, c)| {
                    let distance = (i as i64 - top).abs();
                    let k = if distance < 3 {
                        1.0 - distance as f64 / 3.0
                    } else {
                        0.0
                    };
                    format!("{}{c}", rgb(mix(base, light, k)))
                })
                .collect();
            format!("{BOLD}{body}{RESET}")
        }
    }
}

fn rgb(color: [u8; 3]) -> String {
    format!("\x1b[38;2;{};{};{}m", color[0], color[1], color[2])
}

fn mix(a: [u8; 3], b: [u8; 3], k: f64) -> [u8; 3] {
    std::array::from_fn(|i| (a[i] as f64 + (b[i] as f64 - a[i] as f64) * k).round() as u8)
}

/// Cor por severidade, a mesma regra de antes da status line do router (e do
/// macOS): ≥0,90 vermelho, ≥0,70 amarelo, senão verde.
fn tone(fraction: f64) -> &'static str {
    if fraction >= 0.90 {
        RED
    } else if fraction >= 0.70 {
        YELLOW
    } else {
        GREEN
    }
}

/// `round(fração × largura)` células cheias, o resto vazias.
fn bar(fraction: f64, width: usize) -> String {
    let filled = ((fraction * width as f64).round() as i64).clamp(0, width as i64) as usize;
    "█".repeat(filled) + &"░".repeat(width - filled)
}

fn tokens(n: u64) -> String {
    if n >= 1_000 {
        format!("{}k", (n as f64 / 1_000.0).round() as u64)
    } else {
        n.to_string()
    }
}

/// "seg (28) 9:00" / "Mon (28) 9:00": o reset semanal pode cair em qualquer
/// dia. Hora sem zero à esquerda, de propósito.
fn day_and_time(at: NaiveDateTime, portuguese: bool) -> String {
    const PT: [&str; 7] = ["dom", "seg", "ter", "qua", "qui", "sex", "sáb"];
    const EN: [&str; 7] = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
    let names = if portuguese { PT } else { EN };
    format!(
        "{} ({}) {}:{:02}",
        names[at.weekday().num_days_from_sunday() as usize],
        at.day(),
        at.hour(),
        at.minute()
    )
}

/// O branch lendo o `.git/HEAD`, subindo das pastas — sem rodar o `git`, que
/// custaria um processo a cada render. Worktree: o `.git` é um arquivo que
/// aponta para o gitdir. HEAD destacado: os 7 primeiros do hash.
pub fn git_branch(dir: &Path) -> Option<String> {
    git_branch_below(dir, None)
}

/// Com um teto (como o `GIT_CEILING_DIRECTORIES` do git): a busca não passa
/// dele. Nos testes, a pasta temporária pode morar dentro de um repositório de
/// verdade — e subir até ele também é o comportamento certo fora dos testes.
fn git_branch_below(dir: &Path, ceiling: Option<&Path>) -> Option<String> {
    let mut current = Some(dir);
    for _ in 0..40 {
        let here = current?;
        if let Some(branch) = head_of(here) {
            return Some(branch);
        }
        if ceiling.is_some_and(|top| here == top) {
            return None;
        }
        current = here.parent();
    }
    None
}

fn head_of(dir: &Path) -> Option<String> {
    let dot_git = dir.join(".git");
    let git_dir: PathBuf = if std::fs::metadata(&dot_git).ok()?.is_file() {
        let text = std::fs::read_to_string(&dot_git).ok()?;
        let target = text
            .lines()
            .find_map(|line| line.trim().strip_prefix("gitdir:"))?;
        dir.join(target.trim())
    } else {
        dot_git
    };
    let head = std::fs::read_to_string(git_dir.join("HEAD")).ok()?;
    let head = head.trim();
    Some(
        match head
            .strip_prefix("ref:")
            .and_then(|r| r.trim_start().strip_prefix("refs/heads/"))
        {
            Some(branch) => branch.to_string(),
            None => head.chars().take(7).collect(),
        },
    )
}

/// Fora de um repositório, a pasta: `~` no lugar da home, e as do meio viram
/// `…` quando são muitas.
pub fn shorten_path(path: &str, home: &str) -> String {
    let normalize = |p: &str| p.replace('\\', "/").trim_end_matches('/').to_string();
    let mut shown = normalize(path);
    let home = normalize(home);
    let under_home = !home.is_empty()
        && shown.is_char_boundary(home.len())
        && shown[..home.len()].eq_ignore_ascii_case(&home)
        && shown[home.len()..].chars().next().is_none_or(|c| c == '/');
    if under_home {
        shown = format!("~{}", &shown[home.len()..]);
    }
    let parts: Vec<&str> = shown.split('/').filter(|p| !p.is_empty()).collect();
    if parts.len() > 3 {
        [
            parts[0],
            "…",
            parts[parts.len() - 2],
            parts[parts.len() - 1],
        ]
        .join("/")
    } else {
        shown
    }
}

/// O terminal aceita cor de 24 bits? Pelo ambiente que o Claude Code repassa:
/// `COLORTERM`, o Windows Terminal (`WT_SESSION`), os terminais que a anunciam
/// por `TERM_PROGRAM`, ou um `TERM` "direct"/"truecolor".
pub fn truecolor(env: impl Fn(&str) -> Option<String>) -> bool {
    let lower = |key: &str| env(key).map(|v| v.to_lowercase());
    lower("COLORTERM").is_some_and(|v| v.contains("truecolor") || v.contains("24bit"))
        || env("WT_SESSION").is_some_and(|v| !v.is_empty())
        || env("TERM_PROGRAM")
            .is_some_and(|v| ["vscode", "iTerm.app", "WezTerm", "ghostty"].contains(&v.as_str()))
        || lower("TERM").is_some_and(|v| v.contains("direct") || v.contains("truecolor"))
}

/// A linha de exemplo dos testes — da linha e da escolha que a recorta.
#[cfg(test)]
pub(crate) mod fixtures {
    use super::*;
    use chrono::NaiveDate;

    pub fn at(y: i32, m: u32, d: u32, h: u32, min: u32) -> NaiveDateTime {
        NaiveDate::from_ymd_opt(y, m, d)
            .unwrap()
            .and_hms_opt(h, min, 0)
            .unwrap()
    }

    pub fn style(truecolor: bool, portuguese: bool) -> Style {
        Style {
            truecolor,
            portuguese,
            phase: 0,
        }
    }

    /// Sem as cores: o texto que se vê.
    pub fn plain(text: &str) -> String {
        regex::Regex::new("\x1b\\[[0-9;]*m")
            .unwrap()
            .replace_all(text, "")
            .into_owned()
    }

    pub fn bare(label: Label) -> View {
        View {
            label: Some(label),
            model: None,
            model_id: None,
            effort: None,
            place: None,
            context: None,
            five_hour: None,
            seven_day: None,
            cost_usd: None,
            email: None,
        }
    }

    pub fn full() -> View {
        View {
            model: Some("Opus 5.5 (1M context)".into()),
            model_id: Some("claude-opus-5-5".into()),
            effort: Some("high".into()),
            place: Some("port/windows".into()),
            context: Some(Context {
                used_percent: 51.2,
                input_tokens: 511_000,
                window_size: 1_000_000,
            }),
            five_hour: Some(Window {
                fraction: 0.29,
                resets_local: Some(at(2026, 9, 23, 14, 5)),
            }),
            // 28/09/2026 é uma segunda-feira.
            seven_day: Some(Window {
                fraction: 0.33,
                resets_local: Some(at(2026, 9, 28, 9, 0)),
            }),
            cost_usd: Some(24.771),
            email: Some("conta1@exemplo.com".into()),
            ..bare(Label::Group {
                name: "Trabalho".into(),
                index: 0,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::fixtures::*;
    use super::*;

    #[test]
    fn the_line_has_every_segment_in_order() {
        assert_eq!(
            plain(&render(&full(), &style(false, true))),
            "● Trabalho │ Opus 5.5 high │ port/windows │ █████░░░░░ 51% 511k/1000k │ \
             5h █░░░░ 29% ↻ 14:05  7d ██░░░ 33% ↻ seg (28) 9:00 │ $24.77 │ conta1@exemplo.com"
        );
    }

    /// Os itens tirados pela escolha do usuário somem da linha sem deixar
    /// separador sobrando.
    #[test]
    fn without_the_group_the_line_starts_at_the_next_item() {
        let mut view = full();
        view.label = None;
        let line = plain(&render(&view, &style(false, true)));
        assert!(line.starts_with("Opus 5.5 high │ port/windows │"), "{line}");
    }

    #[test]
    fn the_effort_stands_alone_when_the_model_is_out() {
        let mut view = full();
        view.model = None;
        let line = plain(&render(&view, &style(false, true)));
        assert!(
            line.starts_with("● Trabalho │ high │ port/windows │"),
            "{line}"
        );
        // Com as cores do seletor, como ao lado do modelo.
        assert!(render(&view, &style(false, true)).contains("\x1b[94m\x1b[1mhigh\x1b[0m"));
    }

    #[test]
    fn a_view_with_nothing_is_an_empty_line() {
        let mut view = bare(Label::Account("conta1".into()));
        view.label = None;
        assert_eq!(render(&view, &style(true, true)), "");
    }

    #[test]
    fn the_seven_day_reset_speaks_the_windows_language() {
        let line = plain(&render(&full(), &style(false, false)));
        assert!(line.contains("↻ Mon (28) 9:00"), "{line}");
    }

    #[test]
    fn missing_parts_are_left_out_without_placeholders() {
        let group = || Label::Group {
            name: "Trabalho".into(),
            index: 0,
        };
        assert_eq!(
            plain(&render(&bare(group()), &style(false, true))),
            "● Trabalho"
        );

        // Sem `rate_limits` (1º render da sessão): nada de janela, e nada de
        // "sem uso ainda" — os outros segmentos já dizem que a sessão está viva.
        let mut view = full();
        view.five_hour = None;
        view.seven_day = None;
        let line = plain(&render(&view, &style(false, true)));
        assert!(!line.contains("5h") && !line.contains("7d"), "{line}");

        // Só a janela de 7 dias (a de 5 horas pode faltar sozinha).
        let mut view = full();
        view.five_hour = None;
        let line = plain(&render(&view, &style(false, true)));
        assert!(line.contains("│ 7d ██░░░ 33% ↻ seg (28) 9:00 │"), "{line}");
    }

    #[test]
    fn the_model_name_drops_the_context_suffix() {
        assert_eq!(model_name("Opus 5 (1M context)"), "Opus 5");
        assert_eq!(model_name("Sonnet 5"), "Sonnet 5");
    }

    #[test]
    fn fable_stands_out_in_red_and_the_rest_is_blue() {
        let mut view = full();
        view.model = Some("Fable 5.1".into());
        view.model_id = Some("claude-fable-5-1".into());
        assert!(render(&view, &style(false, true)).contains("\x1b[31m\x1b[1mFable 5.1\x1b[0m"));
        assert!(render(&full(), &style(false, true)).contains("\x1b[94m\x1b[1mOpus 5.5\x1b[0m"));
    }

    fn effort_of(level: &str, truecolor: bool, phase: u64) -> String {
        let mut view = bare(Label::Account("conta1".into()));
        view.model = Some("Opus 5.5".into());
        view.effort = Some(level.into());
        let style = Style {
            truecolor,
            portuguese: true,
            phase,
        };
        let line = render(&view, &style);
        line[line.find("Opus 5.5").unwrap() + "Opus 5.5\x1b[0m ".len()..].to_string()
    }

    #[test]
    fn effort_without_truecolor_uses_the_ansi_palette() {
        assert_eq!(effort_of("low", false, 0), "\x1b[33m\x1b[1mlow\x1b[0m");
        assert_eq!(
            effort_of("medium", false, 0),
            "\x1b[32m\x1b[1mmedium\x1b[0m"
        );
        assert_eq!(effort_of("high", false, 0), "\x1b[94m\x1b[1mhigh\x1b[0m");
        assert_eq!(effort_of("xhigh", false, 0), "\x1b[35m\x1b[1mxhigh\x1b[0m");
        assert_eq!(effort_of("max", false, 0), "\x1b[31m\x1b[1mmax\x1b[0m");
        // Nível que ainda não existe: cinza, sem inventar cor.
        assert_eq!(effort_of("turbo", false, 0), "\x1b[90mturbo\x1b[0m");
    }

    #[test]
    fn effort_with_truecolor_follows_the_claude_code_selector() {
        assert_eq!(
            effort_of("medium", true, 0),
            "\x1b[38;2;78;186;101m\x1b[1mmedium\x1b[0m"
        );
        // `max` gira o arco-íris: a fase desloca as cores a cada segundo.
        assert_eq!(
            effort_of("max", true, 0),
            "\x1b[1m\x1b[38;2;235;95;87mm\x1b[38;2;245;139;87ma\x1b[38;2;250;195;95mx\x1b[0m"
        );
        assert_eq!(
            effort_of("max", true, 1),
            "\x1b[1m\x1b[38;2;245;139;87mm\x1b[38;2;250;195;95ma\x1b[38;2;145;200;130mx\x1b[0m"
        );
        // `xhigh` faz o brilho passear: na fase 0 ele está na 1ª letra.
        assert_eq!(
            effort_of("xhigh", true, 0),
            "\x1b[1m\x1b[38;2;208;180;255mx\x1b[38;2;197;165;255mh\x1b[38;2;186;150;255mi\
             \x1b[38;2;175;135;255mg\x1b[38;2;175;135;255mh\x1b[0m"
        );
    }

    #[test]
    fn secondary_text_is_an_explicit_gray_where_truecolor_exists() {
        // O "faint" (ESC[2m) some sobre fundo escuro no Windows Terminal.
        assert!(render(&full(), &style(true, true)).contains("\x1b[38;2;153;153;153m │ "));
        assert!(render(&full(), &style(false, true)).contains("\x1b[90m │ "));
        assert!(!render(&full(), &style(true, true)).contains("\x1b[2m"));
    }

    #[test]
    fn each_group_keeps_its_color_and_an_account_label_is_yellow() {
        let label = |label| render(&bare(label), &style(false, true));
        let group = |index| Label::Group {
            name: "G".into(),
            index,
        };
        assert!(label(group(0)).starts_with("\x1b[36m"));
        assert!(label(group(1)).starts_with("\x1b[32m"));
        assert!(label(group(4)).starts_with("\x1b[36m"));
        assert!(label(Label::Account("conta1".into())).starts_with("\x1b[33m"));
    }

    #[test]
    fn usage_colors_keep_the_router_thresholds() {
        let colored = |fraction| {
            let mut view = full();
            view.five_hour = Some(Window {
                fraction,
                resets_local: None,
            });
            render(&view, &style(false, true))
        };
        assert!(colored(0.95).contains("\x1b[31m█████ 95%"));
        assert!(colored(0.80).contains("\x1b[33m████░ 80%"));
        assert!(colored(0.10).contains("\x1b[32m█░░░░ 10%"));
    }

    #[test]
    fn bar_fills_proportionally() {
        assert_eq!(bar(0.0, 8), "░░░░░░░░");
        assert_eq!(bar(1.0, 8), "████████");
        assert_eq!(bar(0.5, 8), "████░░░░");
        // arredonda: 0,44 × 8 = 3,52 → 4
        assert_eq!(bar(0.44, 8), "████░░░░");
        // fora de 0–1 não estoura a largura
        assert_eq!(bar(1.7, 5), "█████");
    }

    #[test]
    fn tokens_are_counted_in_thousands() {
        assert_eq!(tokens(999), "999");
        assert_eq!(tokens(1_000), "1k");
        assert_eq!(tokens(1_499), "1k");
        assert_eq!(tokens(1_500), "2k");
        assert_eq!(tokens(511_000), "511k");
    }

    #[test]
    fn the_branch_comes_from_git_head_without_running_git() {
        let tmp = tempfile::tempdir().unwrap();
        let repo = tmp.path().join("repo");
        std::fs::create_dir_all(repo.join(".git")).unwrap();
        std::fs::create_dir_all(repo.join("src").join("deep")).unwrap();
        std::fs::write(repo.join(".git").join("HEAD"), "ref: refs/heads/feat/x\n").unwrap();
        assert_eq!(git_branch(&repo).as_deref(), Some("feat/x"));
        // De uma subpasta, sobe até achar o `.git`.
        assert_eq!(
            git_branch(&repo.join("src").join("deep")).as_deref(),
            Some("feat/x")
        );

        // HEAD destacado: os 7 primeiros do hash.
        std::fs::write(
            repo.join(".git").join("HEAD"),
            "0123456789abcdef0123456789abcdef01234567\n",
        )
        .unwrap();
        assert_eq!(git_branch(&repo).as_deref(), Some("0123456"));

        // Worktree: o `.git` é um ARQUIVO que aponta para o gitdir.
        let wt = tmp.path().join("wt");
        let gitdir = tmp.path().join("real").join("worktrees").join("wt");
        std::fs::create_dir_all(&wt).unwrap();
        std::fs::create_dir_all(&gitdir).unwrap();
        std::fs::write(gitdir.join("HEAD"), "ref: refs/heads/wt-branch\n").unwrap();
        std::fs::write(wt.join(".git"), "gitdir: ../real/worktrees/wt\n").unwrap();
        assert_eq!(git_branch(&wt).as_deref(), Some("wt-branch"));

        // Fora de um repositório (o teto impede de subir até um repositório que
        // por acaso contenha a pasta temporária desta máquina).
        let loose = tmp.path().join("loose");
        std::fs::create_dir_all(&loose).unwrap();
        assert_eq!(git_branch_below(&loose, Some(tmp.path())), None);
    }

    #[test]
    fn a_path_is_shortened_from_home() {
        let home = r"C:\Users\exemplo";
        assert_eq!(
            shorten_path(r"C:\Users\exemplo\Projects\app", home),
            "~/Projects/app"
        );
        assert_eq!(shorten_path(r"c:\users\exemplo\a\b\c\d", home), "~/…/c/d");
        assert_eq!(shorten_path(r"D:\trabalhos\x\", home), "D:/trabalhos/x");
        assert_eq!(shorten_path(r"D:\a\b\c\d", home), "D:/…/c/d");
    }

    #[test]
    fn truecolor_is_read_from_the_terminal_environment() {
        let env = |pairs: &'static [(&'static str, &'static str)]| {
            move |key: &str| {
                pairs
                    .iter()
                    .find(|(k, _)| *k == key)
                    .map(|(_, v)| v.to_string())
            }
        };
        assert!(truecolor(env(&[("COLORTERM", "truecolor")])));
        assert!(truecolor(env(&[("COLORTERM", "24bit")])));
        assert!(truecolor(env(&[("WT_SESSION", "0000")])));
        assert!(truecolor(env(&[("TERM_PROGRAM", "vscode")])));
        assert!(truecolor(env(&[("TERM", "xterm-direct")])));
        assert!(!truecolor(env(&[("TERM", "xterm-256color")])));
        assert!(!truecolor(env(&[])));
    }
}
