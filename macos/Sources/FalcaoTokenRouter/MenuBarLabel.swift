import AppKit
import SwiftUI
import CCUsageCore

/// O que fica sempre visível: ícone, % do bloco de 5h e **qual conta**.
///
/// O nome não é enfeite: com várias contas rotacionando, um número sozinho não
/// diz de quem ele é — e nem o `/status` do Claude Code responde isso para as
/// contas que não estão servindo a sessão de agora.
struct MenuBarLabel: View {
    let snapshot: UsageSnapshot
    /// Os grupos do usuário. A barra mostra a conta ativa do grupo padrão; sem
    /// grupo nenhum, cai no medidor do perfil local.
    let router: RouterConfigStore

    /// A conta e a fração que a barra mostra, do mundo novo quando há grupos.
    private var headline: (name: String, fraction: Double?)? {
        // Grupo padrão primeiro; se não houver, o primeiro grupo com uma ativa.
        let groups = router.config.groups
        guard !groups.isEmpty else { return nil }
        let preferred = router.config.defaultGroup ?? groups.first
        for group in [preferred].compactMap({ $0 }) + groups {
            if let accountID = router.activeByGroup[group.id],
               let account = router.config.account(accountID) {
                return (tiny(account.label), router.usageSnapshot[accountID])
            }
        }
        return nil
    }

    private var fraction: Double? {
        headline?.fraction ?? snapshot.session.fraction
    }

    private var tint: Color { UsageColor.menuBar(fraction) }

    var body: some View {
        HStack(spacing: 4) {
            Image(nsImage: GaugeMark.menuBarImage(fraction: fraction ?? snapshot.session.fraction))
                .renderingMode(.template)
            Text(Format.percent(fraction ?? snapshot.session.fraction))
                .monospacedDigit()
            if let name = headline?.name {
                Text(verbatim: name)
                    .font(.caption2)
                    .lineLimit(1)
            }
        }
        .foregroundStyle(tint)
    }

    /// Nome mínimo para a barra: mantém os dígitos finais, que distinguem contas
    /// da mesma família (`equipe-4` → `equ4`).
    private func tiny(_ name: String) -> String {
        let digits = String(name.reversed().prefix(while: \.isNumber).reversed())
        let letters = name.dropLast(digits.count)
        return digits.isEmpty ? String(name.prefix(5)) : String(letters.prefix(3)) + digits
    }
}
