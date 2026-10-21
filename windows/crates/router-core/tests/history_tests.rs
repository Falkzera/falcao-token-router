//! O histórico de uso (21/10/2026, só do Windows): um ponto por medição da
//! sonda em `<base>\history\<email>.jsonl`, para a aba Histórico do app.

mod common;
use common::*;

use chrono::{Duration, SecondsFormat};
use router_core::engine::group_usage::GroupUsageStore;
use router_core::engine::usage_history::{UsageHistory, UsageHistoryPoint};

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

    UsageHistory::append(
        &point(T0 + 120, Some(0.3)),
        EMAIL,
        tmp.path(),
        ts(T0 + 120),
    )
    .unwrap();
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
    assert!(!text.contains("models"), "sem janela por modelo, nem a chave");
}
