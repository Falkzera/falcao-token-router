import Foundation
import Testing
@testable import CCUsageCore

/// A medição automática de 5 em 5 horas: a regra de QUANDO medir de novo.
@Suite("MeasureSchedule")
struct MeasureScheduleTests {
    private let now = Date(timeIntervalSince1970: 1_792_000_000)

    @Test("conta nunca medida nem tentada vence")
    func nunca() {
        #expect(MeasureSchedule.isDue(lastMeasured: nil, lastAttempt: nil, now: now))
        #expect(MeasureSchedule.nextAt(lastMeasured: nil, lastAttempt: nil) == nil)
    }

    @Test("medida há menos de 5 horas não vence")
    func recente() {
        #expect(!MeasureSchedule.isDue(lastMeasured: now.addingTimeInterval(-4 * 3600),
                                       lastAttempt: nil, now: now))
        #expect(!MeasureSchedule.isDue(lastMeasured: now.addingTimeInterval(-299 * 60),
                                       lastAttempt: nil, now: now))
    }

    @Test("5 horas exatas vencem")
    func exato() {
        #expect(MeasureSchedule.isDue(lastMeasured: now.addingTimeInterval(-5 * 3600),
                                      lastAttempt: nil, now: now))
    }

    /// A conta deslogada falha toda vez: sem a tentativa, seria sondada a cada
    /// volta do laço (3 min). Com ela, de novo só em 5 horas.
    @Test("a tentativa que falhou segura a conta por 5 horas")
    func tentativa() {
        let old = now.addingTimeInterval(-2 * 86_400)
        #expect(!MeasureSchedule.isDue(lastMeasured: old, lastAttempt: now.addingTimeInterval(-3600), now: now))
        #expect(MeasureSchedule.isDue(lastMeasured: old, lastAttempt: now.addingTimeInterval(-5 * 3600), now: now))
        #expect(MeasureSchedule.nextAt(lastMeasured: now.addingTimeInterval(-3600),
                                       lastAttempt: now.addingTimeInterval(-3 * 3600))
                == now.addingTimeInterval(4 * 3600))
    }
}
