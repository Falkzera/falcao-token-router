import Foundation

public enum SnapshotBuilder {
    /// Função pura: mesmos eventos + mesmo `now` = mesmo snapshot.
    /// É o que torna toda a lógica de apresentação testável sem UI.
    public static func build(
        from events: [UsageEvent],
        now: Date,
        calendar: Calendar = .current,
        override: Ceilings?,
        official: OfficialSource? = nil,
        status: UsageSourceStatus = .derivedOnly
    ) -> UsageSnapshot {
        // Sem eventos E sem oficial não há o que montar. Com oficial, há: os
        // números da conta não dependem de existir JSONL local, e descartá-los
        // por falta de histórico deixaria o painel vazio com o dado na mão.
        guard !events.isEmpty || official != nil else { return .empty(at: now) }

        let blocks = BlockBuilder.blocks(from: events, calendar: calendar)
        let ceilings = CeilingCalibrator.calibrate(
            blocks: blocks, now: now, override: override)
        // Sem o override: é a referência que a janela de Ajustes mostra enquanto
        // o usuário digita o dele, e devolver o próprio override não informaria
        // nada. `calibrate` aplica o piso, então isto nunca é zero.
        let calibrated = CeilingCalibrator.calibrate(
            blocks: blocks, now: now, override: nil).blockTokens

        let aggregator = PeriodAggregator(calendar: calendar)
        let today = aggregator.totals(from: events, in: aggregator.todayInterval(now: now))
        let week = aggregator.totals(from: events, in: aggregator.weekInterval(now: now))
        let month = aggregator.totals(from: events, in: aggregator.monthInterval(now: now))
        let rolling = aggregator.totals(from: events, in: aggregator.rolling7Days(now: now))

        let active = blocks.last { $0.isActive(at: now) }

        // O número oficial vence sempre que existe: ele traz a fase real da
        // janela, que a derivação não recupera — o floor de hora perde até 59
        // minutos por bloco e o erro acumula ao longo da cadeia.
        //
        // Sem oficial, o medidor derivado vem zerado em vez de ausente quando
        // não há bloco ativo: acabou de resetar é justamente quando "0% de
        // quanto" informa mais.
        let sessionGauge: UsageSnapshot.Gauge
        if let official, let session = official.report.session {
            sessionGauge = .official(session, from: official)
        } else {
            sessionGauge = .derived(tokens: active?.tokens ?? 0,
                                    ceiling: ceilings.blockTokens,
                                    resetsAt: active?.end)
        }

        // A janela semanal da Anthropic tem reset próprio e não é derivável do
        // histórico local; sem oficial, a UI cai no múltiplo do ritmo típico.
        let weeklyGauge = official.flatMap { source in
            source.report.weeklyAll.map { UsageSnapshot.Gauge.official($0, from: source) }
        }

        // Janela sem nome de modelo é descartada: uma barra anônima não diz de
        // que é o teto que ela mede.
        let scoped = official.map { source in
            source.report.weeklyScoped.compactMap { limit in
                limit.modelName.map {
                    UsageSnapshot.ScopedGauge(modelName: $0,
                                              gauge: .official(limit, from: source))
                }
            }
        } ?? []

        let burnRate: Double? = active.flatMap { block in
            let elapsedMinutes = now.timeIntervalSince(block.start) / 60
            guard elapsedMinutes > 0 else { return nil }
            return Double(block.tokens) / elapsedMinutes
        }

        var unknown = Set<String>()
        for event in events {
            if let name = event.model.unknownName { unknown.insert(name) }
        }

        return UsageSnapshot(
            session: sessionGauge,
            weekly: weeklyGauge,
            scopedWeekly: scoped,
            weeklyPace: Pace(
                tokens: rolling.tokens,
                typical: CeilingCalibrator.typicalWeek(events: events, now: now)),
            today: today, week: week, month: month,
            burnRatePerMinute: burnRate,
            unknownModels: unknown,
            generatedAt: now,
            calibratedBlockCeiling: calibrated,
            sourceStatus: status)
    }
}
