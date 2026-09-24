import Foundation

/// Valor em USD que sabe quando está incompleto.
///
/// `isPartial` existe para que um modelo sem preço conhecido nunca seja contado
/// como zero: a soma dos eventos precificáveis é preservada e o total é marcado
/// como piso, não como valor exato.
public struct Money: Sendable, Equatable {
    public var usd: Decimal
    public var isPartial: Bool

    public static let zero = Money(usd: 0, isPartial: false)

    public init(usd: Decimal, isPartial: Bool) {
        self.usd = usd
        self.isPartial = isPartial
    }

    /// Soma um custo; `nil` significa "esse evento não tem preço conhecido".
    public mutating func add(cost: Decimal?) {
        if let cost { usd += cost } else { isPartial = true }
    }

    public static func + (lhs: Money, rhs: Money) -> Money {
        Money(usd: lhs.usd + rhs.usd, isPartial: lhs.isPartial || rhs.isPartial)
    }
}
