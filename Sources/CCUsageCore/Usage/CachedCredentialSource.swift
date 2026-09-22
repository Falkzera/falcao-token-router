import Foundation

/// Guarda o resultado de uma leitura de credencial, para não reler o keychain a
/// cada ciclo.
///
/// **Não é otimização.** Cada leitura do item `Claude Code-credentials` pode
/// abrir o diálogo de autorização do macOS, porque o item pertence a outro app.
/// Sem cache, a busca ao vivo relia a cada 5 minutos e a cada abertura do
/// painel — e quem tivesse clicado "Permitir" em vez de "Sempre Permitir"
/// recebia um pedido de senha de cinco em cinco minutos.
///
/// O cache guarda **o resultado, inclusive a falha**. Retentar uma leitura que
/// falhou — item ausente, ou autorização negada — recria exatamente o problema
/// que este tipo existe para resolver. A saída é explícita: `invalidate()`.
public final class CachedCredentialSource: CredentialSource, @unchecked Sendable {
    /// `@unchecked` porque o estado mutável é protegido pelo lock abaixo, e não
    /// há caminho que o alcance por fora dele.
    private let lock = NSLock()
    private let wrapped: any CredentialSource
    private let now: @Sendable () -> Date

    /// Duplamente opcional de propósito: `nil` é "nunca li", `.some(nil)` é
    /// "li e não havia nada". Os dois se comportam igual na chamada, mas só o
    /// segundo pode ficar em cache.
    private var cached: ClaudeCredentials??

    public init(wrapping source: any CredentialSource,
                now: @escaping @Sendable () -> Date = Date.init) {
        self.wrapped = source
        self.now = now
    }

    public func credentials() -> ClaudeCredentials? {
        lock.lock()
        defer { lock.unlock() }

        if let cached, !isExpired(cached) { return cached }

        let fresh = wrapped.credentials()
        cached = .some(fresh)
        return fresh
    }

    /// Descarta o cache. Chamado quando o servidor recusa o token: um 401 diz
    /// que o cache está errado mesmo com o `expiresAt` ainda no futuro.
    public func invalidate() {
        lock.lock()
        defer { lock.unlock() }
        cached = nil
    }

    /// Sem `expiresAt` não há quando expirar. Reler por precaução voltaria a
    /// pedir autorização sem que nada tenha indicado necessidade.
    private func isExpired(_ credentials: ClaudeCredentials?) -> Bool {
        guard let expiresAt = credentials?.expiresAt else { return false }
        return expiresAt <= now()
    }
}
