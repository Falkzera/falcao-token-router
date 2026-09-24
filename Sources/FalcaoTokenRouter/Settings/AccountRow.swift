import SwiftUI
import CCUsageCore

// MARK: - Uma conta na lista

struct AccountRow: View {
    let account: Account
    let isActive: Bool
    /// O uso COM a janela de onde veio — mesmo motivo do painel: um número sem
    /// legenda é lido como o das 5 horas, e quase sempre é o semanal.
    let usage: AccountUsage?
    let sampledAt: Date?
    let onSwitch: () -> Void
    let onRelogin: () -> Void
    let onRemove: () -> Void

    private var isVeryStale: Bool {
        sampledAt.map { Date().timeIntervalSince($0) > UsageAge.veryStale } ?? false
    }

    private var usageHelp: String { AccountHelp.text(for: usage) }

    private var staleHelp: String {
        String(format: String(localized: usage?.origin == .probe
                              ? "panel.accounts.stale.help.probe.format"
                              : "panel.accounts.stale.help.sensor.format"),
               Format.duration(Date().timeIntervalSince(sampledAt ?? Date())))
    }

    var body: some View {
        HStack(spacing: 8) {
            Image(systemName: "line.3.horizontal")
                .font(.caption2).foregroundStyle(.tertiary)

            Circle()
                .fill(isActive ? UsageColor.bar(usage?.fraction ?? 0) : .clear)
                .strokeBorder(isActive ? .clear : Color.secondary.opacity(0.35), lineWidth: 1)
                .frame(width: 7, height: 7)

            VStack(alignment: .leading, spacing: 1) {
                Text(verbatim: account.label)
                    .font(.callout).fontWeight(isActive ? .semibold : .regular)
                if let org = account.identity.organizationName, !org.isEmpty {
                    Text(verbatim: org).font(.caption2).foregroundStyle(.secondary)
                }
            }

            Spacer(minLength: 8)

            if case .model = usage?.window, let modelo = usage?.model {
                ModelBadge(window: modelo)
            }
            if isVeryStale {
                Image(systemName: "clock.badge.exclamationmark")
                    .font(.caption2).foregroundStyle(.tertiary)
                    .help(staleHelp)
            }
            if let usage {
                WindowReading(label: "panel.accounts.window.fiveHour",
                              fraction: usage.fiveHour,
                              isBound: usage.window == .fiveHour,
                              isStale: false)
                    .help(usageHelp)
                WindowReading(label: "panel.accounts.window.sevenDay",
                              fraction: usage.sevenDay,
                              isBound: usage.window == .sevenDay,
                              isStale: false)
                    .help(usageHelp)
            } else {
                Text("groups.account.unmeasured")
                    .font(.caption2).foregroundStyle(.tertiary)
                    .frame(width: 38, alignment: .trailing)
                    .help(AccountHelp.text(for: nil))
            }

            if !isActive {
                Button("groups.account.use", action: onSwitch)
                    .controlSize(.small).buttonStyle(.bordered)
            }
            // Ações raras ficam atrás do menu; "usar" fica exposto porque é a
            // ação frequente.
            Menu {
                Button("groups.account.relogin", action: onRelogin)
                Button("groups.account.remove", role: .destructive, action: onRemove)
            } label: { Image(systemName: "ellipsis.circle") }
                .menuStyle(.borderlessButton).fixedSize()
                .help(String(localized: "groups.account.menu.help"))
        }
        .padding(.vertical, 5)
    }
}
