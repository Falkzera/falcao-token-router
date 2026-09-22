import Foundation

public struct Totals: Sendable, Equatable {
    public var tokens: UInt64
    public var money: Money

    public init(tokens: UInt64, money: Money) {
        self.tokens = tokens
        self.money = money
    }

    public static let zero = Totals(tokens: 0, money: .zero)
}

/// Recorta eventos em períodos de calendário e na janela rolling de 7 dias.
///
/// "Semana" tem duas definições deliberadamente distintas: a de calendário
/// (segunda a domingo) alimenta a seção de **valor**, e a rolling de 7 dias
/// alimenta o gauge de **risco**. Cada uma responde a uma pergunta diferente,
/// e a UI rotula ambas explicitamente.
public struct PeriodAggregator: Sendable {
    public let calendar: Calendar

    public init(calendar: Calendar = .current) {
        var cal = calendar
        cal.firstWeekday = 2  // segunda-feira
        self.calendar = cal
    }

    public func todayInterval(now: Date) -> DateInterval {
        calendar.dateInterval(of: .day, for: now)!
    }

    public func weekInterval(now: Date) -> DateInterval {
        calendar.dateInterval(of: .weekOfYear, for: now)!
    }

    public func monthInterval(now: Date) -> DateInterval {
        calendar.dateInterval(of: .month, for: now)!
    }

    public func rolling7Days(now: Date) -> DateInterval {
        DateInterval(start: now.addingTimeInterval(-7 * 24 * 60 * 60), end: now)
    }

    /// Intervalo semiaberto `[start, end)` — a borda final pertence ao próximo período.
    public func totals(from events: [UsageEvent], in interval: DateInterval) -> Totals {
        var totals = Totals.zero
        for event in events
        where event.timestamp >= interval.start && event.timestamp < interval.end {
            totals.tokens += event.totalTokens
            totals.money.add(cost: PricingTable.cost(of: event))
        }
        return totals
    }
}
