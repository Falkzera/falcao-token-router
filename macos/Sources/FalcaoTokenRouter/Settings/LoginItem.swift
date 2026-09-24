import Foundation
import Observation
import ServiceManagement

/// Registro de "abrir no login" via `SMAppService`.
///
/// O estado exibido é **lido do sistema**, nunca de uma preferência nossa. Um
/// app assinado ad-hoc pode ter o registro recusado, e um checkbox marcado
/// sobre um registro que falhou é pior que nenhum checkbox: o usuário acha que
/// resolveu e descobre no próximo reboot.
@MainActor
@Observable
final class LoginItem {
    private(set) var isEnabled: Bool
    /// Preenchido quando o sistema recusa o registro — a UI mostra o motivo.
    private(set) var failure: String?

    init() {
        isEnabled = SMAppService.mainApp.status == .enabled
    }

    /// Relê o estado do sistema. A janela chama isto ao abrir, porque o registro
    /// pode ter sido revogado pelo usuário em Ajustes do Sistema.
    func refresh() {
        isEnabled = SMAppService.mainApp.status == .enabled
        if isEnabled { failure = nil }
    }

    func setEnabled(_ enabled: Bool) {
        do {
            if enabled {
                try SMAppService.mainApp.register()
            } else {
                try SMAppService.mainApp.unregister()
            }
            failure = nil
        } catch {
            failure = error.localizedDescription
        }
        // Confirma contra o sistema em vez de assumir que deu certo.
        isEnabled = SMAppService.mainApp.status == .enabled
    }

    /// Texto explicativo quando o sistema recusou o registro.
    var explanation: String? {
        guard let failure else { return nil }
        return String(format: String(localized: "settings.system.loginItem.failure.format"),
                      "\(failure)")
    }
}
