import Foundation
import Testing
@testable import CCUsageCore

private func freshDefaults() -> UserDefaults {
    UserDefaults(suiteName: "cctc-test-\(UUID().uuidString)")!
}

@Test func planPricesMatchTheSubscriptionTiers() {
    #expect(Plan.pro.monthlyPrice == 20)
    #expect(Plan.max5.monthlyPrice == 100)
    #expect(Plan.max20.monthlyPrice == 200)
}

@Test func returnMultipleDividesMonthlyValueByThePlanPrice() {
    #expect(Plan.max20.returnMultiple(forMonthly: Money(usd: 600, isPartial: false)) == 3.0)
    #expect(Plan.max5.returnMultiple(forMonthly: Money(usd: 600, isPartial: false)) == 6.0)
}

@Test func noReturnMultipleBeforeAnyConsumption() {
    // 0× no dia 1 do mês não informa nada; melhor não mostrar linha nenhuma.
    #expect(Plan.max20.returnMultiple(forMonthly: .zero) == nil)
}

/// Detector neutralizado de propósito: sem isso o teste lê o keychain real,
/// depende do plano de quem executa e paga o custo do prompt.
@MainActor
@Test func defaultsToMax20WithAutomaticCeiling() {
    let settings = AppSettings(defaults: freshDefaults(), detectPlan: { nil })
    #expect(settings.plan == .max20)
    #expect(settings.manualBlockCeiling == nil)
    #expect(settings.ceilingOverride == nil)
}

@MainActor
@Test func settingsSurviveARestart() {
    let defaults = freshDefaults()
    let first = AppSettings(defaults: defaults, detectPlan: { nil })
    first.plan = .max5
    first.manualBlockCeiling = 500_000

    let restarted = AppSettings(defaults: defaults, detectPlan: { nil })
    #expect(restarted.plan == .max5)
    #expect(restarted.manualBlockCeiling == 500_000)
}

@MainActor
@Test func manualCeilingBecomesAnOverrideAndAutomaticClearsIt() {
    let settings = AppSettings(defaults: freshDefaults(), detectPlan: { nil })
    settings.manualBlockCeiling = 214_400_000
    #expect(settings.ceilingOverride == Ceilings(blockTokens: 214_400_000))

    settings.manualBlockCeiling = nil
    #expect(settings.ceilingOverride == nil)
}

@MainActor
@Test func corruptedDefaultsFallBackToTheDefaults() {
    // Payload inválido não pode impedir o app de abrir.
    let defaults = freshDefaults()
    defaults.set(Data("não é json".utf8), forKey: AppSettings.storageKey)
    let settings = AppSettings(defaults: defaults, detectPlan: { nil })
    #expect(settings.plan == .max20)
}

@MainActor
@Test func liveUsageIsOnByDefault() {
    // A fonte "ao vivo" virou o sensor local (sem token, sem rede), então não há
    // mais o custo de privacidade que justificava começar desligado.
    let settings = AppSettings(defaults: freshDefaults(), detectPlan: { nil })
    #expect(settings.liveUsageEnabled == true)
}

@MainActor
@Test func liveUsageChoiceSurvivesRestart() {
    let defaults = freshDefaults()
    let first = AppSettings(defaults: defaults, detectPlan: { nil })
    first.liveUsageEnabled = true

    let restarted = AppSettings(defaults: defaults, detectPlan: { nil })
    #expect(restarted.liveUsageEnabled)
}

@MainActor
@Test func settingsSavedBeforeTheToggleExistedAdoptTheNewDefault() {
    // Payload gravado por uma versão anterior não tem o campo. Como a fonte "ao
    // vivo" agora é o sensor local (sem token), ausência resolve para o novo
    // default ligado — sem custo de privacidade a proteger.
    let defaults = freshDefaults()
    let legacy = #"{"plan":"max5"}"#
    defaults.set(Data(legacy.utf8), forKey: AppSettings.storageKey)

    let settings = AppSettings(defaults: defaults, detectPlan: { nil })
    #expect(settings.plan == .max5)
    #expect(settings.liveUsageEnabled == true)
}

@MainActor
@Suite("Migração do domínio antigo")
struct LegacyDefaultsMigrationTests {
    /// `UserDefaults.standard` é indexado pelo bundle id. Quando o produto foi
    /// renomeado, o app passou a ler um domínio vazio e o plano, os alertas e o
    /// teto do usuário ficaram no antigo — invisíveis, sem nada avisando.
    @Test("preferências do bundle antigo são adotadas quando o novo está vazio")
    func adotaDominioAntigo() throws {
        let antigo = "teste.antigo.\(UUID().uuidString)"
        let novo = "teste.novo.\(UUID().uuidString)"
        let legacy = try #require(UserDefaults(suiteName: antigo))
        let atual = try #require(UserDefaults(suiteName: novo))
        defer {
            legacy.removePersistentDomain(forName: antigo)
            atual.removePersistentDomain(forName: novo)
        }

        let payload = Data(#"{"plan":"max5","liveUsageEnabled":true}"#.utf8)
        legacy.set(payload, forKey: AppSettings.storageKey)

        #expect(AppSettings.migrateLegacyDefaults(into: atual, legacyID: antigo))
        #expect(atual.data(forKey: AppSettings.storageKey) == payload)
        // A origem NÃO é apagada: migração que destrói a origem não tem volta.
        #expect(legacy.data(forKey: AppSettings.storageKey) == payload)

        let settings = AppSettings(defaults: atual, detectPlan: { nil })
        #expect(settings.plan == .max5)
    }

    /// Roda uma vez. Depois da primeira gravação no domínio novo, o que está lá
    /// é a verdade e o antigo não pode voltar por cima.
    @Test("domínio novo já preenchido não é sobrescrito")
    func naoSobrescreveOQueJaExiste() throws {
        let antigo = "teste.antigo.\(UUID().uuidString)"
        let novo = "teste.novo.\(UUID().uuidString)"
        let legacy = try #require(UserDefaults(suiteName: antigo))
        let atual = try #require(UserDefaults(suiteName: novo))
        defer {
            legacy.removePersistentDomain(forName: antigo)
            atual.removePersistentDomain(forName: novo)
        }

        legacy.set(Data(#"{"plan":"max5"}"#.utf8), forKey: AppSettings.storageKey)
        let atualData = Data(#"{"plan":"pro"}"#.utf8)
        atual.set(atualData, forKey: AppSettings.storageKey)

        #expect(!AppSettings.migrateLegacyDefaults(into: atual, legacyID: antigo))
        #expect(atual.data(forKey: AppSettings.storageKey) == atualData)
    }
}
