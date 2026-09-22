import Foundation

/// Preços em USD por milhão de tokens.
public struct Rates: Sendable, Equatable {
    public let input: Decimal
    public let output: Decimal

    public init(input: Decimal, output: Decimal) {
        self.input = input
        self.output = output
    }

    // Multiplicadores fixos sobre o preço de input, iguais para todo modelo.
    // Construídos por significando/expoente porque literais de ponto flutuante
    // passam por Double e introduziriam imprecisão no Decimal.
    private static let write5mMultiplier = Decimal(sign: .plus, exponent: -2, significand: 125) // 1.25
    private static let write1hMultiplier = Decimal(2)
    private static let readMultiplier = Decimal(sign: .plus, exponent: -1, significand: 1)      // 0.1

    public var cacheWrite5m: Decimal { input * Self.write5mMultiplier }
    public var cacheWrite1h: Decimal { input * Self.write1hMultiplier }
    public var cacheRead: Decimal { input * Self.readMultiplier }
}

public enum PricingTable {
    /// Sonnet 5 tem preço introdutório de $2/$10 até 2026-08-31 inclusive;
    /// a partir deste instante volta a $3/$15.
    static let sonnet5IntroEnd: Date = {
        var cal = Calendar(identifier: .gregorian)
        cal.timeZone = TimeZone(identifier: "UTC")!
        return cal.date(from: DateComponents(year: 2026, month: 9, day: 1))!
    }()

    /// `nil` quando o modelo não é reconhecido — o chamador deve marcar o total
    /// como parcial, nunca tratar como zero.
    public static func rates(for model: ModelID, at date: Date, isFast: Bool) -> Rates? {
        switch model {
        case .opus5:
            return isFast ? Rates(input: 10, output: 50) : Rates(input: 5, output: 25)
        case .opus4x:
            return Rates(input: 5, output: 25)
        case .sonnet5:
            return date < sonnet5IntroEnd
                ? Rates(input: 2, output: 10)
                : Rates(input: 3, output: 15)
        case .sonnet4x:
            return Rates(input: 3, output: 15)
        case .haiku45:
            return Rates(input: 1, output: 5)
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
