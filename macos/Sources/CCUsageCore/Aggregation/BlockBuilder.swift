import Foundation

/// Uma janela de 5 horas derivada da atividade.
///
/// A Anthropic não persiste os horários oficiais de reset em lugar nenhum local,
/// então isto é **estimativa** — a UI precisa apresentá-la como tal.
public struct UsageBlock: Sendable, Equatable {
    public let start: Date       // arredondado para baixo à hora cheia
    public let end: Date         // start + 5h
    public let lastEvent: Date
    public let tokens: UInt64
    public let money: Money

    public init(start: Date, end: Date, lastEvent: Date, tokens: UInt64, money: Money) {
        self.start = start
        self.end = end
        self.lastEvent = lastEvent
        self.tokens = tokens
        self.money = money
    }

    public func isActive(at now: Date) -> Bool { now >= start && now < end }
    public func isComplete(at now: Date) -> Bool { now >= end }
}

public enum BlockBuilder {
    public static let duration: TimeInterval = 5 * 60 * 60

    /// Abre um bloco novo quando não há bloco corrente, quando o evento passa de
    /// `início + 5h`, ou quando há um gap de mais de 5h desde o último evento.
    public static func blocks(from events: [UsageEvent],
                              calendar: Calendar = .current) -> [UsageBlock] {
        let sorted = events.sorted { $0.timestamp < $1.timestamp }
        var blocks: [UsageBlock] = []

        var start: Date?
        var last: Date?
        var tokens: UInt64 = 0
        var money = Money.zero

        func closeCurrent() {
            guard let start, let last else { return }
            blocks.append(UsageBlock(start: start,
                                     end: start.addingTimeInterval(duration),
                                     lastEvent: last,
                                     tokens: tokens,
                                     money: money))
        }

        for event in sorted {
            let needsNewBlock: Bool
            if let start, let last {
                needsNewBlock = event.timestamp >= start.addingTimeInterval(duration)
                    || event.timestamp >= last.addingTimeInterval(duration)
            } else {
                needsNewBlock = true
            }

            if needsNewBlock {
                closeCurrent()
                start = calendar.dateInterval(of: .hour, for: event.timestamp)?.start
                    ?? event.timestamp
                tokens = 0
                money = .zero
            }

            tokens += event.totalTokens
            money.add(cost: PricingTable.cost(of: event))
            last = event.timestamp
        }

        closeCurrent()
        return blocks
    }
}
