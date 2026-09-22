//! Porte de `GroupUsageReaderDecayTests` (macOS): o leitor descarta janela com
//! reset vencido, escolhe o maior e nomeia a procedência.

mod common;
use common::*;

use std::path::Path;

use router_core::{
    Account, GroupUsageReader, GroupUsageSample, GroupUsageStore, RouterConfig, UsageOrigin,
    UsageWindow,
};

fn write_sample(
    dir: &Path,
    email: &str,
    five: Option<f64>,
    five_r: Option<i64>,
    seven: Option<f64>,
    seven_r: Option<i64>,
) {
    let sample = GroupUsageSample::new(
        "/x",
        Some(email.to_string()),
        five,
        five_r.map(ts),
        seven,
        seven_r.map(ts),
        ts(1000),
        None,
        UsageOrigin::Sensor,
    );
    GroupUsageStore::write(&sample, email, dir).unwrap();
}

fn setup(
    five: Option<f64>,
    five_r: Option<i64>,
    seven: Option<f64>,
    seven_r: Option<i64>,
) -> (tempfile::TempDir, GroupUsageReader, RouterConfig, Account) {
    let tmp = tempfile::tempdir().unwrap();
    let a = account("a@k.com", "C:/Users/exemplo/.claude-a");
    write_sample(tmp.path(), "a@k.com", five, five_r, seven, seven_r);
    let config = RouterConfig {
        accounts: vec![a.clone()],
        ..Default::default()
    };
    let reader = GroupUsageReader::new(tmp.path());
    (tmp, reader, config, a)
}

#[test]
fn expired_window_is_dropped_and_the_survivor_counts() {
    // 5h a 96% já venceu (reset 1500 < now 2000); vale o 7d a 27%.
    let (_tmp, reader, config, a) = setup(Some(0.96), Some(1500), Some(0.27), Some(99_000));
    let usage = reader.usage_by_account(&config, ts(2000));
    assert_eq!(usage.get(&a.id), Some(&0.27));
}

#[test]
fn fully_expired_sample_disappears() {
    // As duas janelas vencidas → conta some do mapa (presumida fresca).
    let (_tmp, reader, config, a) = setup(Some(0.96), Some(1500), Some(0.90), Some(1600));
    let usage = reader.usage_by_account(&config, ts(2000));
    assert_eq!(usage.get(&a.id), None);
}

#[test]
fn fresh_sample_uses_the_larger_window() {
    let (_tmp, reader, config, a) = setup(Some(0.40), Some(90_000), Some(0.62), Some(99_000));
    let usage = reader.usage_by_account(&config, ts(2000));
    assert_eq!(usage.get(&a.id), Some(&0.62));
}

#[test]
fn missing_reset_keeps_the_window() {
    let (_tmp, reader, config, a) = setup(Some(0.96), None, None, None);
    let usage = reader.usage_by_account(&config, ts(2000));
    assert_eq!(usage.get(&a.id), Some(&0.96));
}

#[test]
fn detail_names_the_window() {
    let (_tmp, reader, config, a) = setup(Some(0.01), Some(90_000), Some(0.66), Some(99_000));
    let detail = reader.detail_by_account(&config, ts(2000));
    let d = detail.get(&a.id).unwrap();
    assert_eq!(d.fraction, 0.66);
    assert_eq!(d.window, UsageWindow::SevenDay);
    assert_eq!(d.five_hour, Some(0.01));
    assert_eq!(d.seven_day, Some(0.66));
}

#[test]
fn detail_reports_five_hour_when_it_leads() {
    let (_tmp, reader, config, a) = setup(Some(0.80), Some(90_000), Some(0.30), Some(99_000));
    let detail = reader.detail_by_account(&config, ts(2000));
    let d = detail.get(&a.id).unwrap();
    assert_eq!(d.fraction, 0.80);
    assert_eq!(d.window, UsageWindow::FiveHour);
}

#[test]
fn tie_prefers_seven_day() {
    // Empate: as duas limitam igual, mas a semanal leva dias para aliviar.
    let (_tmp, reader, config, a) = setup(Some(0.50), Some(90_000), Some(0.50), Some(99_000));
    let detail = reader.detail_by_account(&config, ts(2000));
    assert_eq!(detail.get(&a.id).unwrap().window, UsageWindow::SevenDay);
}

#[test]
fn expired_window_is_not_cited() {
    // 5h=100% de anteontem venceu; o que vale (e é citado) é o 7d.
    let (_tmp, reader, config, a) = setup(Some(1.0), Some(1500), Some(0.28), Some(99_000));
    let detail = reader.detail_by_account(&config, ts(2000));
    let d = detail.get(&a.id).unwrap();
    assert_eq!(d.fraction, 0.28);
    assert_eq!(d.window, UsageWindow::SevenDay);
    assert_eq!(d.five_hour, None);
}
