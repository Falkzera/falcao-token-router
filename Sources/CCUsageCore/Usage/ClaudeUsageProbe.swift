import Foundation

/// A sonda ativa: pergunta o uso ao **binário oficial**, não ao endpoint.
///
/// Existe por causa da lacuna que o sensor passivo não cobre. A status line
/// entrega `rate_limits` com as janelas de 5h e 7 dias, e **só elas** — o limite
/// POR MODELO não vem ali. E é o por modelo que estoura primeiro: uma conta
/// travou com `5h 91%` / `7d 79%` (folga aparente) e `Fable: 100%`. Qualquer
/// rotação que ignore esse número decide com o dado errado.
///
/// `claude "/usage"` imprime as três linhas, inclusive a do modelo:
///
///     Current session: 2% used · resets Sep 18 at 7:29pm (America/Maceio)
///     Current week (all models): 26% used · resets Sep 21 at 8:59am (America/Maceio)
///     Current week (Fable): 0% used · resets Sep 21 at 9am (America/Maceio)
///
/// **Conformidade.** Quem faz a requisição é o cliente oficial, com a
/// credencial dele — é o mesmo que acontece quando o usuário digita `/usage`.
/// Este app continua sem tocar em `api.anthropic.com` e sem ler token nenhum.
///
/// **Custo.** Cada sonda é um processo Node subindo do zero, alguns segundos, e
/// uma requisição de verdade na conta. Por isso ela **não** entra no laço de
/// rotação de 3 minutos: é sob demanda (`router measure`) ou em cadência baixa.
/// O sensor passivo continua sendo a fonte de todo minuto.
public struct ClaudeUsageProbe: Sendable {
    /// Uma janela por modelo, como o `/usage` a nomeia.
    public struct ModelWindow: Sendable, Equatable, Codable {
        /// O nome de exibição que o `/usage` imprime (`Fable`, `Opus`…). Cru, e
        /// não mapeado para um enum: a lista de modelos muda sem avisar, e um
        /// nome que este app não conhece ainda é um limite que estoura.
        public let name: String
        /// 0–1.
        public let percent: Double
        public let resetsAt: Date?

        public init(name: String, percent: Double, resetsAt: Date?) {
            self.name = name
            self.percent = percent
            self.resetsAt = resetsAt
        }
    }

    /// O que a sonda leu de uma conta.
    public struct Reading: Sendable, Equatable {
        public let session: Double?
        public let sessionResetsAt: Date?
        public let weeklyAll: Double?
        public let weeklyAllResetsAt: Date?
        public let models: [ModelWindow]
        /// A linha de plano que o `/usage` imprime no topo, copiada como veio.
        public let plan: String?
    }

    public enum ProbeError: Error, Equatable {
        /// O Claude Code não está instalado em nenhum lugar conhecido.
        case notInstalled
        /// O binário saiu com código diferente de zero — na prática, perfil sem
        /// login. Não é erro para mostrar: quem chama segue para a próxima conta.
        case notSignedIn
        /// Rodou, respondeu, e não havia nenhuma linha `Current …` na saída.
        /// Formato novo: é isto que a próxima versão precisa suportar.
        case unrecognized
        /// Estourou o prazo. Um Node travado não pode segurar a medição inteira.
        case timedOut
    }

    /// Tempo de sobra para um Node frio numa máquina ocupada, curto o bastante
    /// para uma sonda travada não segurar as outras.
    public static let timeout: TimeInterval = 45

    /// Modo `--print`, sem transcript, sem MCP. Cada bandeira paga o seu lugar:
    ///
    /// - `--print` evita o modo interativo e o diálogo de confiança que o Claude
    ///   Code levanta para um diretório que nunca viu.
    /// - `--no-session-persistence` (só existe em modo print) evita gravar um
    ///   transcript por sondagem — sem ele, cada medição deixa uma sessão em
    ///   `<perfil>/projects/`, para sempre.
    /// - `--strict-mcp-config` **sem** nenhum `--mcp-config` significa nenhum
    ///   servidor MCP. Sem ela, toda sondagem sobe o que o usuário tiver
    ///   configurado — numa máquina cheia, uma dúzia de processos Node e as
    ///   conexões deles, nenhuma das quais o `/usage` precisa.
    ///
    /// A telemetria fica **ligada de propósito**: `DISABLE_TELEMETRY` e
    /// `CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC` também desligam a consulta de
    /// feature flags, e a linha semanal por modelo está atrás de um desses
    /// portões — com qualquer um deles setado, `/usage` para de imprimi-la, que
    /// é justamente a linha pela qual esta sonda existe.
    public static let arguments = [
        "--print", "--no-session-persistence", "--strict-mcp-config", "/usage",
    ]

    /// Onde o `/usage` roda: um diretório só, mantido pela vida da instalação.
    ///
    /// O Claude Code indexa a pasta de transcript pelo diretório de trabalho. Um
    /// diretório temporário por chamada deixaria uma pasta de projeto nova e
    /// nunca revisitada a cada medição.
    public static func scratchDirectory(base: URL) throws -> URL {
        let dir = base.appending(path: "probe-scratch")
        try FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
        return dir
    }

    /// Como a saída do binário é obtida. Injetável: um teste que realmente
    /// subisse o Claude Code precisaria de login e rede para ser determinístico,
    /// e o que vale testar — o que o texto significa — está depois disto.
    private let output: @Sendable (ConfigDir) throws -> String

    public init(output: @escaping @Sendable (ConfigDir) throws -> String) {
        self.output = output
    }

    /// A sonda de verdade, sobre o binário instalado.
    public static func system(binary: String? = ClaudeBinary.locate(),
                              scratchBase: URL = RouterPaths().base) -> ClaudeUsageProbe? {
        guard let binary else { return nil }
        return ClaudeUsageProbe { dir in
            try run(binary: binary, configDir: dir, scratchBase: scratchBase)
        }
    }

    /// Mede um perfil. `now` entra por parâmetro porque a escolha do ano da data
    /// de reset depende dele.
    public func read(configDir: ConfigDir, now: Date = Date()) throws -> Reading {
        try Self.parse(output(configDir), now: now)
    }

    // MARK: - Rodar

    private static func run(binary: String, configDir: ConfigDir,
                            scratchBase: URL) throws -> String {
        let scratch = try scratchDirectory(base: scratchBase)

        var env = ProviderEnv.direct(ProcessInfo.processInfo.environment)
        // Mesma assimetria de sempre: apontar a variável para `~/.claude`
        // explicitamente NÃO é o mesmo que não setá-la — o Claude Code lê o
        // `.claude.json` ao lado do diretório quando ela está ausente, e de
        // dentro dele quando está presente. Setá-la no perfil padrão mandaria
        // ele procurar no lugar errado, e a sessão subiria deslogada.
        if let value = configDir.environmentValue { env["CLAUDE_CONFIG_DIR"] = value }
        else { env.removeValue(forKey: "CLAUDE_CONFIG_DIR") }
        // Uma sondagem lançada de DENTRO de uma sessão do Claude Code herdaria
        // estas, e ele se comportaria como sessão aninhada.
        for key in Array(env.keys) where key.hasPrefix("CLAUDE_CODE") || key == "CLAUDECODE" {
            env.removeValue(forKey: key)
        }
        env["PWD"] = scratch.path

        let process = Process()
        process.executableURL = URL(fileURLWithPath: binary)
        process.arguments = arguments
        process.currentDirectoryURL = scratch
        process.environment = env
        // Nunca um terminal: herdando o stdin do pai, o `claude` espera uma
        // entrada que nunca vem e só o prazo o encerra.
        process.standardInput = FileHandle.nullDevice
        let out = Pipe()
        process.standardOutput = out
        // Descartado, não canalizado: um pipe que ninguém lê enche em 64 KB e
        // trava o processo até o cão de guarda matá-lo.
        process.standardError = FileHandle.nullDevice

        try process.run()

        let watchdog = DispatchWorkItem { if process.isRunning { process.terminate() } }
        DispatchQueue.global(qos: .utility).asyncAfter(deadline: .now() + timeout,
                                                       execute: watchdog)
        defer { watchdog.cancel() }

        let data = out.fileHandleForReading.readDataToEndOfFile()
        process.waitUntilExit()

        if process.terminationReason == .uncaughtSignal { throw ProbeError.timedOut }
        guard process.terminationStatus == 0 else {
            // Código diferente de zero é o Claude Code se recusando a responder,
            // que na prática é perfil sem login. Quem chama segue em frente.
            throw ProbeError.notSignedIn
        }
        return String(decoding: data, as: UTF8.self)
    }

    // MARK: - Ler o que ele disse

    /// As linhas que interessam. Tudo abaixo delas é prosa sobre o que puxou o
    /// consumo — aproximada por admissão própria, e nenhum limite.
    private static let line = try! NSRegularExpression(
        pattern: #"^Current (?:(session)|week \(([^)]+)\)):\s*(\d+)%\s*used(?:\s*·\s*resets\s*(.+?))?\s*$"#,
        options: [.anchorsMatchLines])

    public static func parse(_ text: String, now: Date = Date()) throws -> Reading {
        let range = NSRange(text.startIndex..., in: text)
        var session: Double?, sessionReset: Date?
        var weekly: Double?, weeklyReset: Date?
        var models: [ModelWindow] = []

        for match in line.matches(in: text, range: range) {
            func group(_ i: Int) -> String? {
                guard let r = Range(match.range(at: i), in: text) else { return nil }
                return String(text[r])
            }
            guard let percent = group(3).flatMap(Double.init) else { continue }
            let fraction = percent / 100
            // A data é opcional de propósito: perder uma porcentagem que leu
            // perfeitamente bem porque a redação da data mudou é a pior das
            // duas falhas.
            let reset = group(4).flatMap { resetDate(from: $0, now: now) }

            if group(1) != nil {
                session = fraction; sessionReset = reset
            } else if let escopo = group(2) {
                if escopo.lowercased() == "all models" {
                    weekly = fraction; weeklyReset = reset
                } else if !models.contains(where: { $0.name == escopo }) {
                    models.append(ModelWindow(name: escopo, percent: fraction, resetsAt: reset))
                }
            }
        }

        guard session != nil || weekly != nil || !models.isEmpty else {
            throw ProbeError.unrecognized
        }
        return Reading(session: session, sessionResetsAt: sessionReset,
                       weeklyAll: weekly, weeklyAllResetsAt: weeklyReset,
                       models: models, plan: plan(in: text))
    }

    /// A linha de plano do topo, copiada como impressa. Só as primeiras linhas:
    /// mais abaixo o texto fala de consumo, e "Max" aparece em prosa.
    static func plan(in text: String) -> String? {
        let head = text.split(whereSeparator: \.isNewline).prefix(3).joined(separator: "\n")
        for frase in ["Max 20x", "Max 5x", "Pro", "Team", "subscription"] {
            if let faixa = head.range(of: frase, options: .caseInsensitive) {
                return String(head[faixa])
            }
        }
        return nil
    }

    /// `Sep 18 at 7:29pm (America/Maceio)` → `Date`.
    ///
    /// O ano não é impresso, então é escolhido: o candidato mais próximo de
    /// `now`, entre o ano passado, este e o que vem. Qualquer outra regra erra a
    /// virada do ano numa das direções — uma janela que reseta em 2 de janeiro,
    /// lida em 31 de dezembro, é do ano que vem.
    static func resetDate(from text: String, now: Date) -> Date? {
        var stamp = text.trimmingCharacters(in: .whitespaces)
        var zone = TimeZone.current

        // O fuso vem por último, entre parênteses, e tem de sair ANTES do
        // conserto de am/pm abaixo: `America/...` carrega um "am" próprio.
        if let abre = stamp.lastIndex(of: "("), stamp.hasSuffix(")") {
            let nome = String(stamp[stamp.index(after: abre)...].dropLast())
            zone = TimeZone(identifier: nome) ?? zone
            stamp = String(stamp[..<abre]).trimmingCharacters(in: .whitespaces)
        }
        stamp = stamp.replacingOccurrences(of: "am", with: "AM")
            .replacingOccurrences(of: "pm", with: "PM")

        let formatter = DateFormatter()
        formatter.locale = Locale(identifier: "en_US_POSIX")
        formatter.timeZone = zone

        // Duas grafias, porque os minutos somem quando são zero: `Sep 18 at
        // 7:29pm`, mas `Sep 21 at 9am` na hora cheia. Um padrão `h:mma` sozinho
        // lê a primeira e recusa a segunda — uma janela que perde o horário de
        // reset em uma hora de cada sessenta, tempo suficiente para parecer bug
        // e pouco para aparecer numa amostra.
        guard let parsed = ["MMM d 'at' h:mma", "MMM d 'at' ha"].lazy.compactMap({ fmt -> Date? in
            formatter.dateFormat = fmt
            return formatter.date(from: stamp)
        }).first else { return nil }

        var calendar = Calendar(identifier: .gregorian)
        calendar.timeZone = zone
        var parts = calendar.dateComponents([.month, .day, .hour, .minute], from: parsed)
        let ano = calendar.component(.year, from: now)
        return [ano - 1, ano, ano + 1].compactMap { candidato -> Date? in
            parts.year = candidato
            return calendar.date(from: parts)
        }.min { abs($0.timeIntervalSince(now)) < abs($1.timeIntervalSince(now)) }
    }
}
