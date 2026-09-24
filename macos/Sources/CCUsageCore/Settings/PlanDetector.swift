import Foundation

extension Plan {
    /// Mapeia o `rateLimitTier` que o Claude Code grava no keychain.
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

/// Lê o plano do item de keychain que o Claude Code já mantém.
///
/// **Lê exclusivamente `rateLimitTier`.** O mesmo item guarda `accessToken` e
/// `refreshToken`. O `accessToken` só é lido quando o usuário liga "Buscar
/// números ao vivo" nos Ajustes, e mesmo então apenas pelo
/// `KeychainCredentialSource` — este caminho passa `readsAccessToken: false` e
/// o campo nem chega a ser extraído. O `refreshToken` não tem leitor nenhum em
/// lugar algum do app: `ClaudeCredentials` não tem campo para ele.
///
/// A promessa aqui era absoluta e virou condicional quando a busca ao vivo
/// entrou. Condicional e verificável é o que ela pode ser sem mentir.
///
/// A primeira leitura dispara o prompt de keychain do macOS, porque o item
/// pertence ao Claude Code. Negar só faz a detecção falhar — o seletor manual
/// continua valendo.
public enum PlanDetector {
    public static func detect(
        source: any CredentialSource = KeychainCredentialSource(readsAccessToken: false)
    ) -> Plan? {
        guard let tier = source.credentials()?.rateLimitTier else { return nil }
        return Plan(rateLimitTier: tier)
    }
}
