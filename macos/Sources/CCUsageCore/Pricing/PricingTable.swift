import Foundation

/// Preços em USD por milhão de tokens.
public struct Rates: Sendable, Equatable {
    public let input: Decimal
    public let output: Decimal
    /// Fração do preço de input que a leitura de cache custa. 0,1 era a regra de
    /// todo modelo até a geração 5.5/5.1, que a baixou (Opus 5.5: $0,20 sobre
    /// $4; Fable/Mythos 5.1: $0,25 sobre $10).
    private let cacheReadMultiplier: Decimal

    public init(input: Decimal, output: Decimal,
                cacheReadMultiplier: Decimal = Rates.standardReadMultiplier) {
        self.input = input
        self.output = output
        self.cacheReadMultiplier = cacheReadMultiplier
    }

    // Multiplicadores sobre o preço de input. Construídos por
    // significando/expoente porque literais de ponto flutuante passam por Double
    // e introduziriam imprecisão no Decimal.
    private static let write5mMultiplier = Decimal(sign: .plus, exponent: -2, significand: 125) // 1.25
    private static let write1hMultiplier = Decimal(2)
    public static let standardReadMultiplier = Decimal(sign: .plus, exponent: -1, significand: 1) // 0.1

    public var cacheWrite5m: Decimal { input * Self.write5mMultiplier }
    public var cacheWrite1h: Decimal { input * Self.write1hMultiplier }
    public var cacheRead: Decimal { input * cacheReadMultiplier }
}

/// A tabela pública da Anthropic, conferida em 25/09/2026.
///
/// Muda a cada lançamento de modelo; modelo que não está aqui deixa o total
/// "parcial" na tela — nunca zero, nunca um preço chutado.
public enum PricingTable {
    private static let readOpus55 = Decimal(sign: .plus, exponent: -2, significand: 5)   // 0.05
    private static let readFable51 = Decimal(sign: .plus, exponent: -3, significand: 25) // 0.025

    /// `nil` quando o modelo não é reconhecido — o chamador deve marcar o total
    /// como parcial, nunca tratar como zero.
    ///
    /// O Sonnet 5 custa $2/$10. A tabela daqui previa uma volta a $3/$15 a partir
    /// de 01/09/2026, que a tabela oficial de 25/09 não confirma: desde então o
    /// total saía 50% acima, sem a marca de parcial.
    public static func rates(for model: ModelID, at date: Date, isFast: Bool) -> Rates? {
        switch model.resolved {
        case .opus55:
            return isFast ? Rates(input: 8, output: 40, cacheReadMultiplier: readOpus55)
                          : Rates(input: 4, output: 20, cacheReadMultiplier: readOpus55)
        case .opus5:
            return isFast ? Rates(input: 10, output: 50) : Rates(input: 5, output: 25)
        case .opus4x:
            return Rates(input: 5, output: 25)
        case .sonnet55, .sonnet5:
            return Rates(input: 2, output: 10)
        case .sonnet4x:
            return Rates(input: 3, output: 15)
        case .haiku45:
            return Rates(input: 1, output: 5)
        case .fable51:
            return Rates(input: 10, output: 50, cacheReadMultiplier: readFable51)
        case .fable5:
            return Rates(input: 10, output: 50)
        case .unknown:
            return nil
        }
    }

    private static let perMillion = Decimal(1_000_000)

    public static func cost(of event: UsageEvent) -> Decimal? {
        guard let r = rates(for: event.model, at: event.timestamp, isFast: event.isFast) else {
            return nil
        }
        let total = Decimal(event.input) * r.input
            + Decimal(event.output) * r.output
            + Decimal(event.cacheWrite5m) * r.cacheWrite5m
            + Decimal(event.cacheWrite1h) * r.cacheWrite1h
            + Decimal(event.cacheRead) * r.cacheRead
        return total / perMillion
    }
}
