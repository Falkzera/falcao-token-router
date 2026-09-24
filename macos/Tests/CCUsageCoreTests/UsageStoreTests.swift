import Foundation
import Testing
@testable import CCUsageCore

private func makeRootWithOneEvent() throws -> URL {
    let root = URL(fileURLWithPath: NSTemporaryDirectory())
        .appending(path: "cctc-store-\(UUID().uuidString)")
    try FileManager.default.createDirectory(at: root.appending(path: "p"),
                                            withIntermediateDirectories: true)
    let now = Date.ISO8601FormatStyle(includingFractionalSeconds: true).format(Date())
    let line = """
    {"type":"assistant","timestamp":"\(now)","requestId":"r1",\
    "message":{"id":"m1","model":"claude-opus-5","usage":{"input_tokens":0,\
    "output_tokens":1234,"cache_read_input_tokens":0}}}

    """
    try line.write(to: root.appending(path: "p/s.jsonl"), atomically: true, encoding: .utf8)
    return root
}

@MainActor
@Test func refreshLoadsEventsFromDiskIntoTheSnapshot() async throws {
    let root = try makeRootWithOneEvent()
    defer { try? FileManager.default.removeItem(at: root) }

    let store = UsageStore(scanner: ProjectScanner(root: root),
                           cacheURL: root.appending(path: "cache.json"))
    await store.refresh()

    #expect(store.snapshot.today.tokens == 1234)
    #expect(store.snapshot.session.resetsAt != nil)
}

@MainActor
@Test func historySurvivesARestart() async throws {
    // Regressão: o cache guardava só os offsets de leitura. No segundo launch
    // o ingest devolvia zero eventos ("tudo já lido") e o app acordava sem
    // histórico — tetos no piso e todos os gauges cravados em 100%.
    let root = try makeRootWithOneEvent()
    defer { try? FileManager.default.removeItem(at: root) }
    let cacheURL = root.appending(path: "cache.json")

    let first = UsageStore(scanner: ProjectScanner(root: root), cacheURL: cacheURL)
    await first.refresh()
    #expect(first.snapshot.today.tokens == 1234)

    // Processo novo, mesmo cache, nenhum arquivo novo no disco.
    let restarted = UsageStore(scanner: ProjectScanner(root: root), cacheURL: cacheURL)
    restarted.start()
    #expect(restarted.snapshot.today.tokens == 1234)
    restarted.stop()
}

@MainActor
@Test func refreshIsIdempotentAcrossRuns() async throws {
    let root = try makeRootWithOneEvent()
    defer { try? FileManager.default.removeItem(at: root) }

    let store = UsageStore(scanner: ProjectScanner(root: root),
                           cacheURL: root.appending(path: "cache.json"))
    await store.refresh()
    await store.refresh()   // segunda passada não pode duplicar

    #expect(store.snapshot.today.tokens == 1234)
}

// MARK: - Fonte ao vivo

/// Cache gravado em disco com a idade que o teste quiser.
private func writeCache(_ fraction: Double, agedBy age: TimeInterval, at url: URL) throws {
    let percent = Int(fraction * 100)
    let fetchedAtMs = (Date().addingTimeInterval(-age).timeIntervalSince1970 * 1000)
        .rounded()
    let json = """
    { "cachedUsageUtilization": { "fetchedAtMs": \(Int(fetchedAtMs)),
        "utilization": { "limits": [ { "kind": "session", "percent": \(percent),
          "severity": "normal", "is_active": true } ] } } }
    """
    try json.write(to: url, atomically: true, encoding: .utf8)
}

private func liveReportSaying(_ fraction: Double) -> UsageReport {
    UsageReport(limits: [.init(kind: .session, fraction: fraction, severity: .normal,
                               resetsAt: nil, modelName: nil, isActive: true)],
                fetchedAt: Date())
}

@MainActor
@Test func liveReadingReplacesTheStaleCache() async throws {
    // O caso que motivou a mudança: cache de 13h dizendo 35% enquanto o valor
    // real é 6%.
    let root = try makeRootWithOneEvent()
    defer { try? FileManager.default.removeItem(at: root) }
    let cacheFile = root.appending(path: "claude.json")
    try writeCache(0.35, agedBy: 13 * 3600, at: cacheFile)

    let store = UsageStore(scanner: ProjectScanner(root: root),
                           cacheURL: root.appending(path: "cache.json"),
                           cachedUsageURL: cacheFile,
                           liveUsageEnabled: true,
                           fetchLive: { _ in liveReportSaying(0.06) })
    await store.refresh()
    await store.refreshLive()

    #expect(store.snapshot.session.rawFraction == 0.06)
    if case .live = store.snapshot.sourceStatus {} else {
        Issue.record("esperava status ao vivo, veio \(store.snapshot.sourceStatus)")
    }
}

@MainActor
@Test func expiredCredentialFallsBackToTheCacheAndSaysSo() async throws {
    let root = try makeRootWithOneEvent()
    defer { try? FileManager.default.removeItem(at: root) }
    let cacheFile = root.appending(path: "claude.json")
    try writeCache(0.35, agedBy: 13 * 3600, at: cacheFile)

    let store = UsageStore(scanner: ProjectScanner(root: root),
                           cacheURL: root.appending(path: "cache.json"),
                           cachedUsageURL: cacheFile,
                           liveUsageEnabled: true,
                           fetchLive: { _ in throw LiveUsageError.unauthorized })
    await store.refresh()
    await store.refreshLive()

    #expect(store.snapshot.session.rawFraction == 0.35)
    if case .credentialExpired = store.snapshot.sourceStatus {} else {
        Issue.record("esperava credencial expirada, veio \(store.snapshot.sourceStatus)")
    }
}

@MainActor
@Test func theFetcherIsNeverCalledWhileTheToggleIsOff() async throws {
    // A garantia que sustenta a promessa reescrita do PlanDetector: desligado,
    // nada chama a rede e nada lê o token.
    let root = try makeRootWithOneEvent()
    defer { try? FileManager.default.removeItem(at: root) }
    let cacheFile = root.appending(path: "claude.json")
    try writeCache(0.35, agedBy: 60, at: cacheFile)

    final class Counter: @unchecked Sendable { var calls = 0 }
    let counter = Counter()

    let store = UsageStore(scanner: ProjectScanner(root: root),
                           cacheURL: root.appending(path: "cache.json"),
                           cachedUsageURL: cacheFile,
                           liveUsageEnabled: false,
                           fetchLive: { _ in
                               counter.calls += 1
                               return liveReportSaying(0.06)
                           })
    await store.refresh()
    await store.refreshLive()
    store.panelDidOpen()

    // `panelDidOpen()` enfileira uma Task; sem um ponto de suspensão aqui a
    // asserção rodaria antes dela poder executar, e passaria mesmo sem o guard.
    await Task.yield()

    #expect(counter.calls == 0)
    #expect(store.snapshot.session.rawFraction == 0.35)
}

@MainActor
@Test func turningTheToggleOffDiscardsTheLiveNumberImmediately() async throws {
    // O comportamento mais visível desta task: desautorizar a busca não pode
    // deixar na tela o número que ela trouxe. Sem o `lastLive = nil` no didSet,
    // o painel seguiria mostrando 0.06 até o tick seguinte.
    let root = try makeRootWithOneEvent()
    defer { try? FileManager.default.removeItem(at: root) }
    let cacheFile = root.appending(path: "claude.json")
    try writeCache(0.35, agedBy: 13 * 3600, at: cacheFile)

    let store = UsageStore(scanner: ProjectScanner(root: root),
                           cacheURL: root.appending(path: "cache.json"),
                           cachedUsageURL: cacheFile,
                           liveUsageEnabled: true,
                           fetchLive: { _ in liveReportSaying(0.06) })
    await store.refresh()
    await store.refreshLive()
    #expect(store.snapshot.session.rawFraction == 0.06)

    store.liveUsageEnabled = false
    #expect(store.snapshot.session.rawFraction == 0.35)
}

@MainActor
@Test func turningTheToggleBackOnDoesNotFlashTheOldLiveNumber() async throws {
    // Este é o caso que o `lastLive = nil` de fato protege, e que desligar
    // sozinho não exercita: a política já ignora `live` com o toggle desligado.
    // Sem o descarte, religar mostraria o número ao vivo ANTIGO marcado como
    // `.live` — e `age(at:)` devolve nil para `.live`, então nem a idade
    // denunciaria. Com o descarte, o religar cai no cache até a busca nova
    // chegar, e o cache sabe dizer que tem 13h.
    let root = try makeRootWithOneEvent()
    defer { try? FileManager.default.removeItem(at: root) }
    let cacheFile = root.appending(path: "claude.json")
    try writeCache(0.35, agedBy: 13 * 3600, at: cacheFile)

    let store = UsageStore(scanner: ProjectScanner(root: root),
                           cacheURL: root.appending(path: "cache.json"),
                           cachedUsageURL: cacheFile,
                           liveUsageEnabled: true,
                           fetchLive: { _ in liveReportSaying(0.06) })
    await store.refresh()
    await store.refreshLive()
    #expect(store.snapshot.session.rawFraction == 0.06)

    store.liveUsageEnabled = false
    store.liveUsageEnabled = true

    // O rebuild síncrono do didSet já rodou; a busca nova ainda não voltou.
    #expect(store.snapshot.session.rawFraction == 0.35)
    // E a idade denuncia o cache, coisa que `.live` não faria.
    #expect(store.snapshot.session.age(at: Date()) != nil)
}
