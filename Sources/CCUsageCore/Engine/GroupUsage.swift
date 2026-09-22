import Foundation

/// Quem mediu o número de uma conta.
///
/// As duas fontes são oficiais, e ainda assim dizem coisas diferentes — por isso
/// a origem viaja junto com a medida em vez de ficar implícita:
///
/// - `sensor`: o `rate_limits` que o Claude Code entregou na status line da
///   requisição que **esta conta atendeu**. É o gasto dela como ela mesma o
///   viu, e envelhece do jeito mais perigoso: em conta compartilhada, o que os
///   colegas gastaram depois não aparece.
/// - `probe`: uma consulta `/usage` num instante, feita de propósito. Vale para
///   conta **ociosa**, que nunca atendeu nada e por isso não tem amostra do
///   sensor, e é a única que enxerga o limite por modelo.
///
/// Sem esta distinção o painel atribuía ao sensor um número que a sonda tinha
/// acabado de buscar — e a frase que acompanhava ("medido pela sessão da própria
/// conta") era falsa justamente nas contas ociosas, que sessão nenhuma serviu.
public enum UsageOrigin: String, Sendable, Codable, Equatable {
    case sensor, probe
}

/// O uso de uma conta, medido pela fonte que não custa nada e não fere os termos:
/// o `rate_limits` que o próprio Claude Code entrega no stdin da status line.
///
/// A chamada à API é do Claude Code, não nossa — nós só lemos o que ele já traz.
/// É a correção de conformidade (nada de token nosso batendo em `oauth/usage`) e
/// a fonte mais exata ao mesmo tempo, porque vem dos cabeçalhos da resposta e
/// reflete a conta que de fato atendeu.
public struct GroupUsageSample: Sendable, Equatable, Codable {
    /// O perfil (grupo) de onde a amostra veio, pela string de `CLAUDE_CONFIG_DIR`
    /// — ou o padrão quando a variável não estava setada.
    public let configDirRaw: String
    /// E-mail da conta que servia a sessão no instante da amostra, lido do
    /// `.claude.json` do perfil. É o que casa a amostra com uma conta.
    public let email: String?
    public let fiveHourPercent: Double?
    public let fiveHourResetsAt: Date?
    public let sevenDayPercent: Double?
    public let sevenDayResetsAt: Date?
    /// Quando a status line rodou. Uma amostra velha é melhor que nenhuma, mas a
    /// idade decide se ela pode ser mostrada sem aviso.
    public let sampledAt: Date
    /// As janelas POR MODELO, quando alguém as mediu.
    ///
    /// `nil` na amostra do sensor: o `rate_limits` da status line não as traz.
    /// Quem as preenche é a sonda ativa (`ClaudeUsageProbe`), que pergunta ao
    /// binário oficial — e por isso elas carregam **carimbo de tempo próprio**.
    /// Misturá-las na idade da amostra do sensor seria dizer que um número de
    /// ontem tem a idade do de agora, e o por modelo é exatamente o que estoura
    /// primeiro.
    public let models: ModelUsage?
    /// Serializada como opcional porque amostra gravada antes de a sonda existir
    /// só podia vir do sensor. `origin` resolve a ausência; nada no app lê isto.
    private let storedOrigin: UsageOrigin?

    /// Quem mediu as janelas de 5h e 7 dias desta amostra. (As por modelo são
    /// sempre da sonda — o sensor não as recebe.)
    public var origin: UsageOrigin { storedOrigin ?? .sensor }

    private enum CodingKeys: String, CodingKey {
        case configDirRaw, email
        case fiveHourPercent, fiveHourResetsAt
        case sevenDayPercent, sevenDayResetsAt
        case sampledAt, models
        case storedOrigin = "origin"
    }

    public init(configDirRaw: String, email: String?,
                fiveHourPercent: Double?, fiveHourResetsAt: Date?,
                sevenDayPercent: Double?, sevenDayResetsAt: Date?,
                sampledAt: Date, models: ModelUsage? = nil,
                origin: UsageOrigin = .sensor) {
        self.configDirRaw = configDirRaw
        self.email = email
        self.fiveHourPercent = fiveHourPercent
        self.fiveHourResetsAt = fiveHourResetsAt
        self.sevenDayPercent = sevenDayPercent
        self.sevenDayResetsAt = sevenDayResetsAt
        self.sampledAt = sampledAt
        self.models = models
        self.storedOrigin = origin
    }

    /// A mesma amostra, com as janelas por modelo trocadas.
    public func with(models: ModelUsage?) -> GroupUsageSample {
        GroupUsageSample(configDirRaw: configDirRaw, email: email,
                         fiveHourPercent: fiveHourPercent, fiveHourResetsAt: fiveHourResetsAt,
                         sevenDayPercent: sevenDayPercent, sevenDayResetsAt: sevenDayResetsAt,
                         sampledAt: sampledAt, models: models, origin: origin)
    }

    /// Converte a amostra no mesmo `UsageReport` que o painel já consome, para o
    /// sensor virar a fonte oficial no lugar da antiga chamada de API. `nil`
    /// quando não há nenhuma das duas janelas — nada a mostrar.
    public func asUsageReport() -> UsageReport? {
        var limits: [UsageReport.Limit] = []
        if let five = fiveHourPercent {
            limits.append(.init(kind: .session, fraction: five, severity: .normal,
                                resetsAt: fiveHourResetsAt, modelName: nil, isActive: true))
        }
        if let seven = sevenDayPercent {
            limits.append(.init(kind: .weeklyAll, fraction: seven, severity: .normal,
                                resetsAt: sevenDayResetsAt, modelName: nil, isActive: false))
        }
        guard !limits.isEmpty else { return nil }
        return UsageReport(limits: limits, fetchedAt: sampledAt)
    }
}

/// As janelas por modelo de uma conta, e quando foram medidas.
///
/// Carimbo próprio porque a origem é outra: o sensor passivo roda a cada
/// mensagem, a sonda ativa roda quando alguém pede. As duas convivem no mesmo
/// arquivo e envelhecem em ritmos diferentes.
public struct ModelUsage: Sendable, Equatable, Codable {
    public let windows: [ClaudeUsageProbe.ModelWindow]
    public let sampledAt: Date

    public init(windows: [ClaudeUsageProbe.ModelWindow], sampledAt: Date) {
        self.windows = windows
        self.sampledAt = sampledAt
    }

    /// A janela mais apertada ainda válida. Reset já passado é descartado — a
    /// medida envelheceu além do próprio limite que media.
    public func binding(now: Date = Date()) -> ClaudeUsageProbe.ModelWindow? {
        windows
            .filter { $0.resetsAt.map { $0 > now } ?? true }
            .max { $0.percent < $1.percent }
    }
}

/// Onde o sensor grava as amostras e o app as lê.
///
/// Um arquivo por conta (pelo e-mail), em `Application Support`, sobrescrito a
/// cada amostra. Não é histórico — é o último valor conhecido de cada conta,
/// que é o que decide a rotação e o que a barra mostra.
public enum GroupUsageStore {
    /// `~/Library/Application Support/<bundle>/usage/`.
    public static func directory(
        bundleID: String,
        base: URL = FileManager.default.urls(for: .applicationSupportDirectory,
                                             in: .userDomainMask).first!
    ) -> URL {
        base.appending(path: bundleID).appending(path: "usage")
    }

    /// Nome de arquivo estável e seguro para um e-mail (sem `/` nem `:`).
    public static func fileName(forEmail email: String) -> String {
        let safe = email.map { c -> Character in
            c.isLetter || c.isNumber || c == "." || c == "@" || c == "-" ? c : "_"
        }
        return String(safe) + ".json"
    }

    /// Grava a amostra da conta, de forma atômica. Chamado pelo sensor.
    ///
    /// **Preserva o bloco por modelo** quando a amostra nova não o traz. O
    /// sensor escreve a cada mensagem e nunca conhece as janelas por modelo; sem
    /// esta costura, a primeira mensagem depois de uma sondagem apagaria o
    /// número do Fable — o único que enxerga o limite que estoura primeiro. O
    /// carimbo de tempo do bloco preservado é o da sondagem, não o desta
    /// escrita, então nada finge frescor.
    public static func write(_ sample: GroupUsageSample, forEmail email: String,
                             in directory: URL) throws {
        try FileManager.default.createDirectory(at: directory,
                                                withIntermediateDirectories: true)
        let url = directory.appending(path: fileName(forEmail: email))
        let final = sample.models == nil
            ? sample.with(models: read(forEmail: email, in: directory)?.models)
            : sample
        let encoder = JSONEncoder()
        encoder.dateEncodingStrategy = .iso8601
        try encoder.encode(final).write(to: url, options: .atomic)
    }

    /// Lê a amostra de uma conta, se houver. Chamado pelo app.
    public static func read(forEmail email: String, in directory: URL) -> GroupUsageSample? {
        let url = directory.appending(path: fileName(forEmail: email))
        guard let data = try? Data(contentsOf: url) else { return nil }
        let decoder = JSONDecoder()
        decoder.dateDecodingStrategy = .iso8601
        return try? decoder.decode(GroupUsageSample.self, from: data)
    }

    /// Todas as amostras gravadas. Chamado pelo app para montar o quadro.
    public static func readAll(in directory: URL) -> [GroupUsageSample] {
        guard let files = try? FileManager.default.contentsOfDirectory(
            at: directory, includingPropertiesForKeys: nil) else { return [] }
        let decoder = JSONDecoder()
        decoder.dateDecodingStrategy = .iso8601
        return files.filter { $0.pathExtension == "json" }.compactMap { url in
            (try? Data(contentsOf: url)).flatMap { try? decoder.decode(GroupUsageSample.self, from: $0) }
        }
    }
}
