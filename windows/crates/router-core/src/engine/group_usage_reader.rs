//! Traduz as amostras que o sensor grava em "quanto cada conta usou".
//!
//! Limite considerado: o **maior** entre a janela de 5h, a de 7 dias e a mais
//! apertada POR MODELO — qualquer um dos três, ao bater o limiar, dispara a troca.
//! Janela com reset já passado é descartada (a amostra envelheceu além do próprio
//! limite). Conta sem nenhuma janela válida não aparece: o motor a presume fresca.

use std::cmp::Ordering;
use std::collections::HashMap;
use std::path::PathBuf;

use chrono::{DateTime, Utc};

use super::group_model::RouterConfig;
use super::group_usage::{GroupUsageSample, GroupUsageStore, UsageOrigin};
use crate::ids::Id;
use crate::usage::claude_usage_probe::ModelWindow;

/// Qual janela está mandando no número.
#[derive(Clone, PartialEq, Debug)]
pub enum UsageWindow {
    FiveHour,
    SevenDay,
    /// Carrega o nome cru que o `/usage` imprime (`Fable`, `Opus`).
    Model(String),
}

/// O uso que decide, junto com a **procedência** do número, para a UI dizer de
/// qual janela ele veio sem o usuário adivinhar.
#[derive(Clone, PartialEq, Debug)]
pub struct AccountUsage {
    /// O maior entre as janelas válidas — o número que a rotação compara com o
    /// limiar do grupo.
    pub fraction: f64,
    pub window: UsageWindow,
    pub five_hour: Option<f64>,
    pub five_hour_resets_at: Option<DateTime<Utc>>,
    pub seven_day: Option<f64>,
    pub seven_day_resets_at: Option<DateTime<Utc>>,
    pub model: Option<ModelWindow>,
    /// Quando a SONDA rodou — idade própria, porque ela roda sob demanda.
    pub model_sampled_at: Option<DateTime<Utc>>,
    pub origin: UsageOrigin,
    pub sampled_at: DateTime<Utc>,
}

pub struct GroupUsageReader {
    usage_dir: PathBuf,
}

impl GroupUsageReader {
    pub fn new(usage_dir: impl Into<PathBuf>) -> Self {
        GroupUsageReader {
            usage_dir: usage_dir.into(),
        }
    }

    /// A amostra mais recente de cada conta, por e-mail.
    pub fn samples_by_email(&self) -> HashMap<String, GroupUsageSample> {
        let mut by_email: HashMap<String, GroupUsageSample> = HashMap::new();
        for sample in GroupUsageStore::read_all(&self.usage_dir) {
            let Some(email) = sample.email.clone() else {
                continue;
            };
            if let Some(existing) = by_email.get(&email) {
                if existing.sampled_at >= sample.sampled_at {
                    continue;
                }
            }
            by_email.insert(email, sample);
        }
        by_email
    }

    /// A amostra mais recente de cada conta do config, pelo id da conta.
    pub fn samples_by_account(&self, config: &RouterConfig) -> HashMap<Id, GroupUsageSample> {
        let by_email = self.samples_by_email();
        let mut result = HashMap::new();
        for account in &config.accounts {
            if let Some(sample) = by_email.get(&account.identity.email) {
                result.insert(account.id, sample.clone());
            }
        }
        result
    }

    /// O uso de cada conta **com a procedência do número**. Contas sem nenhuma
    /// janela válida não aparecem.
    pub fn detail_by_account(
        &self,
        config: &RouterConfig,
        now: DateTime<Utc>,
    ) -> HashMap<Id, AccountUsage> {
        let mut result = HashMap::new();
        for (id, sample) in self.samples_by_account(config) {
            let five_valid = sample.five_hour_resets_at.is_none_or(|r| r > now);
            let seven_valid = sample.seven_day_resets_at.is_none_or(|r| r > now);
            let five = if five_valid {
                sample.five_hour_percent
            } else {
                None
            };
            let seven = if seven_valid {
                sample.seven_day_percent
            } else {
                None
            };

            // A janela por modelo mais apertada que ainda vale.
            let model = sample.models.as_ref().and_then(|m| m.binding(now));

            // Candidatas na ordem [5h, 7d, modelo]. No empate vale a de horizonte
            // mais longo: `max_by` do Rust devolve o ÚLTIMO dos empatados, então
            // a mais longa (que vem depois) ganha — igual ao `max(by: <=)` do Swift.
            let mut candidates: Vec<(f64, UsageWindow)> = Vec::new();
            if let Some(f) = five {
                candidates.push((f, UsageWindow::FiveHour));
            }
            if let Some(s) = seven {
                candidates.push((s, UsageWindow::SevenDay));
            }
            if let Some(m) = model {
                candidates.push((m.percent, UsageWindow::Model(m.name.clone())));
            }
            let bound = candidates
                .into_iter()
                .max_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(Ordering::Equal));
            let Some((fraction, window)) = bound else {
                continue;
            };

            result.insert(
                id,
                AccountUsage {
                    fraction,
                    window,
                    five_hour: five,
                    five_hour_resets_at: if five.is_none() {
                        None
                    } else {
                        sample.five_hour_resets_at
                    },
                    seven_day: seven,
                    seven_day_resets_at: if seven.is_none() {
                        None
                    } else {
                        sample.seven_day_resets_at
                    },
                    model: model.cloned(),
                    model_sampled_at: if model.is_none() {
                        None
                    } else {
                        sample.models.as_ref().map(|m| m.sampled_at)
                    },
                    origin: sample.origin(),
                    sampled_at: sample.sampled_at,
                },
            );
        }
        result
    }

    /// Uso 0–1 por conta (id), para a decisão de rotação — a mesma fração de
    /// `detail_by_account`, sem a procedência que só a UI precisa.
    pub fn usage_by_account(&self, config: &RouterConfig, now: DateTime<Utc>) -> HashMap<Id, f64> {
        self.detail_by_account(config, now)
            .into_iter()
            .map(|(id, u)| (id, u.fraction))
            .collect()
    }
}
