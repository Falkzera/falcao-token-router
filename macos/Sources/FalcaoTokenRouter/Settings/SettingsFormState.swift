import Foundation
import Observation
import CCUsageCore

/// Estado de digitação da janela de Settings.
///
/// Existe porque `@State` é inatingível sem Xcode (macro do SwiftUI), então
/// estado de view precisa de um dono explícito. Fica fora de `AppSettings` de
/// propósito: isto é rascunho de formulário, não preferência — nada aqui é
/// persistido.
///
/// O rascunho também protege o denominador: enquanto o usuário digita
/// "214400000", os valores intermediários (2, 21, 214…) fariam o gauge saltar
/// se fossem aplicados a cada tecla. Só o commit atravessa.
@MainActor
@Observable
final class SettingsFormState {
    var useManualCeiling = false
    var ceilingDraft = ""

    /// Sincroniza com o que está gravado. Chamado ao abrir a janela.
    func load(from settings: AppSettings) {
        useManualCeiling = settings.manualBlockCeiling != nil
        ceilingDraft = settings.manualBlockCeiling.map(String.init) ?? ""
    }

    /// Aplica o rascunho. Vazio ou não numérico volta para automático em vez de
    /// gravar lixo como teto.
    func commit(to settings: AppSettings) {
        guard useManualCeiling else {
            settings.manualBlockCeiling = nil
            return
        }
        settings.manualBlockCeiling = UInt64(ceilingDraft.trimmingCharacters(in: .whitespaces))
    }
}
