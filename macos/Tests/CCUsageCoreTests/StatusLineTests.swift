import Foundation
import Testing
@testable import CCUsageCore

/// Sem cor e com fase fixa: o que se afirma aqui é o TEXTO e o que entra ou sai
/// da linha, não os códigos ANSI. A pintura tem os seus próprios testes.
private func plain(_ view: StatusLineView, portuguese: Bool = false, phase: UInt64 = 0) -> String {
    let line = view.render(.init(trueColor: false, portuguese: portuguese, phase: phase,
                                 calendar: utc))
    return line.replacingOccurrences(of: "\u{1b}\\[[0-9;]*m", with: "", options: .regularExpression)
}

/// Calendário fixo em UTC: um teste de "quando reseta" não pode depender do
/// fuso de quem roda a suíte.
private let utc: Calendar = {
    var c = Calendar(identifier: .gregorian)
    c.timeZone = TimeZone(identifier: "UTC")!
    return c
}()

/// 2026-09-28 (segunda-feira), 09:00 UTC.
private let segunda = Date(timeIntervalSince1970: 1_790_586_000)

private func cheia() -> StatusLineView {
    StatusLineView(
        label: .group(name: "trabalho", index: 0),
        model: "Opus 5 (1M context)", modelID: "claude-opus-5",
        effort: "high", place: "main",
        context: .init(usedPercent: 34, inputTokens: 68_000, windowSize: 200_000),
        fiveHour: .init(fraction: 0.42, resetsAt: segunda),
        sevenDay: .init(fraction: 0.38, resetsAt: segunda),
        costUSD: 2.81, email: "conta1@exemplo.com")
}

@Suite("Status line — a linha")
struct StatusLineViewTests {
    @Test("a linha completa traz tudo, na ordem")
    func ordem() {
        #expect(plain(cheia()) ==
            "● trabalho │ Opus 5 high │ main │ ███░░░░░░░ 34% 68k/200k │ "
            + "5h ██░░░ 42% ↻ 09:00  7d ██░░░ 38% ↻ Mon (28) 9:00 │ $2.81 │ conta1@exemplo.com")
    }

    @Test("o que não veio some sem deixar marcador")
    func ausentesSomem() {
        // O 1º render de uma sessão não traz `rate_limits`, e o `effort` só vem
        // com modelo que o aceita. Um espaço reservado para o que não existe
        // seria ruído permanente numa linha que se lê de relance.
        var view = cheia()
        view.fiveHour = nil; view.sevenDay = nil; view.effort = nil
        view.context = nil; view.costUSD = nil
        #expect(plain(view) == "● trabalho │ Opus 5 │ main │ conta1@exemplo.com")
    }

    @Test("view vazia é linha vazia, não uma linha de separadores")
    func vazia() {
        #expect(plain(StatusLineView()) == "")
    }

    @Test("o esforço fica ao lado do modelo; sem modelo, sozinho")
    func esforcoSemModelo() {
        var view = StatusLineView(effort: "high")
        #expect(plain(view) == "high")
        view.model = "Sonnet 5"
        #expect(plain(view) == "Sonnet 5 high")
    }

    @Test("a janela do modelo sai do nome — ela já aparece no medidor de contexto")
    func nomeDoModelo() {
        #expect(StatusLineFormat.modelName("Opus 5 (1M context)") == "Opus 5")
        #expect(StatusLineFormat.modelName("Sonnet 5") == "Sonnet 5")
        // Parêntese aninhado não é rótulo de janela: fica como está.
        #expect(StatusLineFormat.modelName("Modelo (a (b))") == "Modelo (a (b))")
    }

    @Test("o Fable sai em vermelho — é o limite que trava a conta primeiro")
    func fableEmVermelho() {
        let fable = StatusLineView(model: "Fable 5.1", modelID: "claude-fable-5-1")
        #expect(fable.render(.init(trueColor: false, portuguese: false, phase: 0)).contains("\u{1b}[31m"))
        let opus = StatusLineView(model: "Opus 5", modelID: "claude-opus-5")
        #expect(opus.render(.init(trueColor: false, portuguese: false, phase: 0)).contains("\u{1b}[94m"))
    }

    @Test("a cor da janela segue a severidade: <70 verde, <90 amarelo, senão vermelho")
    func severidade() {
        func cor(_ f: Double) -> String {
            StatusLineView(fiveHour: .init(fraction: f, resetsAt: nil))
                .render(.init(trueColor: false, portuguese: false, phase: 0))
        }
        #expect(cor(0.69).contains("\u{1b}[32m"))
        #expect(cor(0.70).contains("\u{1b}[33m"))
        #expect(cor(0.90).contains("\u{1b}[31m"))
    }

    @Test("a cor do grupo vem da posição na lista, e dá a volta")
    func corDoGrupo() {
        func cor(_ i: Int) -> String {
            StatusLineView(label: .group(name: "g", index: i))
                .render(.init(trueColor: false, portuguese: false, phase: 0))
        }
        #expect(cor(0).contains(Paint.cyan))
        #expect(cor(4).contains(Paint.cyan))   // 4 % 4 == 0
        #expect(cor(1).contains(Paint.green))
    }

    @Test("conta fora de grupo é amarela, não a cor de um grupo")
    func contaSoltaEmAmarelo() {
        let line = StatusLineView(label: .account("conta1"))
            .render(.init(trueColor: false, portuguese: false, phase: 0))
        #expect(line.contains(Paint.yellow))
    }
}

@Suite("Status line — o quando do reset")
struct StatusLineResetTests {
    @Test("a de 5h dá só a hora; a semanal dá o dia, porque cai em qualquer um")
    func duasFormas() {
        #expect(StatusLineFormat.resetWhen(segunda, withDay: false, portuguese: false, calendar: utc) == "09:00")
        #expect(StatusLineFormat.resetWhen(segunda, withDay: true, portuguese: false, calendar: utc) == "Mon (28) 9:00")
        #expect(StatusLineFormat.resetWhen(segunda, withDay: true, portuguese: true, calendar: utc) == "seg (28) 9:00")
    }

    @Test("a hora da semanal vai sem zero à esquerda — igual ao porte Windows")
    func semZeroAEsquerda() {
        // 2026-09-27 (domingo), 07:05 UTC.
        let domingo = Date(timeIntervalSince1970: 1_790_492_700)
        #expect(StatusLineFormat.resetWhen(domingo, withDay: true, portuguese: true, calendar: utc) == "dom (27) 7:05")
        #expect(StatusLineFormat.resetWhen(domingo, withDay: false, portuguese: true, calendar: utc) == "07:05")
    }
}

@Suite("Status line — pintura e ambiente")
struct StatusLinePaintTests {
    @Test("sem truecolor o esforço é cor chapada — animação de 24 bits vira lixo em 16 cores")
    func semTruecolor() {
        let flat = Paint.effort("max", trueColor: false, phase: 0, gray: Paint.grayANSI)
        #expect(flat == "\u{1b}[31m\u{1b}[1mmax\u{1b}[0m")
        #expect(!flat.contains("38;2;"))
    }

    @Test("com truecolor o max gira, e girar depende só da fase")
    func arcoIris() {
        let a = Paint.effort("max", trueColor: true, phase: 0, gray: Paint.grayTrueColor)
        let b = Paint.effort("max", trueColor: true, phase: 1, gray: Paint.grayTrueColor)
        #expect(a.contains("38;2;"))
        #expect(a != b)
        #expect(a == Paint.effort("max", trueColor: true, phase: 0, gray: Paint.grayTrueColor))
    }

    @Test("nível desconhecido sai em cinza, sem inventar cor")
    func nivelDesconhecido() {
        #expect(Paint.effort("turbo", trueColor: true, phase: 0, gray: Paint.grayANSI)
                == "\u{1b}[90mturbo\u{1b}[0m")
    }

    @Test("a barra arredonda e nunca passa da largura")
    func barra() {
        #expect(Paint.bar(0, width: 5) == "░░░░░")
        #expect(Paint.bar(1, width: 5) == "█████")
        #expect(Paint.bar(0.5, width: 5) == "███░░")     // 2,5 → 3
        #expect(Paint.bar(1.5, width: 5) == "█████")     // acima de 100% não estoura
        #expect(Paint.bar(-1, width: 5) == "░░░░░")
    }

    @Test("o Terminal.app não é truecolor — ele faz 256 cores")
    func terminalAppNaoEhTruecolor() {
        let env = ["TERM_PROGRAM": "Apple_Terminal", "TERM": "xterm-256color"]
        #expect(StatusLineSource.trueColor { env[$0] } == false)
        #expect(StatusLineSource.trueColor { ["TERM_PROGRAM": "ghostty"][$0] } == true)
        #expect(StatusLineSource.trueColor { ["COLORTERM": "truecolor"][$0] } == true)
        #expect(StatusLineSource.trueColor { _ in nil } == false)
    }

    @Test("o caminho encurta pelo meio, com ~ no lugar da home")
    func caminho() {
        // Até três componentes ficam inteiros; do quarto em diante o meio vira "…".
        #expect(StatusLineFormat.shortenPath("/Users/exemplo/a/b", home: "/Users/exemplo") == "~/a/b")
        #expect(StatusLineFormat.shortenPath("/Users/exemplo/a/b/c", home: "/Users/exemplo") == "~/…/b/c")
        #expect(StatusLineFormat.shortenPath("/Users/exemplo/a/b/c/d", home: "/Users/exemplo") == "~/…/c/d")
        #expect(StatusLineFormat.shortenPath("/Users/exemplo", home: "/Users/exemplo") == "~")
        #expect(StatusLineFormat.shortenPath("/opt/x", home: "/Users/exemplo") == "/opt/x")
    }
}

@Suite("Status line — a escolha do usuário")
struct StatusLineChoiceTests {
    private func semCor(_ view: StatusLineView) -> String {
        view.render(.init(trueColor: false, portuguese: false, phase: 0, calendar: utc))
            .replacingOccurrences(of: "\u{1b}\\[[0-9;]*m", with: "", options: .regularExpression)
    }

    @Test("de fábrica, mostra tudo")
    func fabricaMostraTudo() {
        let choice = StatusLineChoice()
        #expect(StatusLineChoice.Item.allCases.allSatisfy { choice.shows($0) })
        #expect(choice.hiddenItems.isEmpty)
    }

    @Test("o item tirado some da linha")
    func tiraItem() {
        var choice = StatusLineChoice()
        choice.setShown(.cost, false)
        choice.setShown(.email, false)
        #expect(semCor(choice.apply(cheia())).contains("$2.81") == false)
        #expect(semCor(choice.apply(cheia())).contains("conta1@exemplo.com") == false)
        // O que ficou continua lá, e os separadores não sobram.
        #expect(semCor(choice.apply(cheia())).hasSuffix("7d ██░░░ 38% ↻ Mon (28) 9:00"))
    }

    @Test("tirar os resets tira os DOIS — um só seria uma linha que mente")
    func resetsVaoJuntos() {
        var choice = StatusLineChoice()
        choice.setShown(.resets, false)
        let line = semCor(choice.apply(cheia()))
        #expect(!line.contains("↻"))
        // As janelas em si ficam.
        #expect(line.contains("5h ██░░░ 42%") && line.contains("7d ██░░░ 38%"))
    }

    @Test("tirar tudo dá linha vazia, não uma fileira de separadores")
    func tudoFora() {
        var choice = StatusLineChoice()
        for item in StatusLineChoice.Item.allCases { choice.setShown(item, false) }
        #expect(semCor(choice.apply(cheia())) == "")
    }

    @Test("guarda os ESCONDIDOS, para um item novo aparecer para quem já tem o arquivo")
    func guardaOsEscondidos() throws {
        var choice = StatusLineChoice()
        choice.setShown(.cost, false)
        let url = URL(fileURLWithPath: NSTemporaryDirectory())
            .appending(path: UUID().uuidString).appending(path: "statusline.json")
        try choice.save(to: url)
        let root = try JSONSerialization.jsonObject(
            with: Data(contentsOf: url)) as? [String: Any]
        #expect(root?["hidden"] as? [String] == ["cost"])
        try? FileManager.default.removeItem(at: url.deletingLastPathComponent())
    }

    @Test("os escondidos ficam na ordem da linha, não na ordem do clique")
    func ordemEstavel() {
        var choice = StatusLineChoice()
        choice.setShown(.email, false)
        choice.setShown(.group, false)
        choice.setShown(.cost, false)
        #expect(choice.hiddenItems == [.group, .cost, .email])
    }

    @Test("religar um item o tira dos escondidos")
    func religa() {
        var choice = StatusLineChoice()
        choice.setShown(.model, false)
        choice.setShown(.model, true)
        #expect(choice.hiddenItems.isEmpty)
        #expect(choice.shows(.model))
    }

    @Test("arquivo ausente, ilegível ou com chave estranha vale a completa")
    func leituraTolerante() {
        // A linha é o SENSOR: ela não pode falhar por causa da escolha.
        let ausente = StatusLineChoice.load(from: URL(fileURLWithPath: "/nao/existe.json"))
        #expect(ausente.hiddenItems.isEmpty)

        let lixo = StatusLineChoice.load(from: URL(fileURLWithPath: "/x")) { _ in Data("nao é json {{".utf8) }
        #expect(lixo.hiddenItems.isEmpty)

        // Chave desconhecida (versão futura) é ignorada sem levar o resto.
        let futuro = StatusLineChoice.load(from: URL(fileURLWithPath: "/x")) { _ in
            Data(#"{"mode":"app","hidden":["cost","coisaNova"],"command":""}"#.utf8)
        }
        #expect(futuro.hiddenItems == [.cost])
    }

    @Test("ida e volta pelo disco preserva a escolha")
    func idaEVolta() throws {
        var choice = StatusLineChoice()
        choice.setShown(.place, false)
        choice.setShown(.effort, false)
        choice.command = "meu-comando --x"
        let url = URL(fileURLWithPath: NSTemporaryDirectory())
            .appending(path: UUID().uuidString).appending(path: "statusline.json")
        try choice.save(to: url)
        let lida = StatusLineChoice.load(from: url)
        #expect(lida.hiddenItems == [.effort, .place])
        #expect(lida.command == "meu-comando --x")
        #expect(lida.mode == .app)
        try? FileManager.default.removeItem(at: url.deletingLastPathComponent())
    }

    @Test("o comando só vale no modo comando, e não em branco")
    func comandoSoNoModoDele() {
        var choice = StatusLineChoice()
        choice.command = "meu"
        #expect(choice.commandToRun == nil)      // modo app
        choice.mode = .command
        #expect(choice.commandToRun == "meu")
        choice.command = "   "
        #expect(choice.commandToRun == nil)      // em branco vale a linha do app
    }
}
