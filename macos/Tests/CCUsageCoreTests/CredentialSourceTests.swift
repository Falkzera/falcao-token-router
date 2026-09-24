import Foundation
import Testing
@testable import CCUsageCore

/// Formato real do item de keychain `Claude Code-credentials`. O `refreshToken`
/// está aqui de propósito: o teste prova que ele atravessa a decodificação sem
/// ganhar leitor.
private let keychainBlob = """
{
  "claudeAiOauth": {
    "accessToken": "sk-ant-oat-TESTE",
    "refreshToken": "sk-ant-ort-TESTE",
    "expiresAt": 1787012535354,
    "refreshTokenExpiresAt": 1789604535354,
    "scopes": ["user:profile", "user:inference"],
    "subscriptionType": "max",
    "rateLimitTier": "default_claude_max_5x"
  }
}
"""

private func source(_ json: String, readsAccessToken: Bool) -> KeychainCredentialSource {
    KeychainCredentialSource(readsAccessToken: readsAccessToken,
                             load: { Data(json.utf8) })
}

/// Um pouco antes de `expiresAt`.
private let beforeExpiry = Date(timeIntervalSince1970: 1_787_012_000)
/// Um pouco depois.
private let afterExpiry = Date(timeIntervalSince1970: 1_787_013_000)

@Test func readsTheAccessTokenWhenAuthorized() {
    let credentials = source(keychainBlob, readsAccessToken: true).credentials()!
    #expect(credentials.accessToken == "sk-ant-oat-TESTE")
    #expect(credentials.usableToken(at: beforeExpiry) == "sk-ant-oat-TESTE")
}

@Test func doesNotReadTheAccessTokenWhenNotAuthorized() {
    // Com o toggle desligado o campo nem é extraído do dicionário. É a garantia
    // que a promessa reescrita do PlanDetector faz.
    let credentials = source(keychainBlob, readsAccessToken: false).credentials()!
    #expect(credentials.accessToken == nil)
    #expect(credentials.usableToken(at: beforeExpiry) == nil)
}

@Test func theTierIsReadableWithoutAuthorizingTheToken() {
    // O plano continua detectável sem que o app precise da credencial — era
    // verdade antes desta mudança e continua sendo.
    let credentials = source(keychainBlob, readsAccessToken: false).credentials()!
    #expect(credentials.rateLimitTier == "default_claude_max_5x")
}

@Test func expiredTokenIsNotUsable() {
    // O accessToken tem vida de horas. Conferir antes evita gastar uma chamada
    // de rede que só pode voltar 401.
    let credentials = source(keychainBlob, readsAccessToken: true).credentials()!
    #expect(credentials.usableToken(at: afterExpiry) == nil)
}

@Test func credentialsHaveNoPlaceToHoldARefreshToken() {
    // O app nunca renova OAuth (decisão L2): escrever no item do Claude Code
    // pode invalidar a sessão dele. A ausência do campo é o que garante isso —
    // não há onde guardar, então não há como usar.
    let mirror = Mirror(reflecting: source(keychainBlob, readsAccessToken: true).credentials()!)
    let labels = mirror.children.compactMap(\.label)
    #expect(!labels.contains { $0.lowercased().contains("refresh") })
}

@Test func absentKeychainItemYieldsNoCredentials() {
    let empty = KeychainCredentialSource(readsAccessToken: true, load: { nil })
    #expect(empty.credentials() == nil)
}

@Test func malformedBlobYieldsNoCredentials() {
    #expect(source("{ isto não é json", readsAccessToken: true).credentials() == nil)
}

@Test func blobWithoutTheOAuthKeyYieldsNoCredentials() {
    // Claude Code 2.1.x pode gravar só estado de MCP, sem `claudeAiOauth`.
    #expect(source(#"{ "mcpOAuth": {} }"#, readsAccessToken: true).credentials() == nil)
}

@Test func planDetectionReadsTheTierThroughTheSharedSource() {
    let detected = PlanDetector.detect(source: source(keychainBlob, readsAccessToken: false))
    #expect(detected == .max5)
}

@Test func planDetectionYieldsNothingWithoutCredentials() {
    let empty = KeychainCredentialSource(readsAccessToken: false, load: { nil })
    #expect(PlanDetector.detect(source: empty) == nil)
}
