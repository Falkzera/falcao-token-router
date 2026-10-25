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

    /// A linha dos gráficos do Histórico (21/10/2026). Não é o `calm`: aquele é
    /// dessaturado de propósito, e numa linha fina ele se lê cinza. É a mesma
    /// cor do porte Windows, um tom por aparência, conferida contra as
    /// superfícies clara e escura (contraste de 3:1 ou mais).
    static func chartLine(_ scheme: ColorScheme) -> Color {
        scheme == .dark
            ? Color(red: 0x52 / 255, green: 0xA7 / 255, blue: 0x6D / 255)
            : Color(red: 0x1E / 255, green: 0x75 / 255, blue: 0x46 / 255)
    }

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
