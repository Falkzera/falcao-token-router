import Foundation

/// Uma medição da sonda, como fica no histórico (21/10/2026). O vocabulário é o
/// da amostra (`fiveHourPercent`… em frações 0–1, datas ISO-8601 sem fração); a
/// janela que a leitura não trouxe fica AUSENTE — nunca zero.
///
/// É o MESMO formato do porte Windows (`usage_history.rs`): um arquivo gravado
/// por um app é lido pelo outro, como o `config.json` e o `usage/`.
public struct UsageHistoryPoint: Sendable, Equatable, Codable {
    public let sampledAt: Date
    public let fiveHourPercent: Double?
    public let fiveHourResetsAt: Date?
    public let sevenDayPercent: Double?
    public let sevenDayResetsAt: Date?
    /// As janelas por modelo da mesma leitura. Nenhum gráfico as usa ainda; vão
    /// junto porque a sonda já as tem, e um histórico que não foi gravado não
    /// volta.
    public let models: [ClaudeUsageProbe.ModelWindow]

    public init(sampledAt: Date, fiveHourPercent: Double?, fiveHourResetsAt: Date?,
                sevenDayPercent: Double?, sevenDayResetsAt: Date?,
                models: [ClaudeUsageProbe.ModelWindow] = []) {
        self.sampledAt = sampledAt
        self.fiveHourPercent = fiveHourPercent
        self.fiveHourResetsAt = fiveHourResetsAt
        self.sevenDayPercent = sevenDayPercent
        self.sevenDayResetsAt = sevenDayResetsAt
        self.models = models
    }

    /// O ponto de uma leitura da sonda, com a hora da medição.
    public init(reading: ClaudeUsageProbe.Reading, sampledAt: Date) {
        self.init(sampledAt: sampledAt,
                  fiveHourPercent: reading.session, fiveHourResetsAt: reading.sessionResetsAt,
                  sevenDayPercent: reading.weeklyAll, sevenDayResetsAt: reading.weeklyAllResetsAt,
                  models: reading.models)
    }

    private enum CodingKeys: String, CodingKey {
        case sampledAt, fiveHourPercent, fiveHourResetsAt, sevenDayPercent, sevenDayResetsAt, models
    }

    public init(from decoder: Decoder) throws {
        let c = try decoder.container(keyedBy: CodingKeys.self)
        sampledAt = try c.decode(Date.self, forKey: .sampledAt)
        fiveHourPercent = try c.decodeIfPresent(Double.self, forKey: .fiveHourPercent)
        fiveHourResetsAt = try c.decodeIfPresent(Date.self, forKey: .fiveHourResetsAt)
        sevenDayPercent = try c.decodeIfPresent(Double.self, forKey: .sevenDayPercent)
        sevenDayResetsAt = try c.decodeIfPresent(Date.self, forKey: .sevenDayResetsAt)
        models = try c.decodeIfPresent([ClaudeUsageProbe.ModelWindow].self, forKey: .models) ?? []
    }

    public func encode(to encoder: Encoder) throws {
        var c = encoder.container(keyedBy: CodingKeys.self)
        try c.encode(sampledAt, forKey: .sampledAt)
        try c.encodeIfPresent(fiveHourPercent, forKey: .fiveHourPercent)
        try c.encodeIfPresent(fiveHourResetsAt, forKey: .fiveHourResetsAt)
        try c.encodeIfPresent(sevenDayPercent, forKey: .sevenDayPercent)
        try c.encodeIfPresent(sevenDayResetsAt, forKey: .sevenDayResetsAt)
        // Sem janela por modelo, nem a chave — como o Windows grava.
        if !models.isEmpty { try c.encode(models, forKey: .models) }
    }
}

/// O histórico de uso de cada conta: um ponto por medição da SONDA, para a aba
/// Histórico desenhar a janela de 5h e a semanal ao longo do tempo.
///
/// Mora fora do `usage/` de propósito. Aquele arquivo é a ÚLTIMA amostra — o
/// número que decide a rotação, regravado pelo sensor a cada render da status
/// line. Aqui fica a sequência: `<base>/history/<email>.jsonl`, uma linha por
/// medição, a mais nova no fim. Só a sonda grava (a medição de 5 em 5 horas, o
/// "Medir contas" e o `router measure`) — o sensor não entra.
public enum UsageHistory {
    /// Quanto o histórico guarda. O gráfico mais longo mostra 4 semanas; o
    /// resto é folga — uns 5 pontos por dia, ~450 linhas por conta.
    public static let retentionDays = 90

    /// A pasta: `<base>/history`.
    public static func directory(base: URL) -> URL { base.appending(path: "history") }

    /// O nome do arquivo do `usage/` (a mesma limpeza do e-mail), com `.jsonl`.
    public static func fileName(forEmail email: String) -> String {
        GroupUsageStore.safeStem(forEmail: email) + ".jsonl"
    }

    /// Os pontos de uma conta, do mais velho ao mais novo. Arquivo ausente =
    /// nenhum ponto; linha que não se lê é pulada, e as outras ficam.
    public static func read(forEmail email: String, in directory: URL) -> [UsageHistoryPoint] {
        let url = directory.appending(path: fileName(forEmail: email))
        guard let data = try? Data(contentsOf: url) else { return [] }
        let decoder = JSONDecoder()
        decoder.dateDecodingStrategy = .iso8601
        return String(decoding: data, as: UTF8.self)
            .split(whereSeparator: \.isNewline)
            .compactMap { try? decoder.decode(UsageHistoryPoint.self, from: Data($0.utf8)) }
            .sorted { $0.sampledAt < $1.sampledAt }
    }

    /// Quando a conta foi medida pela última vez — venha de onde vier a medição
    /// (a agenda, o botão, a CLI).
    public static func lastSampledAt(forEmail email: String, in directory: URL) -> Date? {
        read(forEmail: email, in: directory).last?.sampledAt
    }

    /// Acrescenta um ponto, regravando o arquivo inteiro de forma atômica: uma
    /// queda no meio nunca deixa linha pela metade, e a poda — o que passou de
    /// `retentionDays` antes de `now` — acontece no mesmo passo.
    ///
    /// Sem trava entre processos (o Windows grava sob a trava do motor; aqui
    /// não há uma): o app e um `router measure` medindo a MESMA conta no mesmo
    /// instante podem perder um dos dois pontos — vence quem regravar por
    /// último. O arquivo nunca fica quebrado, e um ponto a menos vira, no
    /// máximo, um buraco no gráfico.
    public static func append(_ point: UsageHistoryPoint, forEmail email: String,
                              in directory: URL, now: Date = Date()) throws {
        let cutoff = now.addingTimeInterval(-TimeInterval(retentionDays) * 86_400)
        var points = read(forEmail: email, in: directory).filter { $0.sampledAt >= cutoff }
        points.append(point)
        points.sort { $0.sampledAt < $1.sampledAt }

        let encoder = JSONEncoder()
        encoder.dateEncodingStrategy = .iso8601
        encoder.outputFormatting = [.sortedKeys, .withoutEscapingSlashes]
        var text = Data()
        for p in points {
            text.append(try encoder.encode(p))
            text.append(0x0A)
        }
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        try text.write(to: directory.appending(path: fileName(forEmail: email)), options: .atomic)
    }

    /// Apaga o histórico de todas as contas — o "Apagar o histórico…" dos
    /// Ajustes: os `.jsonl` da pasta, e só eles. Pasta ausente = nada a apagar.
    /// Devolve quantos arquivos saíram.
    @discardableResult
    public static func clear(in directory: URL) throws -> Int {
        guard FileManager.default.fileExists(atPath: directory.path) else { return 0 }
        let files = try FileManager.default.contentsOfDirectory(
            at: directory, includingPropertiesForKeys: [.isRegularFileKey])
            .filter { url in
                url.pathExtension == "jsonl"
                    && (try? url.resourceValues(forKeys: [.isRegularFileKey]).isRegularFile) == true
            }
        for file in files { try FileManager.default.removeItem(at: file) }
        return files.count
    }
}
