import Foundation

/// A escolha do usuário sobre a status line dos grupos (≙ `choice.rs` do porte
/// Windows). Os Ajustes gravam; o `router statusline` lê a CADA render.
///
/// Mora em `<base>/statusline.json`, a pasta que o app e a CLI já dividem — e
/// **não** no `UserDefaults`: a CLI roda fora do app e não o lê. Também não vai
/// no `config.json`, que é o formato combinado com o Windows.
///
/// A leitura é **tolerante**: arquivo ausente, ilegível ou de outro formato vale
/// a de fábrica, e item desconhecido é ignorado sem levar o resto. A status line
/// nunca pode falhar por causa da escolha — ela é o sensor, e sem ela não há
/// medição.
public struct StatusLineChoice: Equatable, Sendable {
    /// Um item da linha, na ordem em que ela os desenha.
    public enum Item: String, CaseIterable, Sendable {
        case group, model, effort, place, context
        case fiveHour, sevenDay, resets, cost, email
    }

    public enum Mode: String, Sendable {
        /// A linha do app, com os itens escolhidos.
        case app
        /// O comando do usuário, depois do sensor. (Chega no PR seguinte; o
        /// formato já o carrega para não precisar migrar arquivo depois.)
        case command
    }

    public var mode: Mode = .app

    /// Os itens TIRADOS, não os que aparecem. É de propósito: um item novo numa
    /// versão futura aparece para todos, como "a completa menos o que eu tirei".
    /// Guardar os mostrados esconderia toda novidade de quem já tem o arquivo.
    private(set) var hidden: [Item] = []

    /// Guardado também no modo `app`: voltar ao modo comando o traz de volta.
    public var command: String = ""

    public init() {}

    public func shows(_ item: Item) -> Bool { !hidden.contains(item) }

    public mutating func setShown(_ item: Item, _ shown: Bool) {
        hidden.removeAll { $0 == item }
        guard !shown else { return }
        hidden.append(item)
        // Na ordem da linha, para o arquivo ser legível e o diff estável.
        hidden.sort { a, b in
            (Item.allCases.firstIndex(of: a) ?? 0) < (Item.allCases.firstIndex(of: b) ?? 0)
        }
    }

    public var hiddenItems: [Item] { hidden }

    /// O que esta escolha mostra: o que foi tirado sai da `View` ANTES do
    /// `render` — o mesmo recorte na sessão e na prévia dos Ajustes.
    public func apply(_ view: StatusLineView) -> StatusLineView {
        var view = view
        for item in hidden {
            switch item {
            case .group:    view.label = nil
            case .model:    view.model = nil
            case .effort:   view.effort = nil
            case .place:    view.place = nil
            case .context:  view.context = nil
            case .fiveHour: view.fiveHour = nil
            case .sevenDay: view.sevenDay = nil
            case .cost:     view.costUSD = nil
            case .email:    view.email = nil
            case .resets:
                // O "quando" vale para as duas janelas: tirar um reset e deixar
                // o outro seria uma linha que mente sobre qual delas reseta.
                view.fiveHour = view.fiveHour.map { .init(fraction: $0.fraction, resetsAt: nil) }
                view.sevenDay = view.sevenDay.map { .init(fraction: $0.fraction, resetsAt: nil) }
            }
        }
        return view
    }

    /// O comando a rodar depois do sensor: só no modo comando, e só se não
    /// estiver em branco — em branco, vale a linha do app.
    public var commandToRun: String? {
        guard mode == .command else { return nil }
        let trimmed = command.trimmingCharacters(in: .whitespacesAndNewlines)
        return trimmed.isEmpty ? nil : trimmed
    }
}

// MARK: - Disco

extension StatusLineChoice {
    public static func fileURL(base: URL) -> URL {
        base.appending(path: "statusline").appendingPathExtension("json")
    }

    /// A escolha gravada, ou a de fábrica (a completa) se não houver uma que se
    /// leia. Nunca lança: a linha tem de sair de qualquer jeito.
    public static func load(from url: URL,
                            readFile: (URL) -> Data? = { try? Data(contentsOf: $0) }) -> StatusLineChoice {
        guard let data = readFile(url),
              let root = try? JSONSerialization.jsonObject(with: data) as? [String: Any]
        else { return StatusLineChoice() }

        var choice = StatusLineChoice()
        if let mode = root["mode"] as? String, let parsed = Mode(rawValue: mode) {
            choice.mode = parsed
        }
        choice.command = root["command"] as? String ?? ""
        for key in root["hidden"] as? [String] ?? [] {
            // Chave desconhecida (versão futura, arquivo editado à mão) é
            // ignorada sem levar o resto da escolha junto.
            if let item = Item(rawValue: key) { choice.setShown(item, false) }
        }
        return choice
    }

    /// Grava atômico: a CLI lê a cada render e não pode pegar o arquivo pela
    /// metade.
    public func save(to url: URL) throws {
        let root: [String: Any] = [
            "mode": mode.rawValue,
            "hidden": hidden.map(\.rawValue),
            "command": command,
        ]
        let data = try JSONSerialization.data(withJSONObject: root,
                                              options: [.prettyPrinted, .sortedKeys])
        try FileManager.default.createDirectory(at: url.deletingLastPathComponent(),
                                                withIntermediateDirectories: true)
        try data.write(to: url, options: .atomic)
    }
}
