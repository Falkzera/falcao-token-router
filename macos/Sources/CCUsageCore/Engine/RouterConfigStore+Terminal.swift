import Foundation

/// O que o store escreve fora da própria pasta para o terminal funcionar: a
/// função de shell, a linha no `~/.zshrc`, a status line de cada perfil de
/// grupo — e a escolha do que essa linha mostra. Arquivo à parte pela régua
/// de 600 linhas.
extension RouterConfigStore {
    // MARK: - Integração com o terminal

    /// O arquivo de shell que o usuário passa a dar `source`.
    public var shellScriptURL: URL { paths.base.appending(path: "shell.sh") }

    /// A linha que ativa a integração no shell (adicionada automaticamente).
    public var shellSourceLine: String { "source \"\(shellScriptURL.path)\"" }

    /// O profile do shell onde a linha é adicionada (zsh é o padrão do macOS).
    public var profileURL: URL {
        FileManager.default.homeDirectoryForCurrentUser.appending(path: ".zshrc")
    }

    /// `true` quando a linha da integração já está no `~/.zshrc`.
    public var profileHasIntegration: Bool {
        let resolved = profileURL.resolvingSymlinksInPath()
        guard let content = try? String(contentsOf: resolved, encoding: .utf8) else { return false }
        return content.contains(shellScriptURL.path)
    }

    /// `true` quando a integração já foi escrita (script + linha no profile).
    public var shellIntegrationInstalled: Bool {
        FileManager.default.fileExists(atPath: shellScriptURL.path) && profileHasIntegration
    }

    /// Escreve a função de shell e planta a status line do sensor em cada perfil
    /// de grupo. Idempotente. Devolve a linha de `source` para a UI mostrar, ou
    /// `nil` se o caminho do binário não é conhecido.
    @discardableResult
    public func installShellIntegration() -> String? {
        guard let routerPath else {
            lastError = "não foi possível localizar o binário do app"
            return nil
        }
        do {
            try FileManager.default.createDirectory(
                at: paths.base, withIntermediateDirectories: true)
            try ShellIntegration.shellFunction(routerPath: routerPath)
                .write(to: shellScriptURL, atomically: true, encoding: .utf8)
            // Planta a status line em cada perfil de grupo, preservando o
            // settings.json que já houver (o padrão ~/.claude tem `model` etc.),
            // e compartilha histórico + skills para o `--resume` funcionar.
            for group in config.groups {
                try? ShellIntegration.installStatusLine(
                    routerPath: routerPath, into: group.configDir)
                ProfileSharing.link(into: group.configDir, shareHistory: config.shareHistory)
            }
            // Adiciona a linha ao ~/.zshrc automaticamente — é o que torna isto
            // plug-and-play: o usuário não edita arquivo nenhum.
            try? ShellIntegration.ensureInProfile(
                sourceLine: shellSourceLine, scriptPath: shellScriptURL.path,
                profileURL: profileURL)
            lastError = nil
            integrationInstalled = shellIntegrationInstalled
            return shellSourceLine
        } catch {
            lastError = "não foi possível instalar a integração: \(error)"
            integrationInstalled = shellIntegrationInstalled
            return nil
        }
    }

    /// A integração instalada aponta para um binário que não é mais este.
    ///
    /// O `shell.sh` e a `statusLine` de cada perfil guardam o caminho
    /// **absoluto** do `router`, e o `router` mora dentro do `.app`. Mover,
    /// renomear ou reinstalar o app noutro lugar deixa os dois apontando para um
    /// caminho que não existe — e o modo de falha é silencioso, que é o que o
    /// torna grave: a função de shell não acha o binário, o `is-group` falha, e
    /// `claude trabalho` cai no `command claude` do final, abrindo a sessão no
    /// `~/.claude`, **na conta errada**, sem nenhum aviso.
    public var integrationIsStale: Bool {
        guard let routerPath,
              FileManager.default.fileExists(atPath: shellScriptURL.path)
        else { return false }  // não instalada não é obsoleta; é ausente.
        let script = (try? String(contentsOf: shellScriptURL, encoding: .utf8)) ?? ""
        if !script.contains(routerPath) { return true }
        return config.groups.contains {
            ShellIntegration.statusLineIsStale(routerPath: routerPath, in: $0.configDir)
        }
    }

    /// Reescreve a integração quando ela ficou obsoleta. Devolve `true` se
    /// mexeu em alguma coisa.
    ///
    /// Automático, e não um botão, justamente porque a falha é silenciosa: quem
    /// foi atingido é quem não tem como saber que precisa apertá-lo. Roda na
    /// subida do app, depois que o `routerPath` é conhecido.
    @discardableResult
    public func healShellIntegration() -> Bool {
        guard integrationIsStale else { return false }
        return installShellIntegration() != nil
    }

    // MARK: - A escolha da status line

    /// Liga ou desliga um item da linha. Grava na hora: a CLI relê a escolha a
    /// cada render, então o efeito aparece na próxima atualização da sessão,
    /// sem reabrir nada e sem reinstalar a integração.
    public func setStatusLineItem(_ item: StatusLineChoice.Item, shown: Bool) {
        statusLineChoice.setShown(item, shown)
        saveStatusLineChoice()
    }

    /// Volta à linha completa.
    public func restoreStatusLine() {
        statusLineChoice = StatusLineChoice()
        saveStatusLineChoice()
    }

    private func saveStatusLineChoice() {
        do {
            try statusLineChoice.save(to: StatusLineChoice.fileURL(base: paths.base))
            lastError = nil
        } catch {
            lastError = "não foi possível salvar a escolha da status line: \(error)"
        }
    }
}
