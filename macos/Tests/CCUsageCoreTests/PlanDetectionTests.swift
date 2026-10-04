import Foundation
import Testing
@testable import CCUsageCore

private func freshDefaults() -> UserDefaults {
    UserDefaults(suiteName: "cctc-test-\(UUID().uuidString)")!
}

@Test func mapsKnownRateLimitTiers() {
    #expect(Plan(rateLimitTier: "default_claude_max_5x") == .max5)
    #expect(Plan(rateLimitTier: "default_claude_max_20x") == .max20)
    #expect(Plan(rateLimitTier: "default_claude_pro") == .pro)
}

@Test func matchesTiersBySubstringToSurviveRenames() {
    // Só `default_claude_max_5x` foi observado de verdade; os outros são
    // inferidos. Casar por trecho evita quebrar se o prefixo mudar.
    #expect(Plan(rateLimitTier: "enterprise_claude_max_20x_beta") == .max20)
}

@Test func unknownTierYieldsNoPlan() {
    // Tier novo não vira palpite: melhor cair no seletor manual.
    #expect(Plan(rateLimitTier: "default_claude_ultra_9x") == nil)
    #expect(Plan(rateLimitTier: "") == nil)
}

@MainActor
@Test func detectedPlanIsUsedWhenNothingWasChosen() {
    let settings = AppSettings(defaults: freshDefaults(), detectPlan: { .max5 })
    #expect(settings.plan == .max5)
    #expect(settings.detectedPlan == .max5)
}

@MainActor
@Test func anExplicitChoiceOutranksDetection() {
    let defaults = freshDefaults()
    let first = AppSettings(defaults: defaults, detectPlan: { .max5 })
    first.plan = .max20

    let restarted = AppSettings(defaults: defaults, detectPlan: { .max5 })
    #expect(restarted.plan == .max20)          // a escolha do usuário vence
    #expect(restarted.detectedPlan == .max5)   // mas a divergência fica visível
}

@MainActor
@Test func fallsBackToMax20WhenDetectionFails() {
    let settings = AppSettings(defaults: freshDefaults(), detectPlan: { nil })
    #expect(settings.plan == .max20)
    #expect(settings.detectedPlan == nil)
}

// MARK: - Detecção pelo `.claude.json` (sem chaveiro)

private func identity(_ tiers: [String: String]) -> AccountIdentity {
    AccountIdentity(email: "conta1@exemplo.com", organizationName: "Acme",
                    rateLimitTier: nil, raw: tiers.mapValues(JSONValue.string))
}

@Test("o plano vem do tier de USUÁRIO do .claude.json")
func planComesFromTheUserTier() {
    // Numa conta Max de verdade (10/2026) o tier de organização não nomeava
    // plano nenhum; quem diz o plano da pessoa é o de usuário.
    let detected = PlanDetector.detect(identity: {
        identity(["userRateLimitTier": "default_claude_max_5x",
                  "organizationRateLimitTier": "default_claude_ai"])
    })
    #expect(detected == .max5)
}

@Test("sem o tier de usuário, o de organização serve de reserva")
func organizationTierIsTheFallback() {
    let detected = PlanDetector.detect(identity: {
        identity(["organizationRateLimitTier": "default_claude_max_20x"])
    })
    #expect(detected == .max20)
}

@Test("sem .claude.json, ou sem tier conhecido, não há plano detectado")
func noIdentityMeansNoPlan() {
    #expect(PlanDetector.detect(identity: { nil }) == nil)
    #expect(PlanDetector.detect(identity: { identity([:]) }) == nil)
    #expect(PlanDetector.detect(identity: {
        identity(["userRateLimitTier": "default_claude_ultra_9x"])
    }) == nil)
}

@Test("a detecção lê o .claude.json de verdade, ao lado do perfil padrão")
func detectionReadsTheRealFileBesideTheDefaultProfile() throws {
    let home = FileManager.default.temporaryDirectory
        .appending(path: "plan-\(UUID().uuidString)")
    try FileManager.default.createDirectory(at: home, withIntermediateDirectories: true)
    defer { try? FileManager.default.removeItem(at: home) }
    // O do perfil padrão mora AO LADO de `~/.claude`, não dentro.
    try Data("""
    { "oauthAccount": { "emailAddress": "conta1@exemplo.com",
                        "userRateLimitTier": "default_claude_max_20x" } }
    """.utf8).write(to: home.appending(path: ".claude.json"))

    let detected = PlanDetector.detect(identity: {
        AnthropicAdapter().identity(inConfigDir: .standard(home: home.path))
    })
    #expect(detected == .max20)
}
