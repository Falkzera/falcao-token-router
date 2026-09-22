import Foundation
import Testing
@testable import CCUsageCore

private let t0 = Date(timeIntervalSince1970: 1_755_000_000)

/// Conta leituras. É a métrica que interessa: cada leitura do item de keychain
/// do Claude Code pode abrir o diálogo de autorização do macOS.
private final class CountingSource: CredentialSource, @unchecked Sendable {
    private(set) var reads = 0
    var result: ClaudeCredentials?

    init(_ result: ClaudeCredentials?) { self.result = result }

    func credentials() -> ClaudeCredentials? {
        reads += 1
        return result
    }
}

private func credentials(expiresAt: Date?) -> ClaudeCredentials {
    ClaudeCredentials(accessToken: "tok", expiresAt: expiresAt, rateLimitTier: "max_20x")
}

/// Relógio controlado pelo teste, para exercitar expiração sem esperar.
private final class Clock: @unchecked Sendable {
    var now: Date
    init(_ now: Date) { self.now = now }
}

@Test func theSecondReadDoesNotTouchTheKeychain() {
    let inner = CountingSource(credentials(expiresAt: t0.addingTimeInterval(3600)))
    let cached = CachedCredentialSource(wrapping: inner, now: { t0 })

    _ = cached.credentials()
    _ = cached.credentials()
    _ = cached.credentials()

    #expect(inner.reads == 1)
}

@Test func anExpiredCredentialIsReadAgain() {
    let clock = Clock(t0)
    let inner = CountingSource(credentials(expiresAt: t0.addingTimeInterval(60)))
    let cached = CachedCredentialSource(wrapping: inner, now: { clock.now })

    _ = cached.credentials()
    clock.now = t0.addingTimeInterval(120)
    _ = cached.credentials()

    #expect(inner.reads == 2)
}

/// A saída para o 401: o servidor recusou o token que estava em cache, então o
/// cache está errado mesmo que o `expiresAt` ainda não tenha chegado.
@Test func invalidateForcesAFreshRead() {
    let inner = CountingSource(credentials(expiresAt: t0.addingTimeInterval(3600)))
    let cached = CachedCredentialSource(wrapping: inner, now: { t0 })

    _ = cached.credentials()
    cached.invalidate()
    _ = cached.credentials()

    #expect(inner.reads == 2)
}

/// Leitura que falhou — item ausente, ou autorização negada — também fica em
/// cache. Retentar a cada ciclo transformaria uma negativa em diálogo de cinco
/// em cinco minutos, que é o defeito que este tipo existe para consertar.
@Test func aFailedReadIsNotRetriedUntilInvalidated() {
    let inner = CountingSource(nil)
    let cached = CachedCredentialSource(wrapping: inner, now: { t0 })

    _ = cached.credentials()
    _ = cached.credentials()
    #expect(inner.reads == 1)

    cached.invalidate()
    _ = cached.credentials()
    #expect(inner.reads == 2)
}

/// Sem `expiresAt` não há quando expirar, e reler por precaução voltaria a
/// pedir autorização sem motivo.
@Test func aCredentialWithoutExpiryStaysCached() {
    let inner = CountingSource(credentials(expiresAt: nil))
    let cached = CachedCredentialSource(wrapping: inner, now: { t0.addingTimeInterval(9_000_000) })

    _ = cached.credentials()
    _ = cached.credentials()

    #expect(inner.reads == 1)
}
