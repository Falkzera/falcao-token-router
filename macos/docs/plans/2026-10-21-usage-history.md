# Usage History (macOS) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Port the Windows usage history (PR #35) to the macOS app: measure every account with the probe every 5 hours while the app runs, keep each measurement in `history/<email>.jsonl` (the same file the Windows port writes), and show a History tab with two line charts per account, a shortcut from each account, and "Delete history…" in Settings.

**Architecture:** The engine (`CCUsageCore`) gets the history store, one place that records a probe measurement (sample + history point, used by the button, the schedule and `router measure`), the pure "is it due?" rule, the per-group plan and the chart series (cut, split at holes, ticks). The `RouterConfigStore` keeps the scheduled attempts and runs the schedule from the app's existing 180 s loop. The SwiftUI app adds the History tab with **Swift Charts** (no new dependency — it ships with macOS), the shortcut buttons and the Settings section.

**Tech Stack:** Swift 6, SwiftUI, Swift Charts, Observation, swift-testing. macOS 26.

**Where the design comes from:** `windows/docs/specs/2026-10-21-usage-history-design.md` (on the `feat/windows-historico` branch / PR #35 until it merges). Read it first; this plan only adapts it. The Windows code is the reference for every rule: `windows/crates/router-core/src/engine/usage_history.rs`, `engine/measure_schedule.rs`, `engine/router_config_store/rotation.rs`, `windows/app/src-tauri/src/history.rs`. While #35 is open: `git show fork/feat/windows-historico:<path>` (or browse the PR).

> ⚠️ **This plan was written on a Windows machine, without a Swift toolchain.** Every type, signature and call site below was checked against the sources of `main` at `fdb3024` by reading them (21/10/2026), not by compiling. Expect small fixes when `./Scripts/test.sh` first runs; the tests are the contract — fix the code, not the tests.

## Before starting

1. PR #35 merged (or not — the plan works either way):
   - merged: `git fetch origin && git checkout feat/macos-historico && git merge origin/main`
   - not yet: start anyway; Task 9's edits to `windows/` files and to the root docs wait for the merge.
2. `./Scripts/test.sh` green on `main` before touching anything (from `macos/`).

## Global Constraints

- Branch `feat/macos-historico`; Conventional Commits (`feat(macos): …`); commit messages end with the repo's attribution line.
- Comments and `agent.md` in **pt-BR**; identifiers in English; README/CHANGELOG/docs in English.
- **No network calls.** The probe is the official `claude --print /usage`, as today.
- **Never write `@State`** — use `@ViewState` (`Sources/FalcaoTokenRouter/ViewState.swift`).
- Every UI string from `Resources/{en,pt-BR}.lproj/Localizable.strings`; `check-strings.sh` must pass. The History tab's keys live under the existing prefix **`home.history.*`** — *not* a new `history` prefix: `Engine/ProfileSharing.swift` has the literal `"history.jsonl"` (Claude Code's file), and the checker would take it for a missing key. Computed text goes through `Text(verbatim:)`.
- State the UI shows live is a stored **observable** property of the store (a computed property that reads disk does not re-render).
- No production file over 600 lines. `RouterConfigStore.swift` is at 545: the new store code goes in `Engine/RouterConfigStore+History.swift`.
- **The file format is the contract with Windows** — identical: `<base>/history/<email>.jsonl`, same e-mail sanitization as `GroupUsageStore`, one JSON object per line, keys `sampledAt`, `fiveHourPercent`, `fiveHourResetsAt`, `sevenDayPercent`, `sevenDayResetsAt`, `models` (`name`, `percent`, `resetsAt`), fractions 0–1, ISO-8601 dates **without fractional seconds**, absent window = absent key (never 0), `models` omitted when empty, retention **90 days**, whole file rewritten atomically.
- Schedule: every **5 hours** per account; due = newest of (last history point, last scheduled attempt this app run) absent or ≥ 5 h old; checked with the rotation (180 s) from the **second** pass; skipped while the setting is off, a measurement runs or a sign-in is in progress; failures **never** set `lastError`.
- Setting `scheduledMeasure` in `AppSettings`, **on by default**; a payload without the field = on.
- Charts: 5 h window over **7 days**, weekly over **28 days**; more than **7 h 30 min** between two points breaks the line; y axis fixed 0–100%; dashed threshold line; a point without that window is left out (not drawn at 0).
- Fixtures anonymized: `conta1@exemplo.com`, `/Users/exemplo`, org `Acme`.

---

### Task 1: The history store

**Files:**
- Create: `Sources/CCUsageCore/Engine/UsageHistory.swift`
- Modify: `Sources/CCUsageCore/Engine/GroupUsage.swift` (`safeStem`), `Sources/CCUsageCore/Engine/RouterPaths.swift` (`historyDir`)
- Test: `Tests/CCUsageCoreTests/UsageHistoryTests.swift`

**Interfaces (produces):**
- `public struct UsageHistoryPoint: Sendable, Equatable, Codable` — `sampledAt: Date`, `fiveHourPercent: Double?`, `fiveHourResetsAt: Date?`, `sevenDayPercent: Double?`, `sevenDayResetsAt: Date?`, `models: [ClaudeUsageProbe.ModelWindow]`; `init(sampledAt:fiveHourPercent:fiveHourResetsAt:sevenDayPercent:sevenDayResetsAt:models:)`, `init(reading: ClaudeUsageProbe.Reading, sampledAt: Date)`.
- `public enum UsageHistory` — `retentionDays = 90`, `directory(base:) -> URL`, `fileName(forEmail:) -> String`, `read(forEmail:in:) -> [UsageHistoryPoint]`, `lastSampledAt(forEmail:in:) -> Date?`, `append(_:forEmail:in:now:) throws`, `clear(in:) throws -> Int`.
- `GroupUsageStore.safeStem(forEmail:) -> String`; `RouterPaths.historyDir: URL`.

- [ ] **Step 1: Write the failing tests** — `Tests/CCUsageCoreTests/UsageHistoryTests.swift`:

```swift
import Foundation
import Testing
@testable import CCUsageCore

/// O histórico de uso (21/10/2026): um ponto por medição da sonda em
/// `<base>/history/<email>.jsonl` — o MESMO arquivo que o porte Windows grava.
@Suite("UsageHistory (o histórico da sonda)")
struct UsageHistoryTests {
    private let t0 = Date(timeIntervalSince1970: 1_792_000_000)  // outubro de 2026
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
}
```

- [ ] **Step 2: Run** — `./Scripts/test.sh --filter UsageHistory` → compile error (`UsageHistory` not found).

- [ ] **Step 3: Implement**

In `GroupUsage.swift`, replace `fileName(forEmail:)` with:

```swift
    /// O e-mail como pedaço de nome de arquivo, estável e seguro (sem `/` nem
    /// `:`). O histórico usa a mesma limpeza — e o porte Windows também.
    public static func safeStem(forEmail email: String) -> String {
        String(email.map { c -> Character in
            c.isLetter || c.isNumber || c == "." || c == "@" || c == "-" ? c : "_"
        })
    }

    /// Nome de arquivo estável e seguro para um e-mail.
    public static func fileName(forEmail email: String) -> String {
        safeStem(forEmail: email) + ".json"
    }
```

In `RouterPaths.swift`, after `usageDir`:

```swift
    /// O histórico das medições da sonda (21/10/2026): `<base>/history`. Mesmo
    /// formato do porte Windows — ver `UsageHistory`.
    public var historyDir: URL { UsageHistory.directory(base: base) }
```

Create `Sources/CCUsageCore/Engine/UsageHistory.swift`:

```swift
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

    /// Quando a conta foi medida pela última vez — venha de onde vier a medição.
    public static func lastSampledAt(forEmail email: String, in directory: URL) -> Date? {
        read(forEmail: email, in: directory).last?.sampledAt
    }

    /// Acrescenta um ponto, regravando o arquivo inteiro de forma atômica: uma
    /// queda no meio nunca deixa linha pela metade, e a poda — o que passou de
    /// `retentionDays` antes de `now` — acontece no mesmo passo.
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
            at: directory, includingPropertiesForKeys: nil)
            .filter { $0.pathExtension == "jsonl" }
        for file in files { try FileManager.default.removeItem(at: file) }
        return files.count
    }
}
```

- [ ] **Step 4: Run** — `./Scripts/test.sh --filter UsageHistory` → 9 passed; then `./Scripts/test.sh` → all green.
- [ ] **Step 5: Commit** — `feat(macos): histórico de uso por conta no motor`

---

### Task 2: One place records a probe measurement

**Files:**
- Create: `Sources/CCUsageCore/Usage/ProbeRecord.swift`
- Modify: `Sources/CCUsageCore/Engine/RouterConfigStore.swift` (`measureAccounts` uses it), `Sources/router/main.swift` (`Measure.run` uses it)
- Test: `Tests/CCUsageCoreTests/UsageHistoryTests.swift` (append)

**Interfaces:** `public enum ProbeRecord { static func write(_ reading: ClaudeUsageProbe.Reading, email: String, configDir: ConfigDir, usageDir: URL, historyDir: URL, now: Date = Date()) throws }`.

- [ ] **Step 1: Failing tests** — append to `UsageHistoryTests`:

```swift
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
```

- [ ] **Step 2: Run** — fails (`ProbeRecord` not found).
- [ ] **Step 3: Implement** — `Sources/CCUsageCore/Usage/ProbeRecord.swift`:

```swift
import Foundation

/// O que uma medição da sonda grava (21/10/2026): a amostra — o número que
/// decide a rotação — e o ponto do histórico.
///
/// Num lugar só porque três caminhos medem (o "Medir contas", a agenda de 5 em
/// 5 horas e o `router measure`), e antes o store e a CLI montavam a amostra à
/// mão, cada um a sua cópia.
public enum ProbeRecord {
    /// Grava a amostra (origem `probe`, com as janelas por modelo e o carimbo
    /// delas) e depois o ponto do histórico. Falha no histórico não desfaz a
    /// medição: a amostra já está no disco.
    public static func write(_ reading: ClaudeUsageProbe.Reading, email: String,
                             configDir: ConfigDir, usageDir: URL, historyDir: URL,
                             now: Date = Date()) throws {
        let sample = GroupUsageSample(
            configDirRaw: configDir.raw, email: email,
            fiveHourPercent: reading.session, fiveHourResetsAt: reading.sessionResetsAt,
            sevenDayPercent: reading.weeklyAll, sevenDayResetsAt: reading.weeklyAllResetsAt,
            sampledAt: now,
            models: ModelUsage(windows: reading.models, sampledAt: now),
            origin: .probe)
        try GroupUsageStore.write(sample, forEmail: email, in: usageDir)
        try? UsageHistory.append(UsageHistoryPoint(reading: reading, sampledAt: now),
                                 forEmail: email, in: historyDir, now: now)
    }
}
```

In `RouterConfigStore.measureAccounts(in:)`: add `let historyDir = paths.historyDir` next to `usageDir`, and replace the body of the `do` inside the loop with:

```swift
                do {
                    let leitura = try probe.read(configDir: alvo.dir)
                    try ProbeRecord.write(leitura, email: alvo.email, configDir: alvo.dir,
                                          usageDir: usageDir, historyDir: historyDir)
                } catch {
                    erros += 1
                }
```

Also update its doc: "Sob demanda (este botão) — e, com o app aberto, de 5 em 5 horas pela agenda (`runScheduledMeasure`, 21/10/2026). Nunca no laço de 3 minutos: cada conta custa um processo Node e uma requisição." And the doc of `ClaudeUsageProbe` (**Custo**): "sob demanda (`router measure`, "Medir contas") e, desde 21/10/2026, de 5 em 5 horas pela agenda do app — nunca no laço de 3 minutos. Toda medição deixa um ponto no histórico (`UsageHistory`)."

In `router/main.swift`, `Measure.run`: `let paths = RouterPaths()`, `let dir = paths.usageDir`, and inside the `do`:

```swift
                let leitura = try probe.read(configDir: perfil)
                try ProbeRecord.write(leitura, email: conta.identity.email, configDir: perfil,
                                      usageDir: dir, historyDir: paths.historyDir)
                print("  \(conta.label): \(descreve(leitura))")
```

Doc comment of `Measure`: "…então aqui é comando; a medição de 5 em 5 horas é do app (21/10/2026). Toda medição deixa um ponto no histórico, a desta CLI também."

- [ ] **Step 4: Run** — `./Scripts/test.sh` → green.
- [ ] **Step 5: Commit** — `feat(macos): toda medição da sonda deixa um ponto no histórico`

---

### Task 3: The schedule rule, the plan and the scheduled run

**Files:**
- Create: `Sources/CCUsageCore/Engine/MeasureSchedule.swift`, `Sources/CCUsageCore/Engine/RouterConfigStore+History.swift`
- Modify: `Sources/CCUsageCore/Engine/RouterConfigStore.swift` (visibility + new observable state)
- Test: `Tests/CCUsageCoreTests/MeasureScheduleTests.swift`, `Tests/CCUsageCoreTests/HistoryStoreTests.swift`

**Interfaces:**
- `public enum MeasureSchedule { static let every: TimeInterval; static func nextAt(lastMeasured:lastAttempt:) -> Date?; static func isDue(lastMeasured:lastAttempt:now:) -> Bool }`
- `public struct ProbeTarget: Sendable, Equatable { accountID: UUID, email: String, dir: ConfigDir }`
- `public struct MeasurePlan: Sendable, Equatable { groupID: UUID, targets: [ProbeTarget] }` — a struct, not a labeled tuple: the tests use `plans.map(\.groupID)`, and key paths into tuples are not something to bet a compile on.
- On `RouterConfigStore`: `historyByAccount: [UUID: [UsageHistoryPoint]]` (observable), `measureAttempts: [UUID: Date]` (ignored), `signingIn: Bool` (ignored), `probeTargets(in:)`, `scheduledMeasurePlans(now:) -> [MeasurePlan]`, `runScheduledMeasure(now:) async`, `clearHistory() throws -> Int`, `refreshHistory()`.

- [ ] **Step 1: Failing tests**

`Tests/CCUsageCoreTests/MeasureScheduleTests.swift`:

```swift
import Foundation
import Testing
@testable import CCUsageCore

/// A medição automática de 5 em 5 horas: a regra de QUANDO medir de novo.
@Suite("MeasureSchedule")
struct MeasureScheduleTests {
    private let now = Date(timeIntervalSince1970: 1_792_000_000)

    @Test("conta nunca medida nem tentada vence")
    func nunca() {
        #expect(MeasureSchedule.isDue(lastMeasured: nil, lastAttempt: nil, now: now))
        #expect(MeasureSchedule.nextAt(lastMeasured: nil, lastAttempt: nil) == nil)
    }

    @Test("medida há menos de 5 horas não vence")
    func recente() {
        #expect(!MeasureSchedule.isDue(lastMeasured: now.addingTimeInterval(-4 * 3600),
                                       lastAttempt: nil, now: now))
        #expect(!MeasureSchedule.isDue(lastMeasured: now.addingTimeInterval(-299 * 60),
                                       lastAttempt: nil, now: now))
    }

    @Test("5 horas exatas vencem")
    func exato() {
        #expect(MeasureSchedule.isDue(lastMeasured: now.addingTimeInterval(-5 * 3600),
                                      lastAttempt: nil, now: now))
    }

    /// A conta deslogada falha toda vez: sem a tentativa, seria sondada a cada
    /// volta do laço (3 min). Com ela, de novo só em 5 horas.
    @Test("a tentativa que falhou segura a conta por 5 horas")
    func tentativa() {
        let old = now.addingTimeInterval(-2 * 86_400)
        #expect(!MeasureSchedule.isDue(lastMeasured: old, lastAttempt: now.addingTimeInterval(-3600), now: now))
        #expect(MeasureSchedule.isDue(lastMeasured: old, lastAttempt: now.addingTimeInterval(-5 * 3600), now: now))
        #expect(MeasureSchedule.nextAt(lastMeasured: now.addingTimeInterval(-3600),
                                       lastAttempt: now.addingTimeInterval(-3 * 3600))
                == now.addingTimeInterval(4 * 3600))
    }
}
```

`Tests/CCUsageCoreTests/HistoryStoreTests.swift`:

```swift
import Foundation
import Testing
@testable import CCUsageCore

/// O store e o histórico (21/10/2026): o plano da agenda, apagar, o quadro.
@MainActor
@Suite("RouterConfigStore + histórico")
struct HistoryStoreTests {
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

    /// Um ponto no histórico de uma conta, `hours` horas atrás.
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
```

- [ ] **Step 2: Run** — fails to compile (no `MeasureSchedule`, no `scheduledMeasurePlans`…).

- [ ] **Step 3: Implement**

`Sources/CCUsageCore/Engine/MeasureSchedule.swift`:

```swift
import Foundation

/// A medição automática de 5 em 5 horas (21/10/2026): QUANDO cada conta deve
/// ser medida de novo. Pura — o relógio entra por parâmetro.
///
/// A sonda era "botão e comando, nunca laço": cada medição sobe um Node e faz
/// uma requisição por conta. O histórico pediu uma medição a cada 5 horas —
/// umas 5 por conta por dia —, com o app aberto e uma chave nos Ajustes que a
/// desliga. Mesma regra do porte Windows (`measure_schedule.rs`).
public enum MeasureSchedule {
    /// O intervalo entre duas medições da mesma conta.
    public static let every: TimeInterval = 5 * 3600

    /// Quando a conta deve ser medida de novo: 5 horas depois da mais nova
    /// entre a última medição gravada (venha de onde vier) e a última tentativa
    /// da agenda nesta execução do app — sucesso ou falha. A tentativa é o que
    /// segura uma conta deslogada, que sem ela seria sondada a cada volta do
    /// laço. `nil`: nunca medida nem tentada.
    public static func nextAt(lastMeasured: Date?, lastAttempt: Date?) -> Date? {
        [lastMeasured, lastAttempt].compactMap { $0 }.max().map { $0.addingTimeInterval(every) }
    }

    /// A conta vence quando nunca foi medida nem tentada, ou quando a
    /// referência tem 5 horas ou mais.
    public static func isDue(lastMeasured: Date?, lastAttempt: Date?, now: Date) -> Bool {
        nextAt(lastMeasured: lastMeasured, lastAttempt: lastAttempt).map { $0 <= now } ?? true
    }
}
```

In `RouterConfigStore.swift`:
- `measuringGroup`: `public private(set) var` → `public internal(set) var` (the extension file sets it).
- `@ObservationIgnored private let paths` and `private let engine` → drop `private` (internal): the extension in the other file needs them.
- Add, after `measuringGroup`:

```swift
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
```

- At the end of `refreshUsage()`: `refreshHistory()`.

Create `Sources/CCUsageCore/Engine/RouterConfigStore+History.swift`:

```swift
import Foundation

/// Uma conta a medir, com o perfil já decidido pelo motor.
public struct ProbeTarget: Sendable, Equatable {
    public let accountID: UUID
    public let email: String
    public let dir: ConfigDir
}

/// Um grupo na medição automática: as contas dele que venceram, na ordem da
/// lista. A marca "medindo" do grupo acompanha o plano.
public struct MeasurePlan: Sendable, Equatable {
    public let groupID: UUID
    public let targets: [ProbeTarget]
}

/// O histórico e a medição de 5 em 5 horas (21/10/2026). Arquivo à parte do
/// `RouterConfigStore.swift` pela régua de 600 linhas de produção.
extension RouterConfigStore {
    /// As contas de um grupo com o perfil por onde medir cada uma — decidido
    /// aqui, com o config na mão: conta ativa vai pelo perfil do grupo, nunca
    /// pela casa (sondar a casa de uma conta ativa derruba a sessão viva).
    public func probeTargets(in group: AccountGroup) -> [ProbeTarget] {
        config.accounts(in: group).map {
            ProbeTarget(accountID: $0.id, email: $0.identity.email,
                        dir: engine.probeConfigDir(for: $0, config: config))
        }
    }

    /// A medição automática: por grupo, na ordem da lista, um plano só com as
    /// contas VENCIDAS (`MeasureSchedule`). Cada conta vai uma vez — a que está
    /// em dois grupos, no primeiro —, e grupo sem conta vencida fica de fora.
    public func scheduledMeasurePlans(now: Date = Date()) -> [MeasurePlan] {
        let history = paths.historyDir
        var planned = Set<UUID>()
        var plans: [MeasurePlan] = []
        for group in config.groups {
            let targets = probeTargets(in: group).filter { target in
                !planned.contains(target.accountID) && MeasureSchedule.isDue(
                    lastMeasured: UsageHistory.lastSampledAt(forEmail: target.email, in: history),
                    lastAttempt: measureAttempts[target.accountID], now: now)
            }
            guard !targets.isEmpty else { continue }
            planned.formUnion(targets.map(\.accountID))
            plans.append(MeasurePlan(groupID: group.id, targets: targets))
        }
        return plans
    }

    /// Uma olhada na agenda: mede, grupo a grupo, as contas vencidas. Chamada
    /// pelo laço do app (180 s). Silenciosa: falha não vira `lastError` — ele é
    /// da última ação DO USUÁRIO; a conta que não respondeu vira um buraco no
    /// gráfico. Espera medição em curso (o botão) e login aberto.
    public func runScheduledMeasure(now: Date = Date()) async {
        guard measuringGroup == nil, !signingIn else { return }
        let plans = scheduledMeasurePlans(now: now)
        guard !plans.isEmpty else { return }
        for plan in plans {
            for target in plan.targets { measureAttempts[target.accountID] = now }
        }
        let usageDir = paths.usageDir, historyDir = paths.historyDir, scratchBase = paths.base
        for plan in plans {
            // A marca passa de grupo em grupo sem se apagar no meio: o botão
            // não começa outra medição entre dois grupos.
            measuringGroup = plan.groupID
            let targets = plan.targets
            await Task.detached(priority: .utility) {
                guard let probe = ClaudeUsageProbe.system(scratchBase: scratchBase) else { return }
                for target in targets {
                    guard let reading = try? probe.read(configDir: target.dir) else { continue }
                    try? ProbeRecord.write(reading, email: target.email, configDir: target.dir,
                                           usageDir: usageDir, historyDir: historyDir)
                }
            }.value
            refreshUsage()
        }
        measuringGroup = nil
    }

    /// "Apagar o histórico…" (Ajustes): os pontos de todas as contas saem do
    /// disco. As medições continuam: cada conta ganha um novo primeiro ponto na
    /// próxima. Devolve quantos arquivos saíram.
    @discardableResult
    public func clearHistory() throws -> Int {
        let removed = try UsageHistory.clear(in: paths.historyDir)
        refreshHistory()
        return removed
    }

    /// Relê o histórico de cada conta.
    public func refreshHistory() {
        let dir = paths.historyDir
        var byAccount: [UUID: [UsageHistoryPoint]] = [:]
        for account in config.accounts {
            byAccount[account.id] = UsageHistory.read(forEmail: account.identity.email, in: dir)
        }
        historyByAccount = byAccount
    }
}
```

- [ ] **Step 4: Run** — `./Scripts/test.sh` → green.
- [ ] **Step 5: Commit** — `feat(macos): a regra da medição de 5 em 5 horas, o plano por grupo e apagar o histórico`

---

### Task 4: The chart series (pure, in the engine)

**Files:**
- Create: `Sources/CCUsageCore/Engine/UsageHistoryChart.swift`
- Modify: `Sources/CCUsageCore/StatusLine/StatusLineFormat.swift` (public `weekdayName`)
- Test: `Tests/CCUsageCoreTests/UsageHistoryChartTests.swift`

**Interfaces:** `public enum UsageHistoryChart { enum Window { fiveHour, sevenDay }; struct Point: Identifiable { at, fraction, resetsAt }; struct Series { from, to, ticks: [Date], segments: [[Point]], latest, points }; static let fiveHourDays = 7, sevenDayDays = 28, maxGap; static func series(_:window:now:calendar:) -> Series; static func ticks(window:from:now:calendar:) -> [Date] }`; `StatusLineFormat.weekdayName(_ date: Date, portuguese: Bool, calendar: Calendar) -> String`.

- [ ] **Step 1: Failing tests** — `Tests/CCUsageCoreTests/UsageHistoryChartTests.swift`:

```swift
import Foundation
import Testing
@testable import CCUsageCore

/// As séries dos gráficos do Histórico: recorte, buracos, marcas — as mesmas
/// regras do porte Windows (`history.rs`).
@Suite("UsageHistoryChart")
struct UsageHistoryChartTests {
    /// Quarta, 21/10/2026, meio-dia em São Paulo (UTC−3, sem horário de verão).
    private let now: Date = {
        var c = DateComponents(); c.year = 2026; c.month = 10; c.day = 21; c.hour = 15
        var cal = Calendar(identifier: .gregorian); cal.timeZone = TimeZone(identifier: "UTC")!
        return cal.date(from: c)!
    }()
    private let saoPaulo: Calendar = {
        var cal = Calendar(identifier: .gregorian)
        cal.timeZone = TimeZone(secondsFromGMT: -3 * 3600)!
        return cal
    }()

    private func point(hoursAgo: Double, five: Double?, seven: Double?) -> UsageHistoryPoint {
        UsageHistoryPoint(sampledAt: now.addingTimeInterval(-hoursAgo * 3600),
                          fiveHourPercent: five, fiveHourResetsAt: nil,
                          sevenDayPercent: seven, sevenDayResetsAt: nil)
    }

    @Test("cada gráfico guarda o seu período")
    func periodos() {
        let points = [point(hoursAgo: 30 * 24, five: 0.1, seven: 0.9),
                      point(hoursAgo: 10 * 24, five: 0.2, seven: 0.5),
                      point(hoursAgo: 2 * 24, five: 0.3, seven: 0.6)]
        let five = UsageHistoryChart.series(points, window: .fiveHour, now: now, calendar: saoPaulo)
        let seven = UsageHistoryChart.series(points, window: .sevenDay, now: now, calendar: saoPaulo)
        #expect(five.points.map(\.fraction) == [0.3])
        #expect(seven.points.map(\.fraction) == [0.5, 0.6])
        #expect(five.from == now.addingTimeInterval(-7 * 86_400))
        #expect(seven.from == now.addingTimeInterval(-28 * 86_400))
    }

    /// Mais de 7h30 sem ponto quebra a linha; 7h30 exatas ainda ligam.
    @Test("buraco de mais de 7h30 quebra a linha")
    func buraco() {
        let start = 24.0
        let points = [point(hoursAgo: start, five: 0.1, seven: nil),
                      point(hoursAgo: start - 5, five: 0.2, seven: nil),
                      point(hoursAgo: start - 12.5, five: 0.3, seven: nil),
                      point(hoursAgo: start - 21, five: 0.4, seven: nil)]
        let five = UsageHistoryChart.series(points, window: .fiveHour, now: now, calendar: saoPaulo)
        #expect(five.segments.map { $0.map(\.fraction) } == [[0.1, 0.2, 0.3], [0.4]])
        #expect(five.latest?.fraction == 0.4)
    }

    /// Ausência não é zero.
    @Test("ponto sem a janela fica fora do gráfico")
    func semJanela() {
        let points = [point(hoursAgo: 1, five: nil, seven: 0.4)]
        #expect(UsageHistoryChart.series(points, window: .fiveHour, now: now, calendar: saoPaulo).segments.isEmpty)
        #expect(UsageHistoryChart.series(points, window: .sevenDay, now: now, calendar: saoPaulo).latest?.fraction == 0.4)
    }

    /// As marcas: a meia-noite LOCAL de cada dia, e de 7 em 7 dias a partir de
    /// hoje; os nomes dos dias são os da status line.
    @Test("as marcas do eixo do tempo")
    func marcas() {
        let five = UsageHistoryChart.series([], window: .fiveHour, now: now, calendar: saoPaulo)
        #expect(five.ticks.count == 7)
        #expect(five.ticks.map { StatusLineFormat.weekdayName($0, portuguese: true, calendar: saoPaulo) }
                == ["qui", "sex", "sáb", "dom", "seg", "ter", "qua"])
        #expect(five.ticks.map { StatusLineFormat.weekdayName($0, portuguese: false, calendar: saoPaulo) }
                == ["Thu", "Fri", "Sat", "Sun", "Mon", "Tue", "Wed"])
        let seven = UsageHistoryChart.series([], window: .sevenDay, now: now, calendar: saoPaulo)
        #expect(seven.ticks.count == 4)
        // 21/10 00:00 em UTC−3 = 03:00 UTC.
        #expect(seven.ticks.last == now.addingTimeInterval(-12 * 3600))
    }
}
```

- [ ] **Step 2: Run** — fails to compile.

- [ ] **Step 3: Implement**

In `StatusLineFormat.swift`, extract the weekday names (and use them in `resetWhen`):

```swift
    /// O dia da semana abreviado como a linha o escreve ("seg" / "Mon").
    /// Público: as marcas de dia do gráfico do Histórico usam os mesmos nomes.
    public static func weekdayName(_ date: Date, portuguese: Bool,
                                   calendar: Calendar = .current) -> String {
        let pt = ["dom", "seg", "ter", "qua", "qui", "sex", "sáb"]
        let en = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"]
        let weekday = calendar.component(.weekday, from: date) - 1  // 1 = domingo
        return (portuguese ? pt : en)[weekday % 7]
    }
```

and `resetWhen` keeps its output byte for byte, now reading the names from there:

```swift
    public static func resetWhen(_ date: Date, withDay: Bool,
                                 portuguese: Bool = Locale.current.language.languageCode?.identifier == "pt",
                                 calendar: Calendar = .current) -> String {
        let parts = calendar.dateComponents([.day, .hour, .minute], from: date)
        let hour = parts.hour ?? 0, minute = parts.minute ?? 0
        guard withDay else { return String(format: "%02d:%02d", hour, minute) }

        // Hora sem zero à esquerda no formato semanal, de propósito — é o que o
        // porte Windows imprime, e as duas plataformas escrevem igual.
        let day = weekdayName(date, portuguese: portuguese, calendar: calendar)
        return "\(day) (\(parts.day ?? 0)) \(hour):\(String(format: "%02d", minute))"
    }
```

Create `Sources/CCUsageCore/Engine/UsageHistoryChart.swift`:

```swift
import Foundation

/// O que os gráficos do Histórico desenham (21/10/2026), puro e testável: as
/// séries recortadas no período, quebradas nos buracos, e as marcas do eixo.
/// A view só converte tempo em x e fração em y. Mesmas regras do porte
/// Windows (`history.rs`).
public enum UsageHistoryChart {
    public enum Window: Sendable { case fiveHour, sevenDay }

    public struct Point: Sendable, Equatable, Identifiable {
        public let at: Date
        public let fraction: Double
        public let resetsAt: Date?
        public var id: Date { at }
    }

    public struct Series: Sendable, Equatable {
        public let from: Date
        public let to: Date
        /// As meias-noites locais onde o eixo do tempo tem marca.
        public let ticks: [Date]
        /// Os trechos da linha, do mais velho ao mais novo.
        public let segments: [[Point]]
        public var points: [Point] { segments.flatMap { $0 } }
        /// O ponto mais novo do período — o número no título do gráfico.
        public var latest: Point? { segments.last?.last }
    }

    /// Quantos dias cada gráfico mostra, contados de agora para trás.
    public static let fiveHourDays = 7
    public static let sevenDayDays = 28
    /// Um buraco maior que isto entre dois pontos quebra a linha: 1,5 × o
    /// intervalo da agenda (7h30). App fechado, Mac dormindo — o gráfico não
    /// desenha o que não foi medido.
    public static let maxGap: TimeInterval = MeasureSchedule.every * 1.5

    public static func series(_ points: [UsageHistoryPoint], window: Window, now: Date,
                              calendar: Calendar = .current) -> Series {
        let days = window == .fiveHour ? fiveHourDays : sevenDayDays
        let from = now.addingTimeInterval(-TimeInterval(days) * 86_400)
        var segments: [[Point]] = []
        var previous: Date?
        for p in points where p.sampledAt >= from && p.sampledAt <= now {
            let fraction = window == .fiveHour ? p.fiveHourPercent : p.sevenDayPercent
            guard let fraction else { continue }
            let resetsAt = window == .fiveHour ? p.fiveHourResetsAt : p.sevenDayResetsAt
            let drawn = Point(at: p.sampledAt, fraction: fraction, resetsAt: resetsAt)
            if let previous, p.sampledAt.timeIntervalSince(previous) <= maxGap, !segments.isEmpty {
                segments[segments.count - 1].append(drawn)
            } else {
                segments.append([drawn])
            }
            previous = p.sampledAt
        }
        return Series(from: from, to: now,
                      ticks: ticks(window: window, from: from, now: now, calendar: calendar),
                      segments: segments)
    }

    /// A meia-noite de cada dia (5h) ou de 7 em 7 dias contando de hoje
    /// (semanal) — só as que caem dentro do período.
    public static func ticks(window: Window, from: Date, now: Date,
                             calendar: Calendar = .current) -> [Date] {
        let step = window == .fiveHour ? 1 : 7
        let days = window == .fiveHour ? fiveHourDays : sevenDayDays
        let today = calendar.startOfDay(for: now)
        let marks = stride(from: 0, through: days, by: step).compactMap {
            calendar.date(byAdding: .day, value: -$0, to: today)
        }
        return Array(marks.filter { $0 > from && $0 <= now }.reversed())
    }
}
```

- [ ] **Step 4: Run** — `./Scripts/test.sh` → green (StatusLine tests still pass: `resetWhen` output unchanged).
- [ ] **Step 5: Commit** — `feat(macos): as séries dos gráficos do Histórico`

---

### Task 5: The setting "Measure accounts every 5 hours"

**Files:** Modify `Sources/CCUsageCore/Settings/AppSettings.swift`; Test `Tests/CCUsageCoreTests/AppSettingsTests.swift`.

- [ ] **Step 1: Failing test** — append:

```swift
/// A medição de 5 em 5 horas vem ligada de fábrica — e um payload de antes
/// dela (sem o campo) também a liga. Desligada, fica desligada.
@MainActor
@Test func scheduledMeasureIsOnUnlessTurnedOff() {
    let defaults = freshDefaults()
    #expect(AppSettings(defaults: defaults, detectPlan: { nil }).scheduledMeasure)

    let old = freshDefaults()
    old.set(Data(#"{"plan":"max5"}"#.utf8), forKey: AppSettings.storageKey)
    #expect(AppSettings(defaults: old, detectPlan: { nil }).scheduledMeasure)

    let settings = AppSettings(defaults: defaults, detectPlan: { nil })
    settings.scheduledMeasure = false
    #expect(!AppSettings(defaults: defaults, detectPlan: { nil }).scheduledMeasure)
}
```

- [ ] **Step 2: Run** — fails (no member).
- [ ] **Step 3: Implement** — property (after `showInDock`):

```swift
    /// "Medir as contas a cada 5 horas" (21/10/2026, para a aba Histórico):
    /// ligada de fábrica — e um payload de antes dela, sem o campo, também.
    /// Cada medição é o `claude /usage` de cada conta, a mesma consulta do
    /// "Medir contas".
    public var scheduledMeasure: Bool {
        didSet { save() }
    }
```

`init`, right after `self.showInDock = …`:

```swift
        self.scheduledMeasure = stored?.scheduledMeasure ?? true
```

`Payload`, after `showInDock`:

```swift
        /// Ausente em payload de antes da medição automática: resolve para
        /// ligado no init.
        var scheduledMeasure: Bool?
```

`save()`:

```swift
        let payload = Payload(plan: plan, manualBlockCeiling: manualBlockCeiling,
                              liveUsageEnabled: liveUsageEnabled,
                              showInDock: showInDock,
                              scheduledMeasure: scheduledMeasure,
                              alertsEnabled: nil, alerts: alerts)
```

- [ ] **Step 4: Run** — green. **Step 5: Commit** — `feat(macos): ajuste para medir as contas a cada 5 horas`

---

### Task 6: The app runs the schedule

**Files:** Modify `Sources/FalcaoTokenRouter/App.swift`, `Sources/FalcaoTokenRouter/Settings/LoginSheet.swift`.

- [ ] **Step 1: The loop** — replace `startRotation()`:

```swift
    /// O laço da rotação automática, no próprio app enquanto ele vive: a cada
    /// três minutos relê o uso do sensor e deixa cada grupo trocar se passou do
    /// limiar. Três minutos porque o `rate_limits` só muda quando há atividade, e
    /// o endpoint de uso tem orçamento próprio — reconsultar mais rápido não traz
    /// número novo, só trabalho.
    ///
    /// A partir da 2ª volta (3 min depois da subida), olha também a agenda da
    /// medição de 5 em 5 horas (21/10/2026) — na abertura do Mac ela não disputa
    /// a máquina com tudo o mais que sobe junto. Numa `Task` à parte: medir leva
    /// segundos por conta, e a rotação não espera por ela.
    @MainActor private static func startRotation() {
        Task {
            var volta = 0
            while !Task.isCancelled {
                router.refreshUsage()
                router.rotateAll()
                if volta > 0 && settings.scheduledMeasure {
                    Task { await router.runScheduledMeasure() }
                }
                volta += 1
                try? await Task.sleep(for: .seconds(180))
            }
        }
    }
```

- [ ] **Step 2: The sign-in guard** — on the login sheet itself, whose lifetime *is* the login: it also disappears when the window closes with a login open, which an `.onChange` on `GroupsView.pendingLogin` would miss (the view goes away without ever seeing `nil`, and the schedule would stay blocked until the app quits). In `LoginSheet`, the existing `.onAppear` becomes:

```swift
        .onAppear {
            // Com um login aberto a agenda de 5 em 5 horas espera: a sonda
            // poderia mexer na casa que está sendo relogada (21/10/2026).
            store.signingIn = true
            let s = LoginSession(home: pending.home, accountID: pending.accountID,
                                 groupID: pending.groupID)
            session = s
            s.start()
        }
        .onDisappear { store.signingIn = false }
```

- [ ] **Step 3: Build** — `./Scripts/test.sh` (it builds every target) → green.
- [ ] **Step 4: Commit** — `feat(macos): o app mede as contas a cada 5 horas`

---

### Task 7: The History tab, the charts and the shortcut

**Files:**
- Create: `Sources/FalcaoTokenRouter/Settings/HistoryView.swift`, `Settings/HistoryCard.swift`, `Settings/HistoryChart.swift`
- Modify: `Settings/HomeWindow.swift`, `Settings/GroupsView.swift`, `Settings/GroupCard.swift`, `Settings/AccountRow.swift`, `Panel/UsagePanel.swift`, `Panel/UsageColor.swift`

- [ ] **Step 1: Navigation** — in `HomeWindow.swift`:

```swift
enum HomeTab: Hashable {
    case groups
    case history
    case meter
}

/// O atalho de uma conta: qual mostrar no Histórico — no grupo do clique, se
/// veio de um. `seq` muda a cada clique, para o mesmo atalho rolar de novo.
struct HistoryFocus: Equatable {
    let accountID: UUID
    let groupID: UUID?
    let seq: Int
}

@MainActor
@Observable
final class HomeNavigation {
    var tab: HomeTab = .groups
    var historyFocus: HistoryFocus?

    /// Abre o Histórico já numa conta (o atalho da linha da conta e do cartão
    /// da conta no painel).
    func showHistory(of accountID: UUID, in groupID: UUID? = nil) {
        historyFocus = HistoryFocus(accountID: accountID, groupID: groupID,
                                    seq: (historyFocus?.seq ?? 0) + 1)
        tab = .history
    }
}
```

and in the `TabView`, between Groups and the meter:

```swift
            GroupsView(store: router, navigation: navigation)
                .tabItem { Label("home.tab.groups", systemImage: "rectangle.stack") }
                .tag(HomeTab.groups)

            HistoryView(store: router, settings: settings, navigation: navigation)
                .tabItem { Label("home.tab.history", systemImage: "chart.xyaxis.line") }
                .tag(HomeTab.history)
```

- [ ] **Step 2: The line color** — in `UsageColor.swift` (add `import AppKit`):

```swift
    /// A linha dos gráficos do Histórico (21/10/2026): o verde do app com croma
    /// a mais, para não se ler cinza — a mesma do porte Windows, validada
    /// contra as superfícies clara e escura (contraste ≥ 3:1).
    static let chartLine = Color(nsColor: NSColor(name: nil) { appearance in
        appearance.bestMatch(from: [.darkAqua, .vibrantDark]) != nil
            ? NSColor(srgbRed: 0x52 / 255, green: 0xA7 / 255, blue: 0x6D / 255, alpha: 1)
            : NSColor(srgbRed: 0x1E / 255, green: 0x75 / 255, blue: 0x46 / 255, alpha: 1)
    })
```

- [ ] **Step 3: `Settings/HistoryChart.swift`**

```swift
import Charts
import SwiftUI
import CCUsageCore

/// Um gráfico de linha do Histórico (21/10/2026): uma série — a fração de uma
/// janela ao longo do tempo —, eixo vertical FIXO de 0 a 100% (as contas se
/// comparam de relance) e o limiar do grupo tracejado (é com ele que a rotação
/// compara). Swift Charts: vem com o sistema, nenhuma dependência nova.
///
/// Linha de 2 pt, bolinha só no último ponto, no ponto isolado (trecho de um
/// ponto só) e no que o ponteiro escolher; cada trecho é uma série própria,
/// então o buraco quebra a linha em vez de ligar o que não foi medido.
struct HistoryChart: View {
    let title: String
    let series: UsageHistoryChart.Series
    /// O limiar do grupo, em porcentagem (90 = 90%).
    let threshold: Double
    /// A semanal escreve o reset com o dia.
    let weekly: Bool

    @ViewState private var selected: Date?

    private var portuguese: Bool { Locale.current.language.languageCode?.identifier == "pt" }

    /// O ponto mais perto no tempo de onde o ponteiro está.
    private var shown: UsageHistoryChart.Point? {
        guard let selected else { return nil }
        return series.points.min {
            abs($0.at.timeIntervalSince(selected)) < abs($1.at.timeIntervalSince(selected))
        }
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 4) {
            HStack {
                Text(verbatim: title).font(.caption).foregroundStyle(.secondary)
                Spacer()
                Text(verbatim: series.latest.map { UsagePercent.text($0.fraction) } ?? "—")
                    .font(.caption.weight(.semibold).monospacedDigit())
            }
            Chart {
                RuleMark(y: .value("limiar", threshold / 100))
                    .foregroundStyle(UsageColor.critical.opacity(0.75))
                    .lineStyle(StrokeStyle(lineWidth: 1, dash: [4, 3]))
                    .annotation(position: .top, alignment: .trailing) {
                        Text(verbatim: String(format: String(localized: "home.history.chart.threshold.format"),
                                              "\(Int(threshold))%"))
                            .font(.system(size: 9)).foregroundStyle(.secondary)
                    }
                ForEach(Array(series.segments.enumerated()), id: \.offset) { index, segment in
                    ForEach(segment) { point in
                        LineMark(x: .value("quando", point.at), y: .value("uso", point.fraction),
                                 series: .value("trecho", index))
                            .foregroundStyle(UsageColor.chartLine)
                            .lineStyle(StrokeStyle(lineWidth: 2, lineCap: .round, lineJoin: .round))
                        if segment.count == 1 {
                            PointMark(x: .value("quando", point.at), y: .value("uso", point.fraction))
                                .foregroundStyle(UsageColor.chartLine).symbolSize(40)
                        }
                    }
                }
                if let latest = series.latest {
                    PointMark(x: .value("quando", latest.at), y: .value("uso", latest.fraction))
                        .foregroundStyle(UsageColor.chartLine).symbolSize(40)
                }
                if let shown {
                    RuleMark(x: .value("quando", shown.at)).foregroundStyle(.tertiary)
                    PointMark(x: .value("quando", shown.at), y: .value("uso", shown.fraction))
                        .foregroundStyle(UsageColor.chartLine).symbolSize(70)
                        .annotation(position: .top,
                                    overflowResolution: .init(x: .fit(to: .chart), y: .disabled)) {
                            tip(shown)
                        }
                }
            }
            .chartYScale(domain: 0...1)
            .chartXScale(domain: series.from...series.to)
            .chartYAxis {
                AxisMarks(position: .leading, values: [0.0, 0.5, 1.0]) { value in
                    AxisGridLine()
                    AxisValueLabel {
                        if let v = value.as(Double.self) { Text(verbatim: UsagePercent.text(v)) }
                    }
                }
            }
            .chartXAxis {
                AxisMarks(values: series.ticks) { value in
                    AxisGridLine()
                    AxisValueLabel(anchor: .topLeading) {
                        if let date = value.as(Date.self) { Text(verbatim: tickLabel(date)) }
                    }
                }
            }
            .chartXSelection(value: $selected)
            .frame(height: 110)
            .overlay {
                if series.points.isEmpty {
                    Text("home.history.chart.empty").font(.caption).foregroundStyle(.tertiary)
                }
            }
            .accessibilityLabel(Text(verbatim: String(format: String(localized: "home.history.chart.label.format"),
                                                      title, series.latest.map { UsagePercent.text($0.fraction) } ?? "—")))
        }
    }

    /// A dica do ponto: o valor primeiro, depois quando foi medido e quando
    /// aquela janela resetava — as horas escritas como a status line escreve.
    private func tip(_ point: UsageHistoryChart.Point) -> some View {
        VStack(alignment: .leading, spacing: 1) {
            Text(verbatim: UsagePercent.text(point.fraction)).font(.callout.weight(.semibold))
            Text(verbatim: StatusLineFormat.resetWhen(point.at, withDay: true)).font(.caption2)
            if let reset = point.resetsAt {
                Text(verbatim: String(format: String(localized: "home.history.point.resets.format"),
                                      StatusLineFormat.resetWhen(reset, withDay: weekly)))
                    .font(.caption2)
            }
        }
        .foregroundStyle(.secondary)
        .padding(6)
        .background(.regularMaterial, in: .rect(cornerRadius: 6))
    }

    /// 5h: o dia da semana, com os nomes da status line. Semanal: a data.
    private func tickLabel(_ date: Date) -> String {
        guard weekly else { return StatusLineFormat.weekdayName(date, portuguese: portuguese) }
        let day = Calendar.current.component(.day, from: date)
        let month = Calendar.current.component(.month, from: date)
        return portuguese
            ? String(format: "%02d/%02d", day, month)
            : date.formatted(.dateTime.month(.abbreviated).day())
    }
}
```

- [ ] **Step 4: `Settings/HistoryCard.swift`**

```swift
import SwiftUI
import CCUsageCore

/// Uma conta no Histórico: quem é, quando foi medida (e quando volta a ser,
/// com a agenda ligada) e os dois gráficos, com o limiar do grupo.
struct HistoryCard: View {
    let account: Account
    let points: [UsageHistoryPoint]
    let threshold: Double
    let scheduleOn: Bool
    let attempt: Date?
    let highlighted: Bool

    var body: some View {
        let now = Date()
        VStack(alignment: .leading, spacing: 12) {
            HStack(alignment: .firstTextBaseline, spacing: 8) {
                Text(verbatim: account.label).font(.callout.weight(.semibold))
                Text(verbatim: account.identity.email)
                    .font(.caption).foregroundStyle(.secondary).lineLimit(1)
            }
            Text(verbatim: status(now: now)).font(.caption).foregroundStyle(.tertiary)
            HistoryChart(title: String(localized: "home.history.chart.fiveHour"),
                         series: UsageHistoryChart.series(points, window: .fiveHour, now: now),
                         threshold: threshold, weekly: false)
            HistoryChart(title: String(localized: "home.history.chart.sevenDay"),
                         series: UsageHistoryChart.series(points, window: .sevenDay, now: now),
                         threshold: threshold, weekly: true)
        }
        .padding(14)
        .background(.quaternary.opacity(0.4), in: .rect(cornerRadius: 12))
        // Quem chegou pelo atalho: um contorno que some sozinho.
        .overlay {
            RoundedRectangle(cornerRadius: 12)
                .strokeBorder(Color.accentColor, lineWidth: 2)
                .opacity(highlighted ? 1 : 0)
                .animation(.easeOut(duration: 1.8), value: highlighted)
        }
    }

    /// "medida há 2h · próxima às 19:05". Vencida, ou a menos de uma volta da
    /// agenda (3 min), "em instantes".
    private func status(now: Date) -> String {
        let last = points.last?.sampledAt
        let measured = last.map {
            String(format: String(localized: "home.history.account.measured.format"),
                   Format.duration(now.timeIntervalSince($0)))
        } ?? String(localized: "home.history.account.never")
        let next: String
        if !scheduleOn {
            next = String(localized: "home.history.account.scheduleOff")
        } else if let at = MeasureSchedule.nextAt(lastMeasured: last, lastAttempt: attempt),
                  at > now.addingTimeInterval(180) {
            next = String(format: String(localized: "home.history.account.next.format"),
                          StatusLineFormat.resetWhen(at, withDay: false))
        } else {
            next = String(localized: "home.history.account.nextSoon")
        }
        return "\(measured) · \(next)"
    }
}
```

- [ ] **Step 5: `Settings/HistoryView.swift`**

```swift
import SwiftUI
import CCUsageCore

/// A aba Histórico (21/10/2026): para cada conta, a janela de 5h nos últimos 7
/// dias e a semanal nas últimas 4 semanas — cada ponto, uma medição da sonda
/// (a de 5 em 5 horas, o Medir contas, o `router measure`). Grupos na ordem da
/// lista; a conta em dois grupos aparece nos dois. O atalho de cada conta
/// (`navigation.historyFocus`) rola até o cartão dela e o destaca.
struct HistoryView: View {
    @Bindable var store: RouterConfigStore
    @Bindable var settings: AppSettings
    @Bindable var navigation: HomeNavigation

    @ViewState private var highlighted: String?

    static func anchor(_ group: UUID, _ account: UUID) -> String {
        "\(group.uuidString):\(account.uuidString)"
    }

    var body: some View {
        if store.config.groups.isEmpty {
            VStack(spacing: 12) {
                Image(systemName: "chart.xyaxis.line").font(.system(size: 34)).foregroundStyle(.tertiary)
                Text("home.history.empty.title").font(.callout)
                Text("home.history.empty.detail").font(.caption)
                    .foregroundStyle(.secondary).multilineTextAlignment(.center)
            }
            .frame(maxWidth: .infinity, maxHeight: .infinity)
            .padding(32)
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

    private func groupSection(_ group: AccountGroup) -> some View {
        let accounts = store.config.accounts(in: group)
        return VStack(alignment: .leading, spacing: 10) {
            Text(verbatim: group.name).font(.headline)
            if accounts.isEmpty {
                Text("home.history.group.empty").font(.caption).foregroundStyle(.tertiary)
            }
            ForEach(accounts) { account in
                let key = Self.anchor(group.id, account.id)
                HistoryCard(account: account,
                            points: store.historyByAccount[account.id] ?? [],
                            threshold: group.thresholdPercent,
                            scheduleOn: settings.scheduledMeasure,
                            attempt: store.measureAttempts[account.id],
                            highlighted: highlighted == key)
                    .id(key)
            }
        }
    }

    /// O atalho: rola até o cartão da conta — no grupo do clique; senão, o
    /// primeiro em que ela aparece — e o destaca por 1,8 s. Atendido, o pedido
    /// some.
    private func focus(with proxy: ScrollViewProxy) {
        guard let wanted = navigation.historyFocus else { return }
        let group = wanted.groupID ?? store.config.groups.first { group in
            store.config.accounts(in: group).contains { $0.id == wanted.accountID }
        }?.id
        navigation.historyFocus = nil
        guard let group else { return }
        let key = Self.anchor(group, wanted.accountID)
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
```

- [ ] **Step 6: The shortcut in the account row** — `AccountRow`: add `let onHistory: () -> Void` (after `onSwitch`) and, before the `Menu`:

```swift
            // O atalho para o Histórico DESTA conta (21/10/2026): sempre visível,
            // ao lado do ⋯.
            Button(action: onHistory) { Image(systemName: "chart.xyaxis.line") }
                .buttonStyle(.borderless)
                .help(String(localized: "groups.account.history"))
```

`GroupCard`: a new stored property after `onRelogin` (the memberwise init takes the parameters in declaration order):

```swift
    let onRelogin: (Account) -> Void
    /// O atalho de cada conta para o Histórico, já neste grupo.
    let onHistory: (Account) -> Void
```

and the row in `accountList`:

```swift
                AccountRow(account: account,
                           isActive: account.id == active?.id,
                           usage: store.usageDetail[account.id],
                           sampledAt: store.usageSampledAt[account.id],
                           onSwitch: { store.activate(account, in: group) },
                           onHistory: { onHistory(account) },
                           onRelogin: { onRelogin(account) },
                           onRemove: { pendingDelete = account })
```

`GroupsView`: `let navigation: HomeNavigation` after `store`, and the card:

```swift
                            GroupCard(store: store, group: group,
                                      onAddAccount: { startAddAccount(to: group.id) },
                                      onRelogin: { startRelogin($0, in: group.id) },
                                      onHistory: { navigation.showHistory(of: $0.id, in: group.id) })
```

(`HomeWindow` already passes `navigation` — Step 1.)

- [ ] **Step 7: The shortcut in the panel** — in `UsagePanel.accountSection`, after the `if let usage { … } else { … }`:

```swift
            // O atalho para o Histórico desta conta (21/10/2026): abre a janela
            // na aba, já no cartão dela.
            Button {
                navigation.showHistory(of: account.id)
                openWindow(id: HomeWindowID.value)
                NSApp.activate()
            } label: {
                Label("panel.account.history", systemImage: "chart.xyaxis.line").font(.caption2)
            }
            .buttonStyle(.link)
```

- [ ] **Step 8: Build** — `./Scripts/test.sh` → compiles (strings come in Task 8; until then `check-strings.sh` fails — do Task 8 before running if you prefer).
- [ ] **Step 9: Commit** (with Task 8) — `feat(macos): a aba Histórico, os gráficos e o atalho de cada conta`

---

### Task 8: Settings section and strings

**Files:** Modify `Sources/FalcaoTokenRouter/Settings/SettingsView.swift`, `Resources/en.lproj/Localizable.strings`, `Resources/pt-BR.lproj/Localizable.strings`.

- [ ] **Step 1: Settings** — in `SettingsView` add state and a section before `Section("settings.section.system")`:

```swift
    @ViewState private var confirmingClear = false
    /// "Histórico apagado ✓" por 2 s, só depois de o motor confirmar.
    @ViewState private var cleared = false
    @ViewState private var clearFailure: String?
```

```swift
            Section("settings.section.history") {
                Toggle("settings.history.measure", isOn: $settings.scheduledMeasure)
                Text("settings.history.measure.detail")
                    .font(.caption).foregroundStyle(.secondary)
                HStack {
                    Button("settings.history.clear") { confirmingClear = true }
                    if cleared {
                        Text("settings.history.cleared").font(.caption).foregroundStyle(UsageColor.calm)
                    }
                }
                if let clearFailure {
                    Text(verbatim: String(format: String(localized: "settings.history.clear.failed.format"),
                                          clearFailure))
                        .font(.caption).foregroundStyle(UsageColor.critical)
                }
                Text("settings.history.clear.detail").font(.caption).foregroundStyle(.secondary)
            }
            .confirmationDialog("settings.history.clear.title", isPresented: $confirmingClear) {
                Button("settings.history.clear.confirm", role: .destructive) { clearHistory() }
                Button("groups.cancel", role: .cancel) {}
            } message: {
                Text("settings.history.clear.message")
            }
```

```swift
    private func clearHistory() {
        clearFailure = nil
        do {
            try router.clearHistory()
            cleared = true
            Task { try? await Task.sleep(for: .seconds(2)); cleared = false }
        } catch {
            clearFailure = error.localizedDescription
        }
    }
```

- [ ] **Step 2: Strings** — `Scripts/check-strings.sh` stays as it is: every new key is under a prefix it already knows (`home`, `groups`, `panel`, `settings`). A new `history` prefix would break it — `Engine/ProfileSharing.swift` has the literal `"history.jsonl"`, which the checker would read as a key missing from both catalogs. Add to `en.lproj/Localizable.strings`:

```
/* History tab (21/10/2026) */
"home.tab.history" = "History";
"home.history.intro" = "Each point is one measurement of Claude Code's `/usage`: every 5 hours while the app is open, plus each Measure accounts and `router measure`.";
"home.history.empty.title" = "No groups yet";
"home.history.empty.detail" = "Create a group on the Groups tab. Its accounts show up here, with a point for each measurement.";
"home.history.group.empty" = "No accounts in this group yet.";
"home.history.account.measured.format" = "measured %@ ago";
"home.history.account.never" = "no measurements yet";
"home.history.account.next.format" = "next at %@";
"home.history.account.nextSoon" = "next in a moment";
"home.history.account.scheduleOff" = "automatic measuring is off";
"home.history.chart.fiveHour" = "5 hours · last 7 days";
"home.history.chart.sevenDay" = "Weekly · last 4 weeks";
"home.history.chart.threshold.format" = "switch at %@";
"home.history.chart.empty" = "No points in this period.";
"home.history.chart.label.format" = "%1$@ — latest %2$@";
"home.history.point.resets.format" = "resets %@";
"groups.account.history" = "This account's usage history";
"panel.account.history" = "View history";
"settings.section.history" = "History";
"settings.history.measure" = "Measure accounts every 5 hours";
"settings.history.measure.detail" = "Runs Claude Code's `/usage` for each account — the same query as Measure accounts — while the app is open, and keeps each result for the History tab.";
"settings.history.clear" = "Delete history…";
"settings.history.clear.detail" = "Deletes the points of every account on this Mac. Measuring goes on, and the history starts over with the next measurement.";
"settings.history.clear.title" = "Delete the history?";
"settings.history.clear.message" = "Every point of every account is deleted from this Mac, and the History tab starts empty. This can't be undone. Groups, accounts and logins stay as they are.";
"settings.history.clear.confirm" = "Delete";
"settings.history.cleared" = "History deleted ✓";
"settings.history.clear.failed.format" = "Couldn't delete the history: %@";
```

and to `pt-BR.lproj/Localizable.strings`:

```
/* Aba Histórico (21/10/2026) */
"home.tab.history" = "Histórico";
"home.history.intro" = "Cada ponto é uma medição do `/usage` do Claude Code: a cada 5 horas com o app aberto, mais cada Medir contas e cada `router measure`.";
"home.history.empty.title" = "Nenhum grupo ainda";
"home.history.empty.detail" = "Crie um grupo na aba Grupos. As contas dele aparecem aqui, com um ponto por medição.";
"home.history.group.empty" = "Nenhuma conta neste grupo ainda.";
"home.history.account.measured.format" = "medida há %@";
"home.history.account.never" = "sem medições ainda";
"home.history.account.next.format" = "próxima às %@";
"home.history.account.nextSoon" = "próxima em instantes";
"home.history.account.scheduleOff" = "medição automática desligada";
"home.history.chart.fiveHour" = "5 horas · últimos 7 dias";
"home.history.chart.sevenDay" = "Semanal · últimas 4 semanas";
"home.history.chart.threshold.format" = "troca em %@";
"home.history.chart.empty" = "Sem pontos neste período.";
"home.history.chart.label.format" = "%1$@ — último %2$@";
"home.history.point.resets.format" = "reseta %@";
"groups.account.history" = "Histórico de uso desta conta";
"panel.account.history" = "Ver histórico";
"settings.section.history" = "Histórico";
"settings.history.measure" = "Medir as contas a cada 5 horas";
"settings.history.measure.detail" = "Roda o `/usage` do Claude Code em cada conta — a mesma consulta do Medir contas — enquanto o app está aberto, e guarda cada resultado para a aba Histórico.";
"settings.history.clear" = "Apagar o histórico…";
"settings.history.clear.detail" = "Apaga os pontos de todas as contas deste Mac. As medições continuam, e o histórico recomeça na próxima.";
"settings.history.clear.title" = "Apagar o histórico?";
"settings.history.clear.message" = "Todos os pontos de todas as contas saem deste Mac, e a aba Histórico fica vazia. Não dá para desfazer. Grupos, contas e logins ficam como estão.";
"settings.history.clear.confirm" = "Apagar";
"settings.history.cleared" = "Histórico apagado ✓";
"settings.history.clear.failed.format" = "Não foi possível apagar o histórico: %@";
```

- [ ] **Step 3: Run** — `./Scripts/test.sh` (runs `check-strings.sh` first) → green.
- [ ] **Step 4: Look at it** — `./Scripts/bundle.sh --native --install`, open the app:
  - History tab: two charts per account (both appearances), hover a point (value · when · reset), the dashed threshold with its label, the "measured … · next …" line;
  - Groups: the chart button next to each account opens History scrolled to it, highlighted; the panel's account card → **View history** does the same from the menu bar;
  - **Meter** tab (or ⌘,) → **History** section: the switch, **Delete history…** → confirmation → "History deleted ✓";
  - ~3 minutes after launching, `ls ~/Library/Application\ Support/com.synqo.falcao-router/history/` shows one `.jsonl` per account (the first scheduled measurement), and a second pass 3 minutes later measures nothing new.
- [ ] **Step 5: Commit** — `feat(macos): a aba Histórico, os gráficos e o atalho de cada conta` (Tasks 7–8 together).

---

### Task 9: Documentation (after PR #35 is merged into this branch)

- [ ] `macos/README.md`: the History tab, the shortcut, the switch and **Delete history…**; *Where things live* gains `history/` (one `<e-mail>.jsonl` per account, 90 days — the same file the Windows app writes).
- [ ] `macos/CHANGELOG.md`, `[Unreleased]` → `### Added`: **Usage history** (same wording as `windows/CHANGELOG.md`, macOS terms).
- [ ] Root `README.md` and `docs/ARCHITECTURE.md`: the probe now runs every 5 hours on **both** platforms (drop "on macOS it is a button and a command, never a loop"), and `history/<email>.jsonl` is part of the shared format.
- [ ] `docs/PORTING.md`, *What to keep*: `UsageHistory` (the file), `MeasureSchedule` (the rule) and `UsageHistoryChart` (the series).
- [ ] `windows/docs/PLATFORM.md` and `windows/README.md`: `history\` stops being "Windows only"; "the macOS app neither reads nor writes the folder" goes.
- [ ] `agent.md` of every folder touched (`Engine/`, `Usage/`, `StatusLine/`, `Settings/` of the core, `router/`, the app, `Settings/`, `Panel/`, `Tests/`), with the decisions dated 21/10/2026 — the history only from the probe, the same format as Windows, `ProbeRecord` as the one writer, the attempts in the store, the quiet failures, Swift Charts.
- [ ] Commit — `docs(macos): o histórico de uso e a medição de 5 em 5 horas`

---

### Task 10: Full verification and PR

- [ ] `./Scripts/test.sh` green.
- [ ] Cross-platform check: copy a `history/<email>.jsonl` written by the Windows app (or the fixture line in `UsageHistoryTests`) into the Mac's `history/` — the History tab draws it.
- [ ] No real data in the diff: `git diff origin/main --unified=0 | grep -E '^\+' | grep -oE '[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+' | sort -u` shows only `@exemplo.com`.
- [ ] Push to the fork and open the PR against `Falkzera/falcao-token-router:main`, title `feat(macos): histórico de uso, com medição a cada 5 horas`, body modeled on PR #35 (what enters, the decision that changes "never a loop", the shared format, verification).
