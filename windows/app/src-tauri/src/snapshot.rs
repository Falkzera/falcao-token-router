//! O quadro que as janelas leem: grupos, contas, uso com procedência, sessões,
//! o erro da última ação. Montado do store, serializado para o front.
//!
//! Os percentuais saem PRONTOS daqui, pelo `UsagePercent` do núcleo — o mesmo
//! do sensor e da CLI. No macOS havia três conversões e 0,666 virava 67% numa
//! tela e 66% na outra; aqui o front só mostra o texto que recebe.

use std::collections::HashSet;

use chrono::{DateTime, SecondsFormat, Utc};
use router_core::engine::group_usage::UsageOrigin;
use router_core::engine::rotation_engine::RotationError;
use router_core::engine::router_config_store::{RouterConfigStore, StoreError};
use router_core::{AccountGroup, AccountUsage, Id, UsagePercent, UsageWindow};
use serde::Serialize;

fn iso(date: DateTime<Utc>) -> String {
    date.to_rfc3339_opts(SecondsFormat::Secs, true)
}

/// Uma janela do `rate_limits`: a fração, o texto pronto e quando reseta.
#[derive(Clone, PartialEq, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Reading {
    pub fraction: f64,
    pub text: String,
    pub resets_at: Option<String>,
}

impl Reading {
    fn new(fraction: f64, resets_at: Option<DateTime<Utc>>) -> Self {
        Reading {
            fraction,
            text: UsagePercent::text(fraction),
            resets_at: resets_at.map(iso),
        }
    }
}

/// A janela POR MODELO, que só a sonda vê — com carimbo próprio.
#[derive(Clone, PartialEq, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelReading {
    pub name: String,
    pub reading: Reading,
    pub sampled_at: Option<String>,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Bound {
    FiveHour,
    SevenDay,
    Model,
}

/// O uso de uma conta com procedência: qual janela manda, quem mediu, quando.
#[derive(Clone, PartialEq, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageView {
    /// O número que a rotação compara com o limiar (o maior das janelas).
    pub fraction: f64,
    pub text: String,
    pub bound: Bound,
    pub five_hour: Option<Reading>,
    pub seven_day: Option<Reading>,
    pub model: Option<ModelReading>,
    pub origin: UsageOrigin,
    pub sampled_at: String,
}

impl From<&AccountUsage> for UsageView {
    fn from(usage: &AccountUsage) -> Self {
        UsageView {
            fraction: usage.fraction,
            text: UsagePercent::text(usage.fraction),
            bound: match usage.window {
                UsageWindow::FiveHour => Bound::FiveHour,
                UsageWindow::SevenDay => Bound::SevenDay,
                UsageWindow::Model(_) => Bound::Model,
            },
            five_hour: usage
                .five_hour
                .map(|f| Reading::new(f, usage.five_hour_resets_at)),
            seven_day: usage
                .seven_day
                .map(|f| Reading::new(f, usage.seven_day_resets_at)),
            model: usage.model.as_ref().map(|m| ModelReading {
                name: m.name.clone(),
                reading: Reading::new(m.percent, m.resets_at),
                sampled_at: usage.model_sampled_at.map(iso),
            }),
            origin: usage.origin,
            sampled_at: iso(usage.sampled_at),
        }
    }
}

#[derive(Clone, PartialEq, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountView {
    pub id: String,
    pub label: String,
    pub email: String,
    pub organization: Option<String>,
    pub usage: Option<UsageView>,
}

/// Quantas sessões vivas o grupo tem, e quantas trabalham ou esperam o usuário
/// (a diferença entre "há sessões abertas" e "há trabalho em curso").
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionsView {
    pub count: usize,
    pub engaged: usize,
}

#[derive(Clone, PartialEq, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GroupView {
    pub id: String,
    pub name: String,
    pub is_default: bool,
    pub auto_rotate: bool,
    pub threshold_percent: f64,
    /// O comando exato para abrir o grupo no terminal.
    pub command: String,
    pub active_account_id: Option<String>,
    /// Quantas contas perdem o login se o grupo for apagado (as que só estão
    /// nele) — o número da confirmação.
    pub exclusive_count: usize,
    pub sessions: SessionsView,
    /// Na ordem de preferência da rotação.
    pub accounts: Vec<AccountView>,
}

/// A falha da última ação, como FATO: o texto é do catálogo do front.
#[derive(Clone, PartialEq, Debug, Serialize)]
#[serde(rename_all = "camelCase", tag = "code")]
pub enum ErrorView {
    SaveFailed { detail: String },
    ActivateNoCredential,
    ActivateBusyElsewhere { group: String },
    ActivateWriteFailed { detail: String },
    RouterPathUnknown,
    IntegrationFailed { detail: String },
    ProbeUnavailable,
    ProbeFailures { count: usize },
}

#[derive(Clone, PartialEq, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub groups: Vec<GroupView>,
    /// O grupo que está sendo medido pela sonda agora (spinner no botão).
    pub measuring_group: Option<String>,
    pub last_error: Option<ErrorView>,
}

/// `claude trabalho`, ou `claude "Meu Grupo"` com espaço (a função casa sem
/// diferenciar caixa).
pub fn command_for(name: &str) -> String {
    let name = name.trim();
    if name.contains(' ') {
        format!("claude \"{name}\"")
    } else {
        format!("claude {}", name.to_lowercase())
    }
}

fn error_view(error: &StoreError, store: &RouterConfigStore) -> ErrorView {
    match error {
        StoreError::SaveFailed(detail) => ErrorView::SaveFailed {
            detail: detail.clone(),
        },
        StoreError::ActivateFailed(RotationError::NoCredential { .. }) => {
            ErrorView::ActivateNoCredential
        }
        StoreError::ActivateFailed(RotationError::AccountBusyElsewhere { group_id, .. }) => {
            ErrorView::ActivateBusyElsewhere {
                group: store
                    .config()
                    .groups
                    .iter()
                    .find(|g| g.id == *group_id)
                    .map(|g| g.name.clone())
                    .unwrap_or_default(),
            }
        }
        StoreError::ActivateFailed(RotationError::WriteFailed(detail)) => {
            ErrorView::ActivateWriteFailed {
                detail: detail.clone(),
            }
        }
        StoreError::RouterPathUnknown => ErrorView::RouterPathUnknown,
        StoreError::IntegrationFailed(detail) => ErrorView::IntegrationFailed {
            detail: detail.clone(),
        },
        StoreError::ProbeUnavailable => ErrorView::ProbeUnavailable,
        StoreError::ProbeFailures(count) => ErrorView::ProbeFailures { count: *count },
    }
}

fn group_view(store: &RouterConfigStore, group: &AccountGroup) -> GroupView {
    let config = store.config();
    let exclusive: HashSet<Id> = store.exclusive_account_ids(group.id).into_iter().collect();
    let sessions = store
        .live_sessions()
        .get(&group.id)
        .map(|live| SessionsView {
            count: live.len(),
            engaged: live.iter().filter(|s| s.status.is_engaged()).count(),
        })
        .unwrap_or_default();
    GroupView {
        id: group.id.to_string(),
        name: group.name.clone(),
        is_default: group.config_dir.is_default,
        auto_rotate: group.auto_rotate,
        threshold_percent: group.threshold_percent,
        command: command_for(&group.name),
        active_account_id: store.active_by_group().get(&group.id).map(Id::to_string),
        exclusive_count: exclusive.len(),
        sessions,
        accounts: config
            .accounts_in(group)
            .into_iter()
            .map(|account| AccountView {
                id: account.id.to_string(),
                label: account.label().to_string(),
                email: account.identity.email.clone(),
                organization: account
                    .identity
                    .organization_name
                    .clone()
                    .filter(|o| !o.is_empty()),
                usage: store.usage_detail().get(&account.id).map(UsageView::from),
            })
            .collect(),
    }
}

pub fn build(store: &RouterConfigStore, measuring_group: Option<Id>) -> Snapshot {
    Snapshot {
        groups: store
            .config()
            .groups
            .iter()
            .map(|g| group_view(store, g))
            .collect(),
        measuring_group: measuring_group.map(|id| id.to_string()),
        last_error: store.last_error().map(|e| error_view(e, store)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;
    use std::sync::Arc;

    use router_core::engine::anthropic_adapter::AnthropicAdapter;
    use router_core::engine::credential_store::FileCredentialStore;
    use router_core::engine::provider::ProviderAdapter;
    use router_core::engine::router_paths::RouterPaths;
    use serde_json::json;

    fn write(path: &Path, value: serde_json::Value) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, serde_json::to_vec_pretty(&value).unwrap()).unwrap();
    }

    /// Um store de verdade (adapter e credencial reais) sobre arquivos
    /// montados num temporário: dois grupos dedicados, a conta 2 ativa no
    /// primeiro, amostras sintéticas.
    fn sandbox(tmp: &Path) -> RouterConfigStore {
        let paths = RouterPaths::with_app_support(Some(tmp.join("app")));
        let base = paths.base.clone();
        let g1 = base.join("groups").join("G1");
        let g2 = base.join("groups").join("G2");
        let account = |id: &str, email: &str, org: Option<&str>| {
            json!({"id": id, "provider": "anthropic",
                   "identity": {"email": email, "organizationName": org, "raw": {"emailAddress": email}},
                   "home": {"raw": base.join("accounts").join(id).to_string_lossy(), "isDefault": false}})
        };
        write(
            &paths.config_file(),
            json!({
                "version": 1, "shareHistory": true,
                "accounts": [
                    account("AAAAAAAA-0000-4000-8000-000000000001", "equipe-1@exemplo.com", Some("Acme")),
                    account("AAAAAAAA-0000-4000-8000-000000000002", "equipe-2@exemplo.com", Some("Acme")),
                    account("AAAAAAAA-0000-4000-8000-000000000003", "conta1@exemplo.com", None),
                ],
                "groups": [
                    {"id": "11111111-1111-4111-8111-111111111111", "name": "Trabalho", "provider": "anthropic",
                     "accountIDs": ["AAAAAAAA-0000-4000-8000-000000000001", "AAAAAAAA-0000-4000-8000-000000000002"],
                     "configDir": {"raw": g1.to_string_lossy(), "isDefault": false},
                     "thresholdPercent": 90, "autoRotate": true},
                    {"id": "22222222-2222-4222-8222-222222222222", "name": "Meu Pessoal", "provider": "anthropic",
                     "accountIDs": ["AAAAAAAA-0000-4000-8000-000000000003", "AAAAAAAA-0000-4000-8000-000000000001"],
                     "configDir": {"raw": g2.to_string_lossy(), "isDefault": false},
                     "thresholdPercent": 85, "autoRotate": false}
                ]
            }),
        );
        write(
            &g1.join(".claude.json"),
            json!({"oauthAccount": {"emailAddress": "equipe-2@exemplo.com"}}),
        );
        let later = iso(Utc::now() + chrono::Duration::hours(3));
        write(
            &paths.usage_dir().join("equipe-2@exemplo.com.json"),
            json!({"configDirRaw": g1.to_string_lossy(), "email": "equipe-2@exemplo.com",
                   "fiveHourPercent": 0.666, "fiveHourResetsAt": later,
                   "sevenDayPercent": 0.81, "sevenDayResetsAt": later,
                   "sampledAt": iso(Utc::now()), "origin": "sensor"}),
        );
        let adapters: Vec<Arc<dyn ProviderAdapter>> = vec![Arc::new(AnthropicAdapter)];
        RouterConfigStore::new(
            paths,
            tmp.join("home").to_string_lossy().into_owned(),
            Arc::new(FileCredentialStore::new()),
            adapters,
        )
        .with_session_reader(|_| Vec::new())
    }

    #[test]
    fn groups_and_accounts_keep_the_user_order_with_the_active_one_marked() {
        let tmp = tempfile::tempdir().unwrap();
        let snapshot = build(&sandbox(tmp.path()), None);

        let names: Vec<&str> = snapshot.groups.iter().map(|g| g.name.as_str()).collect();
        assert_eq!(names, ["Trabalho", "Meu Pessoal"]);
        let work = &snapshot.groups[0];
        let labels: Vec<&str> = work.accounts.iter().map(|a| a.label.as_str()).collect();
        assert_eq!(labels, ["equipe-1", "equipe-2"], "ordem de preferência");
        assert_eq!(
            work.active_account_id.as_deref(),
            Some("AAAAAAAA-0000-4000-8000-000000000002")
        );
        assert_eq!(work.accounts[0].organization.as_deref(), Some("Acme"));
        assert_eq!(snapshot.groups[1].active_account_id, None);
    }

    /// O texto do % é o do núcleo (meio para longe do zero): 0,666 é 67% aqui,
    /// no sensor e na CLI. A janela que manda (7d, 81%) vem marcada.
    #[test]
    fn percentages_come_ready_from_the_core() {
        let tmp = tempfile::tempdir().unwrap();
        let snapshot = build(&sandbox(tmp.path()), None);
        let usage = snapshot.groups[0].accounts[1].usage.as_ref().unwrap();

        assert_eq!(usage.bound, Bound::SevenDay);
        assert_eq!(usage.text, "81%");
        assert_eq!(usage.five_hour.as_ref().unwrap().text, "67%");
        assert!(usage.five_hour.as_ref().unwrap().resets_at.is_some());
        assert_eq!(usage.origin, UsageOrigin::Sensor);
        assert!(
            snapshot.groups[0].accounts[0].usage.is_none(),
            "sem amostra = pronta"
        );
    }

    /// A conta que está em DOIS grupos não conta para a confirmação de apagar
    /// nenhum deles — ela fica.
    #[test]
    fn the_delete_count_is_of_exclusive_accounts() {
        let tmp = tempfile::tempdir().unwrap();
        let snapshot = build(&sandbox(tmp.path()), None);
        assert_eq!(snapshot.groups[0].exclusive_count, 1, "equipe-2");
        assert_eq!(snapshot.groups[1].exclusive_count, 1, "conta1");
    }

    #[test]
    fn the_terminal_command_quotes_names_with_spaces() {
        assert_eq!(command_for("Trabalho"), "claude trabalho");
        assert_eq!(command_for(" Meu Pessoal "), "claude \"Meu Pessoal\"");
    }

    /// O erro chega como fato com código — o texto é do catálogo do front.
    #[test]
    fn errors_are_facts_with_a_code() {
        let tmp = tempfile::tempdir().unwrap();
        let mut store = sandbox(tmp.path());
        store.measure_unavailable();
        let json = serde_json::to_value(build(&store, None)).unwrap();
        assert_eq!(json["lastError"], json!({"code": "probeUnavailable"}));

        store.finish_measure(router_core::engine::router_config_store::MeasureSummary {
            measured: 1,
            failed: 2,
        });
        let json = serde_json::to_value(build(&store, None)).unwrap();
        assert_eq!(
            json["lastError"],
            json!({"code": "probeFailures", "count": 2})
        );
    }
}
