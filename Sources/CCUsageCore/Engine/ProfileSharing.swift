import Foundation

/// Faz o perfil de um grupo **compartilhar** o histórico e as configurações do
/// usuário com o `~/.claude`, por symlink.
///
/// É o que torna `claude <grupo> --resume` capaz de listar as conversas que você
/// já tem — sem isto, cada grupo teria um histórico próprio e vazio, e uma
/// sessão começada fora do grupo sumiria. Também traz skills, comandos e agentes
/// para dentro do grupo, para a experiência ser a mesma.
///
/// Espelha o que o `cswap` faz (itens compartilhados por symlink), com uma
/// diferença deliberada: **o `settings.json` NÃO é compartilhado**, porque o
/// grupo precisa do seu próprio (com a status line do sensor). Compartilhá-lo
/// faria a status line vazar para o `~/.claude`.
public enum ProfileSharing {
    /// Seguem o usuário para o grupo sempre: o que define comportamento e
    /// ferramentas, e não é preso a uma conta.
    static let sharedItems = ["skills", "commands", "agents", "CLAUDE.md", "keybindings.json"]

    /// Histórico de conversa — ligado só quando o usuário quer histórico único
    /// (o padrão). É o que o `--resume` lê.
    static let historyItems = ["projects", "history.jsonl"]

    /// Cria os symlinks do `~/.claude` para dentro do perfil do grupo.
    ///
    /// Não faz nada para o grupo padrão (cujo perfil já É o `~/.claude` — linkar
    /// para si mesmo seria circular). Nunca sobrescreve o que já existe no grupo,
    /// e ignora um item que o `~/.claude` não tem.
    public static func link(into group: ConfigDir, home: ConfigDir = .standard(),
                            shareHistory: Bool) {
        guard !group.isDefault else { return }
        let fm = FileManager.default
        try? fm.createDirectory(at: group.url, withIntermediateDirectories: true)

        let items = sharedItems + (shareHistory ? historyItems : [])
        for name in items {
            let target = home.url.appending(path: name)
            guard fm.fileExists(atPath: target.path) else { continue }
            let link = group.url.appending(path: name)
            // Já existe algo (link ou dir real) — respeita, não mexe.
            if fm.fileExists(atPath: link.path) || isSymlink(link) { continue }
            try? fm.createSymbolicLink(at: link, withDestinationURL: target)
        }
    }

    private static func isSymlink(_ url: URL) -> Bool {
        (try? url.resourceValues(forKeys: [.isSymbolicLinkKey]))?.isSymbolicLink ?? false
    }
}
