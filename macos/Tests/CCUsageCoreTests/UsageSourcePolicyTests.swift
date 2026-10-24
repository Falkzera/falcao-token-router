import Foundation
import Testing
@testable import CCUsageCore

private let policyNow = Date(timeIntervalSince1970: 1_787_000_000)

private func report(fraction: Double, agedBy age: TimeInterval) -> UsageReport {
    UsageReport(
        limits: [.init(kind: .session, fraction: fraction, severity: .normal,
                       resetsAt: nil, modelName: nil, isActive: true)],
        fetchedAt: policyNow.addingTimeInterval(-age))
}

private let liveReport = report(fraction: 0.06, agedBy: 0)
private let staleCache = report(fraction: 0.35, agedBy: 13 * 3600)

@Test func liveDisabledUsesTheCache() {
    let (source, status) = UsageSourcePolicy.select(
        liveEnabled: false, live: nil, cached: staleCache, now: policyNow)
    #expect(source?.report == staleCache)
    #expect(source?.isLive == false)
    #expect(status == .cached(age: 13 * 3600))
}

@Test func liveEnabledAndSuccessfulUsesTheLiveReport() {
    let (source, status) = UsageSourcePolicy.select(
        liveEnabled: true, live: liveReport, cached: staleCache, now: policyNow)
    // O caso que motivou tudo: cache dizia 35%, ao vivo diz 6%.
    #expect(source?.report == liveReport)
    #expect(source?.isLive == true)
    #expect(status == .live(at: policyNow))
}

@Test("sem amostra recente do sensor é cache com idade, não credencial expirada")
func noRecentSampleIsTheCacheNotAnExpiredCredential() {
    // O sensor não autentica nada: faltar amostra na última hora é o caso comum
    // de uma conta parada, e o painel mostrava "credencial expirada" por ele.
    let (source, status) = UsageSourcePolicy.select(
        liveEnabled: true, live: nil, cached: staleCache, now: policyNow)
    #expect(source?.report == staleCache)
    #expect(source?.isLive == false)
    #expect(status == .cached(age: 13 * 3600))
}

@Test func noSourceAtAllFallsThroughToTheDerivedPath() {
    let (source, status) = UsageSourcePolicy.select(
        liveEnabled: true, live: nil, cached: nil, now: policyNow)
    #expect(source == nil)
    #expect(status == .derivedOnly)
}

@Test func liveDisabledIgnoresALiveReading() {
    // Desligado, nem uma leitura que já chegou vence o cache.
    let (source, status) = UsageSourcePolicy.select(
        liveEnabled: false, live: liveReport, cached: staleCache, now: policyNow)
    #expect(source?.report == staleCache)
    #expect(status == .cached(age: 13 * 3600))
}

@Test func liveDisabledWithoutCacheFallsThroughToTheDerivedPath() {
    let (source, status) = UsageSourcePolicy.select(
        liveEnabled: false, live: nil, cached: nil, now: policyNow)
    #expect(source == nil)
    #expect(status == .derivedOnly)
}

@Test func liveEnabledButNotYetFetchedShowsTheCache() {
    // Primeira abertura: o toggle está ligado mas a chamada ainda não voltou.
    // Mostrar o cache é melhor que mostrar a estimativa derivada.
    let (source, status) = UsageSourcePolicy.select(
        liveEnabled: true, live: nil, cached: staleCache, now: policyNow)
    #expect(source?.report == staleCache)
    #expect(status == .cached(age: 13 * 3600))
}

@Test func cacheFromTheFutureIsClampedToZeroAge() {
    // Relógio ajustado para trás não pode produzir idade negativa, que
    // formataria como "há -2h".
    let future = report(fraction: 0.1, agedBy: -7200)
    let (_, status) = UsageSourcePolicy.select(
        liveEnabled: false, live: nil, cached: future, now: policyNow)
    #expect(status == .cached(age: 0))
}
