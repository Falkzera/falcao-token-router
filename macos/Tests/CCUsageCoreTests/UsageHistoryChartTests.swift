import Foundation
import Testing
@testable import CCUsageCore

/// As séries dos gráficos do Histórico: recorte, buracos, marcas — as mesmas
/// regras do porte Windows (`history.rs`).
@Suite("UsageHistoryChart")
struct UsageHistoryChartTests {
    /// Quarta, 21/10/2026, meio-dia em São Paulo (UTC−3, sem horário de verão).
    private let now: Date = {
        var c = DateComponents()
        c.year = 2026; c.month = 10; c.day = 21; c.hour = 15
        var cal = Calendar(identifier: .gregorian)
        cal.timeZone = TimeZone(identifier: "UTC")!
        return cal.date(from: c)!
    }()
    /// Fuso fixo, para as marcas não dependerem da máquina que roda o teste.
    private let saoPaulo: Calendar = {
        var cal = Calendar(identifier: .gregorian)
        cal.timeZone = TimeZone(secondsFromGMT: -3 * 3600)!
        return cal
    }()

    private func point(hoursAgo: Double, five: Double?, seven: Double?) -> UsageHistoryPoint {
        UsageHistoryPoint(sampledAt: now.addingTimeInterval(-hoursAgo * 3600),
                          fiveHourPercent: five, fiveHourResetsAt: nil,
                          sevenDayPercent: seven, sevenDayResetsAt: nil)
    }

    @Test("cada gráfico guarda o seu período")
    func periodos() {
        let points = [point(hoursAgo: 30 * 24, five: 0.1, seven: 0.9),
                      point(hoursAgo: 10 * 24, five: 0.2, seven: 0.5),
                      point(hoursAgo: 2 * 24, five: 0.3, seven: 0.6)]
        let five = UsageHistoryChart.series(points, window: .fiveHour, now: now, calendar: saoPaulo)
        let seven = UsageHistoryChart.series(points, window: .sevenDay, now: now, calendar: saoPaulo)
        #expect(five.points.map(\.fraction) == [0.3])
        #expect(seven.points.map(\.fraction) == [0.5, 0.6])
        #expect(five.from == now.addingTimeInterval(-7 * 86_400))
        #expect(seven.from == now.addingTimeInterval(-28 * 86_400))
    }

    /// Mais de 7h30 sem ponto quebra a linha; 7h30 exatas ainda ligam.
    @Test("buraco de mais de 7h30 quebra a linha")
    func buraco() {
        let start = 24.0
        let points = [point(hoursAgo: start, five: 0.1, seven: nil),
                      point(hoursAgo: start - 5, five: 0.2, seven: nil),
                      point(hoursAgo: start - 12.5, five: 0.3, seven: nil),
                      point(hoursAgo: start - 21, five: 0.4, seven: nil)]
        let five = UsageHistoryChart.series(points, window: .fiveHour, now: now, calendar: saoPaulo)
        #expect(five.segments.map { $0.map(\.fraction) } == [[0.1, 0.2, 0.3], [0.4]])
        #expect(five.latest?.fraction == 0.4)
    }

    /// Ausência não é zero.
    @Test("ponto sem a janela fica fora do gráfico")
    func semJanela() {
        let points = [point(hoursAgo: 1, five: nil, seven: 0.4)]
        #expect(UsageHistoryChart.series(points, window: .fiveHour, now: now, calendar: saoPaulo)
            .segments.isEmpty)
        #expect(UsageHistoryChart.series(points, window: .sevenDay, now: now, calendar: saoPaulo)
            .latest?.fraction == 0.4)
    }

    /// As marcas: a meia-noite LOCAL de cada dia, e de 7 em 7 dias a partir de
    /// hoje; os nomes dos dias são os da status line.
    @Test("as marcas do eixo do tempo")
    func marcas() {
        let five = UsageHistoryChart.series([], window: .fiveHour, now: now, calendar: saoPaulo)
        #expect(five.ticks.count == 7)
        #expect(five.ticks.map { StatusLineFormat.weekdayName($0, portuguese: true, calendar: saoPaulo) }
                == ["qui", "sex", "sáb", "dom", "seg", "ter", "qua"])
        #expect(five.ticks.map { StatusLineFormat.weekdayName($0, portuguese: false, calendar: saoPaulo) }
                == ["Thu", "Fri", "Sat", "Sun", "Mon", "Tue", "Wed"])
        let seven = UsageHistoryChart.series([], window: .sevenDay, now: now, calendar: saoPaulo)
        #expect(seven.ticks.count == 4)
        // 21/10 00:00 em UTC−3 = 03:00 UTC.
        #expect(seven.ticks.last == now.addingTimeInterval(-12 * 3600))
    }
}
