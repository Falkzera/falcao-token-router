import Charts
import SwiftUI
import CCUsageCore

/// Um gráfico de linha do Histórico (21/10/2026): uma série — a fração de uma
/// janela ao longo do tempo —, eixo vertical FIXO de 0 a 100% (as contas se
/// comparam de relance) e o limiar do grupo tracejado (é com ele que a rotação
/// compara). Swift Charts: vem com o sistema, nenhuma dependência nova.
///
/// Linha de 2 pt, bolinha só no último ponto, no ponto isolado (trecho de um
/// ponto só, que linha nenhuma desenharia) e no que o ponteiro escolher; cada
/// trecho é uma série própria, então o buraco quebra a linha em vez de ligar o
/// que não foi medido.
struct HistoryChart: View {
    let title: String
    let series: UsageHistoryChart.Series
    /// O limiar do grupo, em porcentagem (90 = 90%).
    let threshold: Double
    /// A semanal escreve o reset com o dia, e as marcas do eixo com a data.
    let weekly: Bool

    @Environment(\.colorScheme) private var colorScheme
    @ViewState private var selected: Date?

    private var portuguese: Bool { Locale.current.language.languageCode?.identifier == "pt" }
    private var line: Color { UsageColor.chartLine(colorScheme) }
    private var latestText: String {
        series.latest.map { UsagePercent.text($0.fraction) }
            ?? String(localized: "panel.accounts.window.absent")
    }

    /// O ponto mais perto no tempo de onde o ponteiro está: ele não precisa
    /// acertar a linha.
    private var shown: UsageHistoryChart.Point? {
        guard let selected else { return nil }
        return series.points.min {
            abs($0.at.timeIntervalSince(selected)) < abs($1.at.timeIntervalSince(selected))
        }
    }

    /// Onde vai bolinha: o ponto isolado e o último.
    private var dots: [UsageHistoryChart.Point] {
        var dots = series.segments.filter { $0.count == 1 }.flatMap { $0 }
        if let latest = series.latest, !dots.contains(latest) { dots.append(latest) }
        return dots
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 4) {
            HStack {
                Text(verbatim: title).font(.caption).foregroundStyle(.secondary)
                Spacer()
                Text(verbatim: latestText).font(.caption.weight(.semibold).monospacedDigit())
            }
            chart
                .frame(height: 110)
                .overlay {
                    if series.points.isEmpty {
                        Text("home.history.chart.empty").font(.caption).foregroundStyle(.tertiary)
                    }
                }
                .accessibilityLabel(Text(verbatim: String(
                    format: String(localized: "home.history.chart.label.format"), title, latestText)))
        }
    }

    private var chart: some View {
        Chart {
            thresholdMark
            lineMarks
            selectionMarks
        }
        .chartYScale(domain: 0...1)
        .chartXScale(domain: series.from...series.to)
        .chartYAxis { yAxis }
        .chartXAxis { xAxis }
        .chartXSelection(value: $selected)
    }

    private var thresholdMark: some ChartContent {
        RuleMark(y: .value("limiar", threshold / 100))
            .foregroundStyle(UsageColor.critical.opacity(0.75))
            .lineStyle(StrokeStyle(lineWidth: 1, dash: [4, 3]))
            .annotation(position: .top, alignment: .trailing) {
                Text(verbatim: String(format: String(localized: "home.history.chart.threshold.format"),
                                      "\(Int(threshold))%"))
                    .font(.system(size: 9))
                    .foregroundStyle(.secondary)
            }
    }

    // Os nomes das dimensões são o que o VoiceOver lê de cada ponto: vêm do
    // catálogo, senão o app em inglês fala português.
    private static let axisWhen = String(localized: "home.history.chart.axis.when")
    private static let axisUsage = String(localized: "home.history.chart.axis.usage")
    private static let axisSegment = String(localized: "home.history.chart.axis.segment")

    @ChartContentBuilder
    private var lineMarks: some ChartContent {
        ForEach(Array(series.segments.enumerated()), id: \.offset) { index, segment in
            ForEach(segment) { point in
                LineMark(x: .value(Self.axisWhen, point.at), y: .value(Self.axisUsage, point.fraction),
                         series: .value(Self.axisSegment, index))
                    .foregroundStyle(line)
                    .lineStyle(StrokeStyle(lineWidth: 2, lineCap: .round, lineJoin: .round))
            }
        }
        ForEach(dots) { point in
            PointMark(x: .value(Self.axisWhen, point.at), y: .value(Self.axisUsage, point.fraction))
                .foregroundStyle(line)
                .symbolSize(36)
        }
    }

    @ChartContentBuilder
    private var selectionMarks: some ChartContent {
        if let shown {
            RuleMark(x: .value(Self.axisWhen, shown.at))
                .foregroundStyle(Color.secondary.opacity(0.4))
            PointMark(x: .value(Self.axisWhen, shown.at), y: .value(Self.axisUsage, shown.fraction))
                .foregroundStyle(line)
                .symbolSize(70)
                .annotation(position: .top, spacing: 4,
                            overflowResolution: .init(x: .fit(to: .chart), y: .disabled)) {
                    tip(shown)
                }
        }
    }

    private var yAxis: some AxisContent {
        AxisMarks(position: .leading, values: [0.0, 0.5, 1.0]) { value in
            AxisGridLine()
            AxisValueLabel {
                if let fraction = value.as(Double.self) {
                    Text(verbatim: UsagePercent.text(fraction))
                }
            }
        }
    }

    private var xAxis: some AxisContent {
        AxisMarks(values: series.ticks) { value in
            AxisGridLine()
            AxisValueLabel(anchor: .topLeading) {
                if let date = value.as(Date.self) {
                    Text(verbatim: tickLabel(date))
                }
            }
        }
    }

    /// A dica do ponto, como a do Windows: o valor primeiro, depois quando foi
    /// medido e quando aquela janela resetava — as horas escritas como a status
    /// line escreve.
    private func tip(_ point: UsageHistoryChart.Point) -> some View {
        VStack(alignment: .leading, spacing: 1) {
            Text(verbatim: UsagePercent.text(point.fraction))
                .font(.callout.weight(.semibold))
            Text(verbatim: StatusLineFormat.resetWhen(point.at, withDay: true))
                .font(.caption2)
                .foregroundStyle(.secondary)
            if let reset = point.resetsAt {
                Text(verbatim: String(format: String(localized: "panel.reset.format"),
                                      StatusLineFormat.resetWhen(reset, withDay: weekly)))
                    .font(.caption2)
                    .foregroundStyle(.secondary)
            }
        }
        .padding(6)
        .background(.regularMaterial, in: .rect(cornerRadius: 6))
    }

    /// 5h: o dia da semana, com os nomes da status line. Semanal: a data.
    private func tickLabel(_ date: Date) -> String {
        guard weekly else { return StatusLineFormat.weekdayName(date, portuguese: portuguese) }
        let day = Calendar.current.component(.day, from: date)
        let month = Calendar.current.component(.month, from: date)
        return portuguese
            ? String(format: "%02d/%02d", day, month)
            : date.formatted(.dateTime.month(.abbreviated).day())
    }
}
