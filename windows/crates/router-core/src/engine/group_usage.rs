//! O uso de uma conta, medido pelo `rate_limits` que o Claude Code entrega no
//! stdin da status line. Um arquivo por conta (pelo e-mail), sobrescrito a cada
//! amostra — o último valor conhecido, que é o que decide a rotação.

use std::cmp::Ordering;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::platform::atomic_write::write_atomic;
use crate::usage::claude_usage_probe::ModelWindow;

/// Quem mediu o número de uma conta. A origem viaja junto com a medida em vez de
/// ficar implícita: `sensor` (a requisição que a conta atendeu) e `probe` (uma
/// consulta `/usage` feita de propósito, que vê conta ociosa e o limite por modelo).
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum UsageOrigin {
    Sensor,
    Probe,
}

/// As janelas por modelo de uma conta, e quando foram medidas. Carimbo próprio
/// porque a origem é a sonda (sob demanda), não o sensor (a cada mensagem).
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelUsage {
    pub windows: Vec<ModelWindow>,
    #[serde(with = "crate::time_fmt::required")]
    pub sampled_at: DateTime<Utc>,
}

impl ModelUsage {
    pub fn new(windows: Vec<ModelWindow>, sampled_at: DateTime<Utc>) -> Self {
        ModelUsage {
            windows,
            sampled_at,
        }
    }

    /// A janela mais apertada ainda válida. Reset já passado é descartado — a
    /// medida envelheceu além do próprio limite que media.
    pub fn binding(&self, now: DateTime<Utc>) -> Option<&ModelWindow> {
        self.windows
            .iter()
            .filter(|w| w.resets_at.is_none_or(|r| r > now))
            .max_by(|a, b| a.percent.partial_cmp(&b.percent).unwrap_or(Ordering::Equal))
    }
}

/// A amostra de uso de uma conta, como o sensor a grava.
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GroupUsageSample {
    /// O perfil de onde a amostra veio, pela string de `CLAUDE_CONFIG_DIR` — ou o
    /// padrão quando a variável não estava setada.
    pub config_dir_raw: String,
    /// E-mail da conta que servia a sessão no instante da amostra, lido do
    /// `.claude.json` do perfil. É o que casa a amostra com uma conta.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    /// Frações 0–1 (não porcentagens).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub five_hour_percent: Option<f64>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "crate::time_fmt::optional"
    )]
    pub five_hour_resets_at: Option<DateTime<Utc>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seven_day_percent: Option<f64>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "crate::time_fmt::optional"
    )]
    pub seven_day_resets_at: Option<DateTime<Utc>>,
    /// Quando a status line rodou.
    #[serde(with = "crate::time_fmt::required")]
    pub sampled_at: DateTime<Utc>,
    /// As janelas POR MODELO, quando alguém as mediu. `None` na amostra do sensor
    /// (o `rate_limits` não as traz); quem as preenche é a sonda.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub models: Option<ModelUsage>,
    /// Serializada como opcional porque amostra gravada antes de a sonda existir
    /// só podia vir do sensor. Ausência resolve para `sensor`.
    #[serde(rename = "origin", default, skip_serializing_if = "Option::is_none")]
    stored_origin: Option<UsageOrigin>,
}

impl GroupUsageSample {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        config_dir_raw: impl Into<String>,
        email: Option<String>,
        five_hour_percent: Option<f64>,
        five_hour_resets_at: Option<DateTime<Utc>>,
        seven_day_percent: Option<f64>,
        seven_day_resets_at: Option<DateTime<Utc>>,
        sampled_at: DateTime<Utc>,
        models: Option<ModelUsage>,
        origin: UsageOrigin,
    ) -> Self {
        GroupUsageSample {
            config_dir_raw: config_dir_raw.into(),
            email,
            five_hour_percent,
            five_hour_resets_at,
            seven_day_percent,
            seven_day_resets_at,
            sampled_at,
            models,
            stored_origin: Some(origin),
        }
    }

    /// Quem mediu as janelas de 5h e 7 dias. (As por modelo são sempre da sonda.)
    pub fn origin(&self) -> UsageOrigin {
        self.stored_origin.unwrap_or(UsageOrigin::Sensor)
    }

    /// A mesma amostra, com as janelas por modelo trocadas.
    pub fn with_models(mut self, models: Option<ModelUsage>) -> Self {
        self.models = models;
        self
    }
}

/// Onde o sensor grava as amostras e o app as lê.
pub struct GroupUsageStore;

impl GroupUsageStore {
    /// A pasta de amostras: `<base>\usage`.
    pub fn directory(base: &Path) -> PathBuf {
        base.join("usage")
    }

    /// Nome de arquivo estável e seguro para um e-mail (mantém letra, dígito,
    /// `.`, `@`, `-`; o resto vira `_`).
    pub fn file_name(email: &str) -> String {
        let safe: String = email
            .chars()
            .map(|c| {
                if c.is_alphanumeric() || c == '.' || c == '@' || c == '-' {
                    c
                } else {
                    '_'
                }
            })
            .collect();
        format!("{safe}.json")
    }

    /// Grava a amostra da conta, de forma atômica. **Preserva o bloco por modelo**
    /// quando a amostra nova não o traz (o sensor nunca o conhece; sem esta
    /// costura a 1ª mensagem depois de uma sondagem apagaria o número do Fable).
    /// O carimbo do bloco preservado é o da sondagem, então nada finge frescor.
    pub fn write(sample: &GroupUsageSample, email: &str, dir: &Path) -> std::io::Result<()> {
        let path = dir.join(Self::file_name(email));
        let final_sample = if sample.models.is_none() {
            let prev = Self::read(email, dir).and_then(|s| s.models);
            sample.clone().with_models(prev)
        } else {
            sample.clone()
        };
        let bytes = serde_json::to_vec(&final_sample)?;
        write_atomic(&path, &bytes)
    }

    /// Lê a amostra de uma conta, se houver.
    pub fn read(email: &str, dir: &Path) -> Option<GroupUsageSample> {
        let path = dir.join(Self::file_name(email));
        let data = std::fs::read(&path).ok()?;
        serde_json::from_slice(&data).ok()
    }

    /// Todas as amostras gravadas. Chamado pelo app para montar o quadro.
    pub fn read_all(dir: &Path) -> Vec<GroupUsageSample> {
        let entries = match std::fs::read_dir(dir) {
            Ok(e) => e,
            Err(_) => return Vec::new(),
        };
        entries
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| p.extension().and_then(|x| x.to_str()) == Some("json"))
            .filter_map(|p| std::fs::read(&p).ok())
            .filter_map(|data| serde_json::from_slice(&data).ok())
            .collect()
    }
}
