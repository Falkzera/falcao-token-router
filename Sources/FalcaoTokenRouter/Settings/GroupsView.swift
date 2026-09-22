import SwiftUI
import CCUsageCore

/// A tela onde o usuário monta o rodízio: cria grupos, adiciona contas pelo
/// login oficial, define a ordem de preferência e o limiar de troca.
///
/// É o requisito central do produto — tudo pela UI, sem terminal. O `store` faz
/// o trabalho; esta view só apresenta e chama.
struct GroupsView: View {
    @Bindable var store: RouterConfigStore

    @ViewState private var pendingLogin: PendingLogin?
    @ViewState private var newGroupSheet = false

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            header
            Divider()
            if store.config.groups.isEmpty {
                emptyState
            } else {
                ScrollView {
                    VStack(alignment: .leading, spacing: 20) {
                        ForEach(store.config.groups) { group in
                            GroupCard(store: store, group: group,
                                      onAddAccount: { startAddAccount(to: group.id) },
                                      onRelogin: { startRelogin($0, in: group.id) })
                        }
                    }
                    .padding(16)
                }
            }
            Divider()
            TerminalIntegrationRow(store: store)
            if let error = store.lastError {
                Divider()
                Label(error, systemImage: "exclamationmark.triangle")
                    .font(.caption)
                    .foregroundStyle(UsageColor.critical)
                    .padding(.horizontal, 16).padding(.vertical, 8)
            }
        }
        .onAppear { store.refreshUsage() }
        .sheet(isPresented: $newGroupSheet) {
            NewGroupSheet { name in store.addGroup(name: name) }
        }
        .sheet(item: $pendingLogin) { pending in
            LoginSheet(store: store, pending: pending)
        }
    }

    /// Sem título: a aba já diz "Grupos", e repetir "Grupos e rodízio" logo
    /// abaixo dela gastava a primeira linha da tela dizendo o que já estava
    /// dito. A legenda fica, porque explica o que a tela FAZ.
    ///
    /// A engrenagem que ficava aqui — a volta para os Ajustes — saiu junto com
    /// a janela separada que ela abria.
    private var header: some View {
        HStack {
            Text("groups.subtitle").font(.caption).foregroundStyle(.secondary)
            Spacer(minLength: 12)
            Button {
                newGroupSheet = true
            } label: {
                Label("groups.new", systemImage: "plus")
            }
        }
        .padding(16)
    }

    private var emptyState: some View {
        VStack(spacing: 12) {
            Image(systemName: "rectangle.stack.badge.plus")
                .font(.system(size: 34)).foregroundStyle(.tertiary)
            Text("groups.empty.title").font(.callout)
            Text("groups.empty.detail").font(.caption)
                .foregroundStyle(.secondary).multilineTextAlignment(.center)
            Button("groups.empty.create") { newGroupSheet = true }
                .buttonStyle(.borderedProminent)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .padding(32)
    }

    private func startAddAccount(to groupID: UUID) {
        let begun = store.newAccountHome()
        pendingLogin = PendingLogin(home: begun.home, accountID: begun.accountID,
                                    groupID: groupID)
    }

    /// Relogin de conta cuja credencial morreu: mesma casa, mesmo registro.
    private func startRelogin(_ account: Account, in groupID: UUID) {
        pendingLogin = PendingLogin(home: account.home, accountID: account.id,
                                    groupID: groupID, isRelogin: true)
    }
}

/// A linha de baixo: ativa a integração de shell (um clique edita o `~/.zshrc`)
/// para `claude <grupo>` funcionar — sem o usuário mexer em arquivo.
private struct TerminalIntegrationRow: View {
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

/// Um login em andamento, aguardando o usuário concluir no navegador.
struct PendingLogin: Identifiable {
    let id = UUID()
    let home: ConfigDir
    let accountID: UUID
    let groupID: UUID
    /// `true` quando é RELOGIN de uma conta existente (credencial morta): usa a
    /// mesma casa, e o desfecho renova em vez de adicionar.
    var isRelogin = false
}

// MARK: - Cartão de um grupo

private struct GroupCard: View {
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

// MARK: - Uma conta na lista

private struct AccountRow: View {
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

// MARK: - Folha de novo grupo

private struct NewGroupSheet: View {
    let onCreate: (String) -> Void
    @Environment(\.dismiss) private var dismiss
    @ViewState private var name = ""

    var body: some View {
        VStack(alignment: .leading, spacing: 14) {
            Text("groups.new.title").font(.headline)
            Text("groups.new.detail").font(.caption).foregroundStyle(.secondary)
            TextField("groups.name.placeholder", text: $name)
                .textFieldStyle(.roundedBorder)
                .onSubmit(create)
            HStack {
                Spacer()
                Button("groups.cancel") { dismiss() }
                Button("groups.new.create", action: create)
                    .buttonStyle(.borderedProminent)
                    .disabled(name.trimmingCharacters(in: .whitespaces).isEmpty)
            }
        }
        .padding(20).frame(width: 340)
    }

    private func create() {
        let trimmed = name.trimmingCharacters(in: .whitespaces)
        guard !trimmed.isEmpty else { return }
        onCreate(trimmed); dismiss()
    }
}

// MARK: - Folha de login, dentro do app

/// Conduz o login oficial sem Terminal: mostra o link no próprio modal, o
/// navegador abre no fluxo da Anthropic, e a folha fecha sozinha quando a conta
/// entra. Um campo de código fica à mão para o raro caso do callback não voltar.
private struct LoginSheet: View {
    @Bindable var store: RouterConfigStore
    let pending: PendingLogin
    @Environment(\.dismiss) private var dismiss
    @Environment(\.openURL) private var openURL

    private enum Result: Equatable {
        case pending, added(String), duplicate(String)
        /// Relogin que voltou com OUTRO e-mail (navegador na conta errada).
        case wrongAccount(expected: String, got: String)
    }

    @ViewState private var session: LoginSession?
    @ViewState private var result: Result = .pending
    @ViewState private var showCodeField = false
    @ViewState private var code = ""
    @ViewState private var copied = false

    var body: some View {
        VStack(spacing: 14) {
            if case .added(let label) = result {
                successView(label)
            } else if case .duplicate(let email) = result {
                duplicateView(email)
            } else if case .wrongAccount(let expected, let got) = result {
                wrongAccountView(expected: expected, got: got)
            } else if case .failed(let message) = session?.phase ?? .starting {
                failureView(message)
            } else {
                waitingView
            }
        }
        .padding(24).frame(width: 380)
        .onAppear {
            let s = LoginSession(home: pending.home, accountID: pending.accountID,
                                 groupID: pending.groupID)
            session = s
            s.start()
        }
        .onChange(of: session?.phase) { _, phase in
            if phase == .success { Task { await finish() } }
        }
    }

    private var waitingView: some View {
        VStack(spacing: 14) {
            ProgressView()
            Text(pending.isRelogin ? "groups.relogin.waiting.title"
                                   : "groups.login.waiting.title")
                .font(.headline)

            if let url = session?.authURL {
                Text("groups.login.inapp.detail")
                    .font(.caption).foregroundStyle(.secondary)
                    .multilineTextAlignment(.center)

                HStack(spacing: 6) {
                    Text(verbatim: url.absoluteString)
                        .font(.caption.monospaced())
                        .lineLimit(1).truncationMode(.middle)
                        .padding(6)
                        .frame(maxWidth: .infinity, alignment: .leading)
                        .background(.quaternary.opacity(0.5), in: .rect(cornerRadius: 6))
                    Button {
                        NSPasteboard.general.clearContents()
                        NSPasteboard.general.setString(url.absoluteString, forType: .string)
                        copied = true
                        Task { try? await Task.sleep(for: .seconds(2)); copied = false }
                    } label: { Image(systemName: copied ? "checkmark" : "doc.on.doc") }
                        .controlSize(.small)
                }
                Button("groups.login.openBrowser") { openURL(url) }
                    .controlSize(.small)

                DisclosureGroup(isExpanded: $showCodeField) {
                    HStack(spacing: 6) {
                        TextField("groups.login.code.placeholder", text: $code)
                            .textFieldStyle(.roundedBorder)
                            .onSubmit { session?.submitCode(code) }
                        Button("groups.login.code.submit") { session?.submitCode(code) }
                            .controlSize(.small)
                            .disabled(code.trimmingCharacters(in: .whitespaces).isEmpty)
                    }
                    .padding(.top, 4)
                } label: {
                    Text("groups.login.code.disclosure").font(.caption)
                }
            } else {
                Text("groups.login.inapp.starting")
                    .font(.caption).foregroundStyle(.secondary)
                    .multilineTextAlignment(.center)
            }

            Button("groups.cancel") { session?.cancel(); dismiss() }
        }
    }

    private func successView(_ label: String) -> some View {
        VStack(spacing: 14) {
            Image(systemName: "checkmark.circle.fill")
                .font(.system(size: 34)).foregroundStyle(UsageColor.calm)
            Text(pending.isRelogin ? "groups.relogin.done" : "groups.login.done")
                .font(.headline)
            Text(verbatim: label).font(.callout).foregroundStyle(.secondary)
            Button("groups.login.close") { dismiss() }
                .buttonStyle(.borderedProminent)
        }
    }

    /// Relogin que autenticou OUTRA conta: o registro não muda; explica e
    /// oferece sair do claude.ai e tentar de novo — o mesmo remédio do caso
    /// duplicata do login normal.
    private func wrongAccountView(expected: String, got: String) -> some View {
        VStack(spacing: 12) {
            Image(systemName: "person.crop.circle.badge.exclamationmark")
                .font(.system(size: 30)).foregroundStyle(UsageColor.warning)
            Text("groups.relogin.wrong.title").font(.headline)
            Text(String(format: String(localized: "groups.relogin.wrong.detail.format"),
                        got, expected))
                .font(.caption).foregroundStyle(.secondary)
                .multilineTextAlignment(.center)
            Button("groups.login.duplicate.logout") {
                if let url = URL(string: "https://claude.ai/logout") { openURL(url) }
            }
            .controlSize(.small)
            HStack {
                Button("groups.login.close") { dismiss() }
                Button("groups.login.duplicate.retry") {
                    let s = LoginSession(home: pending.home, accountID: pending.accountID,
                                         groupID: pending.groupID)
                    session = s; result = .pending; s.start()
                }
                .buttonStyle(.borderedProminent)
            }
        }
    }

    /// O login trouxe uma conta que já está no grupo — quase sempre a sessão do
    /// navegador que não trocou. Explica e oferece o caminho: sair no claude.ai.
    private func duplicateView(_ email: String) -> some View {
        VStack(spacing: 12) {
            Image(systemName: "person.crop.circle.badge.exclamationmark")
                .font(.system(size: 30)).foregroundStyle(UsageColor.warning)
            Text("groups.login.duplicate.title").font(.headline)
            Text(String(format: String(localized: "groups.login.duplicate.detail.format"), email))
                .font(.caption).foregroundStyle(.secondary)
                .multilineTextAlignment(.center)
            Button("groups.login.duplicate.logout") {
                if let url = URL(string: "https://claude.ai/logout") { openURL(url) }
            }
            .controlSize(.small)
            HStack {
                Button("groups.login.close") { dismiss() }
                Button("groups.login.duplicate.retry") {
                    // Recomeça o login numa casa nova, mantendo o mesmo grupo.
                    let fresh = store.newAccountHome()
                    let s = LoginSession(home: fresh.home, accountID: fresh.accountID,
                                         groupID: pending.groupID)
                    session = s; result = .pending; s.start()
                }
                .buttonStyle(.borderedProminent)
            }
        }
    }

    private func failureView(_ message: String) -> some View {
        VStack(spacing: 14) {
            Image(systemName: "exclamationmark.triangle.fill")
                .font(.system(size: 30)).foregroundStyle(UsageColor.critical)
            Text("groups.login.failed").font(.headline)
            Text(verbatim: message).font(.caption).foregroundStyle(.secondary)
                .multilineTextAlignment(.center)
            Button("groups.login.close") { dismiss() }
        }
    }

    /// O `claude` avisou "Login successful"; a credencial pode levar um instante
    /// para assentar. Confirma pela sessão ATUAL (o retry troca a casa), por
    /// alguns segundos, e classifica o desfecho.
    private func finish() async {
        guard let session else { return }
        for _ in 0..<12 {
            if pending.isRelogin {
                switch store.finishRelogin(accountID: session.accountID) {
                case .renewed(let account):
                    result = .added(account.label); return
                case .wrongAccount(let expected, let got):
                    result = .wrongAccount(expected: expected, got: got); return
                case .pending: break
                }
            } else {
                switch store.finishPendingLogin(home: session.home,
                                                accountID: session.accountID,
                                                into: session.groupID) {
                case .added(let account):
                    result = .added(account.label); return
                case .duplicate(let email):
                    result = .duplicate(email); return
                case .pending: break
                }
            }
            try? await Task.sleep(for: .milliseconds(400))
        }
    }
}
