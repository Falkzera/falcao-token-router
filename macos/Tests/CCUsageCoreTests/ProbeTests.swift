import Foundation
import Testing
@testable import CCUsageCore

/// A saída REAL do `claude --print /usage`, capturada em setembro de 2026. É
/// fixture, não invenção: o parser existe para ler exatamente isto, e um formato
/// que mude aqui é o sinal de que ele precisa mudar também. Os números e os
/// nomes de skill foram trocados; a FORMA das linhas é a de verdade, que é o que
/// o parser lê.
private let saidaReal = """
You are currently using your subscription to power your Claude Code usage

Current session: 2% used · resets Sep 18 at 7:29pm (America/Maceio)
Current week (all models): 26% used · resets Sep 21 at 8:59am (America/Maceio)
Current week (Fable): 0% used · resets Sep 21 at 9am (America/Maceio)

What's contributing to your limits usage?
Approximate, based on local sessions on this machine — does not include other devices or claude.ai.

Last 24h · 120 requests · 6 sessions
  89% of your usage was at >150k context
  Top skills: /exemplo 15%, /outra 6%
"""

@Suite("ClaudeUsageProbe (a sonda ativa)")
struct ClaudeUsageProbeTests {
    private let agora = Date(timeIntervalSince1970: 1_758_200_000)  // 18/09/2026

    @Test("lê as três janelas da saída real")
    func parseSaidaReal() throws {
        let r = try ClaudeUsageProbe.parse(saidaReal, now: agora)
        #expect(r.session == 0.02)
        #expect(r.weeklyAll == 0.26)
        #expect(r.models.count == 1)
        #expect(r.models.first?.name == "Fable")
        #expect(r.models.first?.percent == 0)
    }

    /// O motivo de a sonda existir: esta linha não chega no `rate_limits` da
    /// status line, e é a que trava a conta primeiro.
    @Test("a janela por modelo é capturada com nome e reset")
    func capturaPorModelo() throws {
        let r = try ClaudeUsageProbe.parse(saidaReal, now: agora)
        let fable = try #require(r.models.first)
        // `9am` — hora cheia, sem minutos. Um padrão `h:mma` sozinho devolveria
        // nil aqui, e a janela perderia o reset em uma hora de cada sessenta.
        let reset = try #require(fable.resetsAt)
        #expect(reset > agora)
    }

    /// A prosa abaixo das janelas fala de porcentagem o tempo todo ("89% of
    /// your usage was at >150k context"). Nada dali pode virar limite.
    @Test("a prosa de contribuição não vira janela")
    func ignoraProsa() throws {
        let r = try ClaudeUsageProbe.parse(saidaReal, now: agora)
        #expect(r.models.allSatisfy { $0.name == "Fable" })
        #expect(r.models.count == 1)
    }

    @Test("saída sem nenhuma linha Current é formato desconhecido")
    func semJanelas() {
        #expect(throws: ClaudeUsageProbe.ProbeError.unrecognized) {
            try ClaudeUsageProbe.parse("Credit balance: $0.00\nnada aqui", now: agora)
        }
    }

    /// Perder a porcentagem porque a redação da data mudou seria a pior das
    /// duas falhas: o número que decide continua legível.
    @Test("data de reset ilegível não derruba a porcentagem")
    func dataIlegivel() throws {
        let texto = "Current session: 44% used · resets quando der (Marte/Olympus)"
        let r = try ClaudeUsageProbe.parse(texto, now: agora)
        #expect(r.session == 0.44)
        #expect(r.sessionResetsAt == nil)
    }

    /// Lido em 31/dez, uma janela que reseta em 2/jan é do ano que vem — não do
    /// que está acabando.
    @Test("o ano do reset é o candidato mais próximo de agora")
    func viradaDeAno() throws {
        var comps = DateComponents()
        comps.year = 2026; comps.month = 12; comps.day = 31; comps.hour = 20
        var cal = Calendar(identifier: .gregorian)
        cal.timeZone = TimeZone(identifier: "UTC")!
        let reveillon = cal.date(from: comps)!

        let reset = try #require(ClaudeUsageProbe.resetDate(
            from: "Jan 2 at 9am (UTC)", now: reveillon))
        #expect(cal.component(.year, from: reset) == 2027)
    }

    @Test("o plano sai da primeira linha, como impresso")
    func plano() {
        #expect(ClaudeUsageProbe.plan(in: saidaReal) == "subscription")
        #expect(ClaudeUsageProbe.plan(in: "Max 20x plan\n\nCurrent session: 1% used") == "Max 20x")
    }
}

@Suite("Sonda: por qual perfil medir")
struct ProbeTargetTests {
    /// A regra que protege a sessão viva. Sondar a casa de uma conta ATIVA faria
    /// o `claude` renovar com o refresh token velho, e a renovação gira a cadeia
    /// — invalidando a cópia do grupo e derrubando a sessão do usuário em
    /// "Login expired" no meio do trabalho.
    @Test("conta ativa é sondada pelo perfil do grupo, nunca pela casa")
    func ativaVaiPeloGrupo() throws {
        let kc = FakeKeychain()
        let adapter = FakeAdapter()
        let engine = RotationEngine(keychain: kc, adapters: [adapter])

        let casa = ConfigDir.dedicated("/tmp/casa-a")
        let conta = Account(provider: .anthropic,
                            identity: AccountIdentity(email: "conta2@exemplo.com", organizationName: nil,
                                                      rateLimitTier: nil,
                                                      raw: ["emailAddress": .string("conta2@exemplo.com")]),
                            home: casa)
        let perfilGrupo = ConfigDir.dedicated("/tmp/grupo-trabalho")
        let grupo = AccountGroup(name: "trabalho", accountIDs: [conta.id], configDir: perfilGrupo)
        let config = RouterConfig(accounts: [conta], groups: [grupo])

        // Ainda não ativou ninguém: a casa é o alvo certo.
        #expect(engine.probeConfigDir(for: conta, config: config) == casa)

        // Ativa a conta no grupo — agora o alvo tem de mudar.
        try? kc.write("cred", service: adapter.keychainService(forConfigDir: casa))
        try engine.activate(conta, in: grupo, config: config)
        #expect(engine.probeConfigDir(for: conta, config: config) == perfilGrupo)
    }
}

@Suite("Por modelo entra na decisão de rotação")
struct ModelWindowRotationTests {
    private func leitor(_ amostra: GroupUsageSample, email: String) throws
        -> (GroupUsageReader, RouterConfig, UUID) {
        let dir = URL(fileURLWithPath: NSTemporaryDirectory())
            .appending(path: "probe-\(UUID().uuidString)")
        try GroupUsageStore.write(amostra, forEmail: email, in: dir)
        let conta = Account(provider: .anthropic,
                            identity: AccountIdentity(email: email, organizationName: nil,
                                                      rateLimitTier: nil, raw: [:]),
                            home: .dedicated("/tmp/c"))
        let config = RouterConfig(accounts: [conta],
                                  groups: [AccountGroup(name: "g", accountIDs: [conta.id],
                                                        configDir: .dedicated("/tmp/g"))])
        return (GroupUsageReader(usageDir: dir), config, conta.id)
    }

    /// O caso real que a lacuna escondia: 5h 91% e 7d 79% parecem folga contra
    /// um limiar de 95%, mas o Fable a 100% já travou a conta.
    @Test("Fable em 100% manda, mesmo com 5h e 7d abaixo do limiar")
    func porModeloVence() throws {
        let agora = Date()
        let amostra = GroupUsageSample(
            configDirRaw: "/tmp/g", email: "conta2@exemplo.com",
            fiveHourPercent: 0.91, fiveHourResetsAt: agora.addingTimeInterval(3600),
            sevenDayPercent: 0.79, sevenDayResetsAt: agora.addingTimeInterval(86400),
            sampledAt: agora,
            models: ModelUsage(windows: [.init(name: "Fable", percent: 1.0,
                                               resetsAt: agora.addingTimeInterval(86400))],
                               sampledAt: agora))
        let (reader, config, id) = try leitor(amostra, email: "conta2@exemplo.com")
        let uso = try #require(reader.detailByAccount(config, now: agora)[id])

        #expect(uso.fraction == 1.0)
        #expect(uso.window == .model("Fable"))
        // As outras duas continuam visíveis: a UI mostra o quadro inteiro.
        #expect(uso.fiveHour == 0.91)
        #expect(uso.sevenDay == 0.79)
    }

    /// Janela por modelo cujo reset já passou não pode deixar a conta "cheia"
    /// para sempre aos olhos da rotação — mesma regra das outras duas.
    @Test("modelo com reset vencido é descartado")
    func modeloVencidoDecai() throws {
        let agora = Date()
        let amostra = GroupUsageSample(
            configDirRaw: "/tmp/g", email: "x@k.com",
            fiveHourPercent: 0.10, fiveHourResetsAt: agora.addingTimeInterval(3600),
            sevenDayPercent: 0.20, sevenDayResetsAt: agora.addingTimeInterval(86400),
            sampledAt: agora,
            models: ModelUsage(windows: [.init(name: "Fable", percent: 1.0,
                                               resetsAt: agora.addingTimeInterval(-60))],
                               sampledAt: agora))
        let (reader, config, id) = try leitor(amostra, email: "x@k.com")
        let uso = try #require(reader.detailByAccount(config, now: agora)[id])

        #expect(uso.fraction == 0.20)
        #expect(uso.window == .sevenDay)
        #expect(uso.model == nil)
    }

    /// O sensor escreve a cada mensagem e nunca conhece as janelas por modelo.
    /// Sem a costura no `write`, a primeira mensagem depois de uma sondagem
    /// apagaria o número do Fable — o único que enxerga o limite que estoura.
    @Test("a escrita do sensor preserva o bloco por modelo")
    func sensorNaoApagaPorModelo() throws {
        let dir = URL(fileURLWithPath: NSTemporaryDirectory())
            .appending(path: "probe-\(UUID().uuidString)")
        let sondado = Date().addingTimeInterval(-600)
        let daSonda = GroupUsageSample(
            configDirRaw: "/tmp/g", email: "x@k.com",
            fiveHourPercent: 0.5, fiveHourResetsAt: nil,
            sevenDayPercent: 0.5, sevenDayResetsAt: nil, sampledAt: sondado,
            models: ModelUsage(windows: [.init(name: "Fable", percent: 0.9, resetsAt: nil)],
                               sampledAt: sondado))
        try GroupUsageStore.write(daSonda, forEmail: "x@k.com", in: dir)

        // Agora o SENSOR escreve, sem saber de modelo nenhum.
        let doSensor = GroupUsageSample(
            configDirRaw: "/tmp/g", email: "x@k.com",
            fiveHourPercent: 0.6, fiveHourResetsAt: nil,
            sevenDayPercent: 0.6, sevenDayResetsAt: nil, sampledAt: Date())
        try GroupUsageStore.write(doSensor, forEmail: "x@k.com", in: dir)

        let lido = try #require(GroupUsageStore.read(forEmail: "x@k.com", in: dir))
        #expect(lido.fiveHourPercent == 0.6)                       // o novo venceu
        #expect(lido.models?.windows.first?.percent == 0.9)        // e o Fable ficou
        // O carimbo preservado é o da SONDA, não o desta escrita: nada finge
        // frescor que não tem.
        let carimbo = try #require(lido.models?.sampledAt)
        #expect(abs(carimbo.timeIntervalSince(sondado)) < 1)
    }
}

@Suite("Procedência da medida (sensor × sonda)")
struct UsageOriginTests {
    private func tmpDir() -> URL {
        URL(fileURLWithPath: NSTemporaryDirectory())
            .appending(path: "origem-\(UUID().uuidString)")
    }

    /// Amostra gravada antes de a sonda existir não tem a chave `origin`. Ela só
    /// podia vir do sensor, e decodificar como tal é o que impede o painel de
    /// ficar sem procedência depois de uma atualização do app.
    @Test("amostra antiga, sem a chave, é do sensor")
    func amostraAntigaEhSensor() throws {
        let antiga = """
        {"configDirRaw":"/tmp/g","email":"x@k.com","fiveHourPercent":0.4,
         "sevenDayPercent":0.5,"sampledAt":"2026-09-18T12:00:00Z"}
        """
        let decoder = JSONDecoder()
        decoder.dateDecodingStrategy = .iso8601
        let amostra = try decoder.decode(GroupUsageSample.self, from: Data(antiga.utf8))
        #expect(amostra.origin == .sensor)
        #expect(amostra.fiveHourPercent == 0.4)
    }

    @Test("a origem sobrevive à ida e volta do disco")
    func origemPersiste() throws {
        let dir = tmpDir()
        let sondada = GroupUsageSample(
            configDirRaw: "/tmp/g", email: "x@k.com",
            fiveHourPercent: 0.1, fiveHourResetsAt: nil,
            sevenDayPercent: 0.2, sevenDayResetsAt: nil,
            sampledAt: Date(), origin: .probe)
        try GroupUsageStore.write(sondada, forEmail: "x@k.com", in: dir)
        #expect(GroupUsageStore.read(forEmail: "x@k.com", in: dir)?.origin == .probe)
    }

    /// O sensor escrevendo por cima de uma sondagem: as janelas de 5h/7d passam
    /// a ser dele (e a origem tem de acompanhar), mas as por modelo ficam, porque
    /// ele nunca as recebe. Uma amostra pode carregar as duas procedências.
    @Test("sensor por cima da sonda: origem vira sensor, modelo fica")
    func sensorPorCimaDaSonda() throws {
        let dir = tmpDir()
        let sondado = Date().addingTimeInterval(-600)
        try GroupUsageStore.write(GroupUsageSample(
            configDirRaw: "/tmp/g", email: "x@k.com",
            fiveHourPercent: 0.5, fiveHourResetsAt: nil,
            sevenDayPercent: 0.5, sevenDayResetsAt: nil, sampledAt: sondado,
            models: ModelUsage(windows: [.init(name: "Fable", percent: 0.9, resetsAt: nil)],
                               sampledAt: sondado),
            origin: .probe), forEmail: "x@k.com", in: dir)

        try GroupUsageStore.write(GroupUsageSample(
            configDirRaw: "/tmp/g", email: "x@k.com",
            fiveHourPercent: 0.6, fiveHourResetsAt: nil,
            sevenDayPercent: 0.6, sevenDayResetsAt: nil, sampledAt: Date(),
            origin: .sensor), forEmail: "x@k.com", in: dir)

        let lido = try #require(GroupUsageStore.read(forEmail: "x@k.com", in: dir))
        #expect(lido.origin == .sensor)                       // 5h/7d são do sensor
        #expect(lido.models?.windows.first?.percent == 0.9)   // o Fable ficou
    }

    /// A origem tem de chegar ao `AccountUsage`, que é o que a UI lê. Sem isso a
    /// tela atribuía ao sensor um número que a sonda tinha acabado de buscar.
    @Test("a origem chega ao AccountUsage que a UI consome")
    func origemChegaNaUI() throws {
        let dir = tmpDir()
        let agora = Date()
        try GroupUsageStore.write(GroupUsageSample(
            configDirRaw: "/tmp/g", email: "ociosa@k.com",
            fiveHourPercent: 0.26, fiveHourResetsAt: agora.addingTimeInterval(3600),
            sevenDayPercent: 0.38, sevenDayResetsAt: agora.addingTimeInterval(86400),
            sampledAt: agora, origin: .probe), forEmail: "ociosa@k.com", in: dir)

        let conta = Account(provider: .anthropic,
                            identity: AccountIdentity(email: "ociosa@k.com",
                                                      organizationName: nil,
                                                      rateLimitTier: nil, raw: [:]),
                            home: .dedicated("/tmp/c"))
        let config = RouterConfig(accounts: [conta],
                                  groups: [AccountGroup(name: "g", accountIDs: [conta.id],
                                                        configDir: .dedicated("/tmp/g"))])
        let uso = try #require(GroupUsageReader(usageDir: dir)
            .detailByAccount(config, now: agora)[conta.id])
        #expect(uso.origin == .probe)
    }
}
