import Foundation

/// Lê e escreve itens de senha genérica no chaveiro do login.
///
/// Injetável: a implementação real shella o `/usr/bin/security`, e os testes
/// passam uma em memória. O motor de rotação só conhece este protocolo.
public protocol KeychainStore: Sendable {
    /// O segredo do item, ou `nil` se ele não existe.
    func read(service: String) -> String?
    /// Cria ou atualiza o item.
    func write(_ secret: String, service: String) throws
    /// Se o item existe, sem decifrar o segredo (não dispara prompt).
    func exists(service: String) -> Bool
    /// Apaga o item. Silencioso se ele já não existe.
    ///
    /// Existe porque "remover a conta" tem de remover a credencial dela: um app
    /// que tira a conta da tela e deixa o refresh token vivo no chaveiro do
    /// usuário promete uma coisa e faz outra.
    func delete(service: String)
}

public enum KeychainError: Error, Equatable {
    case writeFailed(String)
}

/// A implementação sobre o binário `/usr/bin/security`.
///
/// **Usa o mesmo binário que o Claude Code usa**, e é isso que evita o prompt de
/// senha: no macOS, o controle de acesso de um item de chaveiro é ancorado no
/// programa que o criou. Se o Claude Code criou o item com o `security` do
/// sistema e nós lemos com o mesmo `security`, criador e leitor coincidem e o
/// macOS não interrompe. Ler com a Security.framework de dentro do nosso app
/// seria outro programa, e reabriria o prompt a cada leitura — foi a lição que o
/// próprio `cswap` documenta.
///
/// O caminho é fixo, nunca resolvido pelo PATH: é ferramenta de credencial, e um
/// `security` plantado antes no PATH não pode interceptar segredo.
public struct SecurityCLIKeychain: KeychainStore {
    /// Espelha o `getUsername()` do Claude Code: `$USER`, senão o dono do
    /// processo. Precisa bater exatamente, ou apontaríamos para outro item.
    private let account: String
    private static let security = "/usr/bin/security"
    private static let timeout: TimeInterval = 5

    public init(account: String = SecurityCLIKeychain.currentUser()) {
        self.account = account
    }

    public static func currentUser() -> String {
        if let user = ProcessInfo.processInfo.environment["USER"], !user.isEmpty {
            return user
        }
        return NSUserName().isEmpty ? "claude-code-user" : NSUserName()
    }

    public func read(service: String) -> String? {
        let result = run(["find-generic-password", "-a", account, "-w", "-s", service])
        guard result.code == 0 else { return nil }
        // `-w` imprime o valor seguido de um `\n`; tira exatamente esse.
        if result.out.hasSuffix("\n") { return String(result.out.dropLast()) }
        return result.out
    }

    public func exists(service: String) -> Bool {
        // Sem `-w`: consulta só de atributos, não decifra nada, nunca gera
        // prompt — mesmo para item de outro app.
        run(["find-generic-password", "-a", account, "-s", service]).code == 0
    }

    public func delete(service: String) {
        // Sem `-w`: não decifra nada e não abre prompt. Código != 0 é item
        // ausente, que é o resultado desejado de qualquer forma.
        _ = run(["delete-generic-password", "-a", account, "-s", service])
    }

    public func write(_ secret: String, service: String) throws {
        // Valor em hex, entregue pelo stdin do `security -i`, para o segredo
        // nunca aparecer em `argv` (onde um monitor de processos o veria). O
        // mesmo cuidado do Claude Code e do cswap.
        let hex = Data(secret.utf8).map { String(format: "%02x", $0) }.joined()
        let command = "add-generic-password -U -a \(quote(account)) -s \(quote(service)) -X \(hex)\n"
        let result = run(["-i"], stdin: command)
        if result.code != 0 {
            throw KeychainError.writeFailed(result.err.isEmpty
                                            ? "código \(result.code)" : result.err)
        }
    }

    private func quote(_ value: String) -> String {
        "\"\(value.replacingOccurrences(of: "\\", with: "\\\\").replacingOccurrences(of: "\"", with: "\\\""))\""
    }

    private func run(_ args: [String], stdin: String? = nil)
        -> (code: Int32, out: String, err: String) {
        let process = Process()
        process.executableURL = URL(fileURLWithPath: Self.security)
        process.arguments = args
        let outPipe = Pipe(), errPipe = Pipe()
        process.standardOutput = outPipe
        process.standardError = errPipe
        let inPipe = Pipe()
        if stdin != nil { process.standardInput = inPipe }

        do { try process.run() } catch { return (-1, "", "\(error)") }
        if let stdin {
            inPipe.fileHandleForWriting.write(Data(stdin.utf8))
            try? inPipe.fileHandleForWriting.close()
        }
        let out = outPipe.fileHandleForReading.readDataToEndOfFile()
        let err = errPipe.fileHandleForReading.readDataToEndOfFile()
        process.waitUntilExit()
        return (process.terminationStatus,
                String(decoding: out, as: UTF8.self),
                String(decoding: err, as: UTF8.self))
    }
}
