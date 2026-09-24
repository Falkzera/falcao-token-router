import Foundation
import UserNotifications
import CCUsageCore

/// Entrega um alerta ao usuário. Protocolo para o coordenador ser exercitável
/// sem disparar notificação de verdade.
@MainActor
protocol AlertPresenting {
    func present(_ alert: Alert)
}

/// Traduz `Alert` — que carrega fato, não frase — em notificação do sistema.
///
/// Toda a redação em português vive aqui, junto do resto das strings de usuário,
/// e não no `CCUsageCore`. É o que vai permitir localizar o app sem localizar o
/// core junto.
@MainActor
struct UserNotificationPresenter: AlertPresenting {
    func present(_ alert: Alert) {
        let content = UNMutableNotificationContent()
        (content.title, content.body) = Self.copy(for: alert)
        UNUserNotificationCenter.current().add(
            UNNotificationRequest(identifier: Self.identifier(for: alert),
                                  content: content,
                                  trigger: nil))
    }

    private static func copy(for alert: Alert) -> (title: String, body: String) {
        switch alert {
        case let .threshold(window, percent, resetsAt):
            switch window {
            case .session:
                (String(format: String(localized: "alerts.threshold.session.title.format"), percent),
                 sessionReset(resetsAt))
            case .weekly:
                (String(format: String(localized: "alerts.threshold.weekly.title.format"), percent),
                 weeklyReset(resetsAt))
            }
        case let .windowReset(window):
            switch window {
            case .session:
                (String(localized: "alerts.reset.session.title"),
                 String(localized: "alerts.reset.body"))
            case .weekly:
                (String(localized: "alerts.reset.weekly.title"),
                 String(localized: "alerts.reset.body"))
            }
        case .liveRequired:
            (String(localized: "alerts.liveRequired.title"),
             String(localized: "alerts.liveRequired.body"))
        }
    }

    private static func sessionReset(_ resetsAt: Date?) -> String {
        guard let resetsAt else { return "" }
        let remaining = max(0, resetsAt.timeIntervalSince(Date()))
        return String(format: String(localized: "alerts.session.body.format"),
                      Format.clockTime(resetsAt), Format.duration(remaining))
    }

    /// A semanal reseta daqui a dias, então hora sozinha não localiza nada.
    private static func weeklyReset(_ resetsAt: Date?) -> String {
        guard let resetsAt else { return "" }
        return String(format: String(localized: "alerts.weekly.body.format"),
                      resetsAt.formatted(date: .abbreviated, time: .shortened))
    }

    /// Identificador estável em vez de aleatório: se o mesmo alerta for
    /// entregue duas vezes, o sistema substitui em vez de empilhar. A política
    /// já impede a repetição — isto é a rede embaixo dela.
    private static func identifier(for alert: Alert) -> String {
        switch alert {
        case let .threshold(window, percent, resetsAt):
            "threshold-\(window)-\(percent)-\(Int(resetsAt?.timeIntervalSince1970 ?? 0))"
        case let .windowReset(window):
            "reset-\(window)"
        case .liveRequired:
            "live-required"
        }
    }
}
