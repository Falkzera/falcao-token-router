//! A sonda ativa: `claude --print … /usage` e o que o texto significa.
//!
//! Portados de `ProbeTests.swift` (suíte "ClaudeUsageProbe", 7) com a fixture do
//! macOS (data com `at`), e as regressões do Windows com a captura REAL do spike
//! de 22/09/2026 (anonimizada — números sintéticos, zona trocada): a data vem
//! com VÍRGULA (`Sep 22, 8:40pm`), as linhas terminam em CRLF, e um perfil
//! deslogado sai com código 0 e SEM nenhuma linha `Current` — só o resumo do
//! `--print`. Por isso "sem login" é decidido pela ausência das linhas, não pelo
//! código de saída.
//!
//! (As suítes "Por modelo entra na decisão" e "Procedência da medida" do mesmo
//! arquivo Swift estão em `reader_tests.rs`/`sample_tests.rs` e aqui no fim; a
//! "Sonda: por qual perfil medir" está em `rotation_tests.rs`.)

mod common;
use common::*;

use chrono::{DateTime, Datelike, TimeZone, Utc};

use router_core::engine::group_usage::{
    GroupUsageSample, GroupUsageStore, ModelUsage, UsageOrigin,
};
use router_core::engine::group_usage_reader::{GroupUsageReader, UsageWindow};
use router_core::usage::claude_usage_probe::{ClaudeUsageProbe, ProbeError, ProbeOutput};
use router_core::{AccountGroup, ConfigDir, ModelWindow, RouterConfig};

/// A saída do `claude --print /usage` do macOS (setembro de 2026), com os
/// números e os nomes de skill trocados. A FORMA das linhas é a de verdade.
const MACOS_OUTPUT: &str =
    "You are currently using your subscription to power your Claude Code usage

Current session: 2% used · resets Sep 18 at 7:29pm (America/Sao_Paulo)
Current week (all models): 26% used · resets Sep 21 at 8:59am (America/Sao_Paulo)
Current week (Fable): 0% used · resets Sep 21 at 9am (America/Sao_Paulo)

What's contributing to your limits usage?
Approximate, based on local sessions on this machine — does not include other devices or claude.ai.

Last 24h · 120 requests · 6 sessions
  89% of your usage was at >150k context
  Top skills: /exemplo 15%, /outra 6%
";

/// A captura do Windows (Claude Code 2.1.280), anonimizada: CRLF e vírgula.
const WINDOWS_OUTPUT: &str = "You are currently using your subscription to power your Claude Code usage\r\n\r\nCurrent session: 12% used · resets Sep 22, 8:40pm (America/Sao_Paulo)\r\nCurrent week (all models): 34% used · resets Sep 23, 4am (America/Sao_Paulo)\r\nCurrent week (Fable): 56% used · resets Sep 23, 4am (America/Sao_Paulo)\r\n";

/// O que um perfil SEM login imprime no Windows — e sai com código 0.
const WINDOWS_LOGGED_OUT: &str = "Total cost:            $0.0000\r\nTotal duration (API):  0s\r\nTotal duration (wall): 0s\r\nTotal code changes:    0 lines added, 0 lines removed\r\nUsage:                 0 input, 0 output, 0 cache read, 0 cache write\r\n";

/// 18/09/2025 — o comentário do teste Swift diz 2026, mas o valor é 2025. Os
/// testes passam do mesmo jeito: o ano do reset é o mais perto de `agora`.
fn agora() -> DateTime<Utc> {
    ts(1_758_200_000)
}

fn utc(y: i32, mo: u32, d: u32, h: u32, mi: u32) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(y, mo, d, h, mi, 0).unwrap()
}

// --- Portados do Swift ---

#[test]
fn reads_the_three_windows_of_the_real_output() {
    let r = ClaudeUsageProbe::parse(MACOS_OUTPUT, agora()).unwrap();
    assert_eq!(r.session, Some(0.02));
    assert_eq!(r.weekly_all, Some(0.26));
    assert_eq!(r.models.len(), 1);
    assert_eq!(r.models[0].name, "Fable");
    assert_eq!(r.models[0].percent, 0.0);
}

/// O motivo de a sonda existir: esta linha não chega no `rate_limits` da
/// status line, e é a que trava a conta primeiro.
#[test]
fn the_per_model_window_comes_with_name_and_reset() {
    let r = ClaudeUsageProbe::parse(MACOS_OUTPUT, agora()).unwrap();
    // `9am` — hora cheia, sem minutos: um padrão `h:mma` sozinho a perderia.
    let reset = r.models[0].resets_at.expect("reset do Fable");
    assert!(reset > agora());
}

/// A prosa de contribuição fala de porcentagem o tempo todo ("89% of your usage
/// was at >150k context"). Nada dali pode virar limite.
#[test]
fn the_contribution_prose_never_becomes_a_window() {
    let r = ClaudeUsageProbe::parse(MACOS_OUTPUT, agora()).unwrap();
    assert!(r.models.iter().all(|m| m.name == "Fable"));
    assert_eq!(r.models.len(), 1);
}

#[test]
fn output_without_any_current_line_is_an_unknown_format() {
    assert_eq!(
        ClaudeUsageProbe::parse("Credit balance: $0.00\nnada aqui", agora()),
        Err(ProbeError::Unrecognized)
    );
}

/// Perder a porcentagem porque a redação da data mudou seria a pior das duas
/// falhas: o número que decide continua legível.
#[test]
fn an_unreadable_reset_date_keeps_the_percentage() {
    let r = ClaudeUsageProbe::parse(
        "Current session: 44% used · resets quando der (Marte/Olympus)",
        agora(),
    )
    .unwrap();
    assert_eq!(r.session, Some(0.44));
    assert_eq!(r.session_resets_at, None);
}

/// Lido em 31/dez, uma janela que reseta em 2/jan é do ano que vem.
#[test]
fn the_reset_year_is_the_candidate_nearest_to_now() {
    let new_years_eve = utc(2026, 12, 31, 20, 0);
    let reset = ClaudeUsageProbe::reset_date("Jan 2 at 9am (UTC)", new_years_eve).unwrap();
    assert_eq!(reset.year(), 2027);
}

#[test]
fn the_plan_comes_from_the_first_lines_as_printed() {
    assert_eq!(
        ClaudeUsageProbe::plan(MACOS_OUTPUT).as_deref(),
        Some("subscription")
    );
    assert_eq!(
        ClaudeUsageProbe::plan("Max 20x plan\n\nCurrent session: 1% used").as_deref(),
        Some("Max 20x")
    );
}

// --- Regressões do Windows ---

/// A captura real do Windows: CRLF, `·` (U+00B7) e data com vírgula — com e
/// sem minutos. `America/Sao_Paulo` é UTC−3 o ano todo (sem horário de verão).
#[test]
fn reads_the_windows_output_with_comma_dates_and_crlf() {
    let now = utc(2026, 9, 22, 20, 0);
    let r = ClaudeUsageProbe::parse(WINDOWS_OUTPUT, now).unwrap();

    assert_eq!(r.session, Some(0.12));
    assert_eq!(r.session_resets_at, Some(utc(2026, 9, 22, 23, 40)));
    assert_eq!(r.weekly_all, Some(0.34));
    assert_eq!(r.weekly_all_resets_at, Some(utc(2026, 9, 23, 7, 0)));
    assert_eq!(
        r.models,
        vec![ModelWindow::new(
            "Fable",
            0.56,
            Some(utc(2026, 9, 23, 7, 0))
        )]
    );
    assert_eq!(r.plan.as_deref(), Some("subscription"));
}

/// A zona sai ANTES de mexer em am/pm: `America/…` começa com "Am".
#[test]
fn the_zone_is_removed_before_reading_am_pm() {
    let now = utc(2026, 9, 22, 20, 0);
    assert_eq!(
        ClaudeUsageProbe::reset_date("Sep 23, 12am (America/Sao_Paulo)", now),
        Some(utc(2026, 9, 23, 3, 0))
    );
    assert_eq!(
        ClaudeUsageProbe::reset_date("Sep 23, 12:30pm (America/Sao_Paulo)", now),
        Some(utc(2026, 9, 23, 15, 30))
    );
}

/// Deslogado no Windows: código 0 e nenhuma linha `Current` — só o resumo do
/// `--print`. É "sem login", não "formato desconhecido".
#[test]
fn a_logged_out_profile_is_not_signed_in_even_with_exit_code_zero() {
    let out = ProbeOutput {
        exit_code: Some(0),
        stdout: WINDOWS_LOGGED_OUT.to_string(),
    };
    assert_eq!(
        ClaudeUsageProbe::interpret(&out, agora()),
        Err(ProbeError::NotSignedIn)
    );
}

/// Código diferente de zero continua sendo "sem login" (o do macOS).
#[test]
fn a_nonzero_exit_code_is_not_signed_in() {
    let out = ProbeOutput {
        exit_code: Some(1),
        stdout: String::new(),
    };
    assert_eq!(
        ClaudeUsageProbe::interpret(&out, agora()),
        Err(ProbeError::NotSignedIn)
    );
}

/// Há linha `Current`, mas ela não se lê: aí sim é formato novo — o sinal de
/// que o parser precisa mudar, e não de que falta login.
#[test]
fn a_current_line_that_does_not_parse_is_an_unknown_format() {
    let out = ProbeOutput {
        exit_code: Some(0),
        stdout: "Current session: muito usado\r\n".to_string(),
    };
    assert_eq!(
        ClaudeUsageProbe::interpret(&out, agora()),
        Err(ProbeError::Unrecognized)
    );
}

#[test]
fn a_good_output_with_exit_code_zero_is_a_reading() {
    let out = ProbeOutput {
        exit_code: Some(0),
        stdout: WINDOWS_OUTPUT.to_string(),
    };
    let r = ClaudeUsageProbe::interpret(&out, utc(2026, 9, 22, 20, 0)).unwrap();
    assert_eq!(r.session, Some(0.12));
}

/// Uma linha de aviso antes do resumo (o de confiança de pasta, por exemplo,
/// que fala de "projects") não vira plano "pro".
#[test]
fn a_warning_line_does_not_turn_into_a_plan() {
    let text = "Ignoring 4 permissions.allow entries: this workspace has not been trusted, or set projects[x]\r\nYou are currently using your subscription to power your Claude Code usage\r\n";
    assert_eq!(
        ClaudeUsageProbe::plan(text).as_deref(),
        Some("subscription")
    );
}

// --- "Por modelo entra na decisão" e "Procedência" (o resto do ProbeTests.swift) ---

fn one_account_config(email: &str) -> (RouterConfig, router_core::Id) {
    let conta = account(email, "C:/Users/exemplo/c");
    let mut group = AccountGroup::new("g", ConfigDir::dedicated("C:/Users/exemplo/g"));
    group.account_ids = vec![conta.id];
    let id = conta.id;
    (
        RouterConfig {
            accounts: vec![conta],
            groups: vec![group],
            ..Default::default()
        },
        id,
    )
}

/// O caso real que a lacuna escondia: 5h 91% e 7d 79% parecem folga contra um
/// limiar de 95%, mas o Fable a 100% já travou a conta.
#[test]
fn a_model_at_100_percent_rules_even_with_5h_and_7d_below_the_threshold() {
    let tmp = tempfile::tempdir().unwrap();
    let now = Utc::now();
    let later = |h: i64| Some(now + chrono::Duration::hours(h));
    let sample = GroupUsageSample::new(
        "C:/Users/exemplo/g",
        Some("conta2@exemplo.com".into()),
        Some(0.91),
        later(1),
        Some(0.79),
        later(24),
        now,
        Some(ModelUsage::new(
            vec![ModelWindow::new("Fable", 1.0, later(24))],
            now,
        )),
        UsageOrigin::Probe,
    );
    GroupUsageStore::write(&sample, "conta2@exemplo.com", tmp.path()).unwrap();
    let (config, id) = one_account_config("conta2@exemplo.com");

    let usage = GroupUsageReader::new(tmp.path())
        .detail_by_account(&config, now)
        .remove(&id)
        .unwrap();
    assert_eq!(usage.fraction, 1.0);
    assert_eq!(usage.window, UsageWindow::Model("Fable".into()));
    assert_eq!(usage.five_hour, Some(0.91));
    assert_eq!(usage.seven_day, Some(0.79));
}

/// Janela por modelo cujo reset já passou não deixa a conta "cheia" para sempre.
#[test]
fn a_model_window_past_its_reset_is_dropped() {
    let tmp = tempfile::tempdir().unwrap();
    let now = Utc::now();
    let at = |h: i64| Some(now + chrono::Duration::hours(h));
    let sample = GroupUsageSample::new(
        "C:/Users/exemplo/g",
        Some("x@exemplo.com".into()),
        Some(0.10),
        at(1),
        Some(0.20),
        at(24),
        now,
        Some(ModelUsage::new(
            vec![ModelWindow::new(
                "Fable",
                1.0,
                Some(now - chrono::Duration::minutes(1)),
            )],
            now,
        )),
        UsageOrigin::Probe,
    );
    GroupUsageStore::write(&sample, "x@exemplo.com", tmp.path()).unwrap();
    let (config, id) = one_account_config("x@exemplo.com");

    let usage = GroupUsageReader::new(tmp.path())
        .detail_by_account(&config, now)
        .remove(&id)
        .unwrap();
    assert_eq!(usage.fraction, 0.20);
    assert_eq!(usage.window, UsageWindow::SevenDay);
    assert_eq!(usage.model, None);
}

#[test]
fn the_origin_survives_the_round_trip_to_disk() {
    let tmp = tempfile::tempdir().unwrap();
    let sample = GroupUsageSample::new(
        "C:/Users/exemplo/g",
        Some("x@exemplo.com".into()),
        Some(0.1),
        None,
        Some(0.2),
        None,
        Utc::now(),
        None,
        UsageOrigin::Probe,
    );
    GroupUsageStore::write(&sample, "x@exemplo.com", tmp.path()).unwrap();
    assert_eq!(
        GroupUsageStore::read("x@exemplo.com", tmp.path()).map(|s| s.origin()),
        Some(UsageOrigin::Probe)
    );
}

/// O sensor por cima da sonda: 5h/7d passam a ser dele (a origem acompanha),
/// mas as janelas por modelo ficam — ele nunca as recebe.
#[test]
fn the_sensor_over_a_probe_takes_the_origin_and_keeps_the_model() {
    let tmp = tempfile::tempdir().unwrap();
    let probed = Utc::now() - chrono::Duration::minutes(10);
    GroupUsageStore::write(
        &GroupUsageSample::new(
            "C:/Users/exemplo/g",
            Some("x@exemplo.com".into()),
            Some(0.5),
            None,
            Some(0.5),
            None,
            probed,
            Some(ModelUsage::new(
                vec![ModelWindow::new("Fable", 0.9, None)],
                probed,
            )),
            UsageOrigin::Probe,
        ),
        "x@exemplo.com",
        tmp.path(),
    )
    .unwrap();
    GroupUsageStore::write(
        &GroupUsageSample::new(
            "C:/Users/exemplo/g",
            Some("x@exemplo.com".into()),
            Some(0.6),
            None,
            Some(0.6),
            None,
            Utc::now(),
            None,
            UsageOrigin::Sensor,
        ),
        "x@exemplo.com",
        tmp.path(),
    )
    .unwrap();

    let read = GroupUsageStore::read("x@exemplo.com", tmp.path()).unwrap();
    assert_eq!(read.origin(), UsageOrigin::Sensor);
    assert_eq!(read.models.unwrap().windows[0].percent, 0.9);
}

/// A origem chega ao `AccountUsage` que a UI lê — sem isso a tela atribuía ao
/// sensor um número que a sonda tinha acabado de buscar.
#[test]
fn the_origin_reaches_the_account_usage_the_ui_reads() {
    let tmp = tempfile::tempdir().unwrap();
    let now = Utc::now();
    let at = |h: i64| Some(now + chrono::Duration::hours(h));
    GroupUsageStore::write(
        &GroupUsageSample::new(
            "C:/Users/exemplo/g",
            Some("ociosa@exemplo.com".into()),
            Some(0.26),
            at(1),
            Some(0.38),
            at(24),
            now,
            None,
            UsageOrigin::Probe,
        ),
        "ociosa@exemplo.com",
        tmp.path(),
    )
    .unwrap();
    let (config, id) = one_account_config("ociosa@exemplo.com");

    let usage = GroupUsageReader::new(tmp.path())
        .detail_by_account(&config, now)
        .remove(&id)
        .unwrap();
    assert_eq!(usage.origin, UsageOrigin::Probe);
}
