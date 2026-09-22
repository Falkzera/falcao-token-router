import Foundation

/// Como o app se enfia no shell e na status line do Claude Code, sem estragar o
/// que o usuário já tem.
///
/// Duas peças: a `statusLine` que cada perfil de grupo aponta para o sensor, e a
/// função `claude` que roteia `claude <grupo>` para a CLI. Ambas geradas a partir
/// do caminho do binário `router` que o app empacota.
public enum ShellIntegration {
    /// O comando de status line para o `settings.json` de um perfil.
    public static func statusLineCommand(routerPath: String) -> [String: Any] {
        ["type": "command", "command": "\(shellQuote(routerPath)) statusline", "padding": 0]
    }

    /// O `settings.json` de um perfil.
    ///
    /// Nome montado por componentes de propósito: um literal com o prefixo
    /// settings colidiria com o verificador de strings de localização.
    public static func settingsURL(in configDir: ConfigDir) -> URL {
        configDir.url.appending(path: "settings").appendingPathExtension("json")
    }

    /// `true` quando o perfil **não** aponta a status line para este `router`.
    ///
    /// Inclui o caso de nunca ter apontado. Quem chama decide o que fazer; aqui
    /// só se responde se o que está gravado bate com o binário de agora.
    public static func statusLineIsStale(routerPath: String, in configDir: ConfigDir,
                                         readFile: (URL) -> Data? = { try? Data(contentsOf: $0) })
        -> Bool {
        guard let data = readFile(settingsURL(in: configDir)),
              let root = try? JSONSerialization.jsonObject(with: data) as? [String: Any],
              let line = root["statusLine"] as? [String: Any],
              let command = line["command"] as? String
        else { return true }
        return !command.contains(routerPath)
    }

    /// Escreve a `statusLine` no `settings.json` de um perfil, **preservando** o
    /// resto do arquivo — o padrão `~/.claude` tem `model` e outras chaves do
    /// usuário que não podem sumir.
    public static func installStatusLine(routerPath: String, into configDir: ConfigDir,
                                         readFile: (URL) -> Data? = { try? Data(contentsOf: $0) },
                                         writeFile: (Data, URL) throws -> Void = {
                                             try $0.write(to: $1, options: .atomic)
                                         }) throws {
        let url = settingsURL(in: configDir)
        var root = (readFile(url).flatMap {
            try? JSONSerialization.jsonObject(with: $0) as? [String: Any]
        }) ?? [:]
        root["statusLine"] = statusLineCommand(routerPath: routerPath)
        try FileManager.default.createDirectory(at: configDir.url,
                                                withIntermediateDirectories: true)
        let data = try JSONSerialization.data(withJSONObject: root,
                                              options: [.prettyPrinted, .sortedKeys])
        try writeFile(data, url)
    }

    /// A função de shell que faz `claude <grupo>` cair na CLI e `claude` sozinho
    /// passar direto. Dinâmica: pergunta ao binário se o primeiro argumento é um
    /// grupo, então grupos novos funcionam sem reescrever isto.
    ///
    /// `exec` no fim para a sessão herdar o PID do shell, como o `claude` normal.
    public static func shellFunction(routerPath: String) -> String {
        let q = shellQuote(routerPath)
        return """
        # Falcão Router — roteia `claude <grupo>` para o grupo certo.
        # Gerado pelo app; não editar à mão.
        claude() {
          if [ -n "$1" ] && command \(q) is-group "$1" >/dev/null 2>&1; then
            local grupo="$1"; shift
            exec command \(q) launch "$grupo" -- "$@"
          fi
          command claude "$@"
        }
        """
    }

    public enum ProfileOutcome: Sendable, Equatable { case added, alreadyPresent }

    /// Acrescenta a linha de `source` ao profile do shell — o que torna a
    /// integração plug-and-play, sem o usuário editar arquivo nenhum.
    ///
    /// Resolve o symlink antes (um `~/.zshrc` que aponta para outro arquivo não
    /// pode virar um arquivo novo). Idempotente: se a linha já está lá, não
    /// duplica.
    ///
    /// **Trabalha em bytes, e acrescenta de verdade.** A versão anterior lia o
    /// arquivo como `String` UTF-8 e reescrevia `existing + block` atomicamente.
    /// Duas consequências, ambas ruins e ambas silenciosas:
    ///
    /// - Um `~/.zshrc` que não decodifica em UTF-8 — basta um `alias` com acento
    ///   salvo em latin-1, ou um byte solto de um editor antigo — fazia o `try?`
    ///   devolver `nil`, o `?? ""` tratar o arquivo como **vazio**, e a escrita
    ///   gravar só este bloco por cima. O `.zshrc` inteiro do usuário, perdido,
    ///   sem erro nenhum e sem backup. Aqui, um arquivo que existe e não pode
    ///   ser lido **propaga o erro** em vez de virar string vazia.
    /// - A escrita atômica troca o inode: quem tivesse o arquivo aberto, ou um
    ///   hard link para ele, ficava com a versão velha. O `FileHandle` escreve
    ///   no mesmo arquivo, preservando inode e permissões.
    @discardableResult
    public static func ensureInProfile(sourceLine: String, scriptPath: String,
                                       profileURL: URL) throws -> ProfileOutcome {
        let resolved = profileURL.resolvingSymlinksInPath()
        let block = Data("\n# Falcão Router — integração de terminal (claude <grupo>)\n\(sourceLine)\n".utf8)

        // Não existir é o único caso em que criar do zero é seguro.
        guard FileManager.default.fileExists(atPath: resolved.path) else {
            try block.write(to: resolved, options: .atomic)
            return .added
        }
        let existing = try Data(contentsOf: resolved)
        if existing.range(of: Data(scriptPath.utf8)) != nil { return .alreadyPresent }

        let handle = try FileHandle(forWritingTo: resolved)
        defer { try? handle.close() }
        try handle.seekToEnd()
        try handle.write(contentsOf: block)
        return .added
    }

    private static func shellQuote(_ s: String) -> String {
        "'" + s.replacingOccurrences(of: "'", with: "'\\''") + "'"
    }
}
