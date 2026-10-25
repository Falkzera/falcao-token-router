import Foundation

/// De onde veio o número que está na tela.
///
/// Cada caso é uma frase diferente na UI, porque cada um diz uma coisa
/// diferente sobre a idade do número.
public enum UsageSourceStatus: Sendable, Equatable {
    case live(at: Date)
    case cached(age: TimeInterval)
    /// Nenhuma fonte oficial; o `SnapshotBuilder` segue pelo caminho derivado.
    case derivedOnly
}

/// Relatório escolhido, com a origem que a UI precisa saber.
public struct OfficialSource: Sendable, Equatable {
    public let report: UsageReport
    public let isLive: Bool

    public init(report: UsageReport, isLive: Bool) {
        self.report = report
        self.isLive = isLive
    }
}

/// Escolhe entre o número ao vivo (a amostra recente do sensor) e o cache.
///
/// Função pura, sem relógio próprio e sem I/O: `now` entra por parâmetro.
///
/// Ao vivo ligado e sem amostra recente é o caso COMUM — a conta do perfil
/// padrão não serviu nada na última hora —, e não é erro de nada: o cache, com a
/// idade dele, é a melhor fonte que existe. Até 10/2026 esse caso aparecia como
/// "credencial expirada · rode o Claude Code", herança da época em que "ao vivo"
/// era uma chamada de rede com token; o sensor não tem credencial para expirar.
public enum UsageSourcePolicy {
    public static func select(
        liveEnabled: Bool,
        live: UsageReport?,
        cached: UsageReport?,
        now: Date
    ) -> (source: OfficialSource?, status: UsageSourceStatus) {
        if liveEnabled, let live {
            return (OfficialSource(report: live, isLive: true), .live(at: live.fetchedAt))
        }
        guard let cached else { return (nil, .derivedOnly) }
        // Relógio ajustado para trás não pode produzir idade negativa.
        let age = max(0, now.timeIntervalSince(cached.fetchedAt))
        return (OfficialSource(report: cached, isLive: false), .cached(age: age))
    }
}
