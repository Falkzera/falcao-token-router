import Foundation

/// O que o Claude Code entrega à status line (o JSON do esquema da doc oficial)
/// e o que a linha tira dele (≙ `session.rs` do porte Windows).
///
/// Separado do desenho de propósito: a CLI monta a `View` da sessão viva com
/// isto, e a prévia dos Ajustes montará a mesma `View` com a sessão de exemplo —
/// o mesmo caminho, então a prévia nunca discorda do que a sessão mostra.
public enum StatusLineSession {
    /// `(fração 0–1, reset)` de uma janela do `rate_limits`. `used_percentage`
    /// pode vir inteiro ou decimal; `resets_at` é epoch em segundos.
    public static func window(_ limits: [String: Any]?, _ key: String) -> (fraction: Double?, resets: Date?) {
        guard let w = limits?[key] as? [String: Any] else { return (nil, nil) }
        return ((w["used_percentage"] as? Double).map { $0 / 100 },
                (w["resets_at"] as? Double).map { Date(timeIntervalSince1970: $0) })
    }

    /// Quem a linha nomeia: o grupo dono deste perfil, comparando o caminho
    /// normalizado. Fora de um grupo, a conta — ou, sem conta, a pasta.
    public static func label(config: RouterConfig?, dir: ConfigDir, email: String?) -> StatusLineView.Label {
        let mine = URL(fileURLWithPath: dir.raw).standardizedFileURL.path
        if let owner = config?.groups.enumerated().first(where: {
            URL(fileURLWithPath: $0.element.configDir.raw).standardizedFileURL.path == mine
        }) {
            return .group(name: owner.element.name, index: owner.offset)
        }
        if let email { return .account(String(email.prefix(while: { $0 != "@" }))) }
        return .account(URL(fileURLWithPath: dir.raw).lastPathComponent)
    }

    /// A `View` a partir do JSON. O que não veio fica `nil` — e some da linha.
    public static func view(from input: [String: Any], dir: ConfigDir,
                            config: RouterConfig?, email: String?,
                            home: String = NSHomeDirectory(),
                            currentDirectory: String? = FileManager.default.currentDirectoryPath) -> StatusLineView {
        func text(_ path: [String]) -> String? {
            var node: Any? = input
            for key in path { node = (node as? [String: Any])?[key] }
            return (node as? String).flatMap { $0.isEmpty ? nil : $0 }
        }
        func number(_ dict: [String: Any]?, _ key: String) -> Double? { dict?[key] as? Double }

        let limits = input["rate_limits"] as? [String: Any]
        func windowOf(_ key: String) -> StatusLineView.Window? {
            let (fraction, resets) = window(limits, key)
            return fraction.map { StatusLineView.Window(fraction: $0, resetsAt: resets) }
        }

        // A pasta da sessão; sem ela (o prazo do stdin venceu), a do processo,
        // que o Claude Code abre na pasta da sessão.
        let cwd = text(["workspace", "current_dir"])
            ?? text(["workspace", "project_dir"])
            ?? text(["cwd"])
            ?? currentDirectory

        let contextWindow = input["context_window"] as? [String: Any]
        let context: StatusLineView.Context? = {
            guard let size = number(contextWindow, "context_window_size"), size > 0 else { return nil }
            return StatusLineView.Context(
                usedPercent: number(contextWindow, "used_percentage") ?? 0,
                inputTokens: UInt64(number(contextWindow, "total_input_tokens") ?? 0),
                windowSize: UInt64(size))
        }()

        return StatusLineView(
            label: label(config: config, dir: dir, email: email),
            model: text(["model", "display_name"]) ?? text(["model", "id"]),
            modelID: text(["model", "id"]),
            effort: text(["effort", "level"]),
            place: cwd.map { StatusLineSource.gitBranch(at: $0) ?? StatusLineFormat.shortenPath($0, home: home) },
            context: context,
            fiveHour: windowOf("five_hour"),
            sevenDay: windowOf("seven_day"),
            costUSD: number(input["cost"] as? [String: Any], "total_cost_usd"),
            email: email)
    }

    /// A sessão de EXEMPLO: a prévia dos Ajustes e o `doctor` desenham com ela.
    /// Os resets ficam no futuro dentro das janelas, para o "quando" fazer
    /// sentido em qualquer hora do dia em que alguém olhe a prévia.
    public static func sample(now: Date = Date()) -> StatusLineView {
        StatusLineView(
            label: .group(name: "trabalho", index: 0),
            model: "Opus 5 (1M context)", modelID: "claude-opus-5",
            effort: "high", place: "main",
            context: StatusLineView.Context(usedPercent: 34, inputTokens: 68_000, windowSize: 200_000),
            fiveHour: StatusLineView.Window(fraction: 0.42, resetsAt: now.addingTimeInterval(2 * 3600)),
            sevenDay: StatusLineView.Window(fraction: 0.38, resetsAt: now.addingTimeInterval(3 * 86_400)),
            costUSD: 2.81, email: "conta1@exemplo.com")
    }
}
