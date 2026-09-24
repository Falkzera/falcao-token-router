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
