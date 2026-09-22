import Foundation
import Testing
@testable import CCUsageCore

private func date(_ s: String) -> Date {
    try! Date.ISO8601FormatStyle(includingFractionalSeconds: false).parse(s)
}

private func event(_ iso: String, output: UInt32 = 100) -> UsageEvent {
    UsageEvent(timestamp: date(iso), model: .opus5, isFast: false,
               input: 0, output: output, cacheWrite5m: 0, cacheWrite1h: 0,
               cacheRead: 0, dedupeKey: iso)
}

private var utc: Calendar {
    var c = Calendar(identifier: .gregorian)
    c.timeZone = TimeZone(identifier: "UTC")!
    return c
}

@Test func blockStartsFlooredToTheHour() {
    let blocks = BlockBuilder.blocks(from: [event("2026-08-17T10:37:00Z")], calendar: utc)
    #expect(blocks.count == 1)
    #expect(blocks[0].start == date("2026-08-17T10:00:00Z"))
    #expect(blocks[0].end == date("2026-08-17T15:00:00Z"))
}

@Test func eventsWithinFiveHoursShareABlock() {
    let blocks = BlockBuilder.blocks(
        from: [event("2026-08-17T10:10:00Z"), event("2026-08-17T14:00:00Z")], calendar: utc)
    #expect(blocks.count == 1)
    #expect(blocks[0].tokens == 200)
}

@Test func crossingTheWindowOpensANewBlock() {
    // 10:00 + 5h = 15:00; um evento às 15:30 cai fora do primeiro bloco.
    let blocks = BlockBuilder.blocks(
        from: [event("2026-08-17T10:10:00Z"), event("2026-08-17T15:30:00Z")], calendar: utc)
    #expect(blocks.count == 2)
    #expect(blocks[1].start == date("2026-08-17T15:00:00Z"))
}

@Test func gapLongerThanFiveHoursOpensANewBlock() {
    let blocks = BlockBuilder.blocks(
        from: [event("2026-08-17T10:00:00Z"), event("2026-08-18T09:00:00Z")], calendar: utc)
    #expect(blocks.count == 2)
}

@Test func eventsAreSortedBeforeBlocking() {
    let blocks = BlockBuilder.blocks(
        from: [event("2026-08-17T14:00:00Z"), event("2026-08-17T10:10:00Z")], calendar: utc)
    #expect(blocks.count == 1)
    #expect(blocks[0].start == date("2026-08-17T10:00:00Z"))
}

@Test func activeAndCompleteAreDistinct() {
    let blocks = BlockBuilder.blocks(from: [event("2026-08-17T10:10:00Z")], calendar: utc)
    let b = blocks[0]
    #expect(b.isActive(at: date("2026-08-17T12:00:00Z")) == true)
    #expect(b.isComplete(at: date("2026-08-17T12:00:00Z")) == false)
    #expect(b.isActive(at: date("2026-08-17T16:00:00Z")) == false)
    #expect(b.isComplete(at: date("2026-08-17T16:00:00Z")) == true)
}

@Test func unknownModelMakesTheBlockMoneyPartial() {
    let unknown = UsageEvent(timestamp: date("2026-08-17T10:10:00Z"),
                             model: .unknown("claude-opus-9"), isFast: false,
                             input: 1000, output: 0, cacheWrite5m: 0, cacheWrite1h: 0,
                             cacheRead: 0, dedupeKey: "u")
    let blocks = BlockBuilder.blocks(from: [event("2026-08-17T10:10:00Z"), unknown], calendar: utc)
    #expect(blocks[0].money.isPartial == true)
    #expect(blocks[0].money.usd > 0)   // a parte precificável foi preservada
    #expect(blocks[0].tokens == 1100)  // tokens contam mesmo sem preço
}

@Test func noEventsYieldsNoBlocks() {
    #expect(BlockBuilder.blocks(from: [], calendar: utc).isEmpty)
}
