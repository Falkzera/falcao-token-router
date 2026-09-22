//! Formato da amostra `usage/<email>.json`: round-trip, data sem fração, origem
//! ausente = sensor, preservação do bloco por modelo, sanitização do nome.

mod common;
use common::*;

use router_core::{GroupUsageSample, GroupUsageStore, ModelUsage, ModelWindow, UsageOrigin};

fn probe_sample_with_models(email: &str) -> GroupUsageSample {
    let models = ModelUsage::new(
        vec![ModelWindow::new("Fable", 0.23, Some(ts(99_000)))],
        ts(500),
    );
    GroupUsageSample::new(
        "/x",
        Some(email.to_string()),
        Some(0.10),
        Some(ts(90_000)),
        Some(0.20),
        Some(ts(99_000)),
        ts(2000),
        Some(models),
        UsageOrigin::Probe,
    )
}

#[test]
fn sample_round_trips() {
    let sample = probe_sample_with_models("conta1@exemplo.com");
    let data = serde_json::to_vec(&sample).unwrap();
    let back: GroupUsageSample = serde_json::from_slice(&data).unwrap();
    assert_eq!(back, sample);
    assert_eq!(back.origin(), UsageOrigin::Probe);
}

#[test]
fn dates_have_no_fractional_seconds() {
    // sampledAt 1000 = 1970-01-01T00:16:40Z — o decodificador iso8601 do Swift
    // recusa fração, então escrevemos sem ela.
    let sample = GroupUsageSample::new(
        "/x",
        Some("a@b".to_string()),
        Some(0.5),
        Some(ts(90_000)),
        None,
        None,
        ts(1000),
        None,
        UsageOrigin::Sensor,
    );
    let json = serde_json::to_string(&sample).unwrap();
    assert!(
        json.contains("\"sampledAt\":\"1970-01-01T00:16:40Z\""),
        "data com fração ou formato errado: {json}"
    );
}

#[test]
fn missing_origin_resolves_to_sensor() {
    // Amostra legada (antes da sonda) não tinha `origin`.
    let json = r#"{"configDirRaw":"/x","email":"a@b","sampledAt":"2026-09-18T12:00:00Z"}"#;
    let sample: GroupUsageSample = serde_json::from_str(json).unwrap();
    assert_eq!(sample.origin(), UsageOrigin::Sensor);
}

#[test]
fn sensor_write_preserves_the_model_block() {
    let tmp = tempfile::tempdir().unwrap();
    let email = "conta1@exemplo.com";
    // Primeiro a sonda grava com bloco por modelo (carimbo ts 500).
    GroupUsageStore::write(&probe_sample_with_models(email), email, tmp.path()).unwrap();

    // Depois o sensor grava SEM bloco por modelo (a cada mensagem).
    let sensor = GroupUsageSample::new(
        "/x",
        Some(email.to_string()),
        Some(0.11),
        Some(ts(91_000)),
        Some(0.22),
        Some(ts(99_000)),
        ts(3000),
        None,
        UsageOrigin::Sensor,
    );
    GroupUsageStore::write(&sensor, email, tmp.path()).unwrap();

    // O bloco por modelo tem de sobreviver, com o carimbo ORIGINAL (da sonda).
    let back = GroupUsageStore::read(email, tmp.path()).unwrap();
    let models = back.models.as_ref().expect("bloco por modelo preservado");
    assert_eq!(models.sampled_at, ts(500));
    assert_eq!(models.windows[0].name, "Fable");
    // E o resto é a leitura nova do sensor.
    assert_eq!(back.five_hour_percent, Some(0.11));
    assert_eq!(back.origin(), UsageOrigin::Sensor);
}

#[test]
fn file_name_sanitizes_email() {
    assert_eq!(
        GroupUsageStore::file_name("conta1@exemplo.com"),
        "conta1@exemplo.com.json"
    );
    // Caracteres fora de [letra, dígito, . @ -] viram `_`.
    assert_eq!(
        GroupUsageStore::file_name("a+b/c:d@exemplo.com"),
        "a_b_c_d@exemplo.com.json"
    );
}
