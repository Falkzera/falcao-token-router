import Foundation
import Testing
@testable import CCUsageCore

private func date(_ s: String) -> Date {
    try! Date.ISO8601FormatStyle(includingFractionalSeconds: false).parse(s)
}

private func block(_ startISO: String, tokens: UInt64) -> UsageBlock {
    let start = date(startISO)
    return UsageBlock(start: start, end: start.addingTimeInterval(BlockBuilder.duration),
                      lastEvent: start, tokens: tokens, money: .zero)
}

private func event(_ iso: String, output: UInt32) -> UsageEvent {
    UsageEvent(timestamp: date(iso), model: .opus5, isFast: false,
               input: 0, output: output, cacheWrite5m: 0, cacheWrite1h: 0,
               cacheRead: 0, dedupeKey: iso)
}

/// Um evento de 100 tokens por dia, de 1 a 31 de agosto. Qualquer janela de
/// 7 dias inteiramente dentro do período soma 700.
private let steadyAugust = (1...31).map {
    event(String(format: "2026-08-%02dT12:00:00Z", $0), output: 100)
}

// MARK: - Teto do bloco de 5h

@Test func blockCeilingIsTheLargestCompletedBlock() {
    let now = date("2026-08-17T20:00:00Z")
    let blocks = [
        block("2026-08-15T00:00:00Z", tokens: 500_000),
        block("2026-08-16T00:00:00Z", tokens: 900_000),
        block("2026-08-17T18:00:00Z", tokens: 100_000),  // ainda em curso
    ]
    #expect(CeilingCalibrator.blockCeiling(blocks: blocks, now: now) == 900_000)
}

@Test func blockCeilingIgnoresBlocksOlderThanLookback() {
    let now = date("2026-08-17T20:00:00Z")
    let blocks = [
        block("2026-01-01T00:00:00Z", tokens: 9_000_000),  // fora dos 90 dias
        block("2026-08-16T00:00:00Z", tokens: 400_000),
    ]
    #expect(CeilingCalibrator.blockCeiling(blocks: blocks, now: now) == 400_000)
}

@Test func manualOverrideWins() {
    let ceilings = CeilingCalibrator.calibrate(
        blocks: [block("2026-08-16T00:00:00Z", tokens: 900_000)],
        now: date("2026-08-17T20:00:00Z"),
        override: Ceilings(blockTokens: 2_000_000))
    #expect(ceilings.blockTokens == 2_000_000)
}

@Test func blockCeilingIsNeverZeroSoThePercentageStaysDefined() {
    let ceilings = CeilingCalibrator.calibrate(
        blocks: [], now: date("2026-08-17T20:00:00Z"), override: nil)
    #expect(ceilings.blockTokens > 0)
}

// MARK: - Semana típica

@Test func typicalWeekIsTheMedianOfPastWindows() {
    #expect(CeilingCalibrator.typicalWeek(events: steadyAugust,
                                          now: date("2026-09-15T12:00:00Z")) == 700)
}

@Test func aSingleOutlierDayDoesNotShiftTheTypicalWeek() {
    // Mediana, não média: um dia atípico no meio da rotina não redefine o que
    // conta como normal.
    var events = steadyAugust
    events.append(event("2026-08-20T18:00:00Z", output: 1_000_000))
    #expect(CeilingCalibrator.typicalWeek(events: events,
                                          now: date("2026-09-15T12:00:00Z")) == 700)
}

@Test func currentWindowDoesNotShiftTheBaseline() {
    // Regressão do gauge antigo: um pico agora não pode virar a própria
    // referência, senão a comparação se anula.
    var events = steadyAugust
    events.append(event("2026-09-14T12:00:00Z", output: 1_000_000))
    #expect(CeilingCalibrator.typicalWeek(events: events,
                                          now: date("2026-09-15T12:00:00Z")) == 700)
}

@Test func typicalWeekIgnoresHistoryOlderThanThirtyDays() {
    // Um período antigo e pesado não descreve o ritmo atual: pode ser outro
    // projeto, outra fase. A referência olha só os últimos 30 dias.
    let ancient = (1...31).map {
        event(String(format: "2026-05-%02dT12:00:00Z", $0), output: 10_000)
    }
    #expect(CeilingCalibrator.typicalWeek(events: ancient + steadyAugust,
                                          now: date("2026-09-15T12:00:00Z")) == 700)
}

@Test func typicalWeekIsZeroWithoutHistory() {
    #expect(CeilingCalibrator.typicalWeek(events: [], now: date("2026-09-15T12:00:00Z")) == 0)
}

// MARK: - Pace

@Test func paceReportsTheMultipleOverTypical() {
    #expect(Pace(tokens: 1_400, typical: 700).multiple == 2.0)
}

@Test func paceHasNoMultipleWithoutABaseline() {
    // Sem referência não se inventa número: melhor "—" que um múltiplo sobre
    // quase-zero.
    #expect(Pace(tokens: 500, typical: 0).multiple == nil)
}
