import SwiftUI
import CCUsageCore

/// Conduz o login oficial sem Terminal: mostra o link no próprio modal, o
/// navegador abre no fluxo da Anthropic, e a folha fecha sozinha quando a conta
/// entra. Um campo de código fica à mão para o raro caso do callback não voltar.
struct LoginSheet: View {
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
