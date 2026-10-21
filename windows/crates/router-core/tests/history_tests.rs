//! O histórico de uso (21/10/2026, só do Windows): um ponto por medição da
//! sonda em `<base>\history\<email>.jsonl`, para a aba Histórico do app.

mod common;
use common::*;

use chrono::{Duration, SecondsFormat};
use router_core::engine::group_usage::{GroupUsageStore, UsageOrigin};
use router_core::engine::usage_history::{UsageHistory, UsageHistoryPoint};
use router_core::usage::claude_usage_probe::{ClaudeUsageProbe, ProbeOutput, ProbeTarget};
use router_core::{ConfigDir, Id};

const EMAIL: &str = "conta1@exemplo.com";
/// Outubro de 2026.
const T0: i64 = 1_792_000_000;

fn point(secs: i64, five: Option<f64>) -> UsageHistoryPoint {
    UsageHistoryPoint {
        sampled_at: ts(secs),
        five_hour_percent: five,
        five_hour_resets_at: None,
        seven_day_percent: Some(0.5),
        seven_day_resets_at: None,
        models: Vec::new(),
    }
}

#[test]
fn points_come_back_oldest_first() {
    let tmp = tempfile::tempdir().unwrap();
    let now = ts(T0 + 3600);
    UsageHistory::append(&point(T0 + 3600, Some(0.2)), EMAIL, tmp.path(), now).unwrap();
    UsageHistory::append(&point(T0, Some(0.1)), EMAIL, tmp.path(), now).unwrap();

    let fives: Vec<Option<f64>> = UsageHistory::read(EMAIL, tmp.path())
        .iter()
        .map(|p| p.five_hour_percent)
        .collect();
    assert_eq!(fives, [Some(0.1), Some(0.2)]);
    assert_eq!(
        UsageHistory::last_sampled_at(EMAIL, tmp.path()),
        Some(ts(T0 + 3600))
    );
}

#[test]
fn points_older_than_ninety_days_go_with_the_next_write() {
    let tmp = tempfile::tempdir().unwrap();
    UsageHistory::append(&point(T0, Some(0.1)), EMAIL, tmp.path(), ts(T0)).unwrap();
    let later = ts(T0) + Duration::days(UsageHistory::RETENTION_DAYS) + Duration::hours(1);
    UsageHistory::append(
        &point(later.timestamp(), Some(0.2)),
        EMAIL,
        tmp.path(),
        later,
    )
    .unwrap();

    let read = UsageHistory::read(EMAIL, tmp.path());
    assert_eq!(read.len(), 1, "o de mais de 90 dias saiu");
    assert_eq!(read[0].five_hour_percent, Some(0.2));
}

/// Uma linha estragada (escrita à mão, disco cheio) não leva as outras — e a
/// próxima gravação regrava o arquivo só com as boas.
#[test]
fn a_broken_line_is_skipped_and_the_others_stay() {
    let tmp = tempfile::tempdir().unwrap();
    let line = |p: &UsageHistoryPoint| serde_json::to_string(p).unwrap();
    std::fs::write(
        tmp.path().join(UsageHistory::file_name(EMAIL)),
        format!(
            "{}\n{{ meio escrito\n{}\r\n",
            line(&point(T0, Some(0.1))),
            line(&point(T0 + 60, Some(0.2)))
        ),
    )
    .unwrap();

    assert_eq!(UsageHistory::read(EMAIL, tmp.path()).len(), 2);

    UsageHistory::append(&point(T0 + 120, Some(0.3)), EMAIL, tmp.path(), ts(T0 + 120)).unwrap();
    let text = std::fs::read_to_string(tmp.path().join(UsageHistory::file_name(EMAIL))).unwrap();
    assert_eq!(text.lines().count(), 3);
    assert!(!text.contains("meio escrito"));
}

#[test]
fn a_missing_file_is_an_empty_history() {
    let tmp = tempfile::tempdir().unwrap();
    assert!(UsageHistory::read(EMAIL, tmp.path()).is_empty());
    assert_eq!(UsageHistory::last_sampled_at(EMAIL, tmp.path()), None);
}

/// O mesmo nome do `usage\` (a mesma limpeza do e-mail), com `.jsonl`.
#[test]
fn the_file_is_named_like_the_sample_with_jsonl() {
    assert_eq!(
        UsageHistory::file_name("conta 1+x@exemplo.com"),
        "conta_1_x@exemplo.com.jsonl"
    );
    assert_eq!(
        GroupUsageStore::file_name("conta 1+x@exemplo.com"),
        "conta_1_x@exemplo.com.json"
    );
    assert!(UsageHistory::directory(std::path::Path::new("C:/base")).ends_with("history"));
}

/// Como a amostra: data sem fração (o decodificador do Swift a recusa, e o
/// formato em disco é um só), e janela ausente fica fora da linha — nunca zero.
#[test]
fn a_line_has_dates_without_fraction_and_no_absent_window() {
    let tmp = tempfile::tempdir().unwrap();
    let at = ts(T0) + Duration::milliseconds(500);
    let p = UsageHistoryPoint {
        sampled_at: at,
        ..point(T0, None)
    };
    UsageHistory::append(&p, EMAIL, tmp.path(), at).unwrap();

    let text = std::fs::read_to_string(tmp.path().join(UsageHistory::file_name(EMAIL))).unwrap();
    let stamp = ts(T0).to_rfc3339_opts(SecondsFormat::Secs, true);
    assert!(
        text.contains(&format!("\"sampledAt\":\"{stamp}\"")),
        "{text}"
    );
    assert!(!text.contains("fiveHourPercent"), "{text}");
    assert!(text.contains("\"sevenDayPercent\":0.5"), "{text}");
    assert!(
        !text.contains("models"),
        "sem janela por modelo, nem a chave"
    );
}

// --- a sonda grava o ponto ---

/// A captura do Windows, anonimizada (a mesma do `probe_tests.rs`).
const WINDOWS_OUTPUT: &str = "You are currently using your subscription to power your Claude Code usage\r\n\r\nCurrent session: 12% used · resets Sep 22, 8:40pm (America/Sao_Paulo)\r\nCurrent week (all models): 34% used · resets Sep 23, 4am (America/Sao_Paulo)\r\nCurrent week (Fable): 56% used · resets Sep 23, 4am (America/Sao_Paulo)\r\n";

fn probe(stdout: &'static str) -> ClaudeUsageProbe {
    ClaudeUsageProbe::new(move |_| {
        Ok(ProbeOutput {
            exit_code: Some(0),
            stdout: stdout.to_string(),
        })
    })
}

fn target() -> ProbeTarget {
    ProbeTarget {
        account_id: Id::new(),
        label: "conta1".into(),
        email: EMAIL.into(),
        dir: ConfigDir::dedicated("C:/Users/exemplo/casa"),
    }
}

#[test]
fn the_probe_writes_the_sample_and_a_history_point() {
    let tmp = tempfile::tempdir().unwrap();
    let base = tmp.path();
    probe(WINDOWS_OUTPUT)
        .measure_into(&target(), &base.join("usage"), base, ts(T0))
        .unwrap();

    let sample = GroupUsageStore::read(EMAIL, &base.join("usage")).unwrap();
    assert_eq!(sample.origin(), UsageOrigin::Probe);
    let history = UsageHistory::read(EMAIL, &UsageHistory::directory(base));
    assert_eq!(history.len(), 1);
    assert_eq!(history[0].sampled_at, ts(T0));
    assert_eq!(history[0].five_hour_percent, Some(0.12));
    assert_eq!(history[0].seven_day_percent, Some(0.34));
    assert!(history[0].five_hour_resets_at.is_some());
    assert_eq!(history[0].models[0].name, "Fable");
}

/// Uma pasta de histórico que não se grava não desfaz a medição: a amostra
/// (o que decide a rotação) já está no disco.
#[test]
fn a_history_that_cannot_be_written_does_not_fail_the_measurement() {
    let tmp = tempfile::tempdir().unwrap();
    let base = tmp.path();
    // Um ARQUIVO onde a pasta do histórico deveria estar.
    std::fs::write(UsageHistory::directory(base), b"nao e pasta").unwrap();

    let reading = probe(WINDOWS_OUTPUT).measure_into(&target(), &base.join("usage"), base, ts(T0));

    assert!(reading.is_ok(), "{reading:?}");
    assert!(GroupUsageStore::read(EMAIL, &base.join("usage")).is_some());
}

/// A leitura sem a linha da sessão deixa o ponto SEM a janela de 5h — o
/// gráfico não a desenha em zero.
#[test]
fn a_reading_without_the_session_line_leaves_no_five_hour_value() {
    let tmp = tempfile::tempdir().unwrap();
    let base = tmp.path();
    probe("Current week (all models): 40% used\r\n")
        .measure_into(&target(), &base.join("usage"), base, ts(T0))
        .unwrap();

    let history = UsageHistory::read(EMAIL, &UsageHistory::directory(base));
    assert_eq!(history[0].five_hour_percent, None);
    assert_eq!(history[0].seven_day_percent, Some(0.4));
}

/// Medição que falhou (perfil sem login) não deixa ponto: o buraco no gráfico
/// é o registro honesto.
#[test]
fn a_failed_measurement_leaves_no_point() {
    let tmp = tempfile::tempdir().unwrap();
    let base = tmp.path();
    let result = probe("Total cost:            $0.0000\r\n").measure_into(
        &target(),
        &base.join("usage"),
        base,
        ts(T0),
    );
    assert!(result.is_err());
    assert!(UsageHistory::read(EMAIL, &UsageHistory::directory(base)).is_empty());
}
