import Foundation

/// Um provedor de IA cujo CLI o app sabe rotacionar.
///
/// A v1 traz um só — `.anthropic`, para o Claude Code. O tipo existe agora, e
/// não quando o segundo chegar, porque ele decide o formato do arquivo de
/// configuração: uma conta e um grupo carregam o provedor a que pertencem, e um
/// grupo só rotaciona contas do seu próprio provedor. Acrescentar OpenAI ou
/// Gemini depois é somar um caso aqui e uma implementação de `ProviderAdapter`,
/// não mexer no modelo.
public enum Provider: String, Sendable, Codable, CaseIterable, Identifiable {
    case anthropic

    public var id: String { rawValue }

    /// Nome que aparece na UI. Não traduzível: é marca.
    public var displayName: String {
        switch self {
        case .anthropic: "Claude"
        }
    }
}

/// O que o motor precisa saber de um provedor para rotacionar suas contas, sem
/// conhecer nada específico dele.
///
/// Toda a parte que é do Claude Code — o nome do item de chaveiro derivado do
/// diretório, o `.claude.json` com a identidade ao lado, a leitura de uso — vive
/// atrás disto. O `RotationEngine` fala só com o protocolo; trocar de provedor é
/// trocar a implementação, não o motor.
public protocol ProviderAdapter: Sendable {
    var provider: Provider { get }

    /// O item de chaveiro que serve as credenciais de um perfil.
    ///
    /// Para o Claude Code é `Claude Code-credentials` no perfil padrão e
    /// `Claude Code-credentials-<sha256(dir)[:8]>` em qualquer outro — a
    /// assimetria não é detalhe, é o que faz o `claude` sem argumento continuar
    /// funcionando no grupo padrão.
    func keychainService(forConfigDir dir: ConfigDir) -> String

    /// A conta com que um perfil está logado, lida do disco (nunca da rede).
    /// `nil` quando o perfil não tem identidade legível.
    func identity(inConfigDir dir: ConfigDir) -> AccountIdentity?

    /// Grava, no perfil de destino, a identidade da conta que está sendo
    /// ativada — para a tela do provedor mostrar a conta certa e o uso não vir
    /// carimbado da conta anterior.
    func writeIdentity(_ identity: AccountIdentity, toConfigDir dir: ConfigDir) throws

    /// O comando e os argumentos para iniciar uma sessão num grupo. O ambiente
    /// (incluindo o `CLAUDE_CONFIG_DIR`, quando o grupo não é o padrão) é
    /// montado por quem chama.
    func launchCommand() -> (executable: String, arguments: [String])
}
