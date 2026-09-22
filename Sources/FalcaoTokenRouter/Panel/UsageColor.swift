import SwiftUI

/// Semáforo de consumo, definido uma vez só.
///
/// A barra do painel e a cor do menu bar leem os mesmos limiares daqui: se cada
/// um tivesse os seus, um poderia dizer "tranquilo" enquanto o outro já alertava.
enum UsageColor {
    static let calmThreshold = 0.66
    static let warningThreshold = 0.90

    /// Verde dessaturado de propósito: o estado normal é a maior parte do tempo,
    /// e um verde vivo nesse papel vira ruído — além de gastar o contraste que o
    /// vermelho vai precisar.
    static let calm = Color(red: 0.36, green: 0.62, blue: 0.45)
    static let warning = Color(red: 0.88, green: 0.72, blue: 0.25)
    static let critical = Color(red: 0.85, green: 0.32, blue: 0.28)

    static func bar(_ fraction: Double) -> Color {
        switch fraction {
        case ..<calmThreshold: calm
        case ..<warningThreshold: warning
        default: critical
        }
    }

    /// O menu bar usa cinza no estado normal em vez do verde: é um ícone que
    /// fica visível o tempo todo, e cor permanente ali compete com o resto da
    /// barra. A cor só entra quando há algo a dizer.
    static func menuBar(_ fraction: Double?) -> Color {
        guard let fraction else { return .secondary }
        switch fraction {
        case ..<calmThreshold: return .secondary
        case ..<warningThreshold: return warning
        default: return critical
        }
    }
}
