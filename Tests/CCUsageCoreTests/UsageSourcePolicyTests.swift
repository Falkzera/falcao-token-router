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
        liveEnabled: true, live: .success(liveReport), cached: staleCache, now: policyNow)
    // O caso que motivou tudo: cache dizia 35%, ao vivo diz 6%.
    #expect(source?.report == liveReport)
    #expect(source?.isLive == true)
    #expect(status == .live(at: policyNow))
}

@Test func expiredCredentialFallsBackAndSaysSo() {
    for error in [LiveUsageError.noToken, .unauthorized] {
        let (source, status) = UsageSourcePolicy.select(
            liveEnabled: true, live: .failure(error), cached: staleCache, now: policyNow)
        #expect(source?.report == staleCache)
        #expect(status == .credentialExpired(age: 13 * 3600))
    }
}

@Test func networkFailureFallsBackWithADifferentStatus() {
    // Distinto de credencial expirada porque a saída do usuário é outra:
    // esperar, não reautenticar.
    for error in [LiveUsageError.transport, .malformed] {
        let (source, status) = UsageSourcePolicy.select(
            liveEnabled: true, live: .failure(error), cached: staleCache, now: policyNow)
        #expect(source?.report == staleCache)
        #expect(status == .liveUnavailable(age: 13 * 3600))
    }
}

@Test func noSourceAtAllFallsThroughToTheDerivedPath() {
    let (source, status) = UsageSourcePolicy.select(
        liveEnabled: true, live: .failure(.transport), cached: nil, now: policyNow)
    #expect(source == nil)
    #expect(status == .derivedOnly)
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
