import Darwin
import Foundation
import Testing
@testable import CCUsageCore

/// Um registro REAL, copiado de `~/.claude/sessions/<pid>.json` em 18/09/2026.
/// O `pid` é trocado por teste; o resto é como o Claude Code escreve.
private func registro(pid: Int32, status: String = "busy",
                      procStart: String = "Fri Sep 18 12:14:31 2026") -> [String: Any] {
    [
        "pid": NSNumber(value: pid),
        "sessionId": "80ed900f-a4b4-4ed4-a95c-e17e07877d95",
        "cwd": "/Users/exemplo/Projects/falcao-token-router",
        "startedAt": NSNumber(value: 1_789_733_694_030),
        "procStart": procStart,
        "version": "2.1.276",
        "kind": "interactive",
        "entrypoint": "cli",
        "name": "falcao-token-router-b9",
        "status": status,
        "statusUpdatedAt": NSNumber(value: 1_789_755_406_509),
    ]
}

@Suite("SessionRegistry (quais sessões rodam em qual grupo)")
struct SessionRegistryTests {
    @Test("lê um registro real do Claude Code")
    func leRegistroReal() throws {
        let s = try #require(SessionRegistry.session(from: registro(pid: 4242)))
        #expect(s.pid == 4242)
        #expect(s.cwd == "/Users/exemplo/Projects/falcao-token-router")
        #expect(s.folder == "falcao-token-router")
        #expect(s.label == "falcao-token-router-b9")
        #expect(s.status == .busy)
        #expect(s.sessionID == "80ed900f-a4b4-4ed4-a95c-e17e07877d95")
    }

    /// Um estado que este app não conhece ainda é um estado. Virar `idle` seria
    /// afirmar que a sessão está parada — a pior leitura possível quando a
    /// pergunta é se dá para trocar a conta agora.
    @Test("status desconhecido vira .other, não .idle")
    func statusDesconhecido() throws {
        let s = try #require(SessionRegistry.session(from: registro(pid: 1, status: "compacting")))
        #expect(s.status == .other("compacting"))
        #expect(!s.status.isEngaged)
        // Os três observados de verdade numa máquina real, mais o de permissão.
        #expect(LiveSession.Status(raw: "shell") == .shell)
        #expect(LiveSession.Status(raw: "waiting").isEngaged)
        #expect(LiveSession.Status(raw: "busy").isEngaged)
        #expect(!LiveSession.Status(raw: "idle").isEngaged)
    }

    /// Sessão que morreu de forma abrupta deixa o arquivo para trás dizendo
    /// `busy` para sempre. Confiar no arquivo mostraria trabalho que não existe.
    @Test("registro de processo morto é descartado")
    func processoMortoSai() {
        // Um pid que quase certamente não existe, com carimbo coerente.
        let mortos = SessionRegistry.liveSessions(
            in: .dedicated("/tmp/perfil"),
            readFile: { _ in try? JSONSerialization.data(withJSONObject: registro(pid: 999_999)) },
            listing: { _ in [URL(fileURLWithPath: "/tmp/perfil/sessions/999999.json")] })
        #expect(mortos.isEmpty)
    }

    /// O processo deste teste existe, e o carimbo bate: tem de passar.
    @Test("processo vivo com carimbo coerente passa")
    func processoVivoPassa() throws {
        let meu = getpid()
        let inicio = try #require(ProcessLiveness.startTime(pid: meu))
        let formatter = DateFormatter()
        formatter.locale = Locale(identifier: "en_US_POSIX")
        formatter.timeZone = TimeZone(identifier: "UTC")
        formatter.dateFormat = "EEE MMM d HH:mm:ss yyyy"

        let vivos = SessionRegistry.liveSessions(
            in: .dedicated("/tmp/perfil"),
            readFile: { _ in
                try? JSONSerialization.data(
                    withJSONObject: registro(pid: meu,
                                             procStart: formatter.string(from: inicio)))
            },
            listing: { _ in [URL(fileURLWithPath: "/tmp/perfil/sessions/x.json")] })
        #expect(vivos.count == 1)
        #expect(vivos.first?.pid == meu)
    }

    /// Numa máquina que fica dias ligada, pids são reciclados. Sem comparar o
    /// instante de início, um pid reaproveitado ressuscitaria uma sessão morta.
    @Test("pid reciclado é recusado pelo instante de início")
    func pidReciclado() {
        let meu = getpid()
        // O processo existe, mas o registro diz que ele começou em 2020.
        #expect(!ProcessLiveness.isAlive(
            pid: meu, startedAt: Date(timeIntervalSince1970: 1_600_000_000)))
        // Sem carimbo não dá para provar nem desprovar: confia no pid.
        #expect(ProcessLiveness.isAlive(pid: meu, startedAt: nil))
    }

    /// `ctime` preenche o dia com ESPAÇO quando tem um dígito só, e o espaço
    /// duplo quebra o padrão do formatador.
    @Test("procStart com dia de um dígito (espaço duplo) é lido")
    func procStartDiaUmDigito() throws {
        let comEspacoDuplo = try #require(
            SessionRegistry.parseProcStart("Mon Sep  8 09:05:01 2026"))
        let semEspacoDuplo = try #require(
            SessionRegistry.parseProcStart("Mon Sep 8 09:05:01 2026"))
        #expect(comEspacoDuplo == semEspacoDuplo)
    }

    /// O registro é POR PERFIL, e é isso que deixa o produto dizer em qual
    /// grupo cada sessão roda. No perfil padrão fica DENTRO do diretório —
    /// diferente do `.claude.json`, que no padrão mora ao lado.
    @Test("o diretório de sessões fica dentro do perfil, nos dois casos")
    func diretorioPorPerfil() {
        let padrao = SessionRegistry.directory(for: .standard(home: "/Users/x"))
        #expect(padrao.path == "/Users/x/.claude/sessions")
        let grupo = SessionRegistry.directory(for: .dedicated("/opt/g"))
        #expect(grupo.path == "/opt/g/sessions")
    }

    @Test("registro sem pid ou sem cwd é ignorado em vez de quebrar")
    func registroIncompleto() {
        #expect(SessionRegistry.session(from: ["cwd": "/tmp"]) == nil)
        #expect(SessionRegistry.session(from: ["pid": NSNumber(value: 1)]) == nil)
    }
}
