import Foundation
import Testing
@testable import CCUsageCore

@Test func addingKnownCostsSums() {
    var m = Money.zero
    m.add(cost: Decimal(string: "1.50"))
    m.add(cost: Decimal(string: "2.25"))
    #expect(m.usd == Decimal(string: "3.75"))
    #expect(m.isPartial == false)
}

@Test func addingNilCostFlagsPartialButKeepsTheSum() {
    var m = Money.zero
    m.add(cost: Decimal(string: "10.00"))
    m.add(cost: nil)
    #expect(m.usd == Decimal(string: "10.00"))
    #expect(m.isPartial == true)
}

@Test func partialityPropagatesThroughAddition() {
    let a = Money(usd: 1, isPartial: false)
    let b = Money(usd: 2, isPartial: true)
    #expect((a + b).usd == 3)
    #expect((a + b).isPartial == true)
}
