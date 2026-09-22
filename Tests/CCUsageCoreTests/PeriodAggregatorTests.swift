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

private var utcAggregator: PeriodAggregator {
    var c = Calendar(identifier: .gregorian)
    c.timeZone = TimeZone(identifier: "UTC")!
    return PeriodAggregator(calendar: c)
}

@Test func todayIsTheLocalCalendarDay() {
    let interval = utcAggregator.todayInterval(now: date("2026-08-17T23:30:00Z"))
    #expect(interval.start == date("2026-08-17T00:00:00Z"))
    #expect(interval.end == date("2026-08-18T00:00:00Z"))
}

@Test func calendarWeekStartsOnMonday() {
    // 2026-08-17 é uma segunda-feira.
    let interval = utcAggregator.weekInterval(now: date("2026-08-20T12:00:00Z"))
    #expect(interval.start == date("2026-08-17T00:00:00Z"))
    #expect(interval.end == date("2026-08-24T00:00:00Z"))
}

@Test func monthCoversTheWholeCalendarMonth() {
    let interval = utcAggregator.monthInterval(now: date("2026-08-17T12:00:00Z"))
    #expect(interval.start == date("2026-08-01T00:00:00Z"))
    #expect(interval.end == date("2026-09-01T00:00:00Z"))
}

@Test func rollingSevenDaysDiffersFromCalendarWeek() {
    let agg = utcAggregator
    let now = date("2026-08-20T12:00:00Z")
    let rolling = agg.rolling7Days(now: now)
    #expect(rolling.start == date("2026-08-13T12:00:00Z"))
    #expect(rolling.end == now)
    #expect(rolling.start != agg.weekInterval(now: now).start)
}

@Test func totalsCountOnlyEventsInsideTheInterval() {
    let agg = utcAggregator
    let events = [
        event("2026-08-16T23:59:00Z", output: 1),   // ontem
        event("2026-08-17T00:00:00Z", output: 10),  // borda inicial: dentro
        event("2026-08-17T12:00:00Z", output: 20),
        event("2026-08-18T00:00:00Z", output: 40),  // borda final: fora
    ]
    let totals = agg.totals(from: events,
                            in: agg.todayInterval(now: date("2026-08-17T12:00:00Z")))
    #expect(totals.tokens == 30)
}

@Test func totalsGoPartialOnUnknownModel() {
    let agg = utcAggregator
    let unknown = UsageEvent(timestamp: date("2026-08-17T12:00:00Z"),
                             model: .unknown("x"), isFast: false,
                             input: 100, output: 0, cacheWrite5m: 0, cacheWrite1h: 0,
                             cacheRead: 0, dedupeKey: "u")
    let totals = agg.totals(from: [event("2026-08-17T11:00:00Z"), unknown],
                            in: agg.todayInterval(now: date("2026-08-17T12:00:00Z")))
    #expect(totals.money.isPartial == true)
    #expect(totals.tokens == 200)
}

@Test func monthBoundaryDoesNotLeakIntoNextMonth() {
    let agg = utcAggregator
    let events = [event("2026-08-31T23:00:00Z", output: 5),
                  event("2026-09-01T01:00:00Z", output: 7)]
    let august = agg.totals(from: events, in: agg.monthInterval(now: date("2026-08-15T00:00:00Z")))
    #expect(august.tokens == 5)
}
