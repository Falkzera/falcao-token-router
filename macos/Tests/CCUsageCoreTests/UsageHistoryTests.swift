import Foundation
import Testing
@testable import CCUsageCore

/// O histórico de uso (21/10/2026): um ponto por medição da sonda em
/// `<base>/history/<email>.jsonl` — o MESMO arquivo que o porte Windows grava.
@Suite("UsageHistory (o histórico da sonda)")
struct UsageHistoryTests {
    private let t0 = Date(timeIntervalSince1970: 1_792_000_000)  // 14/10/2026
    private let email = "conta1@exemplo.com"

    private func tempDir() -> URL {
        URL(fileURLWithPath: NSTemporaryDirectory())
            .appending(path: "history-test-\(UUID().uuidString)")
    }

    private func point(_ at: Date, five: Double?) -> UsageHistoryPoint {
        UsageHistoryPoint(sampledAt: at, fiveHourPercent: five, fiveHourResetsAt: nil,
                          sevenDayPercent: 0.5, sevenDayResetsAt: nil)
    }

    @Test("os pontos voltam do mais velho ao mais novo")
    func ordem() throws {
        let dir = tempDir()
        let later = t0.addingTimeInterval(3600)
        try UsageHistory.append(point(later, five: 0.2), forEmail: email, in: dir, now: later)
        try UsageHistory.append(point(t0, five: 0.1), forEmail: email, in: dir, now: later)

        #expect(UsageHistory.read(forEmail: email, in: dir).map(\.fiveHourPercent) == [0.1, 0.2])
        #expect(UsageHistory.lastSampledAt(forEmail: email, in: dir) == later)
    }

    @Test("o que passou de 90 dias sai na gravação seguinte")
    func poda() throws {
        let dir = tempDir()
        try UsageHistory.append(point(t0, five: 0.1), forEmail: email, in: dir, now: t0)
        let later = t0.addingTimeInterval(TimeInterval(UsageHistory.retentionDays) * 86_400 + 3600)
        try UsageHistory.append(point(later, five: 0.2), forEmail: email, in: dir, now: later)

        let read = UsageHistory.read(forEmail: email, in: dir)
        #expect(read.count == 1)
        #expect(read.first?.fiveHourPercent == 0.2)
    }

    /// Uma linha estragada não leva as outras — e a próxima gravação regrava o
    /// arquivo só com as boas.
    @Test("linha estragada é pulada e as outras ficam")
    func linhaEstragada() throws {
        let dir = tempDir()
        try FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
        let good1 = #"{"sampledAt":"2026-10-14T00:00:00Z","fiveHourPercent":0.1}"#
        let good2 = #"{"sampledAt":"2026-10-14T00:01:00Z","fiveHourPercent":0.2}"#
        try Data("\(good1)\n{ meio escrito\n\(good2)\r\n".utf8)
            .write(to: dir.appending(path: UsageHistory.fileName(forEmail: email)))

        #expect(UsageHistory.read(forEmail: email, in: dir).count == 2)

        try UsageHistory.append(point(t0, five: 0.3), forEmail: email, in: dir, now: t0)
        let text = try String(contentsOf: dir.appending(path: UsageHistory.fileName(forEmail: email)),
                              encoding: .utf8)
        #expect(text.split(whereSeparator: \.isNewline).count == 3)
        #expect(!text.contains("meio escrito"))
    }

    @Test("sem arquivo, nenhum ponto")
    func semArquivo() {
        let dir = tempDir()
        #expect(UsageHistory.read(forEmail: email, in: dir).isEmpty)
        #expect(UsageHistory.lastSampledAt(forEmail: email, in: dir) == nil)
    }

    @Test("o nome é o do usage, com .jsonl")
    func nome() {
        #expect(UsageHistory.fileName(forEmail: "conta 1+x@exemplo.com") == "conta_1_x@exemplo.com.jsonl")
        #expect(GroupUsageStore.fileName(forEmail: "conta 1+x@exemplo.com") == "conta_1_x@exemplo.com.json")
        #expect(UsageHistory.directory(base: URL(fileURLWithPath: "/base")).lastPathComponent == "history")
    }

    /// Como a amostra e como o Windows: data sem fração, janela ausente fora da
    /// linha — nunca zero —, e sem janela por modelo nem a chave.
    @Test("data sem fração e janela ausente fora da linha")
    func formato() throws {
        let dir = tempDir()
        let at = t0.addingTimeInterval(0.5)
        try UsageHistory.append(point(at, five: nil), forEmail: email, in: dir, now: at)

        let text = try String(contentsOf: dir.appending(path: UsageHistory.fileName(forEmail: email)),
                              encoding: .utf8)
        #expect(text.contains(#""sampledAt":"2026-10-14T"#))
        #expect(!text.contains(".5Z"))
        #expect(!text.contains("fiveHourPercent"))
        #expect(text.contains(#""sevenDayPercent":0.5"#))
        #expect(!text.contains("models"))
    }

    /// O contrato com o Windows: uma linha EXATAMENTE como o porte grava se lê
    /// aqui, com as janelas por modelo.
    @Test("uma linha gravada pelo Windows se lê aqui")
    func linhaDoWindows() throws {
        let dir = tempDir()
        try FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
        let line = #"{"sampledAt":"2026-10-21T12:00:00Z","fiveHourPercent":0.29,"fiveHourResetsAt":"2026-10-21T14:05:00Z","sevenDayPercent":0.33,"sevenDayResetsAt":"2026-10-27T09:00:00Z","models":[{"name":"Fable","percent":0.12,"resetsAt":"2026-10-27T09:00:00Z"}]}"#
        try Data((line + "\n").utf8).write(to: dir.appending(path: UsageHistory.fileName(forEmail: email)))

        let p = try #require(UsageHistory.read(forEmail: email, in: dir).first)
        #expect(p.fiveHourPercent == 0.29)
        #expect(p.sevenDayPercent == 0.33)
        #expect(p.fiveHourResetsAt != nil)
        #expect(p.models.first?.name == "Fable")
        #expect(p.models.first?.percent == 0.12)
    }

    /// "Apagar o histórico…" leva os pontos de todas as contas — os `.jsonl`, e
    /// só eles.
    @Test("apagar leva só os .jsonl")
    func apagar() throws {
        let dir = tempDir()
        try UsageHistory.append(point(t0, five: 0.1), forEmail: email, in: dir, now: t0)
        try UsageHistory.append(point(t0, five: 0.2), forEmail: "conta2@exemplo.com", in: dir, now: t0)
        try Data("nao e historico".utf8).write(to: dir.appending(path: "leia-me.txt"))

        #expect(try UsageHistory.clear(in: dir) == 2)
        #expect(UsageHistory.read(forEmail: email, in: dir).isEmpty)
        #expect(FileManager.default.fileExists(atPath: dir.appending(path: "leia-me.txt").path))
    }

    @Test("apagar sem pasta não é erro")
    func apagarSemPasta() throws {
        #expect(try UsageHistory.clear(in: tempDir()) == 0)
    }

    // MARK: - A medição grava os dois (ProbeRecord)

    private let saida = """
    Current session: 12% used · resets Sep 22 at 8:40pm (America/Sao_Paulo)
    Current week (all models): 34% used · resets Sep 23 at 4am (America/Sao_Paulo)
    Current week (Fable): 56% used · resets Sep 23 at 4am (America/Sao_Paulo)
    """

    @Test("a medição grava a amostra e o ponto do histórico")
    func medicaoGravaOsDois() throws {
        let base = tempDir()
        let reading = try ClaudeUsageProbe.parse(saida, now: t0)
        try ProbeRecord.write(reading, email: email, configDir: .dedicated("/Users/exemplo/casa"),
                              usageDir: base.appending(path: "usage"),
                              historyDir: base.appending(path: "history"), now: t0)

        let sample = try #require(GroupUsageStore.read(forEmail: email, in: base.appending(path: "usage")))
        #expect(sample.origin == .probe)
        #expect(sample.fiveHourPercent == 0.12)
        let history = UsageHistory.read(forEmail: email, in: base.appending(path: "history"))
        #expect(history.count == 1)
        #expect(history.first?.sampledAt == t0)
        #expect(history.first?.fiveHourPercent == 0.12)
        #expect(history.first?.sevenDayPercent == 0.34)
        #expect(history.first?.models.first?.name == "Fable")
    }

    /// Histórico que não se grava não desfaz a medição: a amostra, que decide a
    /// rotação, já está no disco.
    @Test("histórico que não se grava não desfaz a medição")
    func historicoFalhaSemDesfazer() throws {
        let base = tempDir()
        try FileManager.default.createDirectory(at: base, withIntermediateDirectories: true)
        // Um ARQUIVO onde a pasta do histórico deveria estar.
        try Data("nao e pasta".utf8).write(to: base.appending(path: "history"))
        let reading = try ClaudeUsageProbe.parse(saida, now: t0)

        try ProbeRecord.write(reading, email: email, configDir: .dedicated("/Users/exemplo/casa"),
                              usageDir: base.appending(path: "usage"),
                              historyDir: base.appending(path: "history"), now: t0)
        #expect(GroupUsageStore.read(forEmail: email, in: base.appending(path: "usage")) != nil)
    }

    /// Leitura sem a linha da sessão deixa o ponto SEM a janela de 5h.
    @Test("leitura sem a sessão não vira 5h zero")
    func semSessao() throws {
        let base = tempDir()
        let reading = try ClaudeUsageProbe.parse("Current week (all models): 40% used", now: t0)
        try ProbeRecord.write(reading, email: email, configDir: .dedicated("/Users/exemplo/casa"),
                              usageDir: base.appending(path: "usage"),
                              historyDir: base.appending(path: "history"), now: t0)
        let p = try #require(UsageHistory.read(forEmail: email, in: base.appending(path: "history")).first)
        #expect(p.fiveHourPercent == nil)
        #expect(p.sevenDayPercent == 0.4)
    }
}
