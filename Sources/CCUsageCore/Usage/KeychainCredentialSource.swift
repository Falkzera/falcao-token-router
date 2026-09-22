import Foundation
import Security

/// Lê o item de keychain que o Claude Code mantém.
///
/// **É o único ponto do app que toca esse item.** O `PlanDetector` consumia o
/// keychain por conta própria; passou a consumir daqui quando os dois leitores
/// passaram a precisar do mesmo segredo.
///
/// O item pertence ao Claude Code, então a primeira leitura pode abrir o
/// diálogo de autorização do macOS. Negar faz a leitura falhar e o app degrada
/// — nunca trava.
///
/// `load` é injetável para os testes exercitarem a decodificação sem keychain.
public struct KeychainCredentialSource: CredentialSource {
    public static let service = "Claude Code-credentials"

    /// Quando `false`, o `accessToken` **não é extraído do dicionário**. Não é
    /// leitura seguida de descarte: o campo simplesmente não é acessado, que é
    /// o que a promessa do app afirma.
    private let readsAccessToken: Bool
    private let load: @Sendable () -> Data?

    public init(readsAccessToken: Bool,
                load: @escaping @Sendable () -> Data? = KeychainCredentialSource.loadFromKeychain) {
        self.readsAccessToken = readsAccessToken
        self.load = load
    }

    public func credentials() -> ClaudeCredentials? {
        guard let data = load(),
              let root = try? JSONSerialization.jsonObject(with: data) as? [String: Any],
              // Claude Code 2.1.x pode gravar só estado de MCP, sem esta chave.
              let oauth = root["claudeAiOauth"] as? [String: Any]
        else { return nil }

        return ClaudeCredentials(
            accessToken: readsAccessToken ? oauth["accessToken"] as? String : nil,
            // Milissegundos desde a época, como o Claude Code grava.
            expiresAt: (oauth["expiresAt"] as? Double)
                .map { Date(timeIntervalSince1970: $0 / 1000) },
            rateLimitTier: oauth["rateLimitTier"] as? String)
    }

    public static let loadFromKeychain: @Sendable () -> Data? = {
        let query: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: service,
            kSecReturnData as String: true,
            kSecMatchLimit as String: kSecMatchLimitOne,
        ]
        var item: CFTypeRef?
        guard SecItemCopyMatching(query as CFDictionary, &item) == errSecSuccess
        else { return nil }
        return item as? Data
    }
}
