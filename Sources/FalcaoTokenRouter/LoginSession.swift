import AppKit
import Darwin
import Foundation
import Observation
import CCUsageCore

/// Onde está o binário oficial `claude`. Um app de GUI não herda o PATH do
/// shell, então procuramos os lugares conhecidos e, por último, perguntamos a um
/// shell de login.
enum ProviderBinary {
    static func claude() -> String? {
        let home = NSHomeDirectory()
        let candidates = [
            "\(home)/.local/bin/claude",
            "/opt/homebrew/bin/claude",
            "/usr/local/bin/claude",
            "\(home)/.claude/local/claude",
        ]
        for c in candidates where FileManager.default.isExecutableFile(atPath: c) { return c }

        let probe = Process()
        probe.executableURL = URL(fileURLWithPath: "/bin/zsh")
        probe.arguments = ["-lc", "whence -p claude"]
        let pipe = Pipe()
        probe.standardOutput = pipe
        probe.standardError = FileHandle.nullDevice
        try? probe.run()
        probe.waitUntilExit()
        let out = String(decoding: pipe.fileHandleForReading.readDataToEndOfFile(), as: UTF8.self)
            .trimmingCharacters(in: .whitespacesAndNewlines)
        return FileManager.default.isExecutableFile(atPath: out) ? out : nil
    }
}

/// Conduz o `claude auth login` oficial **dentro do app**, sem janela de
/// Terminal.
///
/// A conformidade se mantém: quem autentica é o binário oficial, o token vai
/// para o perfil isolado escrito por ele, e o app nunca vê a senha nem o token —
/// só observa a saída para achar o link e saber quando terminou.
///
/// Roda o `claude` num **pseudo-terminal** (pty) de propósito: assim ele se
/// comporta igual ao Terminal — imprime o link na hora e abre o navegador — em
/// vez de segurar a saída em buffer, como faria sob um pipe simples.
@MainActor
@Observable
final class LoginSession {
    enum Phase: Equatable {
        case starting
        case waiting          // link mostrado, aguardando o navegador
        case success
        case failed(String)
    }

    private(set) var phase: Phase = .starting
    private(set) var authURL: URL?

    let home: ConfigDir
    let accountID: UUID
    let groupID: UUID

    @ObservationIgnored private var process: Process?
    @ObservationIgnored private var master: FileHandle?
    @ObservationIgnored private var buffer = ""

    init(home: ConfigDir, accountID: UUID, groupID: UUID) {
        self.home = home
        self.accountID = accountID
        self.groupID = groupID
    }

    func start() {
        guard let claude = ProviderBinary.claude() else {
            phase = .failed(String(localized: "groups.login.error.noClaude"))
            return
        }
        var mfd: Int32 = 0, sfd: Int32 = 0
        guard openpty(&mfd, &sfd, nil, nil, nil) == 0 else {
            phase = .failed(String(localized: "groups.login.error.pty"))
            return
        }

        let proc = Process()
        proc.executableURL = URL(fileURLWithPath: claude)
        proc.arguments = ["auth", "login"]
        // Sem proxy: se a máquina tem o teamclaude (ou outro) exportado, o login
        // passaria por ele e viria sempre a mesma conta. O login oficial fala
        // direto com a Anthropic.
        var env = ProviderEnv.direct(ProcessInfo.processInfo.environment)
        env["CLAUDE_CONFIG_DIR"] = home.raw
        env["TERM"] = "xterm-256color"
        for key in Array(env.keys) where key.hasPrefix("CLAUDE_CODE") || key == "CLAUDECODE" {
            env.removeValue(forKey: key)
        }
        proc.environment = env

        let slave = FileHandle(fileDescriptor: sfd, closeOnDealloc: false)
        proc.standardInput = slave
        proc.standardOutput = slave
        proc.standardError = slave

        let masterHandle = FileHandle(fileDescriptor: mfd, closeOnDealloc: true)
        master = masterHandle
        process = proc

        masterHandle.readabilityHandler = { [weak self] handle in
            let data = handle.availableData
            if data.isEmpty { handle.readabilityHandler = nil; return }
            let text = String(decoding: data, as: UTF8.self)
            Task { @MainActor [weak self] in self?.ingest(text) }
        }
        proc.terminationHandler = { [weak self] _ in
            Task { @MainActor [weak self] in self?.processEnded() }
        }

        do {
            try proc.run()
            phase = .waiting
        } catch {
            phase = .failed("\(error)")
        }
        // O filho já herdou o slave; o pai fecha a sua cópia.
        close(sfd)
    }

    /// Cola um código de autorização, para o raro caso do navegador não
    /// conseguir devolver sozinho (SSH, container). No fluxo local normal o
    /// callback resolve e isto nem é usado.
    func submitCode(_ code: String) {
        let trimmed = code.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmed.isEmpty else { return }
        master?.write(Data((trimmed + "\r").utf8))
    }

    func cancel() {
        process?.terminationHandler = nil
        master?.readabilityHandler = nil
        process?.terminate()
        cleanup()
    }

    private func ingest(_ text: String) {
        buffer += text
        if authURL == nil, let url = Self.extractURL(from: buffer) {
            authURL = url
        }
        if buffer.contains("Login successful") {
            phase = .success
            cleanup()
        }
    }

    private func processEnded() {
        // Saiu sem sucesso registrado: só marca falha se ainda estávamos
        // esperando (o sucesso pode ter sido detectado pela linha, e aí o
        // processo encerra normalmente logo depois).
        if phase == .waiting || phase == .starting {
            phase = .failed(String(localized: "groups.login.error.ended"))
        }
        cleanup()
    }

    private func cleanup() {
        master?.readabilityHandler = nil
        master = nil
        // O buffer guarda a saída inteira do pty, o link de autorização
        // incluído. Nada obriga a mantê-lo vivo depois do desfecho, e o que não
        // está em memória não vaza para um relatório de falha.
        buffer = ""
    }

    /// Extrai o link de autorização da saída do `claude`.
    static func extractURL(from text: String) -> URL? {
        guard let range = text.range(
            of: #"https://claude\.com/[^\s\x1b"']+"#, options: .regularExpression)
        else { return nil }
        return URL(string: String(text[range]))
    }
}
