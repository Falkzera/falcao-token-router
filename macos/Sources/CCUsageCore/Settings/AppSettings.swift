import Foundation
import Observation

/// Assinatura do Claude. O preço é dado de negócio, não de apresentação — mora
/// aqui junto da `PricingTable`, não na UI.
public enum Plan: String, CaseIterable, Codable, Sendable {
    case pro
    case max5
    case max20

    public var monthlyPrice: Decimal {
        switch self {
        case .pro: 20
        case .max5: 100
        case .max20: 200
        }
    }

    public var label: String {
        switch self {
        case .pro: "Pro"
        case .max5: "Max 5×"
        case .max20: "Max 20×"
        }
    }

    /// Quantas vezes o valor equivalente de API consumido no mês cobre a
    /// mensalidade. `nil` sem consumo — "0×" no dia 1 do mês não informa nada.
    ///
    /// Quando o total do mês está parcial (algum modelo sem preço conhecido),
    /// este múltiplo é piso, não valor exato; quem exibe deve marcar isso.
    public func returnMultiple(forMonthly money: Money) -> Double? {
        guard money.usd > 0 else { return nil }
        return NSDecimalNumber(decimal: money.usd / monthlyPrice).doubleValue
    }
}

/// Preferências do usuário, persistidas em `UserDefaults`.
///
/// Fica no core em vez da camada de UI para ser exercitável sem instanciar
/// janela — mesma regra do resto do módulo.
@MainActor
@Observable
public final class AppSettings {
    public static let storageKey = "settings.v1"

    /// O `CFBundleIdentifier` que o app tinha antes de virar
    /// `falcao-token-router`, em 18/09/2026.
    ///
    /// `UserDefaults.standard` é indexado pelo bundle id. Trocá-lo apontou o app
    /// para um domínio **vazio**, e o plano escolhido, as preferências de alerta
    /// e o teto manual do usuário continuaram existindo no domínio antigo —
    /// invisíveis, sem nada avisando. O app simplesmente abriu com os padrões de
    /// fábrica, que é o tipo de perda que ninguém liga a uma renomeação.
    ///
    /// Mesma família de erro que o `RouterPaths.bundleID` evita de propósito; lá
    /// a consequência seria pior (credencial inalcançável), aqui é silenciosa.
    public static let legacyBundleID = "com.synqo.claudetokencounter"

    public var plan: Plan {
        didSet { save() }
    }

    /// `nil` = calibração automática pelo histórico.
    ///
    /// Só o teto do bloco de 5h é ajustável: a semana típica é uma comparação
    /// com o próprio ritmo, não um teto, então não há o que sobrescrever.
    public var manualBlockCeiling: UInt64? {
        didSet { save() }
    }

    /// Prefere o número recente do sensor ao cache de `~/.claude.json`.
    ///
    /// Ligado por padrão agora: a fonte "ao vivo" deixou de ser uma chamada de
    /// API (que lia o token do Claude Code) e passou a ser o sensor da status
    /// line — arquivo local do próprio usuário, sem token, sem rede. Sem o custo
    /// de privacidade de antes, não há razão para começar desligado.
    public var liveUsageEnabled: Bool {
        didSet { save() }
    }

    /// Mostrar o app no Dock, e não só na barra de menus.
    ///
    /// Desligado de fábrica porque o produto é um widget de barra. Mas uma barra
    /// cheia — dezenove itens, numa tela com notch — **esconde o que não cabe
    /// sem avisar**, e aí o app fica rodando sem interface alguma: sem ícone no
    /// Dock, sem janela, sem ⌘-Tab. Ligado, ele vira um app comum: aparece no
    /// Dock, responde ao ⌘-Tab, e clicar no ícone abre a janela.
    public var showInDock: Bool {
        didSet { save() }
    }

    /// O que notificar. Tudo desligado por padrão: notificação é interrupção, e
    /// interrupção não pode ser o estado inicial de nada. Ligar dispara o
    /// pedido de permissão do sistema — que é o momento certo para pedir,
    /// porque aí existe contexto.
    public var alerts: AlertPreferences {
        didSet { save() }
    }

    /// Plano lido do keychain, quando disponível. Exposto mesmo quando o
    /// usuário escolheu outro: a divergência é informação, não erro — a
    /// detecção pode estar desatualizada depois de um upgrade de plano.
    public private(set) var detectedPlan: Plan?

    @ObservationIgnored private let defaults: UserDefaults

    /// Copia as preferências do domínio antigo quando o novo ainda não tem
    /// nenhuma. Roda uma vez: depois da primeira gravação no domínio novo, a
    /// condição deixa de valer.
    ///
    /// Não apaga o domínio antigo — migração que destrói a origem não tem volta,
    /// e um `defaults delete` errado aqui custaria a configuração de verdade.
    @discardableResult
    static func migrateLegacyDefaults(into defaults: UserDefaults,
                                      legacyID: String = legacyBundleID) -> Bool {
        guard defaults.data(forKey: storageKey) == nil,
              let legacy = UserDefaults(suiteName: legacyID),
              let payload = legacy.data(forKey: storageKey)
        else { return false }
        defaults.set(payload, forKey: storageKey)
        return true
    }

    public init(defaults: UserDefaults = .standard,
                detectPlan: () -> Plan? = { PlanDetector.detect() }) {
        self.defaults = defaults
        // Só no domínio REAL do app. Um `UserDefaults` de suite — teste, ou um
        // chamador que quer isolamento — não é o domínio que foi renomeado, e
        // puxar o antigo para dentro dele contamina o chamador com a
        // configuração da máquina de quem roda. Dois testes de plano padrão
        // quebraram exatamente assim antes desta guarda.
        if defaults === UserDefaults.standard {
            Self.migrateLegacyDefaults(into: defaults)
        }
        let stored = defaults.data(forKey: Self.storageKey)
            .flatMap { try? JSONDecoder().decode(Payload.self, from: $0) }
        let detected = detectPlan()
        self.detectedPlan = detected
        // Escolha explícita vence a detecção, que vence o default. Payload
        // corrompido não pode impedir o app de abrir.
        self.plan = stored?.plan ?? detected ?? .max20
        self.manualBlockCeiling = stored?.manualBlockCeiling
        self.liveUsageEnabled = stored?.liveUsageEnabled ?? true
        self.showInDock = stored?.showInDock ?? false
        // `alertsEnabled` é o formato anterior, de quando havia uma chave só.
        // Migra em vez de descartar: quem tinha ligado não perde a escolha.
        self.alerts = stored?.alerts
            ?? ((stored?.alertsEnabled ?? false) ? .default : .off)
    }

    /// `true` quando a detecção discorda do que está selecionado — a UI avisa
    /// em vez de trocar por baixo do usuário.
    public var planDisagreesWithDetection: Bool {
        guard let detectedPlan else { return false }
        return detectedPlan != plan
    }

    public var ceilingOverride: Ceilings? {
        manualBlockCeiling.map { Ceilings(blockTokens: $0) }
    }

    private struct Payload: Codable {
        var plan: Plan
        var manualBlockCeiling: UInt64?
        /// Ausente em payload gravado antes deste campo existir. `nil` resolve
        /// para `false` no init — nunca para ligado.
        var liveUsageEnabled: Bool?
        var showInDock: Bool?
        /// Formato anterior, com uma chave só. Lido para migração; não é mais
        /// gravado.
        var alertsEnabled: Bool?
        var alerts: AlertPreferences?
    }

    private func save() {
        let payload = Payload(plan: plan, manualBlockCeiling: manualBlockCeiling,
                              liveUsageEnabled: liveUsageEnabled,
                              showInDock: showInDock,
                              alertsEnabled: nil, alerts: alerts)
        guard let data = try? JSONEncoder().encode(payload) else { return }
        defaults.set(data, forKey: Self.storageKey)
    }
}
