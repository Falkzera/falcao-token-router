import Foundation

/// O que o app precisa saber das credenciais que o Claude Code mantém.
///
/// **Não existe campo para `refreshToken`, e isso é o mecanismo, não um
/// esquecimento.** O app nunca renova OAuth: regravar o item de keychain do
/// Claude Code pode invalidar a sessão dele, e manter um token próprio em
/// paralelo dobra a superfície de credencial. Sem campo onde guardar, não há
/// caminho de código que possa usá-lo.
public struct ClaudeCredentials: Sendable, Equatable {
    /// `nil` quando o usuário não autorizou a leitura do token, ou quando o
    /// item não o contém. O restante dos campos continua disponível: detectar
    /// o plano nunca exigiu credencial.
    public let accessToken: String?
    public let expiresAt: Date?
    public let rateLimitTier: String?

    public init(accessToken: String?, expiresAt: Date?, rateLimitTier: String?) {
        self.accessToken = accessToken
        self.expiresAt = expiresAt
        self.rateLimitTier = rateLimitTier
    }

    /// O token, se houver um e ele ainda valer. Vida útil medida em horas, então
    /// conferir aqui evita gastar uma chamada de rede que só pode voltar 401.
    public func usableToken(at now: Date) -> String? {
        guard let accessToken else { return nil }
        if let expiresAt, expiresAt <= now { return nil }
        return accessToken
    }
}

public protocol CredentialSource: Sendable {
    func credentials() -> ClaudeCredentials?
}
