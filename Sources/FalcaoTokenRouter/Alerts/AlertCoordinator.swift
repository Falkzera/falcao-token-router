import Foundation
import Observation
import UserNotifications
import CCUsageCore

/// Liga o store à política e a política à entrega.
///
/// Mora no alvo de UI porque fala com o `UNUserNotificationCenter`. O
/// `UsageStore` continua sem saber que notificação existe: ele publica o
/// snapshot, e quem se interessa escuta.
@MainActor
@Observable
final class AlertCoordinator {
    /// `true` quando o usuário negou a permissão. Uma chave ligada sobre
    /// permissão negada é uma chave que mente, então os Ajustes mostram isto.
    private(set) var isDenied = false

    @ObservationIgnored private var policy = AlertPolicy()
    @ObservationIgnored private let settings: AppSettings
    @ObservationIgnored private let presenter: any AlertPresenting

    init(settings: AppSettings, presenter: any AlertPresenting = UserNotificationPresenter()) {
        self.settings = settings
        self.presenter = presenter
    }

    func handle(_ snapshot: UsageSnapshot) {
        for alert in policy.evaluate(snapshot,
                                     preferences: settings.alerts,
                                     liveEnabled: settings.liveUsageEnabled) {
            presenter.present(alert)
        }
    }

    /// Pede a permissão no momento em que o usuário liga os alertas — não no
    /// lançamento. Um app sem janela que pede permissão ao subir pede sem
    /// contexto nenhum, e a negativa é permanente.
    func alertsEnabledChanged(to enabled: Bool) {
        guard enabled else { return }
        Task { @MainActor in
            _ = try? await UNUserNotificationCenter.current().requestAuthorization(options: [.alert])
            await refreshAuthorization()
        }
    }

    func refreshAuthorization() async {
        isDenied = await UNUserNotificationCenter.current()
            .notificationSettings().authorizationStatus == .denied
    }
}
