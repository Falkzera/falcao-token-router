import Foundation

/// Tudo que a UI precisa desenhar, já calculado. Nenhuma shell conhece JSONL,
/// bloco ou preço — só recebe isto.
public struct UsageSnapshot: Sendable, Equatable {
    /// De onde este medidor veio. Três estados, não dois: o cache do Claude
    /// Code é oficial *e* pode estar horas defasado, e essas são perguntas
    /// diferentes.
    public enum Provenance: Sendable, Hashable {
        case live(at: Date)
        case cached(at: Date)
        case derived
    }

    /// Um medidor de janela.
    public struct Gauge: Sendable, Hashable {
        /// Razão real, sem saturação. Passa de 1 quando o consumo supera o
        /// denominador — e é aí que o número mais informa.
        public let rawFraction: Double
        public let resetsAt: Date?
        public let provenance: Provenance
        /// Gravidade como o servidor a classifica. `nil` no caminho derivado,
        /// que não tem servidor a consultar.
        public let severity: UsageReport.Limit.Severity?
        /// A janela que a Anthropic considera a que aperta agora.
        public let isActive: Bool
        /// Tokens e teto existem apenas no caminho derivado; o oficial devolve
        /// percentual pronto e não expõe os absolutos.
        public let tokens: UInt64?
        public let ceiling: UInt64?

        public var isOfficial: Bool { provenance != .derived }

        /// Saturada em 1, para a barra de progresso — que não pode encher além
        /// do fim.
        public var fraction: Double { min(1.0, rawFraction) }

        public func timeRemaining(at now: Date) -> TimeInterval? {
            guard let resetsAt else { return nil }
            return max(0, resetsAt.timeIntervalSince(now))
        }

        /// Há quanto tempo o dado foi buscado — **só faz sentido no cache**, que
        /// só se move quando o Claude Code roda. Ao vivo a idade é de segundos e
        /// mostrá-la produziria um "há 4s" permanente; derivado não tem busca.
        public func age(at now: Date) -> TimeInterval? {
            guard case let .cached(at) = provenance else { return nil }
            return max(0, now.timeIntervalSince(at))
        }

        public static func official(_ limit: UsageReport.Limit,
                                    from source: OfficialSource) -> Gauge {
            Gauge(rawFraction: limit.fraction,
                  resetsAt: limit.resetsAt,
                  provenance: source.isLive
                      ? .live(at: source.report.fetchedAt)
                      : .cached(at: source.report.fetchedAt),
                  severity: limit.severity,
                  isActive: limit.isActive,
                  tokens: nil, ceiling: nil)
        }

        public static func derived(tokens: UInt64, ceiling: UInt64, resetsAt: Date?) -> Gauge {
            Gauge(rawFraction: ceiling > 0 ? Double(tokens) / Double(ceiling) : 0,
                  resetsAt: resetsAt, provenance: .derived, severity: nil, isActive: false,
                  tokens: tokens, ceiling: ceiling)
        }
    }

    /// Uma janela semanal presa a um modelo. O rótulo vem do payload — o app
    /// não mantém lista de nomes de modelo, porque essa lista envelhece.
    public struct ScopedGauge: Sendable, Hashable {
        public let modelName: String
        public let gauge: Gauge

        public init(modelName: String, gauge: Gauge) {
            self.modelName = modelName
            self.gauge = gauge
        }
    }

    /// Sempre presente. Sem sessão em curso vem zerado e com `resetsAt` nulo —
    /// a UI continua desenhando a barra vazia em vez de trocar de layout.
    public let session: Gauge
    /// Só existe com leitura oficial: a janela semanal da Anthropic tem reset
    /// próprio, que não dá para derivar do histórico local.
    public let weekly: Gauge?
    /// Janelas semanais por modelo, na ordem do payload. Vazio quando não há
    /// nenhuma — que é o caso mais comum.
    public let scopedWeekly: [ScopedGauge]
    /// Comparação com o ritmo típico. Usada quando não há semanal oficial.
    public let weeklyPace: Pace
    public let today: Totals
    public let week: Totals
    public let month: Totals
    /// Tokens por minuto no bloco ativo; `nil` sem bloco ativo.
    public let burnRatePerMinute: Double?
    /// Nomes crus de modelos sem preço conhecido — a UI mostra quais são.
    public let unknownModels: Set<String>
    public let generatedAt: Date
    /// Teto do bloco de 5h como a calibração automática o encontrou —
    /// **ignorando o override manual de propósito**.
    ///
    /// Mora aqui, e não em `Gauge.ceiling`, por duas razões. O medidor oficial
    /// não tem denominador local (`Gauge.official` deixa `ceiling` nulo), então
    /// tirá-lo de lá dava zero justamente no caminho que virou o normal. E o
    /// medidor derivado carrega o teto *efetivo*, que é o override quando existe
    /// — devolvê-lo à tela seria circular, porque ela mostra este número como
    /// referência enquanto o usuário digita o próprio override.
    public let calibratedBlockCeiling: UInt64
    /// De onde vieram os números, incluindo os estados de falha. A UI diz isso
    /// em vez de mostrar um número sem procedência.
    public let sourceStatus: UsageSourceStatus

    public init(
        session: Gauge, weekly: Gauge?, scopedWeekly: [ScopedGauge], weeklyPace: Pace,
        today: Totals, week: Totals, month: Totals,
        burnRatePerMinute: Double?, unknownModels: Set<String>, generatedAt: Date,
        calibratedBlockCeiling: UInt64, sourceStatus: UsageSourceStatus
    ) {
        self.session = session
        self.weekly = weekly
        self.scopedWeekly = scopedWeekly
        self.weeklyPace = weeklyPace
        self.today = today
        self.week = week
        self.month = month
        self.burnRatePerMinute = burnRatePerMinute
        self.unknownModels = unknownModels
        self.generatedAt = generatedAt
        self.calibratedBlockCeiling = calibratedBlockCeiling
        self.sourceStatus = sourceStatus
    }

    public static func empty(at now: Date) -> UsageSnapshot {
        UsageSnapshot(session: .derived(tokens: 0, ceiling: 1, resetsAt: nil),
                      weekly: nil,
                      scopedWeekly: [],
                      weeklyPace: Pace(tokens: 0, typical: 0),
                      today: .zero, week: .zero, month: .zero,
                      burnRatePerMinute: nil, unknownModels: [], generatedAt: now,
                      calibratedBlockCeiling: CeilingCalibrator.floorBlockTokens,
                      sourceStatus: .derivedOnly)
    }
}
