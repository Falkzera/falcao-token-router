import SwiftUI
import CCUsageCore

// MARK: - Cartão de um grupo

struct GroupCard: View {
    @Bindable var store: RouterConfigStore
    let group: AccountGroup
    let onAddAccount: () -> Void
    let onRelogin: (Account) -> Void

    @ViewState private var editingName = false
    @ViewState private var draftName = ""
    @ViewState private var pendingDelete: Account?
    @ViewState private var confirmingGroupDelete = false
    @ViewState private var commandCopied = false

    /// O comando exato para abrir este grupo no terminal. Nome com espaço vai
    /// entre aspas; a função de shell casa sem diferenciar caixa.
    private var command: String {
        let n = group.name.trimmingCharacters(in: .whitespaces)
        return n.contains(" ") ? "claude \"\(n)\"" : "claude \(n.lowercased())"
    }

    private var accounts: [Account] { store.config.accounts(in: group) }
    /// Da fonte observável, não relendo o disco a cada frame.
    private var active: Account? {
        store.activeByGroup[group.id].flatMap { id in store.config.account(id) }
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 10) {
            titleRow
            commandRow
            controlsRow
            Divider()
            if accounts.isEmpty {
                Text("groups.accounts.empty")
                    .font(.caption).foregroundStyle(.secondary)
                    .padding(.vertical, 4)
            } else {
                accountList
            }
            HStack {
                Button {
                    onAddAccount()
                } label: {
                    Label("groups.account.add", systemImage: "person.badge.plus")
                        .font(.caption)
                }
                .buttonStyle(.link)
                Spacer()
                measureButton
            }
        }
        .padding(14)
        .background(.quaternary.opacity(0.4), in: .rect(cornerRadius: 12))
        .confirmationDialog(
            "groups.account.delete.title",
            isPresented: Binding(get: { pendingDelete != nil },
                                 set: { if !$0 { pendingDelete = nil } }),
            presenting: pendingDelete
        ) { account in
            Button("groups.account.delete.confirm", role: .destructive) {
                store.removeAccount(account.id); pendingDelete = nil
            }
            Button("groups.cancel", role: .cancel) { pendingDelete = nil }
        } message: { account in
            Text(String(format: String(localized: "groups.account.delete.message.format"),
                        account.label))
        }
        .confirmationDialog("groups.delete.confirm.title",
                            isPresented: $confirmingGroupDelete) {
            Button("groups.delete.confirm.button", role: .destructive) {
                store.removeGroup(group.id)
            }
            Button("groups.cancel", role: .cancel) {}
        } message: {
            Text(String(format: String(localized: "groups.delete.confirm.message.format"),
                        group.name, accounts.count))
        }
    }

    private var titleRow: some View {
        HStack(spacing: 8) {
            if editingName {
                TextField("groups.name.placeholder", text: $draftName)
                    .textFieldStyle(.roundedBorder)
                    .onSubmit { commitName() }
                Button("groups.name.save") { commitName() }.controlSize(.small)
            } else {
                Text(verbatim: group.name).font(.headline)
                SessionsBadge(sessions: store.liveSessions[group.id] ?? [])
                if group.configDir.isDefault {
                    Text("groups.default.badge")
                        .font(.caption2).padding(.horizontal, 6).padding(.vertical, 2)
                        .background(UsageColor.calm.opacity(0.25), in: .capsule)
                }
                Button {
                    draftName = group.name; editingName = true
                } label: { Image(systemName: "pencil") }
                    .buttonStyle(.borderless).controlSize(.small)
                Spacer()
                Menu {
                    if group.configDir.isDefault {
                        Button("groups.clearDefault") { store.clearDefault() }
                    } else {
                        Button("groups.makeDefault") { store.makeDefault(group.id) }
                    }
                    Button("groups.delete", role: .destructive) {
                        confirmingGroupDelete = true
                    }
                } label: { Image(systemName: "ellipsis.circle") }
                    .menuStyle(.borderlessButton).fixedSize()
            }
        }
    }

    /// Como abrir este grupo no terminal — a resposta para "e agora, como uso?".
    private var commandRow: some View {
        HStack(spacing: 6) {
            Image(systemName: "terminal").font(.caption2).foregroundStyle(.tertiary)
            Text(verbatim: command)
                .font(.caption.monospaced())
                .textSelection(.enabled)
            Button {
                NSPasteboard.general.clearContents()
                NSPasteboard.general.setString(command, forType: .string)
                commandCopied = true
                Task { try? await Task.sleep(for: .seconds(2)); commandCopied = false }
            } label: {
                Image(systemName: commandCopied ? "checkmark" : "doc.on.doc").font(.caption2)
            }
            .buttonStyle(.borderless)
            .help(String(localized: "groups.command.copy.help"))
            if !store.integrationInstalled {
                Text("groups.command.needsInstall")
                    .font(.caption2).foregroundStyle(UsageColor.warning)
            }
            Spacer()
        }
        .padding(.horizontal, 8).padding(.vertical, 5)
        .background(.quaternary.opacity(0.35), in: .rect(cornerRadius: 6))
    }

    private var controlsRow: some View {
        HStack(spacing: 14) {
            Toggle("groups.autoRotate", isOn: Binding(
                get: { group.autoRotate },
                set: { store.setAutoRotate(group.id, $0) }))
                .toggleStyle(.switch).controlSize(.small)
            Spacer()
            Text("groups.threshold.label").font(.caption).foregroundStyle(.secondary)
            Slider(value: Binding(
                get: { group.thresholdPercent },
                set: { store.setThreshold(group.id, percent: $0) }),
                in: 50...100, step: 5)
                .frame(width: 120)
            Text(verbatim: "\(Int(group.thresholdPercent))%")
                .font(.caption.monospacedDigit()).frame(width: 34, alignment: .trailing)
        }
    }

    /// Lista reordenável: a ordem é a preferência de rotação, arrastada à mão.
    private var accountList: some View {
        VStack(spacing: 0) {
            ForEach(accounts) { account in
                AccountRow(account: account,
                           isActive: account.id == active?.id,
                           usage: store.usageDetail[account.id],
                           sampledAt: store.usageSampledAt[account.id],
                           onSwitch: { store.activate(account, in: group) },
                           onRelogin: { onRelogin(account) },
                           onRemove: { pendingDelete = account })
            }
            .onMove { source, destination in
                var ids = accounts.map(\.id)
                ids.move(fromOffsets: source, toOffset: destination)
                store.reorderAccounts(in: group.id, to: ids)
            }
        }
    }

    /// Mede todas as contas do grupo pela sonda ativa.
    ///
    /// Fica aqui, e não no laço de rotação, porque cada conta custa um processo
    /// Node subindo do zero e uma requisição de verdade: é ação do usuário, com
    /// resposta visível enquanto roda.
    @ViewBuilder private var measureButton: some View {
        if store.measuringGroup == group.id {
            HStack(spacing: 5) {
                ProgressView().controlSize(.small)
                Text("groups.measuring").font(.caption).foregroundStyle(.secondary)
            }
        } else {
            Button {
                Task { await store.measureAccounts(in: group) }
            } label: {
                Label("groups.measure", systemImage: "gauge.with.dots.needle.bottom.50percent")
                    .font(.caption)
            }
            .buttonStyle(.link)
            .disabled(accounts.isEmpty || store.measuringGroup != nil)
            .help(String(localized: "groups.measure.help"))
        }
    }

    private func commitName() {
        let trimmed = draftName.trimmingCharacters(in: .whitespaces)
        if !trimmed.isEmpty { store.renameGroup(group.id, to: trimmed) }
        editingName = false
    }
}
