//! A sonda ativa: pergunta o uso ao binário oficial (`claude /usage`), não ao
//! endpoint. Esta fatia (o sensor) só precisa da struct `ModelWindow`, que a
//! amostra guarda; o parser do texto e a execução do processo entram na Fase 4.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Uma janela por modelo, como o `/usage` a nomeia.
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelWindow {
    /// O nome de exibição que o `/usage` imprime (`Fable`, `Opus`…). Cru, e não
    /// mapeado para um enum: a lista de modelos muda sem avisar, e um nome que
    /// este app não conhece ainda é um limite que estoura.
    pub name: String,
    /// 0–1.
    pub percent: f64,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "crate::time_fmt::optional"
    )]
    pub resets_at: Option<DateTime<Utc>>,
}

impl ModelWindow {
    pub fn new(name: impl Into<String>, percent: f64, resets_at: Option<DateTime<Utc>>) -> Self {
        ModelWindow {
            name: name.into(),
            percent,
            resets_at,
        }
    }
}
