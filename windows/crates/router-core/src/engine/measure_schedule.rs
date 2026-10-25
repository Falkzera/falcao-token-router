//! A medição automática de 5 em 5 horas (só do Windows, 21/10/2026): QUANDO
//! cada conta deve ser medida de novo. Puro — o relógio entra por parâmetro.
//!
//! A sonda era "botão e comando, nunca laço": cada medição sobe um `claude` e
//! faz uma requisição por conta. O histórico pediu uma medição a cada 5 horas
//! — umas 5 por conta por dia —, com o app aberto e uma chave nos Ajustes que a
//! desliga.

use chrono::{DateTime, Duration, Utc};

pub struct MeasureSchedule;

impl MeasureSchedule {
    /// O intervalo entre duas medições da mesma conta.
    pub const EVERY_HOURS: i64 = 5;

    pub fn every() -> Duration {
        Duration::hours(Self::EVERY_HOURS)
    }

    /// Quando a conta deve ser medida de novo: 5 horas depois da mais nova
    /// entre a última medição gravada (venha de onde vier) e a última
    /// tentativa da agenda nesta execução do app — sucesso ou falha. A
    /// tentativa é o que segura uma conta deslogada, que sem ela seria sondada
    /// a cada volta do laço. `None`: nunca medida nem tentada.
    pub fn next_at(
        last_measured: Option<DateTime<Utc>>,
        last_attempt: Option<DateTime<Utc>>,
    ) -> Option<DateTime<Utc>> {
        last_measured.max(last_attempt).map(|at| at + Self::every())
    }

    /// A conta vence quando nunca foi medida nem tentada, ou quando a
    /// referência tem 5 horas ou mais.
    pub fn is_due(
        last_measured: Option<DateTime<Utc>>,
        last_attempt: Option<DateTime<Utc>>,
        now: DateTime<Utc>,
    ) -> bool {
        Self::next_at(last_measured, last_attempt).is_none_or(|next| next <= now)
    }
}
