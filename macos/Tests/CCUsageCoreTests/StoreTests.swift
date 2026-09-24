import Foundation
import Testing
@testable import CCUsageCore

/// Exercita a API que a UI de grupos chama, ponta a ponta, com chaveiro e
/// identidade em memória — sem tocar no sistema nem abrir Terminal.
@MainActor
@Suite("RouterConfigStore")
struct RouterConfigStoreTests {

    /// Um store com caminhos temporários e adapter/chaveiro falsos.
    private func makeStore() -> (RouterConfigStore, FakeKeychain, FakeAdapter, RouterPaths) {
        let tmp = URL(fileURLWithPath: NSTemporaryDirectory())
            .appending(path: "router-test-\(UUID().uuidString)")
        let paths = RouterPaths(bundleID: "test.falcao-router", appSupport: tmp)
        let kc = FakeKeychain()
        let adapter = FakeAdapter()
        let store = RouterConfigStore(paths: paths, keychain: kc, adapters: [adapter])
        return (store, kc, adapter, paths)
    }

    /// Simula um login concluído: identidade no perfil + credencial no chaveiro.
    private func seedLogin(_ email: String, home: ConfigDir,
                           kc: FakeKeychain, adapter: FakeAdapter) {
        let identity = AccountIdentity(email: email, organizationName: "Acme",
                                       rateLimitTier: "default_claude_max_5x",
                                       raw: ["emailAddress": .string(email)])
        try? adapter.writeIdentity(identity, toConfigDir: home)
        try? kc.write("cred-\(email)", service: adapter.keychainService(forConfigDir: home))
    }

    @Test("o primeiro grupo é o padrão; o segundo é dedicado")
    func firstGroupIsDefault() {
        let (store, _, _, _) = makeStore()
        let work = store.addGroup(name: "trabalho")
        let personal = store.addGroup(name: "pessoal")

        #expect(work.configDir.isDefault)
        #expect(!personal.configDir.isDefault)
        #expect(store.config.defaultGroup?.id == work.id)
    }

    /// Relogin renova a identidade no registro e, se a conta está ATIVA num
    /// grupo, empurra a credencial nova da casa para o item do grupo — que
    /// guardava a morta que motivou o relogin (o caso real de 26/ago).
    /// "Remover a conta" tem de remover a conta. Até 18/09/2026 só o registro
    /// saía: o item de chaveiro e a pasta da casa ficavam para trás com um
    /// refresh token vivo, para sempre — num produto pago, promessa quebrada.
    @Test("remover a conta apaga a credencial e a casa dela")
    func removerApagaCredencial() throws {
        let (store, kc, adapter, _) = makeStore()
        let group = store.addGroup(name: "trabalho")
        let begun = store.newAccountHome()
        seedLogin("conta9@exemplo.com", home: begun.home, kc: kc, adapter: adapter)
        guard case .added(let account) = store.finishPendingLogin(
            home: begun.home, accountID: begun.accountID, into: group.id) else {
            Issue.record("login não entrou"); return
        }
        let service = adapter.keychainService(forConfigDir: account.home)
        #expect(kc.exists(service: service))
        #expect(FileManager.default.fileExists(atPath: account.home.url.path))

        store.removeAccount(account.id)

        #expect(store.config.account(account.id) == nil)
        #expect(!kc.exists(service: service), "a credencial ficou no chaveiro")
        #expect(!FileManager.default.fileExists(atPath: account.home.url.path),
                "a casa da conta ficou no disco")
    }

    /// Apagar o grupo leva junto as contas que só existiam nele — e, por tabela,
    /// as credenciais delas. Conta compartilhada com outro grupo fica.
    @Test("apagar o grupo apaga a credencial das contas exclusivas")
    func apagarGrupoLevaCredenciais() throws {
        let (store, kc, adapter, _) = makeStore()
        let group = store.addGroup(name: "temporario")
        let begun = store.newAccountHome()
        seedLogin("so-aqui@k.com", home: begun.home, kc: kc, adapter: adapter)
        guard case .added(let account) = store.finishPendingLogin(
            home: begun.home, accountID: begun.accountID, into: group.id) else {
            Issue.record("login não entrou"); return
        }
        let service = adapter.keychainService(forConfigDir: account.home)

        store.removeGroup(group.id)

        #expect(store.config.accounts.isEmpty)
        #expect(!kc.exists(service: service))
    }

    @Test("relogin renova a conta e atualiza o item do grupo quando ativa")
    func reloginRenewsAndPushesToGroup() {
        let (store, kc, adapter, _) = makeStore()
        let group = store.addGroup(name: "trabalho")
        let begun = store.newAccountHome()
        seedLogin("conta2@exemplo.com", home: begun.home, kc: kc, adapter: adapter)
        guard case .added(let account) = store.finishPendingLogin(
            home: begun.home, accountID: begun.accountID, into: group.id) else {
            Issue.record("login inicial não entrou"); return
        }
        store.activate(account, in: store.config.groups[0])

        // O relogin oficial escreve credencial NOVA na casa da conta.
        try? kc.write("cred-NOVA", service: adapter.keychainService(forConfigDir: account.home))

        let outcome = store.finishRelogin(accountID: account.id)

        guard case .renewed = outcome else {
            Issue.record("esperava .renewed, veio \(outcome)"); return
        }
        let groupService = adapter.keychainService(
            forConfigDir: store.config.groups[0].configDir)
        #expect(kc.read(service: groupService) == "cred-NOVA")
    }

    /// Relogin que autenticou OUTRO e-mail não toca no registro — a UI explica.
    @Test("relogin com outra conta vira .wrongAccount e nada muda")
    func reloginWrongAccount() {
        let (store, kc, adapter, _) = makeStore()
        let group = store.addGroup(name: "trabalho")
        let begun = store.newAccountHome()
        seedLogin("conta2@exemplo.com", home: begun.home, kc: kc, adapter: adapter)
        guard case .added(let account) = store.finishPendingLogin(
            home: begun.home, accountID: begun.accountID, into: group.id) else {
            Issue.record("login inicial não entrou"); return
        }

        // O navegador estava logado em outra conta: a casa recebe outra identidade.
        seedLogin("intrusa@exemplo.com", home: account.home, kc: kc, adapter: adapter)

        let outcome = store.finishRelogin(accountID: account.id)

        #expect(outcome == .wrongAccount(expected: "conta2@exemplo.com", got: "intrusa@exemplo.com"))
        #expect(store.config.account(account.id)?.identity.email == "conta2@exemplo.com")
    }

    /// Apagar o grupo leva junto as contas que só existiam nele — órfãs não
    /// aparecem em tela nenhuma. As contas dos outros grupos ficam intactas.
    @Test("apagar grupo remove as contas exclusivas dele")
    func removeGroupDropsExclusiveAccounts() {
        let (store, kc, adapter, _) = makeStore()
        let work = store.addGroup(name: "trabalho")
        let personal = store.addGroup(name: "pessoal")

        let workAcc = store.newAccountHome()
        seedLogin("gov@k.com", home: workAcc.home, kc: kc, adapter: adapter)
        _ = store.finishPendingLogin(home: workAcc.home, accountID: workAcc.accountID,
                                     into: work.id)
        let personalAcc = store.newAccountHome()
        seedLogin("eu@k.com", home: personalAcc.home, kc: kc, adapter: adapter)
        _ = store.finishPendingLogin(home: personalAcc.home, accountID: personalAcc.accountID,
                                     into: personal.id)

        store.removeGroup(work.id)

        #expect(store.config.groups.count == 1)
        #expect(store.config.account(workAcc.accountID) == nil)       // exclusiva saiu
        #expect(store.config.account(personalAcc.accountID) != nil)   // do outro grupo fica
    }

    @Test("tornar outro grupo o padrão tira o padrão do anterior")
    func makeDefaultMovesIt() {
        let (store, _, _, _) = makeStore()
        let work = store.addGroup(name: "trabalho")
        let personal = store.addGroup(name: "pessoal")

        store.makeDefault(personal.id)

        #expect(store.config.groups.first { $0.id == personal.id }?.configDir.isDefault == true)
        #expect(store.config.groups.first { $0.id == work.id }?.configDir.isDefault == false)
        // Continua havendo no máximo um padrão.
        #expect(store.config.groups.filter { $0.configDir.isDefault }.count == 1)
    }

    @Test("um login concluído vira conta no grupo")
    func finishLoginAddsAccount() {
        let (store, kc, adapter, paths) = makeStore()
        let group = store.addGroup(name: "trabalho")

        let accountID = UUID()
        let home = paths.accountHome(accountID)
        seedLogin("conta1@exemplo.com", home: home, kc: kc, adapter: adapter)

        let outcome = store.finishPendingLogin(home: home, accountID: accountID, into: group.id)

        guard case .added(let account) = outcome else { Issue.record("esperava .added"); return }
        #expect(account.identity.email == "conta1@exemplo.com")
        #expect(store.config.accounts.count == 1)
        #expect(store.config.groups.first?.accountIDs == [accountID])
    }

    @Test("a mesma conta de novo vira .duplicate, não outra conta")
    func duplicateLogin() {
        let (store, kc, adapter, paths) = makeStore()
        let group = store.addGroup(name: "pessoal")

        // Primeiro login: a primeira conta entra.
        let id1 = UUID(); let home1 = paths.accountHome(id1)
        seedLogin("pessoal@exemplo.com", home: home1, kc: kc, adapter: adapter)
        store.finishPendingLogin(home: home1, accountID: id1, into: group.id)

        // Segundo login, de OUTRA conta — mas o navegador devolveu a mesma.
        let id2 = UUID(); let home2 = paths.accountHome(id2)
        seedLogin("pessoal@exemplo.com", home: home2, kc: kc, adapter: adapter)
        let outcome = store.finishPendingLogin(home: home2, accountID: id2, into: group.id)

        #expect(outcome == .duplicate(email: "pessoal@exemplo.com"))
        #expect(store.config.accounts.count == 1)  // não duplicou
    }

    @Test("login ainda não concluído não cria conta")
    func unfinishedLoginAddsNothing() {
        let (store, _, _, paths) = makeStore()
        let group = store.addGroup(name: "g")
        let accountID = UUID()
        // Nada semeado: o login não terminou.
        let outcome = store.finishPendingLogin(home: paths.accountHome(accountID),
                                               accountID: accountID, into: group.id)
        #expect(outcome == .pending)
        #expect(store.config.accounts.isEmpty)
    }

    @Test("reordenar reescreve a ordem de preferência")
    func reorder() {
        let (store, kc, adapter, paths) = makeStore()
        let group = store.addGroup(name: "g")
        var ids: [UUID] = []
        for email in ["a@k.com", "b@k.com", "c@k.com"] {
            let id = UUID(); let home = paths.accountHome(id)
            seedLogin(email, home: home, kc: kc, adapter: adapter)
            store.finishPendingLogin(home: home, accountID: id, into: group.id)
            ids.append(id)
        }
        // inverte
        store.reorderAccounts(in: group.id, to: ids.reversed())
        #expect(store.config.groups.first?.accountIDs == ids.reversed())
    }

    @Test("ativar troca a conta que serve o grupo")
    func activate() {
        let (store, kc, adapter, paths) = makeStore()
        let group = store.addGroup(name: "trabalho")

        var accounts: [Account] = []
        for email in ["conta1@exemplo.com", "conta2@exemplo.com"] {
            let id = UUID(); let home = paths.accountHome(id)
            seedLogin(email, home: home, kc: kc, adapter: adapter)
            if case .added(let a) = store.finishPendingLogin(
                home: home, accountID: id, into: group.id) {
                accounts.append(a)
            }
        }
        let g = store.config.groups.first!
        store.activate(accounts[1], in: g)

        #expect(store.lastError == nil)
        #expect(store.activeAccount(in: g)?.id == accounts[1].id)
    }

    @Test("remover conta some dela e das listas de grupo")
    func removeAccount() {
        let (store, kc, adapter, paths) = makeStore()
        let group = store.addGroup(name: "g")
        let id = UUID(); let home = paths.accountHome(id)
        seedLogin("x@k.com", home: home, kc: kc, adapter: adapter)
        store.finishPendingLogin(home: home, accountID: id, into: group.id)

        store.removeAccount(id)
        #expect(store.config.accounts.isEmpty)
        #expect(store.config.groups.first?.accountIDs.isEmpty == true)
    }

    @Test("a configuração persiste entre instâncias")
    func persists() {
        let tmp = URL(fileURLWithPath: NSTemporaryDirectory())
            .appending(path: "router-persist-\(UUID().uuidString)")
        let paths = RouterPaths(bundleID: "test.falcao-router", appSupport: tmp)
        let kc = FakeKeychain(); let adapter = FakeAdapter()

        let first = RouterConfigStore(paths: paths, keychain: kc, adapters: [adapter])
        let group = first.addGroup(name: "trabalho")
        first.setThreshold(group.id, percent: 85)

        // Nova instância lê o mesmo arquivo.
        let second = RouterConfigStore(paths: paths, keychain: kc, adapters: [adapter])
        #expect(second.config.groups.first?.name == "trabalho")
        #expect(second.config.groups.first?.thresholdPercent == 85)
    }

    @Test("uso do sensor vira uso por conta")
    func usageFromSensor() throws {
        let (store, kc, adapter, paths) = makeStore()
        let group = store.addGroup(name: "g")
        let id = UUID(); let home = paths.accountHome(id)
        seedLogin("m@k.com", home: home, kc: kc, adapter: adapter)
        store.finishPendingLogin(home: home, accountID: id, into: group.id)

        // O sensor gravou uma amostra dessa conta.
        try FileManager.default.createDirectory(at: paths.usageDir, withIntermediateDirectories: true)
        let sample = GroupUsageSample(
            configDirRaw: group.configDir.raw, email: "m@k.com",
            fiveHourPercent: 0.4, fiveHourResetsAt: nil,
            sevenDayPercent: 0.72, sevenDayResetsAt: nil, sampledAt: Date())
        try GroupUsageStore.write(sample, forEmail: "m@k.com", in: paths.usageDir)

        store.refreshUsage()
        // Liga no maior dos dois: 72% > 40%.
        #expect(store.usageSnapshot[id] == 0.72)
    }

    @Test("clearDefault deixa nenhum grupo no ~/.claude")
    func clearDefaultFrees() {
        let (store, _, _, _) = makeStore()
        let work = store.addGroup(name: "trabalho")   // vira padrão
        _ = store.addGroup(name: "pessoal")
        #expect(store.config.defaultGroup?.id == work.id)

        store.clearDefault()
        #expect(store.config.defaultGroup == nil)
        #expect(store.config.groups.allSatisfy { !$0.configDir.isDefault })
    }
}
