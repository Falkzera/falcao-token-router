import Foundation

/// Decide o que notificar, a partir da sequência de snapshots.
///
/// Pura e determinística: mesma sequência de entradas, mesma saída. Não lê
/// relógio, não fala com o sistema de notificação, e por isso o comportamento
/// inteiro — inclusive rearme, anti-repetição e mudança de configuração no meio
/// de uma janela — cabe em teste.
public struct AlertPolicy {
    /// O que já aconteceu numa instância de janela. A instância é identificada
    /// pelo `resetsAt`: mudou, é outra janela.
    private struct WindowState {
        let resetsAt: Date
        var firedThresholds: Set<Int>
        /// A fração da avaliação anterior. É o que separa "cruzou agora" de
        /// "já estava acima" — distinção sem a qual ligar um alerta no meio de
        /// uma janela despejaria os cruzamentos que ninguém observou.
        var lastPercent: Double
    }

    private var states: [Alert.Window: WindowState] = [:]
    private var liveRequiredEmitted = false

    public init() {}

    /// Consome um snapshot e devolve o que emitir.
    ///
    /// Primeira condição que casar vence, de cima para baixo — mesmo padrão do
    /// `UsageSourcePolicy`.
    public mutating func evaluate(_ snapshot: UsageSnapshot,
                                  preferences: AlertPreferences,
                                  liveEnabled: Bool) -> [Alert] {
        guard preferences.anyEnabled else { return [] }

        // Sem busca ao vivo o app lê o cache do Claude Code, que já foi medido
        // 13h25 defasado. Alertar sobre aquilo erra nos dois sentidos, e o pior
        // é o silêncio enquanto o usuário estoura.
        guard liveEnabled else {
            defer { liveRequiredEmitted = true }
            return liveRequiredEmitted ? [] : [.liveRequired]
        }

        return Alert.Window.allCases
            .filter { preferences.windows.contains($0) }
            .flatMap { window in
                evaluate(window: window,
                         gauge: gauge(for: window, in: snapshot),
                         preferences: preferences)
            }
    }

    private func gauge(for window: Alert.Window,
                       in snapshot: UsageSnapshot) -> UsageSnapshot.Gauge? {
        switch window {
        case .session: snapshot.session
        case .weekly: snapshot.weekly
        }
    }

    private mutating func evaluate(window: Alert.Window,
                                   gauge: UsageSnapshot.Gauge?,
                                   preferences: AlertPreferences) -> [Alert] {
        // A procedência é verificada **no medidor**, não no status do snapshot:
        // um snapshot `.live` pode conter medidor derivado, quando o payload
        // veio sem a janela de 5h. Checar o status deixaria passar alerta sobre
        // número estimado sob a etiqueta de ao vivo.
        guard let gauge, case .live = gauge.provenance, let resetsAt = gauge.resetsAt else {
            return []
        }

        let percent = gauge.fraction * 100
        guard var state = states[window], state.resetsAt == resetsAt else {
            return openWindow(window, resetsAt: resetsAt, at: percent,
                              preferences: preferences)
        }

        // Cruzou agora é passar de baixo para cima entre duas avaliações. Estar
        // acima sem ter cruzado — porque o alerta acabou de ser ligado, ou o
        // limiar acabou de ser acrescentado — é registrado como resolvido e
        // segue calado: o app não viu aquilo acontecer.
        let eligible = preferences.thresholds.filter { !state.firedThresholds.contains($0)
                                                       && percent >= Double($0) }
        let crossedNow = eligible.filter { state.lastPercent < Double($0) }

        state.firedThresholds.formUnion(eligible)
        state.lastPercent = percent
        states[window] = state

        // Só o mais alto vira alerta: dois banners pelo mesmo salto é a
        // sensação de spam que esta feature existe para não produzir, e o menor
        // já nasceria como notícia velha.
        guard preferences.thresholdsEnabled, let highest = crossedNow.max() else { return [] }
        return [.threshold(window: window, percent: highest, resetsAt: resetsAt)]
    }

    /// Abre uma instância de janela, estabelecendo a linha de base.
    ///
    /// Nenhum limiar dispara aqui: o app não observou o consumo chegar até onde
    /// está — ele começou a olhar agora.
    ///
    /// É também o que dispensa persistir estado: reiniciando no meio de uma
    /// janela, esta linha reconstrói o estado correto a partir do próprio dado.
    private mutating func openWindow(_ window: Alert.Window,
                                     resetsAt: Date,
                                     at percent: Double,
                                     preferences: AlertPreferences) -> [Alert] {
        let hadPreviousWindow = states[window] != nil
        states[window] = WindowState(
            resetsAt: resetsAt,
            firedThresholds: Set(preferences.thresholds.filter { percent >= Double($0) }),
            lastPercent: percent)

        // Saber que a capacidade voltou vale por si — é quando dá para retomar
        // trabalho pesado. Naturalmente contido: parando de usar o Claude Code,
        // o medidor cai para derivado e a checagem de procedência acima já
        // barra, então o aviso acontece em dia de trabalho.
        guard preferences.resetEnabled, hadPreviousWindow else { return [] }
        return [.windowReset(window: window)]
    }
}
