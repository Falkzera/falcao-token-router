import Foundation

/// A conversão de fração (0–1) em porcentagem inteira, **num lugar só**.
///
/// Existe porque três superfícies do mesmo produto formatavam o mesmo número de
/// dois jeitos: o sensor da status line e a tela de Grupos arredondavam, e o
/// painel truncava (`Int(fraction * 100)`). Com 0,666 a status line escrevia
/// 67% e o painel 66% — um ponto de diferença que não é erro de medição, é erro
/// de formatação, e que leva o usuário a desconfiar justamente do número que
/// decide a rotação.
public enum UsagePercent {
    /// 0,666 → 67. Arredonda (meio para cima), que era o comportamento do
    /// sensor — a superfície mais próxima do dado bruto.
    public static func value(_ fraction: Double) -> Int {
        Int((fraction * 100).rounded())
    }

    /// 0,666 → "67%".
    public static func text(_ fraction: Double) -> String {
        "\(value(fraction))%"
    }
}
