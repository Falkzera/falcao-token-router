import SwiftUI
import CCUsageCore

/// Uma conta no Histórico (21/10/2026): quem é, quando foi medida (e quando
/// volta a ser, com a agenda ligada) e os dois gráficos, com o limiar do grupo.
struct HistoryCard: View {
    let account: Account
    let points: [UsageHistoryPoint]
    /// O limiar do grupo deste cartão, em porcentagem.
    let threshold: Double
    let scheduleOn: Bool
    /// A última tentativa da agenda nesta execução do app — entra no "próxima".
    let attempt: Date?
    /// Quem chegou pelo atalho da conta ganha um contorno que some sozinho.
    let highlighted: Bool

    var body: some View {
        let now = Date()
        VStack(alignment: .leading, spacing: 12) {
            VStack(alignment: .leading, spacing: 2) {
                HStack(alignment: .firstTextBaseline, spacing: 8) {
                    Text(verbatim: account.label).font(.callout.weight(.semibold))
                    Text(verbatim: account.identity.email)
                        .font(.caption).foregroundStyle(.secondary).lineLimit(1)
                }
                Text(verbatim: status(now: now)).font(.caption).foregroundStyle(.tertiary)
            }
            HistoryChart(title: String(localized: "home.history.chart.fiveHour"),
                         series: UsageHistoryChart.series(points, window: .fiveHour, now: now),
                         threshold: threshold, weekly: false)
            HistoryChart(title: String(localized: "home.history.chart.sevenDay"),
                         series: UsageHistoryChart.series(points, window: .sevenDay, now: now),
                         threshold: threshold, weekly: true)
        }
        .padding(14)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(.quaternary.opacity(0.4), in: .rect(cornerRadius: 12))
        .overlay {
            RoundedRectangle(cornerRadius: 12)
                .strokeBorder(Color.accentColor, lineWidth: 2)
                .opacity(highlighted ? 1 : 0)
                .animation(.easeOut(duration: 0.6), value: highlighted)
        }
    }

    /// "medida há 2h 5m · próxima às 19:05". Vencida, ou a menos de uma volta
    /// da agenda (ela olha a cada 3 min), "em instantes": senão uma conta
    /// recém-zerada diria "próxima às" a hora de agora.
    private func status(now: Date) -> String {
        let last = points.last?.sampledAt
        let measured = last.map {
            String(format: String(localized: "home.history.account.measured.format"),
                   Format.duration(now.timeIntervalSince($0)))
        } ?? String(localized: "home.history.account.never")
        let next: String
        if !scheduleOn {
            next = String(localized: "home.history.account.scheduleOff")
        } else if let at = MeasureSchedule.nextAt(lastMeasured: last, lastAttempt: attempt),
                  at > now.addingTimeInterval(180) {
            next = String(format: String(localized: "home.history.account.next.format"),
                          StatusLineFormat.resetWhen(at, withDay: false))
        } else {
            next = String(localized: "home.history.account.nextSoon")
        }
        return "\(measured) · \(next)"
    }
}
