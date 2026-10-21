//! O histórico de uso de cada conta: um ponto por medição da SONDA, para a aba
//! Histórico do app desenhar a janela de 5h e a semanal ao longo do tempo (só
//! do Windows, 21/10/2026).
//!
//! Mora fora do `usage\` de propósito. Aquele arquivo é a ÚLTIMA amostra — o
//! número que decide a rotação, regravado pelo sensor a cada render da status
//! line — e é o contrato com o macOS. Aqui fica a sequência:
//! `<base>\history\<email>.jsonl`, uma linha por medição, a mais nova no fim.
//!
//! Só a sonda grava (a medição de 5 em 5 horas, o "Medir contas" e o `router
//! measure`) — decidido com o usuário: o sensor não entra.

use std::io;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};

use super::group_usage::GroupUsageStore;
use crate::platform::atomic_write::{read_retrying, write_atomic};
use crate::usage::claude_usage_probe::{ModelWindow, Reading};

/// Uma medição da sonda, como fica no histórico. O vocabulário é o da amostra
/// (`fiveHourPercent`… em frações 0–1, datas sem fração de segundo); a janela
/// que a leitura não trouxe fica AUSENTE — nunca zero.
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageHistoryPoint {
    #[serde(with = "crate::time_fmt::required")]
    pub sampled_at: DateTime<Utc>,
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
    /// As janelas por modelo da mesma leitura. Nenhum gráfico as usa ainda; vão
    /// junto porque a sonda já as tem, e um histórico que não foi gravado não
    /// volta.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub models: Vec<ModelWindow>,
}

impl UsageHistoryPoint {
    /// O ponto de uma leitura da sonda, com a hora da medição.
    pub fn from_reading(reading: &Reading, sampled_at: DateTime<Utc>) -> Self {
        UsageHistoryPoint {
            sampled_at,
            five_hour_percent: reading.session,
            five_hour_resets_at: reading.session_resets_at,
            seven_day_percent: reading.weekly_all,
            seven_day_resets_at: reading.weekly_all_resets_at,
            models: reading.models.clone(),
        }
    }
}

/// Onde a sonda grava o histórico e o app o lê.
pub struct UsageHistory;

impl UsageHistory {
    /// Quanto o histórico guarda. O gráfico mais longo mostra 4 semanas; o
    /// resto é folga — uns 5 pontos por dia, ~450 linhas por conta.
    pub const RETENTION_DAYS: i64 = 90;

    /// A pasta: `<base>\history`.
    pub fn directory(base: &Path) -> PathBuf {
        base.join("history")
    }

    /// O nome do arquivo do `usage\` (a mesma limpeza do e-mail), com `.jsonl`.
    pub fn file_name(email: &str) -> String {
        format!("{}.jsonl", GroupUsageStore::safe_stem(email))
    }

    /// Os pontos de uma conta, do mais velho ao mais novo. Arquivo ausente =
    /// nenhum ponto; linha que não se lê é pulada, e as outras ficam.
    pub fn read(email: &str, dir: &Path) -> Vec<UsageHistoryPoint> {
        let Ok(bytes) = read_retrying(&dir.join(Self::file_name(email))) else {
            return Vec::new();
        };
        let mut points: Vec<UsageHistoryPoint> = String::from_utf8_lossy(&bytes)
            .lines()
            .filter_map(|line| serde_json::from_str(line.trim()).ok())
            .collect();
        points.sort_by_key(|p| p.sampled_at);
        points
    }

    /// Quando a conta foi medida pela última vez — venha de onde vier a
    /// medição (a agenda, o botão, a CLI).
    pub fn last_sampled_at(email: &str, dir: &Path) -> Option<DateTime<Utc>> {
        Self::read(email, dir).last().map(|p| p.sampled_at)
    }

    /// Acrescenta um ponto, regravando o arquivo inteiro de forma atômica
    /// (temporário + rename): uma queda no meio nunca deixa linha pela metade,
    /// e a poda — o que passou de `RETENTION_DAYS` antes de `now` — acontece no
    /// mesmo passo. Quem chama segura a trava do motor (a sonda grava a amostra
    /// e o ponto sob ela).
    pub fn append(
        point: &UsageHistoryPoint,
        email: &str,
        dir: &Path,
        now: DateTime<Utc>,
    ) -> io::Result<()> {
        let cutoff = now - Duration::days(Self::RETENTION_DAYS);
        let mut points: Vec<UsageHistoryPoint> = Self::read(email, dir)
            .into_iter()
            .filter(|p| p.sampled_at >= cutoff)
            .collect();
        points.push(point.clone());
        points.sort_by_key(|p| p.sampled_at);
        let mut text = String::new();
        for p in &points {
            text.push_str(&serde_json::to_string(p).map_err(io::Error::other)?);
            text.push('\n');
        }
        write_atomic(&dir.join(Self::file_name(email)), text.as_bytes())
    }
}
