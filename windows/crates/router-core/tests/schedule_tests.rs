//! A medição automática de 5 em 5 horas (21/10/2026): a regra de QUANDO medir
//! de novo, pura — o relógio entra por parâmetro.

mod common;
use common::*;

use chrono::Duration;
use router_core::engine::measure_schedule::MeasureSchedule;

const NOW: i64 = 1_792_000_000;

#[test]
fn an_account_never_measured_nor_tried_is_due() {
    assert!(MeasureSchedule::is_due(None, None, ts(NOW)));
    assert_eq!(MeasureSchedule::next_at(None, None), None);
}

#[test]
fn a_measurement_under_five_hours_old_is_not_due() {
    let now = ts(NOW);
    assert!(!MeasureSchedule::is_due(
        Some(now - Duration::hours(4)),
        None,
        now
    ));
    assert!(!MeasureSchedule::is_due(
        Some(now - Duration::minutes(299)),
        None,
        now
    ));
}

#[test]
fn exactly_five_hours_is_due() {
    let now = ts(NOW);
    assert!(MeasureSchedule::is_due(
        Some(now - Duration::hours(5)),
        None,
        now
    ));
    assert_eq!(
        MeasureSchedule::next_at(Some(now - Duration::hours(5)), None),
        Some(now)
    );
}

/// A conta deslogada falha toda vez: sem a tentativa, ela seria sondada a
/// cada volta do laço (3 min). Com ela, de novo só em 5 horas.
#[test]
fn a_failed_attempt_holds_the_account_for_five_hours() {
    let now = ts(NOW);
    let old = Some(now - Duration::days(2));
    assert!(!MeasureSchedule::is_due(
        old,
        Some(now - Duration::hours(1)),
        now
    ));
    assert!(MeasureSchedule::is_due(
        old,
        Some(now - Duration::hours(5)),
        now
    ));
    // A mais nova das duas manda.
    assert_eq!(
        MeasureSchedule::next_at(
            Some(now - Duration::hours(1)),
            Some(now - Duration::hours(3))
        ),
        Some(now + Duration::hours(4))
    );
}
