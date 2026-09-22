import Foundation

/// Um relatório de uso da conta, como a Anthropic o publica.
///
/// A resposta ao vivo de `/api/oauth/usage` e o bloco `cachedUsageUtilization`
/// de `~/.claude.json` são o mesmo payload — o segundo é uma cópia gravada do
/// primeiro, mais um carimbo de quando foi buscado. Por isso existe um tipo só,
/// e não um por origem.
public struct UsageReport: Sendable, Equatable {
    public struct Limit: Sendable, Equatable {
        public enum Kind: Sendable, Equatable {
            case session
            case weeklyAll
            case weeklyScoped
            /// Tipo que este app não conhece. Preserva o nome cru em vez de
            /// descartar: o que aparece aqui é o que a próxima versão precisa
            /// suportar.
            case other(String)

            public init(raw: String) {
                switch raw {
                case "session": self = .session
                case "weekly_all": self = .weeklyAll
                case "weekly_scoped": self = .weeklyScoped
                default: self = .other(raw)
                }
            }
        }

        /// Só `normal` foi observado numa conta real — a sondagem pegou a conta
        /// em 6% e 25%, longe de qualquer alerta. Os nomes das gravidades altas
        /// são desconhecidos, e inventá-los produziria um `.other` silencioso
        /// com cara de suporte: pior que não suportar.
        public enum Severity: Sendable, Hashable {
            case normal
            case other(String)

            public init(raw: String) {
                self = raw == "normal" ? .normal : .other(raw)
            }
        }

        public let kind: Kind
        /// 0…1. O payload grava percentual; convertido na decodificação para
        /// bater com o resto do app, que trabalha em fração.
        public let fraction: Double
        public let severity: Severity
        public let resetsAt: Date?
        /// `scope.model.display_name`. Presente nas janelas por modelo, e é o
        /// rótulo que a UI usa — o app não mantém lista de nomes de modelo.
        public let modelName: String?
        /// A janela que a Anthropic considera a que aperta agora.
        public let isActive: Bool

        public init(kind: Kind, fraction: Double, severity: Severity,
                    resetsAt: Date?, modelName: String?, isActive: Bool) {
            self.kind = kind
            self.fraction = fraction
            self.severity = severity
            self.resetsAt = resetsAt
            self.modelName = modelName
            self.isActive = isActive
        }
    }

    public let limits: [Limit]
    /// Quando este relatório foi obtido. Ao vivo é agora; vindo do cache é
    /// quando o Claude Code buscou, que pode ser horas atrás.
    public let fetchedAt: Date

    public init(limits: [Limit], fetchedAt: Date) {
        self.limits = limits
        self.fetchedAt = fetchedAt
    }

    /// Primeira entrada de cada tipo vence. Uma segunda de mesmo `kind` seria
    /// formato novo, não correção da primeira.
    public var session: Limit? { limits.first { $0.kind == .session } }
    public var weeklyAll: Limit? { limits.first { $0.kind == .weeklyAll } }
    /// Na ordem do payload.
    public var weeklyScoped: [Limit] { limits.filter { $0.kind == .weeklyScoped } }
}
