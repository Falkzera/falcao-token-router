//! O que a bandeja diz: a fração do anel e o tooltip (≙ `MenuBarLabel.swift`).
//!
//! Lógica pura, sobre dados simples — o `tray.rs` monta os dados do store.
//!
//! O anel mostra a conta que serve o grupo PADRÃO; sem padrão (ou sem ativa
//! nele), a do primeiro grupo que tiver uma ativa. O tooltip dá uma linha por
//! grupo, começando por esse, e toda linha com número carrega a **janela, a
//! origem e a idade** (invariante do produto: número sem procedência não vai
//! para a tela) — `Trabalho: equipe-2 · 7d 81% (sensor, 3m)`. Conta sem
//! amostra é "pronta" (anel vazio): o macOS punha ali o % do medidor local, que
//! é número de outra conta.

use chrono::{DateTime, Utc};
use router_core::engine::group_usage::UsageOrigin;
use router_core::{AccountUsage, UsagePercent, UsageWindow};

use crate::i18n::{t, Arg};
use crate::locale::Locale;

/// Um grupo como a bandeja o vê.
pub struct GroupStatus<'a> {
    pub name: &'a str,
    pub is_default: bool,
    /// A conta ativa (rótulo) e o uso dela, se houver amostra.
    pub active: Option<(&'a str, Option<&'a AccountUsage>)>,
}

#[derive(Clone, PartialEq, Debug)]
pub struct TrayView {
    /// A fração do anel; `None` = anel vazio (sem conta ativa, ou "pronta").
    pub fraction: Option<f64>,
    pub tooltip: String,
}

/// O limite do tooltip de ícone de bandeja no Windows: 128 unidades UTF-16 com
/// o terminador (`NOTIFYICONDATAW.szTip`).
pub const TOOLTIP_LIMIT: usize = 127;

/// O índice do grupo de destaque: o padrão, se tiver ativa; senão o primeiro
/// que tiver.
fn headline(groups: &[GroupStatus]) -> Option<usize> {
    groups
        .iter()
        .position(|g| g.is_default && g.active.is_some())
        .or_else(|| groups.iter().position(|g| g.active.is_some()))
}

/// "1h 12m" / "3m" (≙ `Format.duration`).
pub fn duration(locale: Locale, seconds: i64) -> String {
    let total = seconds.max(0);
    let (hours, minutes) = (total / 3600, (total % 3600) / 60);
    if hours > 0 {
        t(
            locale,
            "format.duration.hoursMinutes",
            &[Arg::Int(hours), Arg::Int(minutes)],
        )
    } else {
        t(locale, "format.duration.minutes", &[Arg::Int(minutes)])
    }
}

fn line(locale: Locale, group: &GroupStatus, now: DateTime<Utc>) -> String {
    let Some((label, usage)) = group.active else {
        return t(locale, "tray.tooltip.noActive.format", &[group.name.into()]);
    };
    let Some(usage) = usage else {
        return t(
            locale,
            "tray.tooltip.ready.format",
            &[
                group.name.into(),
                label.into(),
                t(locale, "panel.accounts.ready", &[]).into(),
            ],
        );
    };
    // A janela POR MODELO só vem da sonda, com carimbo próprio.
    let (window, origin, sampled_at) = match &usage.window {
        UsageWindow::FiveHour => (
            t(locale, "panel.accounts.window.fiveHour", &[]),
            usage.origin,
            usage.sampled_at,
        ),
        UsageWindow::SevenDay => (
            t(locale, "panel.accounts.window.sevenDay", &[]),
            usage.origin,
            usage.sampled_at,
        ),
        UsageWindow::Model(name) => (
            name.clone(),
            UsageOrigin::Probe,
            usage.model_sampled_at.unwrap_or(usage.sampled_at),
        ),
    };
    let origin = match origin {
        UsageOrigin::Sensor => t(locale, "tray.origin.sensor", &[]),
        UsageOrigin::Probe => t(locale, "tray.origin.probe", &[]),
    };
    t(
        locale,
        "tray.tooltip.usage.format",
        &[
            group.name.into(),
            label.into(),
            window.into(),
            UsagePercent::text(usage.fraction).into(),
            origin.into(),
            duration(locale, (now - sampled_at).num_seconds()).into(),
        ],
    )
}

fn utf16_len(text: &str) -> usize {
    text.encode_utf16().count()
}

/// Corta um texto em `limit` unidades UTF-16, com reticências.
fn truncate(text: &str, limit: usize) -> String {
    if utf16_len(text) <= limit {
        return text.to_string();
    }
    let mut out = String::new();
    for ch in text.chars() {
        if utf16_len(&out) + ch.len_utf16() + 1 > limit {
            break;
        }
        out.push(ch);
    }
    out.push('…');
    out
}

pub fn tray_view(groups: &[GroupStatus], locale: Locale, now: DateTime<Utc>) -> TrayView {
    if groups.is_empty() {
        return TrayView {
            fraction: None,
            tooltip: t(locale, "tray.tooltip.empty", &[]),
        };
    }
    let first = headline(groups);
    let fraction = first
        .and_then(|i| groups[i].active)
        .and_then(|(_, usage)| usage)
        .map(|u| u.fraction);

    // O grupo de destaque primeiro; os outros na ordem do usuário. Linhas
    // inteiras até o limite do Windows — nunca meia linha, salvo a primeira.
    let order = first
        .into_iter()
        .chain((0..groups.len()).filter(|i| Some(*i) != first));
    let mut tooltip = String::new();
    for (n, i) in order.enumerate() {
        let text = line(locale, &groups[i], now);
        if n == 0 {
            tooltip = truncate(&text, TOOLTIP_LIMIT);
            continue;
        }
        if utf16_len(&tooltip) + 1 + utf16_len(&text) > TOOLTIP_LIMIT {
            break;
        }
        tooltip.push('\n');
        tooltip.push_str(&text);
    }
    TrayView { fraction, tooltip }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use router_core::ModelWindow;

    fn now() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 9, 22, 12, 0, 0).unwrap()
    }

    fn usage(
        fraction: f64,
        window: UsageWindow,
        origin: UsageOrigin,
        age_min: i64,
    ) -> AccountUsage {
        AccountUsage {
            fraction,
            window,
            five_hour: Some(0.1),
            five_hour_resets_at: None,
            seven_day: Some(fraction),
            seven_day_resets_at: None,
            model: None,
            model_sampled_at: None,
            origin,
            sampled_at: now() - chrono::Duration::minutes(age_min),
        }
    }

    #[test]
    fn without_groups_the_tooltip_invites_to_create_one() {
        let view = tray_view(&[], Locale::PtBr, now());
        assert_eq!(view.fraction, None);
        assert!(view.tooltip.contains("crie um grupo"), "{}", view.tooltip);
    }

    /// Toda linha com número diz a janela, a origem e a idade.
    #[test]
    fn a_measured_account_carries_window_origin_and_age() {
        let u = usage(0.81, UsageWindow::SevenDay, UsageOrigin::Sensor, 3);
        let groups = [GroupStatus {
            name: "Trabalho",
            is_default: true,
            active: Some(("equipe-2", Some(&u))),
        }];
        let view = tray_view(&groups, Locale::PtBr, now());
        assert_eq!(view.fraction, Some(0.81));
        assert_eq!(view.tooltip, "Trabalho: equipe-2 · 7d 81% (sensor, 3m)");
    }

    #[test]
    fn a_probed_model_window_says_probe_and_its_own_age() {
        let mut u = usage(
            0.95,
            UsageWindow::Model("Fable".into()),
            UsageOrigin::Sensor,
            1,
        );
        u.model = Some(ModelWindow::new("Fable", 0.95, None));
        u.model_sampled_at = Some(now() - chrono::Duration::minutes(75));
        let groups = [GroupStatus {
            name: "Pessoal",
            is_default: false,
            active: Some(("conta1", Some(&u))),
        }];
        let view = tray_view(&groups, Locale::En, now());
        assert_eq!(view.tooltip, "Pessoal: conta1 · Fable 95% (probe, 1h 15m)");
    }

    /// Conta sem amostra está PRONTA (entra no rodízio sozinha): anel vazio,
    /// nenhum número — e não o % de outra conta.
    #[test]
    fn an_account_without_a_sample_is_ready_with_an_empty_ring() {
        let groups = [GroupStatus {
            name: "Trabalho",
            is_default: true,
            active: Some(("equipe-1", None)),
        }];
        let view = tray_view(&groups, Locale::PtBr, now());
        assert_eq!(view.fraction, None);
        assert_eq!(view.tooltip, "Trabalho: equipe-1 · pronta");
    }

    /// O anel é da conta do grupo PADRÃO; sem ativa no padrão, do primeiro
    /// grupo que tiver uma. E a linha dele vem primeiro.
    #[test]
    fn the_ring_follows_the_default_group_then_the_first_with_an_active_account() {
        let busy = usage(0.40, UsageWindow::FiveHour, UsageOrigin::Sensor, 5);
        let other = usage(0.70, UsageWindow::SevenDay, UsageOrigin::Probe, 30);
        let groups = [
            GroupStatus {
                name: "Pessoal",
                is_default: false,
                active: Some(("conta1", Some(&other))),
            },
            GroupStatus {
                name: "Trabalho",
                is_default: true,
                active: Some(("equipe-1", Some(&busy))),
            },
        ];
        let view = tray_view(&groups, Locale::PtBr, now());
        assert_eq!(view.fraction, Some(0.40));
        let lines: Vec<&str> = view.tooltip.lines().collect();
        assert!(lines[0].starts_with("Trabalho: "), "{lines:?}");
        assert!(lines[1].starts_with("Pessoal: "), "{lines:?}");

        let groups = [
            GroupStatus {
                name: "Trabalho",
                is_default: true,
                active: None,
            },
            GroupStatus {
                name: "Pessoal",
                is_default: false,
                active: Some(("conta1", Some(&other))),
            },
        ];
        let view = tray_view(&groups, Locale::PtBr, now());
        assert_eq!(view.fraction, Some(0.70));
        assert_eq!(
            view.tooltip,
            "Pessoal: conta1 · 7d 70% (sonda, 30m)\nTrabalho: nenhuma conta ativa"
        );
    }

    /// O Windows corta o tooltip em 127 unidades UTF-16: só linhas inteiras
    /// entram depois da primeira.
    #[test]
    fn the_tooltip_fits_the_windows_limit_with_whole_lines() {
        let u = usage(0.5, UsageWindow::FiveHour, UsageOrigin::Sensor, 1);
        let names: Vec<String> = (1..=8).map(|i| format!("grupo-numero-{i}")).collect();
        let groups: Vec<GroupStatus> = names
            .iter()
            .map(|n| GroupStatus {
                name: n,
                is_default: false,
                active: Some(("conta-com-nome-comprido", Some(&u))),
            })
            .collect();
        let view = tray_view(&groups, Locale::En, now());
        assert!(utf16_len(&view.tooltip) <= TOOLTIP_LIMIT);
        for line in view.tooltip.lines() {
            assert!(line.ends_with(')'), "linha cortada: {line}");
        }

        let long = "g".repeat(200);
        let groups = [GroupStatus {
            name: &long,
            is_default: true,
            active: None,
        }];
        let view = tray_view(&groups, Locale::En, now());
        assert!(utf16_len(&view.tooltip) <= TOOLTIP_LIMIT);
        assert!(view.tooltip.ends_with('…'));
    }
}
