import SwiftUI
import CCUsageCore

/// A linha de baixo: ativa a integração de shell (um clique edita o `~/.zshrc`)
/// para `claude <grupo>` funcionar — sem o usuário mexer em arquivo.
struct TerminalIntegrationRow: View {
    @Bindable var store: RouterConfigStore
    /// Feedback do clique em "Reinstalar": o estado da linha já é "pronto" antes
    /// e depois, então sem este flash o clique não teria resposta visível.
    @ViewState private var justInstalled = false

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            HStack {
                Label("groups.terminal.title", systemImage: "terminal")
                    .font(.caption.weight(.semibold))
                Spacer()
                actionButton
            }

            if store.integrationInstalled {
                // Plug-and-play: já editamos o ~/.zshrc. Só falta um terminal novo.
                Label("groups.terminal.ready", systemImage: "checkmark.circle.fill")
                    .font(.caption2).foregroundStyle(UsageColor.calm)
            } else {
                Text("groups.terminal.pitch")
                    .font(.caption2).foregroundStyle(.secondary)
            }
        }
        .padding(.horizontal, 16).padding(.vertical, 10)
    }

    private func install() {
        _ = store.installShellIntegration()
        justInstalled = true
        Task { try? await Task.sleep(for: .seconds(2)); justInstalled = false }
    }

    @ViewBuilder private var actionButton: some View {
        if justInstalled {
            Label("groups.terminal.reinstalled", systemImage: "checkmark")
                .font(.caption).foregroundStyle(UsageColor.calm)
        } else if store.integrationInstalled {
            Button("groups.terminal.reinstall") { install() }
                .controlSize(.small)
        } else {
            Button("groups.terminal.activate") { install() }
                .controlSize(.small).buttonStyle(.borderedProminent)
        }
    }
}
