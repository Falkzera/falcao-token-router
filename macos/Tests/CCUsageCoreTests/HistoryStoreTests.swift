import Foundation
import Testing
@testable import CCUsageCore

/// O store e o histórico (21/10/2026): o plano da agenda de 5 em 5 horas, a
/// tentativa que segura a conta, a ativa pelo perfil do grupo e apagar.
@MainActor
@Suite("RouterConfigStore + histórico")
struct HistoryStoreTests {
    /// Um store com caminhos temporários e adapter/chaveiro falsos — o mesmo
    /// arranjo do `StoreTests`.
    private func makeStore() -> (RouterConfigStore, FakeKeychain, FakeAdapter, RouterPaths) {
        let tmp = URL(fileURLWithPath: NSTemporaryDirectory())
            .appending(path: "router-history-test-\(UUID().uuidString)")
        let paths = RouterPaths(bundleID: "test.falcao-router", appSupport: tmp)
        let kc = FakeKeychain()
        let adapter = FakeAdapter()
        return (RouterConfigStore(paths: paths, keychain: kc, adapters: [adapter]), kc, adapter, paths)
    }

    /// Login completo numa casa nova, adicionado ao grupo — o `seedLogin` do
    /// `StoreTests`, mais o `finishPendingLogin`. `nil` se o login não entrou:
    /// quem chama falha no `#require`, sem derrubar a suíte.
    private func addAccount(_ email: String, to groupID: UUID, store: RouterConfigStore,
                            kc: FakeKeychain, adapter: FakeAdapter) -> Account? {
        let begun = store.newAccountHome()
        let identity = AccountIdentity(email: email, organizationName: "Acme",
                                       rateLimitTier: "default_claude_max_5x",
                                       raw: ["emailAddress": .string(email)])
        try? adapter.writeIdentity(identity, toConfigDir: begun.home)
        try? kc.write("cred-\(email)", service: adapter.keychainService(forConfigDir: begun.home))
        guard case .added(let account) = store.finishPendingLogin(
            home: begun.home, accountID: begun.accountID, into: groupID) else { return nil }
        return account
    }

    /// Um ponto no histórico de uma conta, `hoursAgo` horas atrás.
    private func measured(_ email: String, hoursAgo: Double, paths: RouterPaths) throws {
        let at = Date().addingTimeInterval(-hoursAgo * 3600)
        try UsageHistory.append(UsageHistoryPoint(sampledAt: at, fiveHourPercent: 0.1, fiveHourResetsAt: nil,
                                                  sevenDayPercent: 0.2, sevenDayResetsAt: nil),
                                forEmail: email, in: paths.historyDir)
    }

    @Test("a agenda mede, grupo a grupo, só as contas vencidas")
    func soAsVencidas() throws {
        let (store, kc, adapter, paths) = makeStore()
        let work = store.addGroup(name: "trabalho")
        let personal = store.addGroup(name: "pessoal")
        let a = try #require(addAccount("conta1@exemplo.com", to: work.id, store: store, kc: kc, adapter: adapter))
        let b = try #require(addAccount("conta2@exemplo.com", to: work.id, store: store, kc: kc, adapter: adapter))
        let c = try #require(addAccount("conta3@exemplo.com", to: personal.id, store: store, kc: kc, adapter: adapter))
        try measured("conta1@exemplo.com", hoursAgo: 1, paths: paths)  // não vence
        try measured("conta3@exemplo.com", hoursAgo: 6, paths: paths)  // vence
        // a conta2 nunca foi medida: vence

        let plans = store.scheduledMeasurePlans()
        try #require(plans.count == 2)
        #expect(plans.map(\.groupID) == [work.id, personal.id])
        #expect(plans[0].targets.map(\.accountID) == [b.id])
        #expect(plans[1].targets.map(\.accountID) == [c.id])
        #expect(!plans.flatMap(\.targets).contains { $0.accountID == a.id })
    }

    @Test("conta em dois grupos é medida uma vez só, no primeiro")
    func contaEmDoisGrupos() throws {
        let (store, kc, adapter, paths) = makeStore()
        let work = store.addGroup(name: "trabalho")
        let personal = store.addGroup(name: "pessoal")
        let shared = try #require(addAccount("conta1@exemplo.com", to: work.id, store: store, kc: kc, adapter: adapter))
        _ = try #require(addAccount("conta2@exemplo.com", to: personal.id, store: store, kc: kc, adapter: adapter))
        // A compartilhada entra também no 2º grupo, pelo `config.json`.
        var config = store.config
        config.groups[1].accountIDs.insert(shared.id, at: 0)
        try JSONEncoder().encode(config).write(to: paths.configFile)
        let reopened = RouterConfigStore(paths: paths, keychain: kc, adapters: [adapter])

        let plans = reopened.scheduledMeasurePlans()
        #expect(plans.flatMap(\.targets).filter { $0.accountID == shared.id }.count == 1)
        #expect(plans.first?.groupID == work.id)
        #expect(plans.flatMap(\.targets).count == 2)
    }

    @Test("a tentativa recente segura a conta fora do plano")
    func tentativaRecente() throws {
        let (store, kc, adapter, _) = makeStore()
        let group = store.addGroup(name: "trabalho")
        let a = try #require(addAccount("conta1@exemplo.com", to: group.id, store: store, kc: kc, adapter: adapter))

        store.measureAttempts[a.id] = Date().addingTimeInterval(-3600)
        #expect(store.scheduledMeasurePlans().isEmpty)
        store.measureAttempts[a.id] = Date().addingTimeInterval(-5 * 3600)
        #expect(store.scheduledMeasurePlans().count == 1)
    }

    /// Na agenda também, a conta ATIVA é sondada pelo perfil do grupo — nunca
    /// pela casa (a regra do `probeConfigDir`).
    @Test("a ativa é sondada pelo perfil do grupo")
    func ativaPeloGrupo() throws {
        let (store, kc, adapter, _) = makeStore()
        let group = store.addGroup(name: "trabalho")
        let a = try #require(addAccount("conta1@exemplo.com", to: group.id, store: store, kc: kc, adapter: adapter))
        // Como o `activate()` do `StoreTests`: o 1º grupo é o padrão, mas
        // identidade e chaveiro são os falsos, em memória.
        store.activate(a, in: try #require(store.config.groups.first))
        #expect(store.lastError == nil)

        let target = try #require(store.scheduledMeasurePlans().first?.targets.first)
        #expect(target.dir.raw == store.config.groups[0].configDir.raw)
        #expect(target.dir.raw != a.home.raw)
    }

    /// A agenda é silenciosa e espera: com login aberto ou medição em curso,
    /// nada é tentado — a conta continua vencida para a próxima volta.
    @Test("com login aberto a agenda espera")
    func esperaLogin() async throws {
        let (store, kc, adapter, _) = makeStore()
        let group = store.addGroup(name: "trabalho")
        let a = try #require(addAccount("conta1@exemplo.com", to: group.id, store: store, kc: kc, adapter: adapter))

        store.signingIn = true
        await store.runScheduledMeasure()
        #expect(store.measureAttempts[a.id] == nil)
        #expect(store.measuringGroup == nil)
        #expect(store.lastError == nil)
    }

    @Test("apagar o histórico esvazia todas as contas, que voltam a vencer")
    func apagar() throws {
        let (store, kc, adapter, paths) = makeStore()
        let group = store.addGroup(name: "trabalho")
        let a = try #require(addAccount("conta1@exemplo.com", to: group.id, store: store, kc: kc, adapter: adapter))
        _ = try #require(addAccount("conta2@exemplo.com", to: group.id, store: store, kc: kc, adapter: adapter))
        try measured("conta1@exemplo.com", hoursAgo: 1, paths: paths)
        try measured("conta2@exemplo.com", hoursAgo: 7, paths: paths)
        store.refreshHistory()
        #expect(store.historyByAccount[a.id]?.count == 1)

        #expect(try store.clearHistory() == 2)
        #expect(store.historyByAccount[a.id]?.isEmpty == true)
        #expect(store.scheduledMeasurePlans().first?.targets.count == 2)
    }
}
