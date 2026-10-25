import SwiftUI
import CCUsageCore

/// A aba Histórico (21/10/2026): para cada conta, a janela de 5h nos últimos 7
/// dias e a semanal nas últimas 4 semanas — cada ponto, uma medição da sonda
/// (a de 5 em 5 horas, o "Medir contas", o `router measure`). Grupos na ordem
/// da lista; a conta em dois grupos aparece nos dois, cada um com o seu
/// limiar. O atalho de cada conta (`navigation.historyFocus`) rola até o
/// cartão dela e o destaca.
struct HistoryView: View {
    @Bindable var store: RouterConfigStore
    @Bindable var settings: AppSettings
    @Bindable var navigation: HomeNavigation

    @ViewState private var highlighted: String?

    /// O endereço de um cartão: a mesma conta pode aparecer em dois grupos.
    private static func anchor(_ group: UUID, _ account: UUID) -> String {
        "\(group.uuidString):\(account.uuidString)"
    }

    var body: some View {
        if store.config.groups.isEmpty {
            emptyState
        } else {
            ScrollViewReader { proxy in
                ScrollView {
                    VStack(alignment: .leading, spacing: 20) {
                        Text("home.history.intro").font(.caption).foregroundStyle(.secondary)
                        ForEach(store.config.groups) { group in
                            groupSection(group)
                        }
                    }
                    .padding(16)
                }
                .onAppear {
                    store.refreshHistory()
                    focus(with: proxy)
                }
                .onChange(of: navigation.historyFocus) { _, _ in focus(with: proxy) }
            }
        }
    }

    private var emptyState: some View {
        VStack(spacing: 12) {
            Image(systemName: "chart.xyaxis.line")
                .font(.system(size: 34)).foregroundStyle(.tertiary)
            Text("home.history.empty.title").font(.callout)
            Text("home.history.empty.detail").font(.caption)
                .foregroundStyle(.secondary).multilineTextAlignment(.center)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .padding(32)
    }

    private func groupSection(_ group: AccountGroup) -> some View {
        let accounts = store.config.accounts(in: group)
        return VStack(alignment: .leading, spacing: 10) {
            Text(verbatim: group.name).font(.headline)
            if accounts.isEmpty {
                Text("home.history.group.empty").font(.caption).foregroundStyle(.tertiary)
            }
            ForEach(accounts) { account in
                HistoryCard(account: account,
                            points: store.historyByAccount[account.id] ?? [],
                            threshold: group.thresholdPercent,
                            scheduleOn: settings.scheduledMeasure,
                            attempt: store.measureAttempts[account.id],
                            highlighted: highlighted == Self.anchor(group.id, account.id))
                    .id(Self.anchor(group.id, account.id))
            }
        }
    }

    /// O atalho: rola até o cartão da conta — no grupo do clique; senão, o
    /// primeiro em que ela aparece — e o destaca por 1,8 s. Atendido, o pedido
    /// some, para não rolar de novo na próxima visita à aba.
    private func focus(with proxy: ScrollViewProxy) {
        guard let wanted = navigation.historyFocus else { return }
        navigation.historyFocus = nil
        let firstGroup = store.config.groups.first { group in
            store.config.accounts(in: group).contains { $0.id == wanted.accountID }
        }
        guard let groupID = wanted.groupID ?? firstGroup?.id else { return }
        let key = Self.anchor(groupID, wanted.accountID)
        Task { @MainActor in
            // Uma volta para o conteúdo existir antes de rolar até ele.
            try? await Task.sleep(for: .milliseconds(60))
            withAnimation { proxy.scrollTo(key, anchor: .top) }
            highlighted = key
            try? await Task.sleep(for: .seconds(1.8))
            if highlighted == key { highlighted = nil }
        }
    }
}
