import Foundation

public struct Ceilings: Sendable, Equatable, Codable {
    public var blockTokens: UInt64

    public init(blockTokens: UInt64) {
        self.blockTokens = blockTokens
    }
}

/// Ritmo da janela corrente comparado ao ritmo típico do usuário.
///
/// Substitui o antigo gauge de "% do teto semanal". Aquele denominador era o
/// maior consumo já observado, o que só funciona com uso estável: em fase de
/// crescimento o numerador sobe mais rápido que o denominador e a barra vive
/// estourada — e um indicador sempre no vermelho deixa de ser sinal.
///
/// Este não promete limite nenhum. Diz apenas quantas vezes a semana atual é
/// maior que a mediana das anteriores, e por isso não satura.
public struct Pace: Sendable, Equatable {
    public let tokens: UInt64
    /// Mediana das janelas de 7 dias anteriores.
    public let typical: UInt64

    public init(tokens: UInt64, typical: UInt64) {
        self.tokens = tokens
        self.typical = typical
    }

    /// `nil` quando ainda não há histórico suficiente para uma referência —
    /// melhor não mostrar nada do que mostrar um múltiplo sobre quase-zero.
    public var multiple: Double? {
        guard typical > 0 else { return nil }
        return Double(tokens) / Double(typical)
    }
}

/// Deriva o denominador dos percentuais de risco.
///
/// A Anthropic não publica os limites do Max, então o teto é o **maior consumo
/// já observado** — se o usuário chegou até ali sem travar, o limite real é ao
/// menos isso. Sempre uma estimativa, e a UI diz isso.
public enum CeilingCalibrator {
    public static let defaultLookback: TimeInterval = 90 * 24 * 60 * 60

    /// A semana típica olha uma janela mais curta que o teto do bloco.
    ///
    /// São perguntas diferentes: o teto de 5h quer o maior pico já alcançado,
    /// e quanto mais histórico melhor. A semana típica quer "o que é normal
    /// **para você agora**" — e três meses atrás pode ser outra pessoa, com
    /// outro projeto e outro ritmo. Trinta dias acompanham mudança de patamar
    /// em vez de ancorar a referência num passado que já não descreve o uso.
    public static let paceLookback: TimeInterval = 30 * 24 * 60 * 60

    /// Pisos para o primeiro launch, quando ainda não há histórico suficiente.
    /// Existem só para o percentual não dividir por zero.
    static let floorBlockTokens: UInt64 = 1_000_000

    public static func blockCeiling(
        blocks: [UsageBlock], now: Date, lookback: TimeInterval = defaultLookback
    ) -> UInt64 {
        let cutoff = now.addingTimeInterval(-lookback)
        return blocks
            .filter { $0.isComplete(at: now) && $0.start >= cutoff }
            .map(\.tokens)
            .max() ?? 0
    }

    /// Mediana das janelas de 7 dias que **não se sobrepõem** à janela corrente,
    /// amostradas uma por dia.
    ///
    /// Só janelas sem sobreposição entram: qualquer uma que compartilhe dias com
    /// a atual deixaria o consumo de agora influenciar a própria referência.
    /// Mediana em vez de média porque um único dia atípico não deve deslocar o
    /// que se considera "normal".
    public static func typicalWeek(
        events: [UsageEvent], now: Date, lookback: TimeInterval = paceLookback
    ) -> UInt64 {
        let window: TimeInterval = 7 * 24 * 60 * 60
        let day: TimeInterval = 24 * 60 * 60
        let cutoff = now.addingTimeInterval(-lookback)
        let sorted = events
            .filter { $0.timestamp >= cutoff }
            .sorted { $0.timestamp < $1.timestamp }
        guard let firstEvent = sorted.first?.timestamp else { return 0 }

        var samples: [UInt64] = []
        var end = firstEvent.addingTimeInterval(window)
        let lastEnd = now.addingTimeInterval(-window)
        var tail = 0, head = 0, running: UInt64 = 0

        while end <= lastEnd {
            let start = end.addingTimeInterval(-window)
            while head < sorted.count, sorted[head].timestamp < end {
                running += sorted[head].totalTokens
                head += 1
            }
            while tail < head, sorted[tail].timestamp < start {
                running -= sorted[tail].totalTokens
                tail += 1
            }
            samples.append(running)
            end = end.addingTimeInterval(day)
        }

        guard !samples.isEmpty else { return 0 }
        samples.sort()
        return samples[samples.count / 2]
    }

    /// Override manual vence. Sem override, calibra pelo histórico e nunca
    /// devolve zero.
    public static func calibrate(
        blocks: [UsageBlock], now: Date, override: Ceilings?
    ) -> Ceilings {
        if let override { return override }
        return Ceilings(blockTokens: max(blockCeiling(blocks: blocks, now: now), floorBlockTokens))
    }
}
