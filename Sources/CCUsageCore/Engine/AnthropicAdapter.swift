import Foundation

/// O que é específico do Claude Code, atrás do `ProviderAdapter`.
///
/// Concentra num lugar só cada coisa que se descobriu observando o Claude Code
/// em agosto/2026 — o nome do item de chaveiro, o `.claude.json` com a
/// identidade, onde ele mora. Nada disto está documentado, então quando uma
/// versão do Claude Code mudar algo, muda-se aqui, e o motor não sente.
public struct AnthropicAdapter: ProviderAdapter {
    public let provider: Provider = .anthropic

    /// Injetável para os testes lerem e escreverem em arquivos temporários.
    private let readFile: @Sendable (URL) -> Data?
    private let writeFile: @Sendable (Data, URL) throws -> Void

    public init(
        readFile: @escaping @Sendable (URL) -> Data? = { try? Data(contentsOf: $0) },
        writeFile: @escaping @Sendable (Data, URL) throws -> Void = { data, url in
            try data.write(to: url, options: .atomic)
        }
    ) {
        self.readFile = readFile
        self.writeFile = writeFile
    }

    public func keychainService(forConfigDir dir: ConfigDir) -> String {
        dir.isDefault
            ? "Claude Code-credentials"
            : "Claude Code-credentials-\(dir.keychainHash)"
    }

    public func identity(inConfigDir dir: ConfigDir) -> AccountIdentity? {
        guard let data = readFile(dir.globalConfigURL),
              let root = try? JSONSerialization.jsonObject(with: data) as? [String: Any],
              let oauth = root["oauthAccount"] as? [String: Any],
              let email = oauth["emailAddress"] as? String
        else { return nil }

        return AccountIdentity(
            email: email,
            organizationName: oauth["organizationName"] as? String,
            // O tier fica em `organizationRateLimitTier`; o de usuário é o
            // fallback para conta pessoal sem organização.
            rateLimitTier: (oauth["organizationRateLimitTier"] as? String)
                ?? (oauth["userRateLimitTier"] as? String),
            raw: oauth.mapValues(JSONValue.init(any:)))
    }

    public func writeIdentity(_ identity: AccountIdentity, toConfigDir dir: ConfigDir) throws {
        let url = dir.globalConfigURL
        // Garante o diretório do perfil: um grupo dedicado pode nunca ter sido
        // criado no disco antes da primeira ativação, e escrever o
        // `.claude.json` num diretório inexistente falharia. Para o perfil
        // padrão, o pai é o home, que sempre existe.
        try FileManager.default.createDirectory(
            at: url.deletingLastPathComponent(), withIntermediateDirectories: true)
        // Preserva o resto do `.claude.json` — só a identidade e o que é
        // atrelado a ela mudam. Um arquivo ausente vira um objeto novo.
        var root = (readFile(url)
            .flatMap { try? JSONSerialization.jsonObject(with: $0) as? [String: Any] }) ?? [:]

        root["oauthAccount"] = identity.raw.mapValues(\.anyValue)
        // Sem esta chave o Claude Code trata o perfil como primeira execução e
        // abre o assistente de login — sem nem consultar o chaveiro. Identidade
        // escrita = conta já autenticada, então o onboarding está feito.
        root["hasCompletedOnboarding"] = true
        // O cache de uso pertence à conta anterior; deixá-lo carimbaria a nova
        // com números que não são dela até o Claude Code sobrescrever.
        root.removeValue(forKey: "cachedUsageUtilization")

        let data = try JSONSerialization.data(withJSONObject: root,
                                              options: [.prettyPrinted, .sortedKeys])
        try writeFile(data, url)
    }

    public func launchCommand() -> (executable: String, arguments: [String]) {
        ("claude", [])
    }
}
