import Foundation

/// Ajustes de ambiente para falar com o provedor **direto**, sem intermediário.
///
/// Existe por um motivo concreto: a máquina pode ter um proxy (o `teamclaude`,
/// que este produto substitui) exportado globalmente — `HTTPS_PROXY`,
/// `NODE_EXTRA_CA_CERTS`, etc. Sob esse proxy, `claude auth login` e as sessões
/// de grupo passariam pelo intermediário, que **fixa uma conta** — e aí todo
/// login vira a mesma conta e toda sessão é servida pela conta errada.
///
/// O login e o lançamento oficiais têm de ignorar o proxy. Este helper remove
/// exatamente as variáveis que o desviariam.
public enum ProviderEnv {
    /// Variáveis que redirecionam requisições por um proxy. Removidas antes de
    /// logar ou lançar uma sessão de grupo.
    public static let proxyKeys = [
        "HTTP_PROXY", "HTTPS_PROXY", "ALL_PROXY",
        "http_proxy", "https_proxy", "all_proxy",
        "NODE_EXTRA_CA_CERTS",
    ]

    /// Variáveis que trocam **quem atende** a sessão — credencial ou endpoint.
    ///
    /// O proxy não é a única forma de furar o rodízio, e as outras são piores
    /// porque não parecem com um proxy:
    ///
    /// - `ANTHROPIC_API_KEY` / `ANTHROPIC_AUTH_TOKEN` fazem o Claude Code servir
    ///   a sessão por **chave de API**, não pela conta OAuth do grupo. O grupo
    ///   ativa a conta certa, a sessão não usa nenhuma delas, e o gasto vai para
    ///   a chave. Pior: o sensor lê o `rate_limits` dessa sessão e grava a
    ///   amostra sob o e-mail do `.claude.json` do perfil — carimbando na conta
    ///   do grupo um consumo que não é dela, e o rodízio passa a decidir com
    ///   número falso.
    /// - `ANTHROPIC_BASE_URL` e as variantes Bedrock/Vertex mandam a sessão para
    ///   outro endpoint, que é exatamente a falha do `teamclaude` (um
    ///   intermediário que fixa uma conta) com outro nome.
    /// - `ANTHROPIC_CUSTOM_HEADERS` injeta cabeçalho arbitrário, inclusive de
    ///   autorização.
    ///
    /// Removidas no login e no lançamento: nos dois casos a premissa do produto
    /// é que quem atende é a conta OAuth que o grupo ativou, e nada além dela.
    ///
    /// O segundo bloco veio do porte para Windows (issue #5, @viniventur), que
    /// leu o JavaScript empacotado do Claude Code 2.1.280; cada nome foi
    /// conferido aqui com `strings` no binário do macOS antes de entrar:
    ///
    /// - `CLAUDE_CODE_OAUTH_TOKEN` (e as variantes por descritor de arquivo,
    ///   refresh e sessão) entregam um token pelo AMBIENTE. Com uma delas
    ///   setada, a sessão é servida por esse token, não pela conta que o grupo
    ///   ativou — o rodízio vira teatro.
    /// - `CLAUDE_SECURESTORAGE_CONFIG_DIR` **tem precedência** sobre
    ///   `CLAUDE_CONFIG_DIR` quando o Claude Code procura a credencial. Setada,
    ///   o app escreveria o item do grupo enquanto o Claude Code lê outro.
    /// - `CLAUDE_CODE_CUSTOM_OAUTH_URL` e `ANTHROPIC_PROFILE` trocam o servidor
    ///   de autenticação e o perfil de conta, respectivamente.
    public static let credentialKeys = [
        "ANTHROPIC_API_KEY", "ANTHROPIC_AUTH_TOKEN", "ANTHROPIC_CUSTOM_HEADERS",
        "ANTHROPIC_BASE_URL", "ANTHROPIC_BEDROCK_BASE_URL", "ANTHROPIC_VERTEX_BASE_URL",
        "CLAUDE_CODE_USE_BEDROCK", "CLAUDE_CODE_USE_VERTEX",
        "CLAUDE_CODE_OAUTH_TOKEN", "CLAUDE_CODE_OAUTH_TOKEN_FILE_DESCRIPTOR",
        "CLAUDE_CODE_OAUTH_REFRESH_TOKEN", "CLAUDE_CODE_API_KEY_FILE_DESCRIPTOR",
        "CLAUDE_CODE_SESSION_ACCESS_TOKEN", "CLAUDE_CODE_CUSTOM_OAUTH_URL",
        "CLAUDE_SECURESTORAGE_CONFIG_DIR", "ANTHROPIC_PROFILE",
    ]

    /// O ambiente sem nada que desvie a sessão da conta do grupo.
    public static func direct(_ environment: [String: String]) -> [String: String] {
        var env = environment
        for key in proxyKeys + credentialKeys { env.removeValue(forKey: key) }
        return env
    }
}
