import Foundation

extension Plan {
    /// Mapeia o tier que o Claude Code grava no `oauthAccount` do `.claude.json`
    /// (`userRateLimitTier`).
    ///
    /// Só `default_claude_max_5x` foi observado de verdade; as outras variantes
    /// são inferidas pela nomenclatura. Por isso o casamento é por trecho e não
    /// por igualdade: se a Anthropic mudar o prefixo, `enterprise_claude_max_5x`
    /// continua resolvendo.
    ///
    /// Tier desconhecido devolve `nil` em vez de chutar — um plano errado vira
    /// um múltiplo de retorno errado, e é melhor cair no seletor manual.
    public init?(rateLimitTier: String) {
        let tier = rateLimitTier.lowercased()
        if tier.contains("max_20x") { self = .max20 }
        else if tier.contains("max_5x") { self = .max5 }
        else if tier.contains("pro") { self = .pro }
        else { return nil }
    }
}

/// Lê o plano do `.claude.json` do perfil padrão — o mesmo arquivo de onde o app
/// já tira a identidade de cada conta.
///
/// **Não toca o chaveiro.** Até 10/2026 o plano vinha do item
/// `Claude Code-credentials`: o app decodificava o blob inteiro — tokens
/// incluídos — via `SecItemCopyMatching`, só para ler um campo, e a primeira
/// leitura abria o diálogo de autorização do macOS. O `oauthAccount` do
/// `.claude.json` traz o mesmo tier, sem segredo nenhum ao lado. Com isto, nada
/// no app decodifica a credencial: ela só é copiada, como blob, entre itens.
///
/// `userRateLimitTier` primeiro: é o plano da pessoa. Numa conta Max, em
/// 10/2026, o `organizationRateLimitTier` não nomeava plano nenhum — ele só vale
/// como reserva.
///
/// Sem `.claude.json`, ou com tier desconhecido, devolve `nil` — o seletor
/// manual continua valendo.
public enum PlanDetector {
    public static func detect(
        identity: () -> AccountIdentity? = { AnthropicAdapter().identity(inConfigDir: .standard()) }
    ) -> Plan? {
        guard let raw = identity()?.raw else { return nil }
        return ["userRateLimitTier", "organizationRateLimitTier"]
            .lazy
            .compactMap { raw[$0]?.stringValue }
            .compactMap(Plan.init(rateLimitTier:))
            .first
    }
}
