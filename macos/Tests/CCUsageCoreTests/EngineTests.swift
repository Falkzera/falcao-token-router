import Foundation
import Testing
@testable import CCUsageCore

// MARK: - Chaveiro em memória, para exercitar o motor sem tocar no sistema.

final class FakeKeychain: KeychainStore, @unchecked Sendable {
    private let lock = NSLock()
    private var items: [String: String]
    private(set) var writes: [(service: String, secret: String)] = []

    init(_ initial: [String: String] = [:]) { self.items = initial }

    func read(service: String) -> String? {
        lock.lock(); defer { lock.unlock() }
        return items[service]
    }
    /// Itens cuja escrita falha — para exercitar o espelho que não grava.
    var failWritesTo: Set<String> = []

    func write(_ secret: String, service: String) throws {
        lock.lock(); defer { lock.unlock() }
        if failWritesTo.contains(service) { throw CocoaError(.fileWriteNoPermission) }
        items[service] = secret
        writes.append((service, secret))
    }
    func exists(service: String) -> Bool {
        lock.lock(); defer { lock.unlock() }
        return items[service] != nil
    }
    func delete(service: String) {
        lock.lock(); defer { lock.unlock() }
        items[service] = nil
        deletes.append(service)
    }
    private(set) var deletes: [String] = []
}

/// Adapter falso: identidade e chaveiro em memória, para o motor não precisar de
/// arquivo nem de `security`. O nome do item segue a mesma regra do real.
final class FakeAdapter: ProviderAdapter, @unchecked Sendable {
    let provider: Provider = .anthropic
    private let lock = NSLock()
    private var identities: [String: AccountIdentity] = [:]  // por configDir.raw

    func keychainService(forConfigDir dir: ConfigDir) -> String {
        dir.isDefault ? "svc-default" : "svc-\(dir.keychainHash)"
    }
    func identity(inConfigDir dir: ConfigDir) -> AccountIdentity? {
        lock.lock(); defer { lock.unlock() }
        return identities[dir.raw]
    }
    /// Liga a falha na gravação da identidade (o `.claude.json` que não grava).
    var failIdentityWrites = false

    func writeIdentity(_ identity: AccountIdentity, toConfigDir dir: ConfigDir) throws {
        lock.lock(); defer { lock.unlock() }
        if failIdentityWrites { throw CocoaError(.fileWriteNoPermission) }
        identities[dir.raw] = identity
    }
    func clearIdentity(inConfigDir dir: ConfigDir) throws {
        lock.lock(); defer { lock.unlock() }
        identities[dir.raw] = nil
    }
    func launchCommand() -> (executable: String, arguments: [String]) { ("claude", []) }
}

private func ident(_ email: String, tier: String = "default_claude_max_5x") -> AccountIdentity {
    AccountIdentity(email: email, organizationName: "Acme",
                    rateLimitTier: tier, raw: ["emailAddress": .string(email)])
}

// MARK: - Derivação do item de chaveiro (o contrato com o Claude Code)

@Suite("ConfigDir e nome de item")
struct ConfigDirTests {
    let adapter = AnthropicAdapter()

    /// O item real do perfil padrão, medido no chaveiro, é sem sufixo.
    @Test("perfil padrão usa o item sem sufixo")
    func defaultService() {
        let dir = ConfigDir.standard(home: "/Users/exemplo")
        #expect(adapter.keychainService(forConfigDir: dir) == "Claude Code-credentials")
        #expect(dir.environmentValue == nil)
        // O .claude.json do padrão fica AO LADO, não dentro.
        #expect(dir.globalConfigURL.path == "/Users/exemplo/.claude.json")
    }

    /// O formato foi conferido contra itens REAIS do chaveiro em agosto/2026;
    /// os caminhos aqui foram trocados por neutros quando o repositório abriu, e
    /// os valores recalculados. O que o teste prova continua sendo o mesmo: NFC,
    /// sha256 da string crua, oito hex.
    @Test("perfil dedicado deriva o hash que o Claude Code usa")
    func hashedService() {
        let dir = ConfigDir.dedicated("/Users/exemplo/.claude-trabalho")
        #expect(dir.keychainHash == "1c369ada")
        #expect(adapter.keychainService(forConfigDir: dir)
                == "Claude Code-credentials-1c369ada")
        #expect(dir.environmentValue == "/Users/exemplo/.claude-trabalho")
        // Com CLAUDE_CONFIG_DIR setado, o .claude.json fica DENTRO.
        #expect(dir.globalConfigURL.path == "/Users/exemplo/.claude-trabalho/.claude.json")
    }

    @Test("um segundo caminho confere também")
    func hashedSecondPath() {
        #expect(ConfigDir.dedicated("/Users/exemplo/.claude-pessoal").keychainHash == "3cb04cd6")
    }

    /// O hash é da string crua, não do caminho resolvido: uma barra final muda o
    /// item, e é por isso que se guarda a string exata.
    @Test("hash é sensível à string exata")
    func rawSensitive() {
        let a = ConfigDir.dedicated("/Users/exemplo/.claude-trabalho").keychainHash
        let b = ConfigDir.dedicated("/Users/exemplo/.claude-trabalho/").keychainHash
        #expect(a != b)
    }
}

// MARK: - A troca em si

@Suite("RotationEngine")
struct RotationEngineTests {
    private func setup() -> (RotationEngine, FakeKeychain, FakeAdapter,
                             RouterConfig, Account, Account, AccountGroup) {
        let kc = FakeKeychain()
        let adapter = FakeAdapter()
        let engine = RotationEngine(keychain: kc, adapters: [adapter])

        let contaA = Account(provider: .anthropic, identity: ident("conta-a@exemplo.com"),
                           home: .dedicated("/Users/exemplo/.claude-pessoal"))
        let contaB = Account(provider: .anthropic, identity: ident("conta-b@exemplo.com"),
                           home: .dedicated("/Users/exemplo/.claude-trabalho"))
        // Cada conta tem credencial na sua casa.
        try? kc.write("cred-a", service: adapter.keychainService(forConfigDir: contaA.home))
        try? kc.write("cred-b", service: adapter.keychainService(forConfigDir: contaB.home))

        let group = AccountGroup(name: "trabalho", accountIDs: [contaA.id, contaB.id],
                                 configDir: .standard(home: "/Users/exemplo"))
        let config = RouterConfig(accounts: [contaA, contaB], groups: [group])
        return (engine, kc, adapter, config, contaA, contaB, group)
    }

    @Test("ativar copia o segredo da casa para o item do grupo e grava a identidade")
    func activateCopies() throws {
        let (engine, kc, adapter, config, contaA, _, group) = setup()
        try engine.activate(contaA, in: group, config: config)

        let groupService = adapter.keychainService(forConfigDir: group.configDir)
        #expect(kc.read(service: groupService) == "cred-a")
        #expect(adapter.identity(inConfigDir: group.configDir)?.email
                == "conta-a@exemplo.com")
        #expect(engine.activeAccount(in: group, config: config)?.id == contaA.id)
    }

    /// A regra que evita a cópia vencida: antes de trocar, o token vivo do grupo
    /// volta para a casa da conta que sai.
    @Test("trocar espelha o token do grupo de volta para a casa da conta que sai")
    func switchMirrorsBack() throws {
        let (engine, kc, adapter, config, contaA, contaB, group) = setup()
        try engine.activate(contaA, in: group, config: config)

        // Simula o Claude Code renovando o token no item do grupo.
        let groupService = adapter.keychainService(forConfigDir: group.configDir)
        try kc.write("cred-a-RENOVADO", service: groupService)

        // Troca para contaB: o token renovado tem de voltar para a casa da contaA.
        try engine.activate(contaB, in: group, config: config)

        let casaA = adapter.keychainService(forConfigDir: contaA.home)
        #expect(kc.read(service: casaA) == "cred-a-RENOVADO")
        #expect(kc.read(service: groupService) == "cred-b")
        #expect(engine.activeAccount(in: group, config: config)?.id == contaB.id)
    }

    /// Regressão do "Login expired" de 26/ago: relançar `claude trabalho` com a
    /// mesma conta ativa reescrevia o item do grupo com a cópia velha da casa —
    /// e o refresh token gira, então a cópia velha está morta. Reativar a ativa
    /// tem de preservar o token vivo do grupo e espelhá-lo para a casa.
    @Test("reativar a conta ativa preserva o token vivo e atualiza a casa")
    func reactivatePreservesLiveToken() throws {
        let (engine, kc, adapter, config, contaA, _, group) = setup()
        try engine.activate(contaA, in: group, config: config)

        // Simula o Claude Code renovando o token no item do grupo.
        let groupService = adapter.keychainService(forConfigDir: group.configDir)
        try kc.write("cred-a-RENOVADO", service: groupService)

        try engine.activate(contaA, in: group, config: config)  // relançamento

        #expect(kc.read(service: groupService) == "cred-a-RENOVADO")
        #expect(kc.read(service: adapter.keychainService(forConfigDir: contaA.home))
                == "cred-a-RENOVADO")
    }

    /// O ciclo periódico que mantém a casa viva enquanto a conta está ativa.
    @Test("mirrorActive espelha o token do grupo para a casa da conta ativa")
    func mirrorActiveKeepsHomeFresh() throws {
        let (engine, kc, adapter, config, contaA, _, group) = setup()
        try engine.activate(contaA, in: group, config: config)
        let groupService = adapter.keychainService(forConfigDir: group.configDir)
        try kc.write("cred-a-RENOVADO", service: groupService)

        engine.mirrorActive(in: group, config: config)

        #expect(kc.read(service: adapter.keychainService(forConfigDir: contaA.home))
                == "cred-a-RENOVADO")
    }

    /// A falha que já matou contas: a mesma conta ativa em dois grupos.
    @Test("recusa ativar conta que já serve outro grupo")
    func refusesDuplicate() throws {
        let kc = FakeKeychain()
        let adapter = FakeAdapter()
        let engine = RotationEngine(keychain: kc, adapters: [adapter])

        let shared = Account(provider: .anthropic, identity: ident("x@y.com"),
                             home: .dedicated("/Users/exemplo/.claude-x"))
        try kc.write("cred-x", service: adapter.keychainService(forConfigDir: shared.home))

        let g1 = AccountGroup(name: "trabalho", accountIDs: [shared.id],
                              configDir: .standard(home: "/Users/exemplo"))
        let g2 = AccountGroup(name: "pessoal", accountIDs: [shared.id],
                              configDir: .dedicated("/Users/exemplo/.claude-pessoal"))
        var config = RouterConfig(accounts: [shared], groups: [g1, g2])

        try engine.activate(shared, in: g1, config: config)
        // reflete no config para o segundo grupo enxergar a conta como ocupada
        config = RouterConfig(accounts: [shared], groups: [g1, g2], shareHistory: config.shareHistory)

        #expect(throws: RotationEngine.RotationError.self) {
            try engine.activate(shared, in: g2, config: config)
        }
    }

    @Test("conta sem credencial na casa é erro nomeado, não crash")
    func noCredential() {
        let kc = FakeKeychain()
        let adapter = FakeAdapter()
        let engine = RotationEngine(keychain: kc, adapters: [adapter])
        let orphan = Account(provider: .anthropic, identity: ident("z@y.com"),
                             home: .dedicated("/Users/exemplo/.claude-z"))
        let group = AccountGroup(name: "g", accountIDs: [orphan.id],
                                 configDir: .standard(home: "/Users/exemplo"))
        let config = RouterConfig(accounts: [orphan], groups: [group])
        #expect(throws: RotationEngine.RotationError.noCredential(accountID: orphan.id)) {
            try engine.activate(orphan, in: group, config: config)
        }
    }
}

// MARK: - A decisão de quando trocar

@Suite("Decisão de rotação")
struct RotationDecisionTests {
    private func fixture() -> (RotationEngine, RouterConfig, AccountGroup, Account, Account) {
        let kc = FakeKeychain()
        let adapter = FakeAdapter()
        let engine = RotationEngine(keychain: kc, adapters: [adapter])
        let a = Account(provider: .anthropic, identity: ident("a@k.com"),
                        home: .dedicated("/Users/exemplo/.claude-a"))
        let b = Account(provider: .anthropic, identity: ident("b@k.com"),
                        home: .dedicated("/Users/exemplo/.claude-b"))
        try? kc.write("ca", service: adapter.keychainService(forConfigDir: a.home))
        try? kc.write("cb", service: adapter.keychainService(forConfigDir: b.home))
        let group = AccountGroup(name: "g", accountIDs: [a.id, b.id],
                                 configDir: .standard(home: "/Users/exemplo"),
                                 thresholdPercent: 90)
        // deixa 'a' ativa
        try? engine.activate(a, in: group, config: RouterConfig(accounts: [a, b], groups: [group]))
        let config = RouterConfig(accounts: [a, b], groups: [group])
        return (engine, config, group, a, b)
    }

    @Test("não troca enquanto a ativa tem folga")
    func staysBelowThreshold() {
        let (engine, config, group, a, b) = fixture()
        let target = engine.rotationTarget(for: group, config: config,
                                           usage: [a.id: 0.5, b.id: 0.1])
        #expect(target == nil)
    }

    @Test("troca para a próxima com folga quando a ativa passa do limiar")
    func rotatesWhenOver() {
        let (engine, config, group, a, b) = fixture()
        let target = engine.rotationTarget(for: group, config: config,
                                           usage: [a.id: 0.95, b.id: 0.1])
        #expect(target?.id == b.id)
    }

    /// Fail-safe: ativa estourada mas nenhuma outra qualifica -> mantém, não trava.
    @Test("sem candidato com folga, mantém a atual")
    func noQualifyingKeepsCurrent() {
        let (engine, config, group, a, b) = fixture()
        let target = engine.rotationTarget(for: group, config: config,
                                           usage: [a.id: 0.95, b.id: 0.97])
        #expect(target == nil)
    }

    /// Regressão de um deadlock real: a conta ativa a 96% e a seguinte nunca
    /// no sensor passivo, conta nunca usada não tem amostra — exigi-la impedia
    /// a rotação para sempre (só mede quem serve; só serve quem é escolhida).
    /// Sem medição = presumida fresca; a primeira mensagem dela corrige.
    @Test("conta sem medição é presumida fresca e vira destino")
    func unmeasuredIsPresumedFresh() {
        let (engine, config, group, a, b) = fixture()
        let target = engine.rotationTarget(for: group, config: config,
                                           usage: [a.id: 0.95])  // b sem medição
        #expect(target?.id == b.id)
    }

    @Test("autoRotate desligado nunca troca sozinho")
    func autoRotateOff() {
        let (engine, config, group, a, b) = fixture()
        var off = group; off.autoRotate = false
        let cfg = RouterConfig(accounts: config.accounts, groups: [off])
        let target = engine.rotationTarget(for: off, config: cfg,
                                           usage: [a.id: 0.99, b.id: 0.0])
        #expect(target == nil)
    }
}

// MARK: - Configuração persistível

@Suite("RouterConfig")
struct RouterConfigTests {
    @Test("sobrevive a um ciclo de codificação")
    func roundTrips() throws {
        let a = Account(provider: .anthropic, identity: ident("a@k.com"),
                        home: .dedicated("/Users/exemplo/.claude-a"), nickname: "principal")
        let group = AccountGroup(name: "trabalho", accountIDs: [a.id],
                                 configDir: .standard(home: "/Users/exemplo"))
        let config = RouterConfig(accounts: [a], groups: [group], shareHistory: true)

        let data = try JSONEncoder().encode(config)
        let back = try JSONDecoder().decode(RouterConfig.self, from: data)

        #expect(back == config)
        #expect(back.account(a.id)?.nickname == "principal")
        #expect(back.defaultGroup?.name == "trabalho")
    }

    @Test("preserva campos desconhecidos do oauthAccount")
    func preservesOpaqueIdentity() throws {
        let raw: [String: JSONValue] = [
            "emailAddress": .string("a@k.com"),
            "organizationUuid": .string("abc-123"),
            "campoNovoQueNaoConhecemos": .bool(true),
        ]
        let identity = AccountIdentity(email: "a@k.com", organizationName: "K",
                                       rateLimitTier: nil, raw: raw)
        let data = try JSONEncoder().encode(identity)
        let back = try JSONDecoder().decode(AccountIdentity.self, from: data)
        #expect(back.raw["campoNovoQueNaoConhecemos"] == .bool(true))
    }
}

@Suite("AnthropicAdapter (real, arquivo)")
struct AnthropicAdapterFileTests {
    /// Regressão: escrever a identidade num perfil dedicado que ainda não existe
    /// no disco tem de criar o diretório, não falhar. (Pegou um bug real no
    /// `router launch` do primeiro grupo dedicado.)
    @Test("writeIdentity cria o diretório do perfil se faltar")
    func createsDirectory() throws {
        let tmp = URL(fileURLWithPath: NSTemporaryDirectory())
            .appending(path: "adapter-\(UUID().uuidString)")
        let dir = ConfigDir.dedicated(tmp.path)   // não existe ainda
        let adapter = AnthropicAdapter()
        let identity = AccountIdentity(
            email: "z@k.com", organizationName: "K", rateLimitTier: nil,
            raw: ["emailAddress": .string("z@k.com"), "organizationName": .string("K")])

        try adapter.writeIdentity(identity, toConfigDir: dir)

        let back = adapter.identity(inConfigDir: dir)
        #expect(back?.email == "z@k.com")
        try? FileManager.default.removeItem(at: tmp)
    }

    /// A regravação preserva o resto do `.claude.json` e some com o cache de uso
    /// da conta anterior.
    @Test("writeIdentity preserva o arquivo e limpa o cache de uso")
    func preservesAndClears() throws {
        let tmp = URL(fileURLWithPath: NSTemporaryDirectory())
            .appending(path: "adapter-\(UUID().uuidString)")
        try FileManager.default.createDirectory(at: tmp, withIntermediateDirectories: true)
        let dir = ConfigDir.dedicated(tmp.path)
        let json = try JSONSerialization.data(withJSONObject: [
            "oauthAccount": ["emailAddress": "old@k.com"],
            "cachedUsageUtilization": ["fetchedAtMs": 1],
            "projects": ["/x": ["hasTrustDialogAccepted": true]],
        ])
        try json.write(to: dir.globalConfigURL)

        try AnthropicAdapter().writeIdentity(
            AccountIdentity(email: "new@k.com", organizationName: nil, rateLimitTier: nil,
                            raw: ["emailAddress": .string("new@k.com")]),
            toConfigDir: dir)

        let root = try JSONSerialization.jsonObject(
            with: Data(contentsOf: dir.globalConfigURL)) as! [String: Any]
        #expect((root["oauthAccount"] as? [String: Any])?["emailAddress"] as? String == "new@k.com")
        #expect(root["cachedUsageUtilization"] == nil)          // limpo
        #expect(root["projects"] != nil)                        // preservado
        #expect(root["hasCompletedOnboarding"] as? Bool == true) // sem isto, tela de login
        try? FileManager.default.removeItem(at: tmp)
    }
}

@Suite("ProviderEnv")
struct ProviderEnvTests {
    @Test("remove as variáveis de proxy, preserva o resto")
    func stripsProxy() {
        let env = [
            "HTTPS_PROXY": "http://127.0.0.1:3456",
            "https_proxy": "http://127.0.0.1:3456",
            "NODE_EXTRA_CA_CERTS": "/x/teamclaude-ca.pem",
            "PATH": "/usr/bin", "HOME": "/Users/exemplo",
        ]
        let direct = ProviderEnv.direct(env)
        #expect(direct["HTTPS_PROXY"] == nil)
        #expect(direct["https_proxy"] == nil)
        #expect(direct["NODE_EXTRA_CA_CERTS"] == nil)
        #expect(direct["PATH"] == "/usr/bin")
        #expect(direct["HOME"] == "/Users/exemplo")
    }

    /// Chave de API vence a conta OAuth: a sessão sobe servida por ela, o
    /// rodízio ativa uma conta que ninguém usa, e o sensor ainda carimba o
    /// consumo da chave no e-mail do perfil — o rodízio passa a decidir com
    /// número que não é da conta.
    @Test("remove credencial e endpoint alternativos")
    func stripsCredentialOverrides() {
        let env = [
            "ANTHROPIC_API_KEY": "sk-ant-xxx",
            "ANTHROPIC_AUTH_TOKEN": "tok",
            "ANTHROPIC_BASE_URL": "https://gateway.interno/v1",
            "ANTHROPIC_CUSTOM_HEADERS": "Authorization: Bearer x",
            "CLAUDE_CODE_USE_BEDROCK": "1",
            // Os do porte Windows (issue #5): token pelo ambiente e o
            // redirecionamento do lugar da credencial.
            "CLAUDE_CODE_OAUTH_TOKEN": "sk-ant-oat-TESTE",
            "CLAUDE_CODE_SESSION_ACCESS_TOKEN": "sess",
            "CLAUDE_SECURESTORAGE_CONFIG_DIR": "/outro/lugar",
            "ANTHROPIC_PROFILE": "outro",
            "ANTHROPIC_MODEL": "claude-opus-5", "PATH": "/usr/bin",
        ]
        let direct = ProviderEnv.direct(env)
        for chave in ProviderEnv.credentialKeys {
            #expect(direct[chave] == nil, "\(chave) sobreviveu")
        }
        // O que não decide QUEM atende continua valendo: a escolha de modelo é
        // do usuário, e apagá-la mudaria a sessão dele sem motivo.
        #expect(direct["ANTHROPIC_MODEL"] == "claude-opus-5")
        #expect(direct["PATH"] == "/usr/bin")
    }

    /// Issue #11: o perfil lido de outro lugar, a identidade federada e os
    /// endpoints alternativos serviam a sessão por algo que não é a conta que o
    /// grupo ativou — e o sensor carimbava o consumo no e-mail do perfil.
    @Test("remove o perfil alternativo, a identidade federada e as famílias Foundry/AWS")
    func stripsTheIssue11Variables() {
        let env = [
            "ANTHROPIC_CONFIG_DIR": "/outro/perfil",
            "ANTHROPIC_IDENTITY_TOKEN": "jwt", "ANTHROPIC_IDENTITY_TOKEN_FILE": "/x/jwt",
            "ANTHROPIC_FEDERATION_RULE_ID": "r", "ANTHROPIC_ORGANIZATION_ID": "o",
            "CLAUDE_CODE_USE_FOUNDRY": "1", "AWS_BEARER_TOKEN_BEDROCK": "b",
            // Famílias por prefixo: inclusive um nome que ainda não existe hoje.
            "ANTHROPIC_FOUNDRY_RESOURCE": "acme", "ANTHROPIC_FOUNDRY_ALGO_NOVO": "x",
            "ANTHROPIC_AWS_WORKSPACE_ID": "w",
            "PATH": "/usr/bin",
        ]
        let direct = ProviderEnv.direct(env)
        #expect(direct.keys.sorted() == ["PATH"])
    }

    /// No macOS o nome de variável tem caixa, e o Claude Code lê a maiúscula —
    /// apagar `anthropic_aws_x` mexeria no ambiente do usuário sem motivo.
    @Test("o prefixo compara com caixa, como o sistema")
    func prefixesAreCaseSensitive() {
        let direct = ProviderEnv.direct(["anthropic_aws_coisa": "fica"])
        #expect(direct["anthropic_aws_coisa"] == "fica")
    }
}

// MARK: - O leitor de uso e o tempo

@Suite("GroupUsageReader (decaimento por reset)")
struct GroupUsageReaderDecayTests {
    private func fixture(five: Double?, fiveResets: Date?,
                         seven: Double?, sevenResets: Date?) -> (GroupUsageReader, RouterConfig, Account, URL) {
        let tmp = URL(fileURLWithPath: NSTemporaryDirectory())
            .appending(path: "usage-\(UUID().uuidString)")
        try? FileManager.default.createDirectory(at: tmp, withIntermediateDirectories: true)
        let a = Account(provider: .anthropic, identity: ident("a@k.com"),
                        home: .dedicated("/Users/exemplo/.claude-a"))
        let sample = GroupUsageSample(
            configDirRaw: "/x", email: "a@k.com",
            fiveHourPercent: five, fiveHourResetsAt: fiveResets,
            sevenDayPercent: seven, sevenDayResetsAt: sevenResets,
            sampledAt: Date(timeIntervalSince1970: 1000))
        try? GroupUsageStore.write(sample, forEmail: "a@k.com", in: tmp)
        let config = RouterConfig(accounts: [a], groups: [])
        return (GroupUsageReader(usageDir: tmp), config, a, tmp)
    }

    /// Regressão: uma conta a 96% na janela de 5h continuaria "cheia" para
    /// sempre depois do reset — e o grupo nunca voltaria para ela.
    @Test("janela cujo reset passou é descartada; vale a que sobrou")
    func expiredWindowDropped() {
        let now = Date(timeIntervalSince1970: 2000)
        let (reader, config, a, tmp) = fixture(
            five: 0.96, fiveResets: Date(timeIntervalSince1970: 1500),   // já passou
            seven: 0.27, sevenResets: Date(timeIntervalSince1970: 99_000))
        #expect(reader.usageByAccount(config, now: now)[a.id] == 0.27)
        try? FileManager.default.removeItem(at: tmp)
    }

    @Test("amostra com as duas janelas vencidas some do mapa (presumida fresca)")
    func fullyExpiredDisappears() {
        let now = Date(timeIntervalSince1970: 2000)
        let (reader, config, a, tmp) = fixture(
            five: 0.96, fiveResets: Date(timeIntervalSince1970: 1500),
            seven: 0.90, sevenResets: Date(timeIntervalSince1970: 1600))
        #expect(reader.usageByAccount(config, now: now)[a.id] == nil)
        try? FileManager.default.removeItem(at: tmp)
    }

    @Test("resets no futuro: vale o maior das duas janelas")
    func freshSampleUsesMax() {
        let now = Date(timeIntervalSince1970: 2000)
        let (reader, config, a, tmp) = fixture(
            five: 0.40, fiveResets: Date(timeIntervalSince1970: 90_000),
            seven: 0.62, sevenResets: Date(timeIntervalSince1970: 99_000))
        #expect(reader.usageByAccount(config, now: now)[a.id] == 0.62)
        try? FileManager.default.removeItem(at: tmp)
    }

    /// Amostra sem timestamps de reset (sensor antigo) segue valendo — melhor
    /// um número velho honesto que fingir frescor.
    @Test("sem timestamp de reset, a janela continua contando")
    func missingResetKeepsWindow() {
        let now = Date(timeIntervalSince1970: 2000)
        let (reader, config, a, tmp) = fixture(
            five: 0.96, fiveResets: nil,
            seven: nil, sevenResets: nil)
        #expect(reader.usageByAccount(config, now: now)[a.id] == 0.96)
        try? FileManager.default.removeItem(at: tmp)
    }

    /// O caso que fez o Falcão desconfiar do app: o painel dizia 66% enquanto a
    /// status line dizia `5h 1%`. Os dois estavam certos — 66% é o SEMANAL —
    /// mas o painel não dizia de qual janela falava. A procedência agora vem no
    /// dado, para a UI poder rotular.
    @Test("o detalhe diz de qual janela o número veio")
    func detailNamesTheWindow() {
        let now = Date(timeIntervalSince1970: 2000)
        let (reader, config, a, tmp) = fixture(
            five: 0.01, fiveResets: Date(timeIntervalSince1970: 90_000),
            seven: 0.66, sevenResets: Date(timeIntervalSince1970: 99_000))
        let detail = reader.detailByAccount(config, now: now)[a.id]
        #expect(detail?.fraction == 0.66)
        #expect(detail?.window == .sevenDay)
        // As duas medidas seguem disponíveis para o tooltip mostrar o quadro.
        #expect(detail?.fiveHour == 0.01)
        #expect(detail?.sevenDay == 0.66)
        try? FileManager.default.removeItem(at: tmp)
    }

    @Test("quando quem manda é a janela de 5h, é ela que aparece")
    func detailReportsFiveHourWhenItLeads() {
        let now = Date(timeIntervalSince1970: 2000)
        let (reader, config, a, tmp) = fixture(
            five: 0.80, fiveResets: Date(timeIntervalSince1970: 90_000),
            seven: 0.30, sevenResets: Date(timeIntervalSince1970: 99_000))
        let detail = reader.detailByAccount(config, now: now)[a.id]
        #expect(detail?.fraction == 0.80)
        #expect(detail?.window == .fiveHour)
        try? FileManager.default.removeItem(at: tmp)
    }

    /// Empate: as duas limitam igual, mas a semanal leva dias para aliviar.
    @Test("no empate vale a semanal, que demora mais a aliviar")
    func detailPrefersSevenDayOnTie() {
        let now = Date(timeIntervalSince1970: 2000)
        let (reader, config, a, tmp) = fixture(
            five: 0.50, fiveResets: Date(timeIntervalSince1970: 90_000),
            seven: 0.50, sevenResets: Date(timeIntervalSince1970: 99_000))
        #expect(reader.detailByAccount(config, now: now)[a.id]?.window == .sevenDay)
        try? FileManager.default.removeItem(at: tmp)
    }

    /// Janela expirada não pode ser citada como procedência: uma conta tem
    /// `5h = 100%` gravado de anteontem e o número que vale é o semanal.
    @Test("janela vencida não vira a procedência do número")
    func expiredWindowIsNotCited() {
        let now = Date(timeIntervalSince1970: 2000)
        let (reader, config, a, tmp) = fixture(
            five: 1.0, fiveResets: Date(timeIntervalSince1970: 1500),   // já passou
            seven: 0.28, sevenResets: Date(timeIntervalSince1970: 99_000))
        let detail = reader.detailByAccount(config, now: now)[a.id]
        #expect(detail?.fraction == 0.28)
        #expect(detail?.window == .sevenDay)
        #expect(detail?.fiveHour == nil)
        try? FileManager.default.removeItem(at: tmp)
    }
}

/// A formatação da porcentagem, que precisa ser a MESMA nas três superfícies.
@Suite("UsagePercent (uma formatação só)")
struct UsagePercentTests {
    /// O bug real: o sensor arredondava e o painel truncava. Com 0,666 a status
    /// line escrevia 67% e o painel 66% — um ponto de diferença que não vinha de
    /// medição nenhuma, e que faz o usuário duvidar do número que decide a troca.
    @Test("arredonda em vez de truncar")
    func roundsRatherThanTruncates() {
        #expect(UsagePercent.value(0.666) == 67)
        #expect(UsagePercent.value(0.664) == 66)
        #expect(UsagePercent.text(0.666) == "67%")
    }

    @Test("os extremos não escorregam")
    func edges() {
        #expect(UsagePercent.text(0) == "0%")
        #expect(UsagePercent.text(1) == "100%")
        #expect(UsagePercent.text(0.005) == "1%")
    }
}


// MARK: - Uma conta, um lugar (10/2026)

@Suite("Uma conta, um lugar")
struct OneAccountOnePlaceTests {
    /// O `~/.claude` destes testes; o grupo roda num perfil dedicado.
    private let home = ConfigDir.standard(home: "/Users/exemplo")

    private func setup() -> (RotationEngine, FakeKeychain, FakeAdapter, RouterConfig,
                             Account, Account, AccountGroup) {
        let kc = FakeKeychain()
        let adapter = FakeAdapter()
        let engine = RotationEngine(keychain: kc, adapters: [adapter], defaultProfile: home)
        let a = Account(provider: .anthropic, identity: ident("conta-a@exemplo.com"),
                        home: .dedicated("/Users/exemplo/casas/a"))
        let b = Account(provider: .anthropic, identity: ident("conta-b@exemplo.com"),
                        home: .dedicated("/Users/exemplo/casas/b"))
        try? kc.write("cred-a", service: adapter.keychainService(forConfigDir: a.home))
        try? kc.write("cred-b", service: adapter.keychainService(forConfigDir: b.home))
        let group = AccountGroup(name: "trabalho", accountIDs: [a.id, b.id],
                                 configDir: .dedicated("/Users/exemplo/grupos/t"))
        return (engine, kc, adapter, RouterConfig(accounts: [a, b], groups: [group]),
                a, b, group)
    }

    /// Depois de "deixar o ~/.claude livre", a conta que estava lá continua
    /// servindo o `claude` puro. Ativá-la no perfil dedicado, a partir da casa,
    /// deixava a cadeia em dois itens — e a primeira renovação matava a outra.
    @Test("a conta do ~/.claude, sem grupo nele, não é ativada em outro perfil")
    func accountLiveInTheDefaultProfileIsBusy() throws {
        let (engine, _, adapter, config, a, b, group) = setup()
        try adapter.writeIdentity(a.identity, toConfigDir: home)

        #expect(throws: RotationEngine.RotationError.accountLiveInDefaultProfile(accountID: a.id)) {
            try engine.activate(a, in: group, config: config)
        }
        #expect(engine.nextAccount(for: group, config: config, usage: [:])?.id == b.id)
    }

    @Test("a rotação pula a conta sem credencial na casa e tenta a seguinte")
    func accountWithoutCredentialIsSkipped() {
        let (engine, kc, adapter, config, a, b, group) = setup()
        kc.delete(service: adapter.keychainService(forConfigDir: a.home))
        #expect(engine.nextAccount(for: group, config: config, usage: [:])?.id == b.id)
    }

    /// Até 10/2026 a conta ativa noutro grupo continuava sendo a escolhida: a
    /// troca recusava, a volta seguinte escolhia a mesma, e o grupo ficava
    /// preso na conta estourada — calado.
    @Test("a rotação pula a conta ativa em outro grupo e tenta a seguinte")
    func accountActiveElsewhereIsSkipped() throws {
        let (engine, _, adapter, config, a, b, group) = setup()
        let other = AccountGroup(name: "pessoal", accountIDs: [a.id],
                                 configDir: .dedicated("/Users/exemplo/grupos/p"))
        let both = RouterConfig(accounts: config.accounts, groups: [group, other])
        try adapter.writeIdentity(a.identity, toConfigDir: other.configDir)

        #expect(engine.nextAccount(for: group, config: both, usage: [:])?.id == b.id)
    }

    /// Ativa sem amostra é tão fresca quanto qualquer candidata sem amostra. A
    /// regra antiga a tratava como estourada, e um "Usar" manual numa conta
    /// ainda não medida era desfeito três minutos depois.
    @Test("a ativa sem amostra fica onde está")
    func activeWithoutSampleStays() throws {
        let (engine, _, _, config, _, b, group) = setup()
        try engine.activate(b, in: group, config: config)
        #expect(engine.rotationTarget(for: group, config: config, usage: [:]) == nil)
    }

    /// O `~/.claude` de quem já usava o `claude` antes do app tem um login que
    /// não é conta do router. A rotação não o troca sozinha: apagaria esse
    /// login sem cópia. Ativar ali continua possível como escolha explícita.
    @Test("perfil com login de fora: a rotação não troca sozinha, o \"Usar\" troca")
    func foreignLoginIsNotRotatedAway() throws {
        let (engine, _, adapter, config, a, _, group) = setup()
        let foreign = AccountIdentity(email: "fora@exemplo.com", organizationName: nil,
                                      rateLimitTier: nil, raw: [:])
        try adapter.writeIdentity(foreign, toConfigDir: group.configDir)

        #expect(engine.rotationTarget(for: group, config: config, usage: [:]) == nil)
        try engine.activate(a, in: group, config: config)
        #expect(engine.activeAccount(in: group, config: config)?.id == a.id)
    }

    /// Com o segredo novo e a identidade velha, o espelho seguinte gravaria a
    /// credencial da conta nova na casa da que saiu.
    @Test("identidade que não grava devolve o item do grupo ao que era")
    func failedIdentityWriteRollsTheSecretBack() throws {
        let (engine, kc, adapter, config, a, b, group) = setup()
        try engine.activate(a, in: group, config: config)
        let groupService = adapter.keychainService(forConfigDir: group.configDir)

        adapter.failIdentityWrites = true
        #expect(throws: RotationEngine.RotationError.self) {
            try engine.activate(b, in: group, config: config)
        }
        #expect(kc.read(service: groupService) == "cred-a")
        #expect(engine.activeAccount(in: group, config: config)?.id == a.id)
    }

    /// O espelho é o que guarda o token vivo da conta que sai. Se ele não grava,
    /// sobrescrever o item do grupo perderia esse token.
    @Test("espelho que falha aborta a troca")
    func failedMirrorAbortsTheSwitch() throws {
        let (engine, kc, adapter, config, a, b, group) = setup()
        try engine.activate(a, in: group, config: config)
        let groupService = adapter.keychainService(forConfigDir: group.configDir)
        try kc.write("cred-a-RENOVADO", service: groupService)

        kc.failWritesTo = [adapter.keychainService(forConfigDir: a.home)]
        #expect(throws: RotationEngine.RotationError.self) {
            try engine.activate(b, in: group, config: config)
        }
        #expect(kc.read(service: groupService) == "cred-a-RENOVADO")
        #expect(engine.activeAccount(in: group, config: config)?.id == a.id)
    }

    @Test("esvaziar um perfil dedicado tira item e identidade; o ~/.claude nunca")
    func resetProfileNeverTouchesTheDefault() throws {
        let (engine, kc, adapter, _, a, _, group) = setup()
        try kc.write("viva", service: adapter.keychainService(forConfigDir: group.configDir))
        try adapter.writeIdentity(a.identity, toConfigDir: group.configDir)
        try kc.write("do-claude-puro", service: adapter.keychainService(forConfigDir: home))
        try adapter.writeIdentity(a.identity, toConfigDir: home)

        engine.resetProfile(group.configDir, provider: .anthropic)
        engine.resetProfile(home, provider: .anthropic)

        #expect(!kc.exists(service: adapter.keychainService(forConfigDir: group.configDir)))
        #expect(adapter.identity(inConfigDir: group.configDir) == nil)
        #expect(kc.read(service: adapter.keychainService(forConfigDir: home)) == "do-claude-puro")
        #expect(adapter.identity(inConfigDir: home)?.email == a.identity.email)
    }
}
