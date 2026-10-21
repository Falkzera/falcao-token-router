//! A aba Histórico (21/10/2026): o que cada conta mediu ao longo do tempo,
//! pronto para desenhar. Duas séries por conta — a janela de 5h nos últimos 7
//! dias e a semanal nas últimas 4 semanas —, recortadas, quebradas nos buracos
//! e com os textos escritos aqui: o % pelo `UsagePercent` do núcleo, as horas
//! pelo `reset_when` da status line, no fuso do Windows e no idioma do app. A
//! tela só converte tempo em x e fração em y.
//!
//! Os pontos são as medições da SONDA (`router_core::engine::usage_history`):
//! a de 5 em 5 horas, o "Medir contas" e o `router measure`.

use std::collections::HashMap;

use chrono::{DateTime, Datelike, Duration, NaiveDate, SecondsFormat, TimeZone, Utc};
use router_core::engine::measure_schedule::MeasureSchedule;
use router_core::engine::router_config_store::RouterConfigStore;
use router_core::engine::usage_history::{UsageHistory, UsageHistoryPoint};
use router_core::statusline::view::{reset_when, weekday_abbrev};
use router_core::{Account, Id, UsagePercent};
use serde::Serialize;
use tauri::{AppHandle, Manager};

use crate::locale::Locale;
use crate::settings::SettingsStore;
use crate::state::AppState;

/// Quantos dias cada gráfico mostra, contados de agora para trás.
const FIVE_HOUR_DAYS: i64 = 7;
const SEVEN_DAY_DAYS: i64 = 28;

/// Um buraco maior que isto entre dois pontos quebra a linha: 1,5 × o
/// intervalo da agenda (7h30). App fechado, PC dormindo — o gráfico não
/// desenha o que não foi medido.
fn max_gap() -> Duration {
    Duration::minutes(MeasureSchedule::EVERY_HOURS * 60 * 3 / 2)
}

fn iso(at: DateTime<Utc>) -> String {
    at.to_rfc3339_opts(SecondsFormat::Secs, true)
}

/// Um ponto como a tela o desenha: a fração (posição), o % pronto e as horas
/// escritas.
#[derive(Clone, PartialEq, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryPoint {
    pub at: String,
    pub fraction: f64,
    pub text: String,
    /// Quando foi medido, como a status line escreve um reset semanal:
    /// "qua (21) 10:05".
    pub label: String,
    /// Quando aquela janela resetava, se a leitura disse.
    pub resets_label: Option<String>,
}

/// Uma marca do eixo do tempo: a meia-noite local e o que se escreve nela.
#[derive(Clone, PartialEq, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryTick {
    pub at: String,
    pub label: String,
}

#[derive(Clone, PartialEq, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HistorySeries {
    pub from: String,
    pub to: String,
    pub ticks: Vec<HistoryTick>,
    /// Os trechos da linha, do mais velho ao mais novo.
    pub segments: Vec<Vec<HistoryPoint>>,
    /// O ponto mais novo do período — o número no título do gráfico.
    pub latest: Option<HistoryPoint>,
}

#[derive(Clone, PartialEq, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountHistory {
    pub id: String,
    pub label: String,
    pub email: String,
    pub last_measured_at: Option<String>,
    /// Com a agenda ligada: quando a conta é medida de novo. Pode já ter
    /// passado — a tela diz "em instantes".
    pub next_measure_at: Option<String>,
    /// A hora da próxima, escrita ("19:05").
    pub next_measure_label: Option<String>,
    pub five_hour: HistorySeries,
    pub seven_day: HistorySeries,
}

#[derive(Clone, PartialEq, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GroupHistory {
    pub id: String,
    pub name: String,
    /// O limiar do grupo: a linha tracejada dos dois gráficos.
    pub threshold_percent: f64,
    pub accounts: Vec<AccountHistory>,
}

#[derive(Clone, PartialEq, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryView {
    pub schedule_on: bool,
    pub groups: Vec<GroupHistory>,
}

/// As duas janelas que o Histórico desenha.
#[derive(Clone, Copy)]
enum Window {
    FiveHour,
    SevenDay,
}

impl Window {
    fn days(self) -> i64 {
        match self {
            Window::FiveHour => FIVE_HOUR_DAYS,
            Window::SevenDay => SEVEN_DAY_DAYS,
        }
    }

    /// A fração e o reset desta janela num ponto. Ausente: o ponto não entra
    /// neste gráfico (nunca em zero).
    fn of(self, point: &UsageHistoryPoint) -> Option<(f64, Option<DateTime<Utc>>)> {
        match self {
            Window::FiveHour => point
                .five_hour_percent
                .map(|f| (f, point.five_hour_resets_at)),
            Window::SevenDay => point
                .seven_day_percent
                .map(|f| (f, point.seven_day_resets_at)),
        }
    }

    /// O reset de 5h cai nas próximas horas (só a hora); o semanal, com o dia.
    fn reset_with_day(self) -> bool {
        matches!(self, Window::SevenDay)
    }
}

/// Uma hora escrita como a status line a escreve, no fuso dado.
fn written<Tz: TimeZone>(at: DateTime<Utc>, with_day: bool, tz: &Tz, portuguese: bool) -> String {
    reset_when(at.with_timezone(tz).naive_local(), with_day, portuguese)
}

/// A meia-noite local de um dia. Num salto de horário de verão à meia-noite
/// (o Brasil já teve), a primeira hora que existe.
fn local_midnight<Tz: TimeZone>(date: NaiveDate, tz: &Tz) -> Option<DateTime<Utc>> {
    let midnight = date.and_hms_opt(0, 0, 0)?;
    tz.from_local_datetime(&midnight)
        .earliest()
        .or_else(|| {
            tz.from_local_datetime(&(midnight + Duration::hours(1)))
                .earliest()
        })
        .map(|at| at.with_timezone(&Utc))
}

/// As marcas do eixo do tempo: a meia-noite de cada dia (5h: o dia da semana)
/// ou de 7 em 7 dias contando de hoje (semanal: a data) — só as que caem
/// dentro do período.
fn ticks<Tz: TimeZone>(
    window: Window,
    from: DateTime<Utc>,
    now: DateTime<Utc>,
    tz: &Tz,
    portuguese: bool,
) -> Vec<HistoryTick> {
    let step = match window {
        Window::FiveHour => 1,
        Window::SevenDay => 7,
    };
    let today = now.with_timezone(tz).date_naive();
    let mut ticks = Vec::new();
    for back in (0..=window.days()).step_by(step) {
        let date = today - Duration::days(back);
        let Some(at) = local_midnight(date, tz) else {
            continue;
        };
        if at <= from || at > now {
            continue;
        }
        let label = match window {
            Window::FiveHour => weekday_abbrev(date.weekday(), portuguese).to_string(),
            Window::SevenDay if portuguese => date.format("%d/%m").to_string(),
            Window::SevenDay => date.format("%b %-d").to_string(),
        };
        ticks.push(HistoryTick { at: iso(at), label });
    }
    ticks.reverse();
    ticks
}

/// Uma série: os pontos do período que têm a janela, quebrados nos buracos.
fn series<Tz: TimeZone>(
    points: &[UsageHistoryPoint],
    window: Window,
    now: DateTime<Utc>,
    tz: &Tz,
    portuguese: bool,
) -> HistorySeries {
    let from = now - Duration::days(window.days());
    let mut segments: Vec<Vec<HistoryPoint>> = Vec::new();
    let mut previous: Option<DateTime<Utc>> = None;
    for point in points
        .iter()
        .filter(|p| p.sampled_at >= from && p.sampled_at <= now)
    {
        let Some((fraction, resets_at)) = window.of(point) else {
            continue;
        };
        let drawn = HistoryPoint {
            at: iso(point.sampled_at),
            fraction,
            text: UsagePercent::text(fraction),
            label: written(point.sampled_at, true, tz, portuguese),
            resets_label: resets_at.map(|at| written(at, window.reset_with_day(), tz, portuguese)),
        };
        let joins = previous.is_some_and(|before| point.sampled_at - before <= max_gap());
        match segments.last_mut() {
            Some(segment) if joins => segment.push(drawn),
            _ => segments.push(vec![drawn]),
        }
        previous = Some(point.sampled_at);
    }
    let latest = segments.last().and_then(|s| s.last()).cloned();
    HistorySeries {
        from: iso(from),
        to: iso(now),
        ticks: ticks(window, from, now, tz, portuguese),
        segments,
        latest,
    }
}

fn account_history<Tz: TimeZone>(
    account: &Account,
    points: &[UsageHistoryPoint],
    attempt: Option<DateTime<Utc>>,
    schedule_on: bool,
    now: DateTime<Utc>,
    tz: &Tz,
    portuguese: bool,
) -> AccountHistory {
    let last = points.last().map(|p| p.sampled_at);
    // Com a agenda ligada: 5 horas depois da mais nova entre a medição e a
    // tentativa. Nunca medida nem tentada: já (a próxima volta a mede).
    let next = schedule_on.then(|| MeasureSchedule::next_at(last, attempt).unwrap_or(now));
    AccountHistory {
        id: account.id.to_string(),
        label: account.label().to_string(),
        email: account.identity.email.clone(),
        last_measured_at: last.map(iso),
        next_measure_at: next.map(iso),
        next_measure_label: next.map(|at| written(at, false, tz, portuguese)),
        five_hour: series(points, Window::FiveHour, now, tz, portuguese),
        seven_day: series(points, Window::SevenDay, now, tz, portuguese),
    }
}

/// O Histórico inteiro: grupos na ordem da lista, contas na ordem de rotação
/// (a que está em dois grupos aparece nos dois). Fuso e relógio por parâmetro:
/// os testes fixam os dois.
pub fn build<Tz: TimeZone>(
    store: &RouterConfigStore,
    attempts: &HashMap<Id, DateTime<Utc>>,
    schedule_on: bool,
    now: DateTime<Utc>,
    tz: &Tz,
    portuguese: bool,
) -> HistoryView {
    let dir = store.paths().history_dir();
    let config = store.config();
    HistoryView {
        schedule_on,
        groups: config
            .groups
            .iter()
            .map(|group| GroupHistory {
                id: group.id.to_string(),
                name: group.name.clone(),
                threshold_percent: group.threshold_percent,
                accounts: config
                    .accounts_in(group)
                    .into_iter()
                    .map(|account| {
                        let points = UsageHistory::read(&account.identity.email, &dir);
                        account_history(
                            account,
                            &points,
                            attempts.get(&account.id).copied(),
                            schedule_on,
                            now,
                            tz,
                            portuguese,
                        )
                    })
                    .collect(),
            })
            .collect(),
    }
}

/// A aba Histórico pede o quadro dela — ao abrir e a cada `snapshot-changed`.
#[tauri::command]
pub fn get_history(app: AppHandle) -> HistoryView {
    let state = app.state::<AppState>();
    let schedule_on = app.state::<SettingsStore>().get().scheduled_measure;
    let attempts = state.measure_attempts();
    let store = state.store();
    build(
        &store,
        &attempts,
        schedule_on,
        Utc::now(),
        &chrono::Local,
        state.locale == Locale::PtBr,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;
    use std::sync::Arc;

    use chrono::FixedOffset;
    use router_core::engine::anthropic_adapter::AnthropicAdapter;
    use router_core::engine::credential_store::FileCredentialStore;
    use router_core::engine::provider::ProviderAdapter;
    use router_core::engine::router_paths::RouterPaths;
    use serde_json::json;

    const A1: &str = "AAAAAAAA-0000-4000-8000-000000000001";
    const A2: &str = "AAAAAAAA-0000-4000-8000-000000000002";
    const G1: &str = "11111111-1111-4111-8111-111111111111";
    const G2: &str = "22222222-2222-4222-8222-222222222222";

    /// Quarta, 21/10/2026, meio-dia em São Paulo (UTC−3, sem horário de verão).
    fn now() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 10, 21, 15, 0, 0).unwrap()
    }

    fn sao_paulo() -> FixedOffset {
        FixedOffset::west_opt(3 * 3600).unwrap()
    }

    /// Dois grupos: Trabalho (conta1, conta2; limiar 90) e Pessoal (conta2;
    /// limiar 85) — a conta2 está nos dois.
    fn store(tmp: &Path) -> RouterConfigStore {
        let paths = RouterPaths::with_app_support(Some(tmp.join("app")));
        let base = paths.base.clone();
        let account = |id: &str, email: &str| {
            json!({"id": id, "provider": "anthropic",
                   "identity": {"email": email, "organizationName": null, "raw": {"emailAddress": email}},
                   "home": {"raw": base.join("accounts").join(id).to_string_lossy(), "isDefault": false}})
        };
        let group = |id: &str, name: &str, accounts: &[&str], threshold: f64| {
            json!({"id": id, "name": name, "provider": "anthropic", "accountIDs": accounts,
                   "configDir": {"raw": base.join("groups").join(id).to_string_lossy(), "isDefault": false},
                   "thresholdPercent": threshold, "autoRotate": true})
        };
        std::fs::create_dir_all(&base).unwrap();
        let config = json!({
            "version": 1, "shareHistory": true,
            "accounts": [account(A1, "conta1@exemplo.com"), account(A2, "conta2@exemplo.com")],
            "groups": [group(G1, "Trabalho", &[A1, A2], 90.0), group(G2, "Pessoal", &[A2], 85.0)],
        });
        std::fs::write(paths.config_file(), serde_json::to_vec(&config).unwrap()).unwrap();
        let adapters: Vec<Arc<dyn ProviderAdapter>> = vec![Arc::new(AnthropicAdapter)];
        RouterConfigStore::new(
            paths,
            tmp.join("home").to_string_lossy().into_owned(),
            Arc::new(FileCredentialStore::new()),
            adapters,
        )
        .with_session_reader(|_| Vec::new())
    }

    fn point(at: DateTime<Utc>, five: Option<f64>, seven: Option<f64>) -> UsageHistoryPoint {
        UsageHistoryPoint {
            sampled_at: at,
            five_hour_percent: five,
            five_hour_resets_at: None,
            seven_day_percent: seven,
            seven_day_resets_at: None,
            models: Vec::new(),
        }
    }

    fn record(store: &RouterConfigStore, email: &str, points: &[UsageHistoryPoint]) {
        for p in points {
            UsageHistory::append(p, email, &store.paths().history_dir(), now()).unwrap();
        }
    }

    fn view(
        store: &RouterConfigStore,
        attempts: &HashMap<Id, DateTime<Utc>>,
        schedule_on: bool,
        portuguese: bool,
    ) -> HistoryView {
        build(
            store,
            attempts,
            schedule_on,
            now(),
            &sao_paulo(),
            portuguese,
        )
    }

    fn fractions(series: &HistorySeries) -> Vec<f64> {
        series
            .segments
            .iter()
            .flatten()
            .map(|p| p.fraction)
            .collect()
    }

    #[test]
    fn groups_and_accounts_follow_the_list_with_the_threshold() {
        let tmp = tempfile::tempdir().unwrap();
        let history = view(&store(tmp.path()), &HashMap::new(), true, true);

        let names: Vec<&str> = history.groups.iter().map(|g| g.name.as_str()).collect();
        assert_eq!(names, ["Trabalho", "Pessoal"]);
        assert_eq!(history.groups[0].threshold_percent, 90.0);
        assert_eq!(history.groups[1].threshold_percent, 85.0);
        let emails = |g: &GroupHistory| -> Vec<String> {
            g.accounts.iter().map(|a| a.email.clone()).collect()
        };
        assert_eq!(
            emails(&history.groups[0]),
            ["conta1@exemplo.com", "conta2@exemplo.com"]
        );
        assert_eq!(
            emails(&history.groups[1]),
            ["conta2@exemplo.com"],
            "a conta em dois grupos aparece nos dois"
        );
    }

    #[test]
    fn each_chart_keeps_its_own_period() {
        let tmp = tempfile::tempdir().unwrap();
        let store = store(tmp.path());
        record(
            &store,
            "conta1@exemplo.com",
            &[
                point(now() - Duration::days(30), Some(0.1), Some(0.9)),
                point(now() - Duration::days(10), Some(0.2), Some(0.5)),
                point(now() - Duration::days(2), Some(0.3), Some(0.6)),
            ],
        );
        let account = &view(&store, &HashMap::new(), true, true).groups[0].accounts[0];

        assert_eq!(fractions(&account.five_hour), [0.3]);
        assert_eq!(fractions(&account.seven_day), [0.5, 0.6]);
        assert_eq!(account.five_hour.from, iso(now() - Duration::days(7)));
        assert_eq!(account.seven_day.from, iso(now() - Duration::days(28)));
        assert_eq!(account.five_hour.to, iso(now()));
    }

    /// App fechado, PC dormindo: mais de 7h30 sem ponto quebra a linha — o
    /// gráfico não liga o que não foi medido. 7h30 exatas ainda ligam.
    #[test]
    fn a_hole_longer_than_seven_and_a_half_hours_breaks_the_line() {
        let tmp = tempfile::tempdir().unwrap();
        let store = store(tmp.path());
        let start = now() - Duration::days(1);
        record(
            &store,
            "conta1@exemplo.com",
            &[
                point(start, Some(0.1), None),
                point(start + Duration::hours(5), Some(0.2), None),
                point(start + Duration::minutes(5 * 60 + 450), Some(0.3), None),
                point(start + Duration::hours(21), Some(0.4), None),
            ],
        );
        let five = &view(&store, &HashMap::new(), true, true).groups[0].accounts[0].five_hour;

        let shape: Vec<Vec<f64>> = five
            .segments
            .iter()
            .map(|s| s.iter().map(|p| p.fraction).collect())
            .collect();
        assert_eq!(shape, [vec![0.1, 0.2, 0.3], vec![0.4]]);
        assert_eq!(five.latest.as_ref().map(|p| p.fraction), Some(0.4));
    }

    /// Ausência não é zero: o ponto sem a janela de 5h fica fora desse gráfico.
    #[test]
    fn a_point_without_the_window_is_left_out_not_drawn_at_zero() {
        let tmp = tempfile::tempdir().unwrap();
        let store = store(tmp.path());
        record(
            &store,
            "conta1@exemplo.com",
            &[point(now() - Duration::hours(1), None, Some(0.4))],
        );
        let account = &view(&store, &HashMap::new(), true, true).groups[0].accounts[0];

        assert!(account.five_hour.segments.is_empty());
        assert!(account.five_hour.latest.is_none());
        assert_eq!(
            account.seven_day.latest.as_ref().map(|p| p.text.as_str()),
            Some("40%")
        );
    }

    /// As marcas: a meia-noite de cada dia (com os nomes da status line) e as
    /// de 7 em 7 dias contando de hoje, nos dois idiomas.
    #[test]
    fn the_time_axis_marks_days_and_weeks_in_both_languages() {
        let tmp = tempfile::tempdir().unwrap();
        let store = store(tmp.path());
        let labels = |s: &HistorySeries| -> Vec<String> {
            s.ticks.iter().map(|t| t.label.clone()).collect()
        };

        let pt = &view(&store, &HashMap::new(), true, true).groups[0].accounts[0];
        assert_eq!(
            labels(&pt.five_hour),
            ["qui", "sex", "sáb", "dom", "seg", "ter", "qua"]
        );
        assert_eq!(labels(&pt.seven_day), ["30/09", "07/10", "14/10", "21/10"]);
        // A marca é a meia-noite LOCAL: 21/10 00:00 em UTC−3 = 03:00Z.
        assert_eq!(pt.seven_day.ticks[3].at, "2026-10-21T03:00:00Z");

        let en = &view(&store, &HashMap::new(), true, false).groups[0].accounts[0];
        assert_eq!(
            labels(&en.five_hour),
            ["Thu", "Fri", "Sat", "Sun", "Mon", "Tue", "Wed"]
        );
        assert_eq!(
            labels(&en.seven_day),
            ["Sep 30", "Oct 7", "Oct 14", "Oct 21"]
        );
    }

    /// O % é o do núcleo (0,666 → 67%), e as horas vêm escritas como a status
    /// line as escreve, no fuso dado.
    #[test]
    fn points_come_with_the_percent_and_the_times_written() {
        let tmp = tempfile::tempdir().unwrap();
        let store = store(tmp.path());
        let mut p = point(
            Utc.with_ymd_and_hms(2026, 10, 21, 13, 5, 0).unwrap(),
            Some(0.666),
            Some(0.81),
        );
        p.five_hour_resets_at = Some(Utc.with_ymd_and_hms(2026, 10, 21, 15, 0, 0).unwrap());
        p.seven_day_resets_at = Some(Utc.with_ymd_and_hms(2026, 10, 26, 12, 0, 0).unwrap());
        record(&store, "conta1@exemplo.com", &[p]);
        let account = &view(&store, &HashMap::new(), true, true).groups[0].accounts[0];

        let five = account.five_hour.latest.clone().unwrap();
        assert_eq!(five.text, "67%");
        assert_eq!(five.label, "qua (21) 10:05");
        assert_eq!(five.resets_label.as_deref(), Some("12:00"));
        let seven = account.seven_day.latest.clone().unwrap();
        assert_eq!(seven.text, "81%");
        assert_eq!(seven.resets_label.as_deref(), Some("seg (26) 9:00"));
    }

    #[test]
    fn the_next_measurement_is_five_hours_after_the_newest_of_point_and_attempt() {
        let tmp = tempfile::tempdir().unwrap();
        let store = store(tmp.path());
        record(
            &store,
            "conta1@exemplo.com",
            &[point(now() - Duration::hours(2), Some(0.1), Some(0.2))],
        );
        let a1 = Id::parse(A1).unwrap();
        let first = |attempts: &HashMap<Id, DateTime<Utc>>, on: bool| {
            view(&store, attempts, on, true).groups[0].accounts[0].clone()
        };

        let measured = first(&HashMap::new(), true);
        assert_eq!(
            measured.last_measured_at,
            Some(iso(now() - Duration::hours(2)))
        );
        assert_eq!(
            measured.next_measure_at,
            Some(iso(now() + Duration::hours(3)))
        );
        assert_eq!(measured.next_measure_label.as_deref(), Some("15:00"));

        let tried = first(&HashMap::from([(a1, now() - Duration::hours(1))]), true);
        assert_eq!(tried.next_measure_at, Some(iso(now() + Duration::hours(4))));

        let off = first(&HashMap::new(), false);
        assert_eq!(off.next_measure_at, None);
        assert_eq!(off.next_measure_label, None);

        // Nunca medida nem tentada: já — a próxima volta da agenda a mede.
        let never = view(&store, &HashMap::new(), true, true).groups[0].accounts[1].clone();
        assert_eq!(never.last_measured_at, None);
        assert_eq!(never.next_measure_at, Some(iso(now())));
    }

    /// O contrato com o front (`types.ts`): camelCase.
    #[test]
    fn the_view_speaks_camel_case_to_the_front() {
        let tmp = tempfile::tempdir().unwrap();
        let store = store(tmp.path());
        record(
            &store,
            "conta1@exemplo.com",
            &[point(now() - Duration::hours(1), Some(0.1), Some(0.2))],
        );
        let json = serde_json::to_value(view(&store, &HashMap::new(), true, true)).unwrap();

        assert_eq!(json["scheduleOn"], true);
        assert_eq!(json["groups"][0]["thresholdPercent"], 90.0);
        let account = &json["groups"][0]["accounts"][0];
        assert!(account["nextMeasureLabel"].is_string());
        assert!(account["lastMeasuredAt"].is_string());
        assert_eq!(account["sevenDay"]["latest"]["text"], "20%");
        assert!(account["fiveHour"]["segments"][0][0]["resetsLabel"].is_null());
    }
}
