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
    public internal(set) var config: RouterConfig
    /// Uso 0–1 por conta, lido do sensor. Observável, para a lista de grupos
    /// redesenhar as barras quando o número muda.
    public internal(set) var usageSnapshot: [UUID: Double] = [:]
    /// Quando a amostra de cada conta foi colhida — a idade que a UI mostra
    /// para nunca fingir um frescor que o sensor passivo não tem.
    public internal(set) var usageSampledAt: [UUID: Date] = [:]
    /// O mesmo uso de `usageSnapshot`, mas com a janela de onde o número veio e
    /// as duas medidas. É o que deixa a UI rotular "66% · 7d" em vez de um "66%"
    /// solto que o usuário lê como sendo o das 5 horas.
    public internal(set) var usageDetail: [UUID: AccountUsage] = [:]
    /// As sessões do Claude Code vivas em cada grupo (grupo → sessões).
    ///
    /// Vem do registro que o próprio Claude Code escreve em
    /// `<perfil>/sessions/`, que é POR PERFIL — então dá para dizer qual sessão
    /// roda em qual grupo, e por qual conta ela está sendo atendida. É a
    /// resposta que o `/status` não dá, e o contra-veneno do modo de falha
    /// silencioso: sessão que devia estar num grupo e subiu no perfil padrão
    /// aparece do lado errado da lista.
    public internal(set) var liveSessions: [UUID: [LiveSession]] = [:]
    /// A conta ativa de cada grupo (grupo → conta). Observável, para a barra e o
    /// painel mostrarem quem serve cada grupo sem reler o disco a cada frame.
    public internal(set) var activeByGroup: [UUID: UUID] = [:]
    /// As contas que se mostraram SEM LOGIN, com quando: a sonda não achou login
    /// no perfil, ou "Usar" não achou credencial na casa. É o selo amarelo da
    /// tela — o aviso da medição dizia só QUANTAS, e não havia como saber qual
    /// relogar. Sai quando a conta prova o contrário (responde à sonda, relogin
    /// renovado, amostra do sensor mais nova que a marca). Só em memória: o
    /// formato em disco é o contrato com o Windows.
    public internal(set) var signedOut: [UUID: Date] = [:]
    /// Última falha de uma ação, para a UI mostrar. `nil` quando tudo correu bem.
    public internal(set) var lastError: String?
    /// Contas com relogin em andamento. A rotação não espelha a ativa delas
    /// enquanto isso: o relogin grava a credencial NOVA na casa, e o espelho
    /// (grupo → casa) escreveria por cima a MORTA que motivou o relogin — que o
    /// fim do relogin empurraria de volta para o grupo (issue #12).
    @ObservationIgnored var reloginInFlight: Set<UUID> = []
    /// O grupo que está sendo sondado agora, ou `nil`. Observável porque a
    /// sondagem leva segundos — sem indicação, o clique parece não fazer nada.
    /// A agenda de 5 em 5 horas (`RouterConfigStore+History.swift`) usa a mesma
    /// marca, e por isso o `set` é do módulo, não do arquivo.
    public internal(set) var measuringGroup: UUID?
    /// O histórico de cada conta (conta → pontos), o que a aba Histórico
    /// desenha (21/10/2026). Observável e em memória, relido no `refreshUsage`:
    /// uma computada que lesse o disco não re-renderizaria a tela.
    public internal(set) var historyByAccount: [UUID: [UsageHistoryPoint]] = [:]
    /// A última tentativa da medição automática em cada conta, nesta execução
    /// do app — o que segura uma conta deslogada (ver `MeasureSchedule`).
    @ObservationIgnored public internal(set) var measureAttempts: [UUID: Date] = [:]
    /// Um login (ou relogin) está aberto na tela: a agenda espera. A sonda
    /// poderia mexer na casa que está sendo relogada. Quem liga e desliga é a
    /// `LoginSheet`, do aparecer ao sumir.
    @ObservationIgnored public var signingIn = false
    /// Espelho OBSERVÁVEL de `shellIntegrationInstalled`: aquele é computado
    /// lendo o disco, e computado não dispara re-render — o clique em "Ativar"
    /// funcionava e a tela ficava parada, parecendo botão morto.
    public internal(set) var integrationInstalled = false

    /// O que a status line dos grupos mostra. **Observável e em memória**: uma
    /// computada que lesse o disco não re-renderizaria a tela ao mudar — foi
    /// exatamente o bug do botão "Ativar". O disco é a cópia, não a fonte da UI.
    public internal(set) var statusLineChoice = StatusLineChoice()
    /// Caminho do binário `router` empacotado, que a integração de shell e a
    /// status line apontam. Setado pelo app ao iniciar; `nil` fora do app.
    @ObservationIgnored public var routerPath: String?

    /// `true` só quando o `config.json` existia e foi lido inteiro. É a trava
    /// das limpezas que apagam o que o registro não cita: com o registro vazio
    /// por falha de leitura, tudo pareceria órfão.
    @ObservationIgnored var configLoadedFromDisk = false
    /// Um `config.json` que não pôde ser lido NEM guardado de lado nunca é
    /// sobrescrito — ver `init`.
    @ObservationIgnored private var refuseToSave = false

    // Do módulo, e não do arquivo: o store mora em cinco arquivos (este e as
    // extensões `+Accounts`, `+Measure`, `+Terminal` e `+History`), pela régua
    // das 600 linhas. Fora do módulo, nada disto aparece.
    @ObservationIgnored let paths: RouterPaths
    @ObservationIgnored let engine: RotationEngine
    @ObservationIgnored let keychain: any KeychainStore
    @ObservationIgnored let login: AccountLoginService
    @ObservationIgnored let usage: GroupUsageReader

    public init(paths: RouterPaths = RouterPaths(),
                keychain: any KeychainStore = SecurityCLIKeychain(),
                adapters: [any ProviderAdapter] = [AnthropicAdapter()]) {
        self.paths = paths
        self.keychain = keychain
        self.engine = RotationEngine(keychain: keychain, adapters: adapters)
        self.login = AccountLoginService(adapter: adapters.first ?? AnthropicAdapter(), paths: paths)
        self.usage = GroupUsageReader(usageDir: paths.usageDir)
        // Um `config.json` que existe e não decodifica (edição à mão, uma versão
        // mais nova que gravou um campo que esta não conhece) virava registro
        // vazio, sem aviso, e o primeiro `save` gravava por cima: grupos e
        // contas sumiam do registro, com casas e itens de chaveiro órfãos.
        // Agora ele vai para o lado com data, e a tela diz onde está.
        var loaded = RouterConfig()
        var problem: String?
        switch Self.load(from: paths.configFile) {
        case .missing:
            break
        case .loaded(let config):
            loaded = config
            self.configLoadedFromDisk = true
        case .unreadable:
            let stamp = ISO8601DateFormatter().string(from: Date())
                .replacingOccurrences(of: ":", with: "-")
            let aside = paths.base.appending(path: "config.unreadable-\(stamp).json")
            if (try? FileManager.default.moveItem(at: paths.configFile, to: aside)) != nil {
                problem = "o config.json não pôde ser lido e foi guardado como \(aside.lastPathComponent) — os grupos começam vazios"
            } else {
                self.refuseToSave = true
                problem = "o config.json não pôde ser lido; nada será gravado por cima dele"
            }
        }
        self.config = loaded
        self.lastError = problem
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

    private enum Loaded { case missing, loaded(RouterConfig), unreadable }

    private static func load(from url: URL) -> Loaded {
        guard FileManager.default.fileExists(atPath: url.path) else { return .missing }
        guard let data = try? Data(contentsOf: url),
              let config = try? JSONDecoder().decode(RouterConfig.self, from: data)
        else { return .unreadable }
        return .loaded(config)
    }

    func save() {
        guard !refuseToSave else { return }
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
        guard let group = config.groups.first(where: { $0.id == id }) else { return }
        // A casa da ativa recebe o token vivo ANTES de o grupo sumir — uma conta
        // que outro grupo mantém continua com a cadeia em dia.
        engine.mirrorActive(in: group, config: config)
        // Leva junto as contas que só existiam neste grupo: órfãs não aparecem
        // em tela nenhuma e ficariam ocupando a config para sempre. A credencial
        // de cada uma sai junto — ver `removeAccount`.
        let exclusive = Set(group.accountIDs)
            .subtracting(config.groups.filter { $0.id != id }.flatMap(\.accountIDs))
        config.groups.removeAll { $0.id == id }
        // Pelo mesmo caminho da remoção avulsa, para a credencial sair junto.
        for accountID in exclusive { removeAccount(accountID) }
        save()
        // O item do grupo e a pasta do perfil dedicado saem também: sem o
        // grupo, ninguém mais os sobrescreve, e o item guardava um refresh
        // token vivo para sempre. Com sessão viva lá, ficam — derrubar o
        // trabalho de alguém não é consequência aceitável de arrumar a lista.
        // O `~/.claude` nunca (o login dele é do `claude` puro).
        if !group.configDir.isDefault,
           SessionRegistry.liveSessions(in: group.configDir).isEmpty {
            engine.resetProfile(group.configDir, provider: group.provider)
            try? FileManager.default.removeItem(at: group.configDir.url)
        }
    }

    /// Tira o status de padrão de todos os grupos: nenhum passa a usar o
    /// `~/.claude`, cada um fica no seu perfil dedicado. Isto protege sessões que
    /// já rodam no `~/.claude` (o `claude` puro) — o router deixa de tocar lá, e
    /// a conta que estava lá fica fora da rotação enquanto o `~/.claude` for
    /// dela (ver `RotationEngine.whereElseLive`).
    public func clearDefault() {
        for i in config.groups.indices where config.groups[i].configDir.isDefault {
            moveGroup(at: i, toDefault: false)
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
                moveGroup(at: i, toDefault: shouldBeDefault)
            }
        }
        save()
    }

    /// Troca o perfil de um grupo sem deixar uma conta viva em dois lugares.
    ///
    /// Até 10/2026 trocar o padrão só mudava o caminho, e a rotação, na volta
    /// seguinte, ativava no perfil novo a mesma conta que continuava viva no
    /// antigo — duas cópias de um refresh token que gira. E o perfil dedicado de
    /// um grupo é sempre o mesmo caminho: voltando a ele, a identidade velha que
    /// ficou lá fazia o espelho gravar uma cadeia morta na casa da conta.
    ///
    /// Então: espelha a conta ativa (a casa fica com o token vivo); esvazia o
    /// perfil dedicado de destino (nada velho entra) e o dedicado que fica para
    /// trás (nada vivo sobra). O preço, deliberado: uma sessão que ainda rode
    /// no dedicado que ficou para trás perde a credencial na próxima
    /// requisição — melhor que uma conta morta em silêncio. O `~/.claude` nunca
    /// é esvaziado: o login dele é do `claude` puro.
    private func moveGroup(at i: Int, toDefault: Bool) {
        let group = config.groups[i]
        engine.mirrorActive(in: group, config: config)
        let target = paths.groupConfigDir(group.id, isDefault: toDefault)
        engine.resetProfile(target, provider: group.provider)
        engine.resetProfile(group.configDir, provider: group.provider)
        config.groups[i].configDir = target
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
        // Durante a medição a sonda decide, conta a conta, por qual perfil
        // medir; ativar alguém no meio a faria sondar a casa de uma conta ativa.
        guard measuringGroup == nil else {
            lastError = "espere a medição terminar para trocar de conta"
            return
        }
        do {
            try engine.activate(account, in: group, config: config)
            lastError = nil
            refreshUsage()
        } catch {
            // Sem credencial na casa a conta está sem login neste Mac — a mesma
            // marca que a sonda poria. Achar a credencial não tira a marca: ela
            // pode estar vencida.
            if case .noCredential(let id)? = error as? RotationEngine.RotationError {
                signedOut[id] = Date()
            }
            lastError = "não foi possível trocar de conta: \(error)"
        }
    }

    /// A conta se mostrou sem login e ainda não provou o contrário.
    public func isSignedOut(_ accountID: UUID) -> Bool { signedOut[accountID] != nil }

    /// Relê o uso do sensor e a conta ativa de cada grupo, e publica ambos —
    /// e, desde 21/10/2026, o histórico de cada conta, para a aba Histórico
    /// ganhar o ponto novo sem esperar. Chamado pelo laço do app, e ao abrir a
    /// lista de grupos.
    public func refreshUsage() {
        let detail = usage.detailByAccount(config)
        usageDetail = detail
        usageSnapshot = detail.mapValues(\.fraction)
        usageSampledAt = usage.samplesByAccount(config).mapValues(\.sampledAt)
        // Amostra mais nova que a marca: a conta serviu uma sessão DEPOIS de se
        // mostrar sem login (um `/login` no terminal, sem o app) — o login
        // voltou. A data vai ao disco em ISO-8601 sem fração de segundo, então
        // uma do mesmo segundo da marca não conta: fica para a próxima.
        let amostras = usageSampledAt
        let mantidas = signedOut.filter { marca in
            guard let amostra = amostras[marca.key] else { return true }
            return amostra <= marca.value
        }
        if mantidas.count != signedOut.count { signedOut = mantidas }
        var active: [UUID: UUID] = [:]
        var sessions: [UUID: [LiveSession]] = [:]
        for group in config.groups {
            if let a = engine.activeAccount(in: group, config: config) { active[group.id] = a.id }
            let vivas = SessionRegistry.liveSessions(in: group.configDir)
            if !vivas.isEmpty { sessions[group.id] = vivas }
        }
        activeByGroup = active
        liveSessions = sessions
        refreshHistory()
    }

    /// Roda uma volta da rotação automática em todos os grupos: cada grupo que
    /// passou do limiar e tem para onde ir, troca. Fail-safe: quem não tem
    /// destino fica onde está. Usa o `usageSnapshot` mais recente.
    public func rotateAll() {
        // A sonda decide o perfil de cada conta na hora de medi-la; trocar a
        // ativa no meio a faria medir pela casa uma conta que acabou de ficar
        // ativa. A volta seguinte, três minutos depois, faz o que esta não fez.
        guard measuringGroup == nil else { return }
        for group in config.groups {
            // Relogin da ativa em andamento: nem espelho nem troca neste grupo
            // até ele terminar — ver `reloginInFlight`.
            if let active = engine.activeAccount(in: group, config: config),
               reloginInFlight.contains(active.id) { continue }
            // O espelhamento periódico: a casa da conta ativa recebe o token
            // vivo do grupo, senão o refresh token dela morre na prateleira.
            engine.mirrorActive(in: group, config: config)
            guard let target = engine.rotationTarget(
                for: group, config: config, usage: usageSnapshot) else { continue }
            activate(target, in: group)
        }
    }
}
