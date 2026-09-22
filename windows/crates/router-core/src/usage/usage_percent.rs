//! A conversão de fração (0–1) em porcentagem inteira, **num lugar só**.
//!
//! Existe porque três superfícies do mesmo produto formatavam o mesmo número de
//! dois jeitos: o sensor e a tela de Grupos arredondavam, o painel truncava. Com
//! 0,666 a status line escrevia 67% e o painel 66% — um ponto de diferença que
//! não é erro de medição, é erro de formatação, e que leva o usuário a
//! desconfiar justamente do número que decide a rotação.

pub struct UsagePercent;

impl UsagePercent {
    /// 0,666 → 67. Arredonda (meio para longe do zero), como o `.rounded()` do
    /// Swift — que é o que o `f64::round` do Rust também faz.
    pub fn value(fraction: f64) -> i64 {
        (fraction * 100.0).round() as i64
    }

    /// 0,666 → "67%".
    pub fn text(fraction: f64) -> String {
        format!("{}%", Self::value(fraction))
    }
}
