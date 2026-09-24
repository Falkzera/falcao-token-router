import Foundation

/// O que a status line lê do ambiente: o branch do repositório e se o terminal
/// aceita cor de 24 bits (≙ `git_branch`/`truecolor` de `view.rs` no porte).
///
/// **Nada de processo por render.** O branch vem de ler o `.git/HEAD`, não de
/// rodar o `git`: a status line é desenhada a cada atualização da sessão, e um
/// processo por render é custo que o usuário paga sem ver.
public enum StatusLineSource {
    /// O branch, subindo das pastas até achar um `.git`. Worktree e submódulo:
    /// o `.git` é um ARQUIVO que aponta para o gitdir. HEAD destacado: os 7
    /// primeiros do hash, como o git mostra.
    public static func gitBranch(at path: String, ceiling: String? = nil,
                                 fileManager: FileManager = .default) -> String? {
        var current = URL(fileURLWithPath: path).standardizedFileURL
        // Teto de 40 níveis: um caminho patológico não vira laço infinito.
        for _ in 0..<40 {
            if let branch = head(in: current, fileManager: fileManager) { return branch }
            if let ceiling, current.path == URL(fileURLWithPath: ceiling).standardizedFileURL.path {
                return nil
            }
            let parent = current.deletingLastPathComponent()
            if parent.path == current.path { return nil }
            current = parent
        }
        return nil
    }

    private static func head(in dir: URL, fileManager: FileManager) -> String? {
        let dotGit = dir.appending(path: ".git")
        var isDirectory: ObjCBool = false
        guard fileManager.fileExists(atPath: dotGit.path, isDirectory: &isDirectory) else { return nil }

        let gitDir: URL
        if isDirectory.boolValue {
            gitDir = dotGit
        } else {
            guard let text = try? String(contentsOf: dotGit, encoding: .utf8),
                  let line = text.split(separator: "\n").first(where: { $0.contains("gitdir:") }),
                  let target = line.split(separator: ":", maxSplits: 1).last
            else { return nil }
            gitDir = dir.appending(path: target.trimmingCharacters(in: .whitespaces))
        }

        guard let head = try? String(contentsOf: gitDir.appending(path: "HEAD"), encoding: .utf8)
        else { return nil }
        let trimmed = head.trimmingCharacters(in: .whitespacesAndNewlines)
        if let ref = trimmed.split(separator: ":", maxSplits: 1).last,
           trimmed.hasPrefix("ref:") {
            let name = ref.trimmingCharacters(in: .whitespaces)
            if name.hasPrefix("refs/heads/") { return String(name.dropFirst("refs/heads/".count)) }
        }
        return String(trimmed.prefix(7))
    }

    /// O terminal aceita cor de 24 bits? Pelo ambiente que o Claude Code
    /// repassa. Sem isso a animação do `effort` sairia como lixo num terminal
    /// de 16 cores, então a resposta decide entre pintar e não pintar.
    public static func trueColor(_ env: (String) -> String? = { ProcessInfo.processInfo.environment[$0] }) -> Bool {
        if let colorterm = env("COLORTERM")?.lowercased(),
           colorterm.contains("truecolor") || colorterm.contains("24bit") { return true }
        // O Terminal.app NÃO entra: ele faz 256 cores, não 24 bits, e a
        // animação do `effort` sairia como lixo nele.
        if let program = env("TERM_PROGRAM"),
           ["vscode", "iTerm.app", "WezTerm", "ghostty"].contains(program) { return true }
        if let term = env("TERM")?.lowercased(),
           term.contains("direct") || term.contains("truecolor") { return true }
        return false
    }
}
