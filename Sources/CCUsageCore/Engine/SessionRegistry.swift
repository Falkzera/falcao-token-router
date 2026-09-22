import Darwin
import Foundation

/// Uma sessão do Claude Code viva agora, lida do registro que ele próprio
/// escreve.
public struct LiveSession: Sendable, Equatable, Identifiable {
    /// O que a sessão está fazendo. Valores observados numa máquina real em
    /// 18/09/2026: `busy`, `idle`, `shell`. O `waiting` aparece quando ela pede
    /// permissão. Desconhecido vira `.other` com o nome cru, em vez de virar
    /// `idle` — um estado que este app não entende ainda é um estado, e fingir
    /// que é ocioso é a pior das leituras possíveis.
    public enum Status: Sendable, Equatable {
        case busy, idle, shell, waiting
        case other(String)

        public init(raw: String?) {
            switch raw {
            case "busy": self = .busy
            case "idle": self = .idle
            case "shell": self = .shell
            case "waiting": self = .waiting
            case let outro?: self = .other(outro)
            case nil: self = .other("")
            }
        }

        /// `true` quando a sessão está trabalhando ou esperando o usuário — os
        /// dois casos em que trocar a conta por baixo dela se faz sentir.
        public var isEngaged: Bool {
            switch self {
            case .busy, .waiting: true
            default: false
            }
        }
    }

    public let pid: Int32
    public let sessionID: String?
    /// Onde a sessão roda. É o que dá nome útil a ela na tela.
    public let cwd: String
    /// O nome que o Claude Code derivou (`api-sei-f1`, `falcao-gym-27`).
    public let name: String?
    public let startedAt: Date?
    public let status: Status
    public let statusUpdatedAt: Date?

    public var id: Int32 { pid }

    /// A última pasta do `cwd` — o rótulo curto que cabe numa linha.
    public var folder: String { (cwd as NSString).lastPathComponent }

    /// O rótulo de tela: o nome derivado se houver, senão a pasta.
    public var label: String { name?.isEmpty == false ? name! : folder }

    public init(pid: Int32, sessionID: String?, cwd: String, name: String?,
                startedAt: Date?, status: Status, statusUpdatedAt: Date?) {
        self.pid = pid
        self.sessionID = sessionID
        self.cwd = cwd
        self.name = name
        self.startedAt = startedAt
        self.status = status
        self.statusUpdatedAt = statusUpdatedAt
    }
}

/// Quais sessões do Claude Code estão vivas **em cada perfil**.
///
/// O Claude Code grava um `<perfil>/sessions/<pid>.json` por sessão, e o
/// registro é **por perfil** — confirmado numa máquina real: sete sessões em
/// `~/.claude/sessions/` e uma em `<grupo>/sessions/`. Isso é exatamente o que
/// este produto precisa e não tinha como saber: **qual sessão roda em qual
/// grupo**, e portanto por qual conta ela está sendo atendida.
///
/// É a resposta para a pergunta que o `/status` do Claude Code não dá, e o
/// contra-veneno do modo de falha silencioso: num terminal aberto antes da
/// instalação, `claude trabalho` vira argumento e a sessão sobe no `~/.claude`,
/// na conta errada, sem nada avisando. Vendo as sessões por perfil, o erro
/// aparece.
///
/// Nada aqui escreve. É leitura de arquivo local do próprio usuário.
public enum SessionRegistry {
    /// O diretório onde o Claude Code registra as sessões de um perfil.
    ///
    /// **Dentro** do perfil nos dois casos — diferente do `.claude.json`, que no
    /// perfil padrão mora ao lado. Conferido no disco: `~/.claude/sessions/` e
    /// `<grupo>/sessions/`.
    public static func directory(for dir: ConfigDir) -> URL {
        dir.url.appending(path: "sessions")
    }

    /// As sessões vivas de um perfil.
    ///
    /// Um arquivo sobrevive ao processo que o escreveu: sessão que morreu de
    /// forma abrupta deixa o registro para trás dizendo `busy` para sempre. Por
    /// isso cada entrada é conferida contra o processo de verdade, e não contra
    /// o que o arquivo afirma.
    public static func liveSessions(in dir: ConfigDir,
                                    readFile: (URL) -> Data? = { try? Data(contentsOf: $0) },
                                    listing: ((URL) -> [URL])? = nil) -> [LiveSession] {
        let pasta = directory(for: dir)
        let arquivos = listing?(pasta) ?? ((try? FileManager.default.contentsOfDirectory(
            at: pasta, includingPropertiesForKeys: nil)) ?? [])

        return arquivos
            .filter { $0.pathExtension == "json" }
            .compactMap { url -> LiveSession? in
                guard let data = readFile(url),
                      let raw = try? JSONSerialization.jsonObject(with: data) as? [String: Any]
                else { return nil }
                return session(from: raw)
            }
            .filter { ProcessLiveness.isAlive(pid: $0.pid, startedAt: $0.startedAt) }
            .sorted { ($0.startedAt ?? .distantPast) > ($1.startedAt ?? .distantPast) }
    }

    /// Decodificado com tolerância de propósito: o arquivo é escrito por outro
    /// programa, no calendário de release dele, e um campo novo nunca pode
    /// custar uma sessão que daria para mostrar.
    static func session(from raw: [String: Any]) -> LiveSession? {
        guard let pid = (raw["pid"] as? NSNumber)?.int32Value,
              let cwd = raw["cwd"] as? String
        else { return nil }

        // `procStart` é o instante do PROCESSO; `startedAt` é o do registro, uns
        // segundos depois. Para conferir um pid reciclado vale o primeiro.
        let inicio = (raw["procStart"] as? String).flatMap(parseProcStart)
            ?? (raw["startedAt"] as? NSNumber).map {
                Date(timeIntervalSince1970: $0.doubleValue / 1000)
            }

        return LiveSession(
            pid: pid,
            sessionID: raw["sessionId"] as? String,
            cwd: cwd,
            name: raw["name"] as? String,
            startedAt: inicio,
            status: .init(raw: raw["status"] as? String),
            statusUpdatedAt: (raw["statusUpdatedAt"] as? NSNumber)
                .map { Date(timeIntervalSince1970: $0.doubleValue / 1000) })
    }

    /// `procStart` vem como `Fri Sep 18 12:14:31 2026` — um `ctime`, em **UTC**,
    /// com o dia do mês preenchido com espaço quando tem um dígito só.
    static func parseProcStart(_ text: String) -> Date? {
        let formatter = DateFormatter()
        formatter.locale = Locale(identifier: "en_US_POSIX")
        formatter.timeZone = TimeZone(identifier: "UTC")
        formatter.dateFormat = "EEE MMM d HH:mm:ss yyyy"
        // O espaço duplo do dia de um dígito quebraria o padrão.
        let limpo = text.split(separator: " ", omittingEmptySubsequences: true)
            .joined(separator: " ")
        return formatter.date(from: limpo)
    }
}

/// O pid ainda existe — e ainda é o **mesmo** processo?
///
/// Conferir o pid sozinho não basta numa máquina que fica dias ligada: pids são
/// reciclados, e um pid reaproveitado ressuscitaria uma sessão morta. Comparar o
/// instante de início resolve.
public enum ProcessLiveness {
    /// Larga o bastante para absorver a distância entre o processo subir e a
    /// sessão se registrar, apertada o bastante para um pid reciclado não passar.
    static let reuseTolerance: TimeInterval = 5 * 60

    public static func isAlive(pid: Int32, startedAt: Date?) -> Bool {
        guard exists(pid: pid) else { return false }
        guard let startedAt, let real = startTime(pid: pid) else {
            // Sem como provar nem desprovar: confia no pid, em vez de esconder
            // uma sessão que provavelmente é real.
            return true
        }
        return abs(real.timeIntervalSince(startedAt)) < reuseTolerance
    }

    static func exists(pid: Int32) -> Bool {
        if kill(pid, 0) == 0 { return true }
        // EPERM é "existe, mas é de outro dono".
        return errno == EPERM
    }

    /// O instante em que o processo subiu, pelo kernel.
    public static func startTime(pid: Int32) -> Date? {
        var info = kinfo_proc()
        var size = MemoryLayout<kinfo_proc>.stride
        var mib: [Int32] = [CTL_KERN, KERN_PROC, KERN_PROC_PID, pid]
        guard sysctl(&mib, u_int(mib.count), &info, &size, nil, 0) == 0, size > 0
        else { return nil }
        let t = info.kp_proc.p_starttime
        return Date(timeIntervalSince1970: Double(t.tv_sec) + Double(t.tv_usec) / 1_000_000)
    }
}
