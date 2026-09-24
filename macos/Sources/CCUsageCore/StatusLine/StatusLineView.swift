import Foundation

/// A linha que a status line de um grupo imprime (≙ `view.rs` do porte Windows):
///
///     ● grupo │ Modelo effort │ branch │ contexto │ 5h … ↻ hh:mm  7d … ↻ dia (dd) h:mm │ $custo │ e-mail
///
/// **Por que ela é completa.** Até 24/09/2026 a linha do router era
/// `conta 5h 7d`, e ela SUBSTITUI a `statusLine` do perfil — inclusive a do
/// grupo padrão, que é o `~/.claude` do usuário. Quem tinha uma status line
/// própria a perdia ao ativar a integração de terminal, trocada por duas
/// porcentagens. O router precisa ser dono daquela linha (a linha **é** o
/// sensor), então a saída foi enriquecê-la em vez de empobrecer a sessão: o
/// GRUPO no lugar do nome do perfil, e o e-mail da conta ativa no fim — que é
/// onde a troca de conta aparece. Descoberto no primeiro teste real do porte
/// Windows, portado para cá.
///
/// **Pura.** O relógio (a fase das animações), o idioma e o suporte a cor entram
/// como `Style`; o que a linha mostra entra como `View`, já montada. O sensor (a
/// gravação da amostra) não passa por aqui. É o que permite testar a linha sem
/// sessão, e o que vai permitir a prévia dos Ajustes desenhar com este código.
public struct StatusLineView {
    /// Quem a linha nomeia primeiro: o grupo dono do perfil, ou — fora de um
    /// grupo conhecido — a conta.
    public enum Label {
        case group(name: String, index: Int)
        case account(String)
    }

    /// Uma janela do `rate_limits`: fração 0–1 e o reset.
    public struct Window {
        public let fraction: Double
        public let resetsAt: Date?
        public init(fraction: Double, resetsAt: Date?) {
            self.fraction = fraction
            self.resetsAt = resetsAt
        }
    }

    /// A janela de contexto: `used_percentage` (0–100) e os tokens.
    public struct Context {
        public let usedPercent: Double
        public let inputTokens: UInt64
        public let windowSize: UInt64
        public init(usedPercent: Double, inputTokens: UInt64, windowSize: UInt64) {
            self.usedPercent = usedPercent
            self.inputTokens = inputTokens
            self.windowSize = windowSize
        }
    }

    public var label: Label?
    public var model: String?
    public var modelID: String?
    public var effort: String?
    public var place: String?
    public var context: Context?
    public var fiveHour: Window?
    public var sevenDay: Window?
    public var costUSD: Double?
    public var email: String?

    public init(label: Label? = nil, model: String? = nil, modelID: String? = nil,
                effort: String? = nil, place: String? = nil, context: Context? = nil,
                fiveHour: Window? = nil, sevenDay: Window? = nil,
                costUSD: Double? = nil, email: String? = nil) {
        self.label = label; self.model = model; self.modelID = modelID
        self.effort = effort; self.place = place; self.context = context
        self.fiveHour = fiveHour; self.sevenDay = sevenDay
        self.costUSD = costUSD; self.email = email
    }

    public struct Style {
        public let trueColor: Bool
        public let portuguese: Bool
        /// A fase das animações, em segundos. Entra de fora para a mesma
        /// `View` com a mesma fase dar sempre a mesma linha.
        public let phase: UInt64
        public let calendar: Calendar

        public init(trueColor: Bool, portuguese: Bool, phase: UInt64, calendar: Calendar = .current) {
            self.trueColor = trueColor
            self.portuguese = portuguese
            self.phase = phase
            self.calendar = calendar
        }
    }

    /// A linha inteira. O que não veio — o primeiro render de uma sessão não tem
    /// `rate_limits`, e o `effort` só vem com modelo que o aceita — fica de
    /// fora, **sem marcador**: um espaço reservado para o que não existe é ruído
    /// permanente numa linha que se lê de relance.
    public func render(_ style: Style) -> String {
        let gray = Paint.gray(trueColor: style.trueColor)
        var segments: [String] = []

        if let label {
            let (color, name): (String, String) = switch label {
            case .group(let name, let index): (Paint.groupColors[index % Paint.groupColors.count], name)
            case .account(let name): (Paint.yellow, name)
            }
            segments.append("\(color)\(Paint.bold)●\(Paint.reset) \(color)\(name)\(Paint.reset)")
        }

        // O esforço vai ao lado do modelo; sem o modelo, sozinho.
        let painted = effort.map { Paint.effort($0, trueColor: style.trueColor, phase: style.phase, gray: gray) }
        switch (model, painted) {
        case (.some(let display), let effort):
            let color = StatusLineFormat.modelColor(display, id: modelID)
            var segment = "\(color)\(Paint.bold)\(StatusLineFormat.modelName(display))\(Paint.reset)"
            if let effort { segment += " " + effort }
            segments.append(segment)
        case (.none, .some(let effort)):
            segments.append(effort)
        case (.none, .none):
            break
        }

        if let place {
            segments.append("\(gray)\(place)\(Paint.reset)")
        }

        if let context {
            let fraction = context.usedPercent / 100
            let tone = Paint.tone(fraction)
            segments.append("\(tone)\(Paint.bar(fraction, width: 10))\(Paint.reset) "
                + "\(tone)\(UsagePercent.value(fraction))%\(Paint.reset) "
                + "\(gray)\(StatusLineFormat.tokens(context.inputTokens))/"
                + "\(StatusLineFormat.tokens(context.windowSize))\(Paint.reset)")
        }

        let windows: [String] = [("5h", fiveHour, false), ("7d", sevenDay, true)]
            .compactMap { name, window, withDay in
                guard let window else { return nil }
                let tone = Paint.tone(window.fraction)
                var text = "\(gray)\(name)\(Paint.reset) "
                    + "\(tone)\(Paint.bar(window.fraction, width: 5)) "
                    + "\(UsagePercent.value(window.fraction))%\(Paint.reset)"
                if let at = window.resetsAt {
                    let when = StatusLineFormat.resetWhen(at, withDay: withDay,
                                                          portuguese: style.portuguese,
                                                          calendar: style.calendar)
                    text += " \(gray)↻ \(when)\(Paint.reset)"
                }
                return text
            }
        if !windows.isEmpty { segments.append(windows.joined(separator: "  ")) }

        if let costUSD {
            segments.append("\(gray)$\(String(format: "%.2f", costUSD))\(Paint.reset)")
        }
        if let email {
            segments.append("\(gray)\(email)\(Paint.reset)")
        }
        return segments.joined(separator: "\(gray) │ \(Paint.reset)")
    }
}
