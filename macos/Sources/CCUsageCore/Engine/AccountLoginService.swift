import Foundation

/// Observa o resultado do **fluxo oficial** de login da Anthropic.
///
/// Os termos exigem que o login termine no fluxo da própria Anthropic (ver as
/// citações no estudo de conformidade): nada de webview embutida, nada de
/// coletar senha. Então o app não faz login — a `LoginSession` (na camada de
/// app) roda o binário oficial num pty, num perfil isolado, e este serviço só
/// **lê o desfecho no disco**: identidade + credencial na casa da conta.
public struct AccountLoginService: Sendable {
    private let adapter: any ProviderAdapter

    public init(adapter: any ProviderAdapter = AnthropicAdapter(),
                paths: RouterPaths = RouterPaths()) {
        self.adapter = adapter
    }

    /// A identidade que apareceu num perfil depois do login, ou `nil` se ainda
    /// não terminou. O app chama isto num laço curto até obter resposta.
    public func loginResult(inHome home: ConfigDir, keychain: any KeychainStore) -> AccountIdentity? {
        // Só conta como pronto quando as DUAS coisas existem: a identidade no
        // `.claude.json` e a credencial no chaveiro. Uma sem a outra é login
        // pela metade.
        guard let identity = adapter.identity(inConfigDir: home),
              keychain.exists(service: adapter.keychainService(forConfigDir: home))
        else { return nil }
        return identity
    }
}
