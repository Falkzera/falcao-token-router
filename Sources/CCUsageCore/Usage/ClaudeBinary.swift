import Foundation

/// Onde está o binário oficial `claude`.
///
/// Vive no core, e não na camada de app, porque tanto o app (para o login em
/// pty) quanto a CLI `router` (para a sonda de uso) precisam dele.
///
/// `which` não resolve: um app lançado pelo Finder herda um `PATH` de
/// `/usr/bin:/bin:/usr/sbin:/sbin`, e nenhum dos lugares onde o Claude Code se
/// instala está nele. Então a busca é por caminhos conhecidos, e o shell de
/// login é o último recurso.
public enum ClaudeBinary {
    /// Os lugares onde o Claude Code se instala, do instalador mais recente
    /// para o mais antigo.
    static let knownPaths = [
        ".local/bin/claude",      // o instalador nativo
        ".claude/local/claude",   // o layout de quem migrou do npm
        ".bun/bin/claude",
        ".volta/bin/claude",      // o diretório de shims do Volta
        "Library/pnpm/claude",    // o bin global do pnpm no macOS
        ".npm-global/bin/claude", // npm com prefixo de usuário
    ]

    /// Caminhos de sistema, relativos à raiz para um teste poder apontar a busca
    /// inteira para um diretório temporário.
    static let systemPaths = ["opt/homebrew/bin/claude", "usr/local/bin/claude"]

    /// Onde um gerenciador de versão do Node põe um `npm install -g`: um
    /// diretório nomeado pela versão, que nenhum caminho fixo consegue soletrar.
    ///
    /// Mais nova primeiro — atualizar o Node deixa toda árvore antiga no lugar,
    /// e só a atual é certamente a instalação em uso. Comparação `.numeric`
    /// porque os nomes são `v20.20.2` e `v22.22.3`: a ordem alfabética põe 20
    /// acima de 22.
    static func nodeVersionPaths(home: String, fileManager: FileManager) -> [String] {
        let versions = (home as NSString).appendingPathComponent(".nvm/versions/node")
        guard let names = try? fileManager.contentsOfDirectory(atPath: versions) else { return [] }
        return names
            .sorted { $0.compare($1, options: .numeric) == .orderedDescending }
            .map { (versions as NSString).appendingPathComponent("\($0)/bin/claude") }
    }

    /// A cópia que vive dentro do app de desktop do Claude não serve.
    ///
    /// Ela guarda o token no armazenamento próprio do app de desktop, não no
    /// item de chaveiro que este produto gerencia — perguntar o uso a ela
    /// responderia por outra conta, ou por nenhuma.
    static let desktopOwned = "/Library/Application Support/Claude/"

    /// O caminho do binário, ou `nil` se o Claude Code não está instalado em
    /// nenhum lugar conhecido.
    public static func locate(home: String = NSHomeDirectory(),
                              root: String = "/",
                              fileManager: FileManager = .default,
                              askLoginShell: Bool = true) -> String? {
        let candidates =
            knownPaths.map { (home as NSString).appendingPathComponent($0) }
            + nodeVersionPaths(home: home, fileManager: fileManager)
            + systemPaths.map { (root as NSString).appendingPathComponent($0) }

        for path in candidates where fileManager.isExecutableFile(atPath: path) {
            // Resolve o symlink antes de julgar: a entrada do Homebrew aponta
            // para o Caskroom, e a cópia do app de desktop poderia igualmente
            // estar linkada em algum lugar do PATH.
            let resolved = URL(fileURLWithPath: path).resolvingSymlinksInPath().path
            if resolved.contains(desktopOwned) { continue }
            return path
        }
        guard askLoginShell else { return nil }
        return fromLoginShell(fileManager: fileManager)
    }

    /// Último recurso: pergunta a um shell de login. Custa uns 100ms e só roda
    /// quando nenhum caminho conhecido existe.
    private static func fromLoginShell(fileManager: FileManager) -> String? {
        let probe = Process()
        probe.executableURL = URL(fileURLWithPath: "/bin/zsh")
        probe.arguments = ["-lc", "whence -p claude"]
        let pipe = Pipe()
        probe.standardOutput = pipe
        probe.standardError = FileHandle.nullDevice
        guard (try? probe.run()) != nil else { return nil }
        let out = pipe.fileHandleForReading.readDataToEndOfFile()
        probe.waitUntilExit()
        let path = String(decoding: out, as: UTF8.self)
            .trimmingCharacters(in: .whitespacesAndNewlines)
        return fileManager.isExecutableFile(atPath: path) ? path : nil
    }
}
