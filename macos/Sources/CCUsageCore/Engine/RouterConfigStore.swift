import Foundation
import Observation

/// O dono da configuração do produto, e a ponte entre a UI e o motor.
///
/// A UI edita grupos e contas por aqui; cada mudança persiste o `RouterConfig` em
/// disco. As ações que tocam o sistema (ativar conta, rotacionar) passam pelo
/// `RotationEngine`. Fica no core, sem SwiftUI, para ser exercitável sem janela —
/// mesma regra do resto do módulo.
@MainActor
@Observable
public final class RouterConfigStore {
    public private(set) var config: RouterConfig
    /// Uso 0–1 por conta, lido do sensor. Observável, para a lista de grupos
    /// redesenhar as barras quando o número muda.
    public private(set) var usageSnapshot: [UUID: Double] = [:]
    /// Quando a amostra de cada conta foi colhida — a idade que a UI mostra
    /// para nunca fingir um frescor que o sensor passivo não tem.
    public private(set) var usageSampledAt: [UUID: Date] = [:]
    /// O mesmo uso de `usageSnapshot`, mas com a janela de onde o número veio e
    /// as duas medidas. É o que deixa a UI rotular "66% · 7d" em vez de um "66%"
    /// solto que o usuário lê como sendo o das 5 horas.
    public private(set) var usageDetail: [UUID: AccountUsage] = [:]
    /// As sessões do Claude Code vivas em cada grupo (grupo → sessões).
    ///
    /// Vem do registro que o próprio Claude Code escreve em
    /// `<perfil>/sessions/`, que é POR PERFIL — então dá para dizer qual sessão
    /// roda em qual grupo, e por qual conta ela está sendo atendida. É a
    /// resposta que o `/status` não dá, e o contra-veneno do modo de falha
    /// silencioso: sessão que devia estar num grupo e subiu no perfil padrão
    /// aparece do lado errado da lista.
    public private(set) var liveSessions: [UUID: [LiveSession]] = [:]
    /// A conta ativa de cada grupo (grupo → conta). Observável, para a barra e o
    /// painel mostrarem quem serve cada grupo sem reler o disco a cada frame.
    public private(set) var activeByGroup: [UUID: UUID] = [:]
    /// Última falha de uma ação, para a UI mostrar. `nil` quando tudo correu bem.
    public private(set) var lastError: String?
    /// O grupo que está sendo sondado agora, ou `nil`. Observável porque a
    /// sondagem leva segundos — sem indicação, o clique parece não fazer nada.
    public private(set) var measuringGroup: UUID?
    /// Espelho OBSERVÁVEL de `shellIntegrationInstalled`: aquele é computado
    /// lendo o disco, e computado não dispara re-render — o clique em "Ativar"
    /// funcionava e a tela ficava parada, parecendo botão morto.
    public private(set) var integrationInstalled = false

    /// O que a status line dos grupos mostra. **Observável e em memória**: uma
    /// computada que lesse o disco não re-renderizaria a tela ao mudar — foi
    /// exatamente o bug do botão "Ativar". O disco é a cópia, não a fonte da UI.
    public private(set) var statusLineChoice = StatusLineChoice()
    /// Caminho do binário `router` empacotado, que a integração de shell e a
    /// status line apontam. Setado pelo app ao iniciar; `nil` fora do app.
    @ObservationIgnored public var routerPath: String?

    @ObservationIgnored private let paths: RouterPaths
    @ObservationIgnored private let engine: RotationEngine
    @ObservationIgnored private let keychain: any KeychainStore
    @ObservationIgnored private let login: AccountLoginService
    @ObservationIgnored private let usage: GroupUsageReader

    public init(paths: RouterPaths = RouterPaths(),
                keychain: any KeychainStore = SecurityCLIKeychain(),
                adapters: [any ProviderAdapter] = [AnthropicAdapter()]) {
        self.paths = paths
        self.keychain = keychain
        self.engine = RotationEngine(keychain: keychain, adapters: adapters)
        self.login = AccountLoginService(adapter: adapters.first ?? AnthropicAdapter(), paths: paths)
        self.usage = GroupUsageReader(usageDir: paths.usageDir)
        self.config = Self.load(from: paths.configFile) ?? RouterConfig()
        self.integrationInstalled = shellIntegrationInstalled
        self.statusLineChoice = StatusLineChoice.load(from: StatusLineChoice.fileURL(base: paths.base))
        // Publica o quadro completo aqui, e não só no primeiro laço.
        //
        // A etiqueta do menu bar é desenhada no lançamento, antes de qualquer
        // `refreshUsage()` rodar. Com `activeByGroup` vazio nesse instante,
        // `MenuBarLabel.headline` devolvia nil e a barra mostrava só a
        // porcentagem, **sem o nome da conta** — que é a única coisa que ela
        // existe para dizer quando há várias contas rodando. E o rótulo do
        // `MenuBarExtra` vive num `NSStatusItem`, onde o rastreamento de
        // observação não é confiável, então ele não se corrigia depois.
        refreshUsage()
    }

    // MARK: - Persistência

    private static func load(from url: URL) -> RouterConfig? {
        guard let data = try? Data(contentsOf: url) else { return nil }
        return try? JSONDecoder().decode(RouterConfig.self, from: data)
    }

    private func save() {
        do {
            try FileManager.default.createDirectory(
                at: paths.base, withIntermediateDirectories: true)
            let data = try JSONEncoder().encode(config)
            try data.write(to: paths.configFile, options: .atomic)
            // Só o dono lê. Não há segredo aqui — a credencial mora no chaveiro
            // —, mas há e-mail, organização e plano de cada conta, e o padrão do
            // `umask` deixava isso legível para qualquer usuário da máquina.
            try FileManager.default.setAttributes(
                [.posixPermissions: 0o600], ofItemAtPath: paths.configFile.path)
        } catch {
            lastError = "não foi possível salvar a configuração: \(error)"
        }
    }

    // MARK: - Grupos (o que a UI chama)

    /// Cria um grupo. O primeiro grupo vira o padrão (`~/.claude`); os seguintes
    /// ganham perfil dedicado. Qual é o padrão pode ser mudado depois.
    @discardableResult
    public func addGroup(name: String, provider: Provider = .anthropic) -> AccountGroup {
        let isFirst = config.groups.isEmpty
        let id = UUID()
        let group = AccountGroup(
            id: id, name: name, provider: provider,
            configDir: paths.groupConfigDir(id, isDefault: isFirst))
        config.groups.append(group)
        save()
        return group
    }

    public func renameGroup(_ id: UUID, to name: String) {
        guard let i = config.groups.firstIndex(where: { $0.id == id }) else { return }
        config.groups[i].name = name
        save()
    }

    public func setThreshold(_ id: UUID, percent: Double) {
        guard let i = config.groups.firstIndex(where: { $0.id == id }) else { return }
        config.groups[i].thresholdPercent = min(100, max(50, percent))
        save()
    }

    public func setAutoRotate(_ id: UUID, _ on: Bool) {
        guard let i = config.groups.firstIndex(where: { $0.id == id }) else { return }
        config.groups[i].autoRotate = on
        save()
    }

    /// Reordena as contas de um grupo — a ordem é a preferência de rotação.
    /// Recebe a lista já reordenada pela UI, para não depender do
    /// `move(fromOffsets:)` do SwiftUI (que o core não importa).
    public func reorderAccounts(in groupID: UUID, to ordered: [UUID]) {
        guard let i = config.groups.firstIndex(where: { $0.id == groupID }) else { return }
        // Mantém só os IDs que o grupo já tinha, na ordem pedida — ignora
        // qualquer id estranho que a UI tenha passado por engano.
        let known = Set(config.groups[i].accountIDs)
        config.groups[i].accountIDs = ordered.filter(known.contains)
        save()
    }

    public func removeGroup(_ id: UUID) {
        // Leva junto as contas que só existiam neste grupo: órfãs não aparecem
        // em tela nenhuma e ficariam ocupando a config para sempre. A credencial
        // de cada uma sai junto — ver `removeAccount`.
        let exclusive = Set(config.groups.first { $0.id == id }?.accountIDs ?? [])
            .subtracting(config.groups.filter { $0.id != id }.flatMap(\.accountIDs))
        config.groups.removeAll { $0.id == id }
        // Pelo mesmo caminho da remoção avulsa, para a credencial sair junto.
        for accountID in exclusive { removeAccount(accountID) }
        save()
    }

    /// Tira o status de padrão de todos os grupos: nenhum passa a usar o
    /// `~/.claude`, cada um fica no seu perfil dedicado. Isto protege sessões que
    /// já rodam no `~/.claude` (o `claude` puro) — o router deixa de tocar lá.
    public func clearDefault() {
        for i in config.groups.indices where config.groups[i].configDir.isDefault {
            config.groups[i].configDir = paths.groupConfigDir(
                config.groups[i].id, isDefault: false)
        }
        save()
    }

    /// Torna um grupo o padrão (`~/.claude`), tirando o padrão de quem era. No
    /// máximo um grupo é padrão.
    public func makeDefault(_ id: UUID) {
        for i in config.groups.indices {
            let wasDefault = config.groups[i].configDir.isDefault
            let shouldBeDefault = config.groups[i].id == id
            if wasDefault != shouldBeDefault {
                config.groups[i].configDir = paths.groupConfigDir(
                    config.groups[i].id, isDefault: shouldBeDefault)
            }
        }
        save()
    }

    // MARK: - Contas

    /// Reserva a casa de uma conta nova (o perfil onde ela fará login), sem
    /// lançar nada. Quem conduz o login (o app, in-app) usa este caminho; o
    /// resultado é confirmado depois por `finishPendingLogin`.
    public func newAccountHome() -> (home: ConfigDir, accountID: UUID) {
        let id = UUID()
        let home = paths.accountHome(id)
        try? FileManager.default.createDirectory(at: home.url, withIntermediateDirectories: true)
        return (home, id)
    }

    /// Caminho do perfil onde uma conta deve logar — para o app apontar o
    /// `CLAUDE_CONFIG_DIR` do login oficial.
    public func homePath(_ home: ConfigDir) -> String { home.raw }

    /// O resultado de checar um login pendente.
    public enum LoginOutcome: Sendable, Equatable {
        /// Ainda não terminou; continue observando.
        case pending
        /// Entrou uma conta nova; foi adicionada ao grupo.
        case added(Account)
        /// O login trouxe uma conta que já está no grupo — quase sempre porque o
        /// navegador ainda estava logado nela. O e-mail diz qual.
        case duplicate(email: String)
    }

    /// Confere se um login pendente terminou. Distingue "ainda não" de "veio a
    /// mesma conta de novo", que é o caso comum quando o navegador não pediu para
    /// escolher a conta.
    @discardableResult
    public func finishPendingLogin(home: ConfigDir, accountID: UUID,
                                   into groupID: UUID, provider: Provider = .anthropic)
        -> LoginOutcome {
        guard let identity = login.loginResult(inHome: home, keychain: keychain) else {
            return .pending
        }
        // A mesma conta já está neste grupo: rejeita e diz o e-mail, para a UI
        // explicar (provavelmente a sessão do navegador não trocou).
        let inGroup = config.groups.first { $0.id == groupID }?.accountIDs ?? []
        if config.accounts.contains(where: {
            $0.identity.email == identity.email && inGroup.contains($0.id)
        }) {
            return .duplicate(email: identity.email)
        }
        let account = Account(id: accountID, provider: provider,
                              identity: identity, home: home)
        config.accounts.append(account)
        if let i = config.groups.firstIndex(where: { $0.id == groupID }) {
            config.groups[i].accountIDs.append(account.id)
        }
        save()
        return .added(account)
    }

    /// O resultado de checar um relogin pendente.
    public enum ReloginOutcome: Sendable, Equatable {
        /// Ainda não terminou; continue observando.
        case pending
        /// A credencial nova entrou, na mesma conta. Se ela estava ativa num
        /// grupo, o item do grupo já recebeu a credencial nova.
        case renewed(Account)
        /// O navegador logou OUTRA conta. Nada do registro muda; a UI explica
        /// e oferece tentar de novo.
        case wrongAccount(expected: String, got: String)
    }

    /// Confere se o relogin de uma conta existente terminou. Relogin reusa a
    /// MESMA casa (o `claude auth login` sobrescreve credencial e identidade),
    /// então sucesso aqui significa: casa com identidade + credencial de novo.
    @discardableResult
    public func finishRelogin(accountID: UUID) -> ReloginOutcome {
        guard let i = config.accounts.firstIndex(where: { $0.id == accountID }),
              let identity = login.loginResult(inHome: config.accounts[i].home,
                                               keychain: keychain)
        else { return .pending }

        let expected = config.accounts[i].identity.email
        guard identity.email == expected else {
            return .wrongAccount(expected: expected, got: identity.email)
        }
        config.accounts[i].identity = identity  // tier/organização podem ter mudado
        save()
        // Conta ativa num grupo: o item do grupo guarda a credencial MORTA que
        // motivou o relogin — a nova precisa ir por cima, senão a próxima
        // sessão continua no "Login expired".
        for group in config.groups
        where engine.activeAccount(in: group, config: config)?.id == accountID {
            engine.pushHomeToGroup(config.accounts[i], in: group)
        }
        refreshUsage()
        return .renewed(config.accounts[i])
    }

    /// Remove a conta do registro **e apaga a credencial dela**.
    ///
    /// Até 18/09/2026 só o registro saía: o item de chaveiro e a pasta da casa
    /// ficavam para trás, com um refresh token vivo, para sempre. Num produto
    /// pago isso é promessa quebrada — "remover a conta" tem de remover a conta.
    ///
    /// O item do GRUPO não é tocado de propósito, mesmo quando é esta conta que
    /// o serve: pode haver sessão viva atendida por ele agora, e derrubar o
    /// trabalho de alguém não é consequência aceitável de arrumar a lista. Ele é
    /// sobrescrito na próxima ativação, e sem a conta no registro ninguém mais a
    /// escolhe.
    public func removeAccount(_ accountID: UUID) {
        guard let account = config.account(accountID) else { return }
        let service = engine.homeKeychainService(for: account)

        config.accounts.removeAll { $0.id == accountID }
        for i in config.groups.indices {
            config.groups[i].accountIDs.removeAll { $0 == accountID }
        }
        save()

        keychain.delete(service: service)
        try? FileManager.default.removeItem(at: account.home.url)
    }

    public func setNickname(_ accountID: UUID, _ nickname: String?) {
        guard let i = config.accounts.firstIndex(where: { $0.id == accountID }) else { return }
        config.accounts[i].nickname = nickname?.isEmpty == true ? nil : nickname
        save()
    }

    // MARK: - Rotação (ações que tocam o sistema)

    /// A conta que serve um grupo agora.
    public func activeAccount(in group: AccountGroup) -> Account? {
        engine.activeAccount(in: group, config: config)
    }

    /// Troca manual: ativa a conta escolhida no grupo. Erros viram `lastError`.
    /// Relê a conta ativa em seguida, para o indicador da UI se mover na hora —
    /// senão o clique parece não fazer nada.
    public func activate(_ account: Account, in group: AccountGroup) {
        do {
            try engine.activate(account, in: group, config: config)
            lastError = nil
            refreshUsage()
        } catch {
            lastError = "não foi possível trocar de conta: \(error)"
        }
    }

    /// Mede as contas de um grupo com a **sonda ativa**, e publica o resultado.
    ///
    /// É o que o sensor passivo não consegue: conta ociosa nunca serviu
    /// mensagem nenhuma, então não tem amostra — aparecia como "pronta", sem
    /// número, e a rotação a presumia fresca. A sonda pergunta ao binário
    /// oficial e traz o quadro inteiro, inclusive o limite POR MODELO, que não
    /// chega no `rate_limits` da status line e é o que estoura primeiro.
    ///
    /// Sob demanda, e não no laço: cada conta custa um processo Node subindo do
    /// zero e uma requisição de verdade. Medir cinco contas leva ~15s.
    public func measureAccounts(in group: AccountGroup) async {
        guard measuringGroup == nil else { return }
        measuringGroup = group.id
        defer { measuringGroup = nil }

        // O perfil de cada conta é decidido AQUI, com o config na mão: conta
        // ativa vai pelo perfil do grupo, nunca pela casa (ver `probeConfigDir`
        // — sondar a casa de uma conta ativa derruba a sessão viva).
        let alvos = config.accounts(in: group).map {
            (email: $0.identity.email, dir: engine.probeConfigDir(for: $0, config: config))
        }
        let usageDir = paths.usageDir
        let scratchBase = paths.base

        let falhas = await Task.detached(priority: .userInitiated) { () -> Int in
            guard let probe = ClaudeUsageProbe.system(scratchBase: scratchBase) else { return -1 }
            var erros = 0
            for alvo in alvos {
                do {
                    let leitura = try probe.read(configDir: alvo.dir)
                    let agora = Date()
                    let amostra = GroupUsageSample(
                        configDirRaw: alvo.dir.raw, email: alvo.email,
                        fiveHourPercent: leitura.session,
                        fiveHourResetsAt: leitura.sessionResetsAt,
                        sevenDayPercent: leitura.weeklyAll,
                        sevenDayResetsAt: leitura.weeklyAllResetsAt,
                        sampledAt: agora,
                        models: ModelUsage(windows: leitura.models, sampledAt: agora),
                        origin: .probe)
                    try GroupUsageStore.write(amostra, forEmail: alvo.email, in: usageDir)
                } catch {
                    erros += 1
                }
            }
            return erros
        }.value

        // Texto cru, como o resto dos `lastError` deste arquivo: o core não
        // carrega catálogo (a CLI `router` o linka e não tem bundle de app).
        switch falhas {
        case -1: lastError = "binário `claude` não encontrado — a sonda precisa dele"
        case 0: lastError = nil
        default: lastError = "\(falhas) conta(s) não responderam à sonda — veja se precisam de Relogar"
        }
        refreshUsage()
    }

    /// Relê o uso do sensor e a conta ativa de cada grupo, e publica ambos.
    /// Chamado pelo laço do app, e ao abrir a lista de grupos.
    public func refreshUsage() {
        let detail = usage.detailByAccount(config)
        usageDetail = detail
        usageSnapshot = detail.mapValues(\.fraction)
        usageSampledAt = usage.samplesByAccount(config).mapValues(\.sampledAt)
        var active: [UUID: UUID] = [:]
        var sessions: [UUID: [LiveSession]] = [:]
        for group in config.groups {
            if let a = engine.activeAccount(in: group, config: config) { active[group.id] = a.id }
            let vivas = SessionRegistry.liveSessions(in: group.configDir)
            if !vivas.isEmpty { sessions[group.id] = vivas }
        }
        activeByGroup = active
        liveSessions = sessions
    }

    /// Quantas sessões vivas um grupo tem agora.
    public func sessionCount(in groupID: UUID) -> Int { liveSessions[groupID]?.count ?? 0 }

    // MARK: - Integração com o terminal

    /// O arquivo de shell que o usuário passa a dar `source`.
    public var shellScriptURL: URL { paths.base.appending(path: "shell.sh") }

    /// A linha que ativa a integração no shell (adicionada automaticamente).
    public var shellSourceLine: String { "source \"\(shellScriptURL.path)\"" }

    /// O profile do shell onde a linha é adicionada (zsh é o padrão do macOS).
    public var profileURL: URL {
        FileManager.default.homeDirectoryForCurrentUser.appending(path: ".zshrc")
    }

    /// `true` quando a linha da integração já está no `~/.zshrc`.
    public var profileHasIntegration: Bool {
        let resolved = profileURL.resolvingSymlinksInPath()
        guard let content = try? String(contentsOf: resolved, encoding: .utf8) else { return false }
        return content.contains(shellScriptURL.path)
    }

    /// `true` quando a integração já foi escrita (script + linha no profile).
    public var shellIntegrationInstalled: Bool {
        FileManager.default.fileExists(atPath: shellScriptURL.path) && profileHasIntegration
    }

    /// Escreve a função de shell e planta a status line do sensor em cada perfil
    /// de grupo. Idempotente. Devolve a linha de `source` para a UI mostrar, ou
    /// `nil` se o caminho do binário não é conhecido.
    @discardableResult
    public func installShellIntegration() -> String? {
        guard let routerPath else {
            lastError = "não foi possível localizar o binário do app"
            return nil
        }
        do {
            try FileManager.default.createDirectory(
                at: paths.base, withIntermediateDirectories: true)
            try ShellIntegration.shellFunction(routerPath: routerPath)
                .write(to: shellScriptURL, atomically: true, encoding: .utf8)
            // Planta a status line em cada perfil de grupo, preservando o
            // settings.json que já houver (o padrão ~/.claude tem `model` etc.),
            // e compartilha histórico + skills para o `--resume` funcionar.
            for group in config.groups {
                try? ShellIntegration.installStatusLine(
                    routerPath: routerPath, into: group.configDir)
                ProfileSharing.link(into: group.configDir, shareHistory: config.shareHistory)
            }
            // Adiciona a linha ao ~/.zshrc automaticamente — é o que torna isto
            // plug-and-play: o usuário não edita arquivo nenhum.
            try? ShellIntegration.ensureInProfile(
                sourceLine: shellSourceLine, scriptPath: shellScriptURL.path,
                profileURL: profileURL)
            lastError = nil
            integrationInstalled = shellIntegrationInstalled
            return shellSourceLine
        } catch {
            lastError = "não foi possível instalar a integração: \(error)"
            integrationInstalled = shellIntegrationInstalled
            return nil
        }
    }

    /// A integração instalada aponta para um binário que não é mais este.
    ///
    /// O `shell.sh` e a `statusLine` de cada perfil guardam o caminho
    /// **absoluto** do `router`, e o `router` mora dentro do `.app`. Mover,
    /// renomear ou reinstalar o app noutro lugar deixa os dois apontando para um
    /// caminho que não existe — e o modo de falha é silencioso, que é o que o
    /// torna grave: a função de shell não acha o binário, o `is-group` falha, e
    /// `claude trabalho` cai no `command claude` do final, abrindo a sessão no
    /// `~/.claude`, **na conta errada**, sem nenhum aviso.
    public var integrationIsStale: Bool {
        guard let routerPath,
              FileManager.default.fileExists(atPath: shellScriptURL.path)
        else { return false }  // não instalada não é obsoleta; é ausente.
        let script = (try? String(contentsOf: shellScriptURL, encoding: .utf8)) ?? ""
        if !script.contains(routerPath) { return true }
        return config.groups.contains {
            ShellIntegration.statusLineIsStale(routerPath: routerPath, in: $0.configDir)
        }
    }

    /// Reescreve a integração quando ela ficou obsoleta. Devolve `true` se
    /// mexeu em alguma coisa.
    ///
    /// Automático, e não um botão, justamente porque a falha é silenciosa: quem
    /// foi atingido é quem não tem como saber que precisa apertá-lo. Roda na
    /// subida do app, depois que o `routerPath` é conhecido.
    @discardableResult
    public func healShellIntegration() -> Bool {
        guard integrationIsStale else { return false }
        return installShellIntegration() != nil
    }

    /// Roda uma volta da rotação automática em todos os grupos: cada grupo que
    /// passou do limiar e tem para onde ir, troca. Fail-safe: quem não tem
    /// destino fica onde está. Usa o `usageSnapshot` mais recente.
    public func rotateAll() {
        for group in config.groups {
            // O espelhamento periódico: a casa da conta ativa recebe o token
            // vivo do grupo, senão o refresh token dela morre na prateleira.
            engine.mirrorActive(in: group, config: config)
            guard let target = engine.rotationTarget(
                for: group, config: config, usage: usageSnapshot) else { continue }
            activate(target, in: group)
        }
    }

    // MARK: - A escolha da status line

    /// Liga ou desliga um item da linha. Grava na hora: a CLI relê a escolha a
    /// cada render, então o efeito aparece na próxima atualização da sessão,
    /// sem reabrir nada e sem reinstalar a integração.
    public func setStatusLineItem(_ item: StatusLineChoice.Item, shown: Bool) {
        statusLineChoice.setShown(item, shown)
        saveStatusLineChoice()
    }

    /// Volta à linha completa.
    public func restoreStatusLine() {
        statusLineChoice = StatusLineChoice()
        saveStatusLineChoice()
    }

    private func saveStatusLineChoice() {
        do {
            try statusLineChoice.save(to: StatusLineChoice.fileURL(base: paths.base))
            lastError = nil
        } catch {
            lastError = "não foi possível salvar a escolha da status line: \(error)"
        }
    }
}
