import Foundation

/// O que os gráficos do Histórico desenham (21/10/2026), puro e testável: as
/// séries recortadas no período, quebradas nos buracos, e as marcas do eixo.
/// A view só converte tempo em x e fração em y. Mesmas regras do porte
/// Windows (`history.rs`).
public enum UsageHistoryChart {
    public enum Window: Sendable { case fiveHour, sevenDay }

    public struct Point: Sendable, Equatable, Identifiable {
        public let at: Date
        public let fraction: Double
        public let resetsAt: Date?
        public var id: Date { at }
    }

    public struct Series: Sendable, Equatable {
        public let from: Date
        public let to: Date
        /// As meias-noites locais onde o eixo do tempo tem marca.
        public let ticks: [Date]
        /// Os trechos da linha, do mais velho ao mais novo.
        public let segments: [[Point]]
        public var points: [Point] { segments.flatMap { $0 } }
        /// O ponto mais novo do período — o número no título do gráfico.
        public var latest: Point? { segments.last?.last }
    }

    /// Quantos dias cada gráfico mostra, contados de agora para trás.
    public static let fiveHourDays = 7
    public static let sevenDayDays = 28
    /// Um buraco maior que isto entre dois pontos quebra a linha: 1,5 × o
    /// intervalo da agenda (7h30). App fechado, Mac dormindo — o gráfico não
    /// desenha o que não foi medido.
    public static let maxGap: TimeInterval = MeasureSchedule.every * 1.5

    /// A série de uma janela. Ponto sem aquela janela fica de fora — ausência
    /// não é zero.
    public static func series(_ points: [UsageHistoryPoint], window: Window, now: Date,
                              calendar: Calendar = .current) -> Series {
        let days = window == .fiveHour ? fiveHourDays : sevenDayDays
        let from = now.addingTimeInterval(-TimeInterval(days) * 86_400)
        var segments: [[Point]] = []
        var previous: Date?
        for p in points where p.sampledAt >= from && p.sampledAt <= now {
            let fraction = window == .fiveHour ? p.fiveHourPercent : p.sevenDayPercent
            guard let fraction else { continue }
            let resetsAt = window == .fiveHour ? p.fiveHourResetsAt : p.sevenDayResetsAt
            let drawn = Point(at: p.sampledAt, fraction: fraction, resetsAt: resetsAt)
            if let previous, p.sampledAt.timeIntervalSince(previous) <= maxGap, !segments.isEmpty {
                segments[segments.count - 1].append(drawn)
            } else {
                segments.append([drawn])
            }
            previous = p.sampledAt
        }
        return Series(from: from, to: now,
                      ticks: ticks(window: window, from: from, now: now, calendar: calendar),
                      segments: segments)
    }

    /// A meia-noite de cada dia (5h) ou de 7 em 7 dias contando de hoje
    /// (semanal) — só as que caem dentro do período, da mais velha à mais nova.
    public static func ticks(window: Window, from: Date, now: Date,
                             calendar: Calendar = .current) -> [Date] {
        let step = window == .fiveHour ? 1 : 7
        let days = window == .fiveHour ? fiveHourDays : sevenDayDays
        let today = calendar.startOfDay(for: now)
        let marks = stride(from: 0, through: days, by: step).compactMap {
            calendar.date(byAdding: .day, value: -$0, to: today)
        }
        return Array(marks.filter { $0 > from && $0 <= now }.reversed())
    }
}
