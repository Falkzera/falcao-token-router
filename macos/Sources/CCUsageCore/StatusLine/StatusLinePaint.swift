import Foundation

/// As cores da status line dos grupos: os códigos ANSI, a cor por severidade, a
/// barra, e a animação do `effort` (≙ a parte de pintura de `view.rs` no porte
/// Windows).
///
/// Puro e sem relógio próprio: a fase das animações entra por parâmetro, o que
/// é o que torna a prévia dos Ajustes e um teste reprodutíveis.
enum Paint {
    static let reset = "\u{1b}[0m"
    static let bold = "\u{1b}[1m"
    static let red = "\u{1b}[31m"
    static let green = "\u{1b}[32m"
    static let yellow = "\u{1b}[33m"
    static let magenta = "\u{1b}[35m"
    static let cyan = "\u{1b}[36m"
    static let blue = "\u{1b}[94m"

    /// Texto secundário. O "faint" (`ESC[2m`) escurece a cor pela metade e some
    /// sobre fundo escuro em vários terminais: um cinza claro explícito quando
    /// há truecolor, e o bright-black do ANSI onde não há.
    static let grayTrueColor = "\u{1b}[38;2;153;153;153m"
    static let grayANSI = "\u{1b}[90m"

    static func gray(trueColor: Bool) -> String { trueColor ? grayTrueColor : grayANSI }

    /// A cor de cada grupo pela posição na lista do usuário: estável entre
    /// sessões e diferente entre vizinhos. Amarelo fica para a conta fora de
    /// um grupo.
    static let groupColors = [cyan, green, magenta, blue]

    /// Cor por severidade, a mesma régua do resto do produto: ≥0,90 vermelho,
    /// ≥0,70 amarelo, senão verde.
    static func tone(_ fraction: Double) -> String {
        if fraction >= 0.90 { return red }
        if fraction >= 0.70 { return yellow }
        return green
    }

    /// `round(fração × largura)` células cheias, o resto vazias.
    static func bar(_ fraction: Double, width: Int) -> String {
        let filled = min(max(Int((fraction * Double(width)).rounded()), 0), width)
        return String(repeating: "█", count: filled) + String(repeating: "░", count: width - filled)
    }

    static func rgb(_ color: (UInt8, UInt8, UInt8)) -> String {
        "\u{1b}[38;2;\(color.0);\(color.1);\(color.2)m"
    }

    static func mix(_ a: (UInt8, UInt8, UInt8), _ b: (UInt8, UInt8, UInt8), _ k: Double) -> (UInt8, UInt8, UInt8) {
        func channel(_ x: UInt8, _ y: UInt8) -> UInt8 {
            UInt8((Double(x) + (Double(y) - Double(x)) * k).rounded())
        }
        return (channel(a.0, b.0), channel(a.1, b.1), channel(a.2, b.2))
    }

    /// As cores do seletor do `/effort` no tema escuro do Claude Code.
    private static let rainbow: [(UInt8, UInt8, UInt8)] = [
        (235, 95, 87), (245, 139, 87), (250, 195, 95), (145, 200, 130),
        (130, 170, 220), (155, 130, 200), (200, 130, 180),
    ]

    private enum Finish {
        case flat((UInt8, UInt8, UInt8))
        case glow(base: (UInt8, UInt8, UInt8), light: (UInt8, UInt8, UInt8))
        case rainbow
    }

    private static func palette(_ level: String) -> (Finish, String)? {
        switch level {
        case "low":    return (.flat((255, 193, 7)), yellow)
        case "medium": return (.flat((78, 186, 101)), green)
        case "high":   return (.flat((177, 185, 249)), blue)
        case "xhigh":  return (.glow(base: (175, 135, 255), light: (208, 180, 255)), magenta)
        case "max":    return (.rainbow, red)
        default:       return nil
        }
    }

    /// O nível de esforço pintado. Sem truecolor cai para a cor ANSI chapada —
    /// animação de 24 bits num terminal de 16 cores vira lixo, não enfeite.
    /// Nível desconhecido sai em cinza, sem inventar cor.
    static func effort(_ level: String, trueColor: Bool, phase: UInt64, gray: String) -> String {
        guard let (finish, ansi) = palette(level) else { return "\(gray)\(level)\(reset)" }
        guard trueColor else { return "\(ansi)\(bold)\(level)\(reset)" }

        switch finish {
        case .flat(let color):
            return "\(rgb(color))\(bold)\(level)\(reset)"

        case .rainbow:
            let body = level.enumerated().map { i, c in
                rgb(rainbow[(i + Int(phase)) % rainbow.count]) + String(c)
            }.joined()
            return "\(bold)\(body)\(reset)"

        case .glow(let base, let light):
            // A faixa clara anda uma letra por segundo (a fase vem do relógio).
            let count = level.count
            let top = Int(phase % UInt64(count + 4))
            let body = level.enumerated().map { i, c -> String in
                let distance = abs(i - top)
                let k = distance < 3 ? 1.0 - Double(distance) / 3.0 : 0.0
                return rgb(mix(base, light, k)) + String(c)
            }.joined()
            return "\(bold)\(body)\(reset)"
        }
    }
}
