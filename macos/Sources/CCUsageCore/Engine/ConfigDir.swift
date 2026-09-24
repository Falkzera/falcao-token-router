import Foundation
import CryptoKit

/// Um diretório de configuração do Claude Code — um perfil.
///
/// Guarda a **string crua** exportada como `CLAUDE_CONFIG_DIR`, porque é ela, e
/// não o caminho resolvido, que o Claude Code normaliza em NFC e hasheia no nome
/// do item de chaveiro. Resolver o caminho (tirar um `~`, um `./`, uma barra
/// final) mudaria o hash e apontaria para um item que não existe. Confirmado
/// contra os itens reais no chaveiro em agosto/2026.
public struct ConfigDir: Sendable, Equatable, Hashable, Codable {
    /// A string exatamente como vai para o ambiente. Para o perfil padrão é o
    /// caminho de `~/.claude`; para os outros, o caminho do diretório do grupo.
    public let raw: String

    /// `true` quando é `~/.claude` — o perfil que o `claude` usa sem nenhuma
    /// variável de ambiente.
    ///
    /// Não é cosmético: setar `CLAUDE_CONFIG_DIR=~/.claude` **não** é o mesmo que
    /// não setar — com a variável presente a sessão sobe deslogada. Então o
    /// perfil padrão se distingue por não exportar variável nenhuma, e é isto que
    /// esta marca controla.
    public let isDefault: Bool

    public init(raw: String, isDefault: Bool) {
        self.raw = raw
        self.isDefault = isDefault
    }

    /// O perfil padrão, `~/.claude`. Nunca exporta `CLAUDE_CONFIG_DIR`.
    public static func standard(home: String = NSHomeDirectory()) -> ConfigDir {
        ConfigDir(raw: "\(home)/.claude", isDefault: true)
    }

    /// Um perfil dedicado, num caminho qualquer. Exporta `CLAUDE_CONFIG_DIR`.
    public static func dedicated(_ path: String) -> ConfigDir {
        ConfigDir(raw: path, isDefault: false)
    }

    /// A URL do diretório em si.
    public var url: URL { URL(fileURLWithPath: raw) }

    /// O `.claude.json` deste perfil.
    ///
    /// Assimetria confirmada na fonte: no perfil padrão ele mora **ao lado** do
    /// diretório (`~/.claude.json`, não `~/.claude/.claude.json`); num perfil com
    /// `CLAUDE_CONFIG_DIR` setado, mora **dentro** (`<dir>/.claude.json`).
    public var globalConfigURL: URL {
        isDefault
            ? url.deletingLastPathComponent().appending(path: ".claude.json")
            : url.appending(path: ".claude.json")
    }

    /// A variável de ambiente que uma sessão neste perfil precisa — ou `nil` no
    /// padrão, que não exporta nada.
    public var environmentValue: String? { isDefault ? nil : raw }

    /// O sufixo de 8 hex que o Claude Code deriva de um caminho de perfil.
    ///
    /// `sha256(NFC(raw))[:8]`. Só faz sentido para perfis não-padrão; o padrão
    /// usa o nome de item sem sufixo.
    public var keychainHash: String {
        let normalized = raw.precomposedStringWithCanonicalMapping  // NFC
        let digest = SHA256.hash(data: Data(normalized.utf8))
        return digest.prefix(4).map { String(format: "%02x", $0) }.joined()
    }
}
