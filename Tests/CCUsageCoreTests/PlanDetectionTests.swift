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
