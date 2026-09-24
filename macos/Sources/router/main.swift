import Foundation
import CCUsageCore

/// A CLI que o app empacota e que faz a ponte com o terminal.
///
///   router statusline      — o sensor: lê `rate_limits` do stdin e grava por
///                            conta. Nenhuma chamada de rede, nenhum token nosso.
///   router launch <grupo>  — ativa a melhor conta do grupo e sobe o `claude`.
///   router is-group <nome> — código 0 se `<nome>` é um grupo (para o shell).
///   router rotate          — uma volta da rotação (para um LaunchAgent).
///   router doctor          — confere a instalação e diz o que está torto.
///   router measure         — a SONDA: pergunta o uso ao binário oficial,
///                            inclusive o limite POR MODELO que o sensor não vê.
///
/// Tudo best-effort do lado do sensor (não pode travar a status line do usuário);
/// o resto reporta erro e sai com código diferente de zero.

let args = Array(CommandLine.arguments.dropFirst())
let command = args.first ?? "statusline"

switch command {
case "statusline":
    Statusline.run()
case "launch":
    Launcher.run(Array(args.dropFirst()))
case "is-group":
    exit(Launcher.isGroup(args.dropFirst().first) ? 0 : 1)
case "rotate":
    Launcher.rotate()
case "doctor":
    exit(Doctor.run() ? 0 : 1)
case "measure":
    exit(Measure.run(Array(args.dropFirst())) ? 0 : 1)
default:
    FileHandle.standardError.write(Data("uso: router [statusline|launch <grupo>|is-group <nome>|rotate|doctor|measure [grupo]]\n".utf8))
    exit(2)
}

// MARK: - Sensor

enum Statusline {
    static func run() {
        let input = readStdin()
        let dir = currentConfigDir()
        let adapter = AnthropicAdapter()
        let email = adapter.identity(inConfigDir: dir)?.email

        let limits = input["rate_limits"] as? [String: Any] ?? [:]
        let five = window(limits, "five_hour")
        let seven = window(limits, "seven_day")

        if let email {
            let sample = GroupUsageSample(
                configDirRaw: dir.raw, email: email,
                fiveHourPercent: five.pct, fiveHourResetsAt: five.resets,
                sevenDayPercent: seven.pct, sevenDayResetsAt: seven.resets,
                sampledAt: Date(), origin: .sensor)
            try? GroupUsageStore.write(sample, forEmail: email,
                                      in: GroupUsageStore.directory(bundleID: RouterPaths.bundleID))
        }

        // A LINHA. Até 24/09/2026 daqui saía `conta 5h 7d` — e como o router é
        // dono da `statusLine` do perfil (a linha é o sensor), quem tinha a sua
        // a perdia ao ativar a integração. Agora sai a completa, com o grupo na
        // frente e o e-mail da conta ativa no fim, que é onde a troca aparece.
        let config = Launcher.loadConfig()
        let view = StatusLineSession.view(from: input, dir: dir, config: config, email: email)
        // A escolha é relida a CADA render, de propósito: mudar um item nos
        // Ajustes tem efeito na próxima atualização da sessão, sem reabrir nada.
        // Ausente ou ilegível vale a completa — a linha nunca falha por causa
        // dela, porque sem a linha não há sensor.
        let choice = StatusLineChoice.load(from: StatusLineChoice.fileURL(base: RouterPaths().base))
        let style = StatusLineView.Style(
            trueColor: StatusLineSource.trueColor(),
            portuguese: Locale.current.language.languageCode?.identifier == "pt",
            // A fase das animações vem do relógio: uma volta por segundo.
            phase: UInt64(Date().timeIntervalSince1970))
        print(choice.apply(view).render(style))
    }

    static func readStdin() -> [String: Any] {
        let data = FileHandle.standardInput.readDataToEndOfFile()
        guard !data.isEmpty,
              let obj = try? JSONSerialization.jsonObject(with: data) as? [String: Any]
        else { return [:] }
        return obj
    }

    static func currentConfigDir() -> ConfigDir {
        if let raw = ProcessInfo.processInfo.environment["CLAUDE_CONFIG_DIR"], !raw.isEmpty {
            return .dedicated(raw)
        }
        return .standard()
    }

    static func window(_ limits: [String: Any], _ key: String) -> (pct: Double?, resets: Date?) {
        guard let w = limits[key] as? [String: Any] else { return (nil, nil) }
        return ((w["used_percentage"] as? Double).map { $0 / 100 },
                (w["resets_at"] as? Double).map { Date(timeIntervalSince1970: $0) })
    }

}

// MARK: - Lançador e rotação

enum Launcher {
    static func loadConfig() -> RouterConfig? {
        guard let data = try? Data(contentsOf: RouterPaths().configFile) else { return nil }
        return try? JSONDecoder().decode(RouterConfig.self, from: data)
    }

    static func isGroup(_ name: String?) -> Bool {
        guard let name, let config = loadConfig() else { return false }
        let launcher = SessionLauncher(keychain: SecurityCLIKeychain())
        return launcher.group(named: name, in: config) != nil
    }

    /// `router launch <grupo> -- <args do claude>`
    static func run(_ argv: [String]) {
        guard let name = argv.first else { fail("uso: router launch <grupo>") }
        guard let config = loadConfig() else { fail("configuração não encontrada — crie um grupo no app") }

        // Tudo depois de `--` vai para o claude.
        let passthrough: [String]
        if let dash = argv.firstIndex(of: "--") { passthrough = Array(argv[(dash + 1)...]) }
        else { passthrough = Array(argv.dropFirst()) }

        let keychain = SecurityCLIKeychain()
        let launcher = SessionLauncher(keychain: keychain)
        guard let group = launcher.group(named: name, in: config) else {
            fail("grupo desconhecido: \(name)")
        }
        let usage = GroupUsageReader(usageDir: RouterPaths().usageDir).usageByAccount(config)

        do {
            let plan = try launcher.prepare(group: group, config: config,
                                            usage: usage, arguments: passthrough)
            // Garante o sensor no perfil do grupo, para ESTA sessão já registrar
            // o uso — senão a conta fica sem número no painel até alguém instalar
            // a integração à mão. Preserva o settings.json que houver.
            if let selfPath = selfExecutablePath() {
                try? ShellIntegration.installStatusLine(
                    routerPath: selfPath, into: group.configDir)
            }
            // Compartilha histórico e skills do ~/.claude, para o `--resume`
            // enxergar as conversas e as ferramentas serem as mesmas.
            ProfileSharing.link(into: group.configDir, shareHistory: config.shareHistory)
            // Herda o ambiente, mas SEM proxy: se a máquina ainda tem o
            // teamclaude exportado, a sessão passaria por ele e seria servida
            // pela conta que ele fixa, não pela do grupo. Seta CLAUDE_CONFIG_DIR
            // só quando o grupo não é o padrão. `execvp` substitui este processo
            // pelo claude, que herda o PID do shell — igual ao `claude` normal.
            var env = ProviderEnv.direct(ProcessInfo.processInfo.environment)
            if let dir = plan.configDirEnv { env["CLAUDE_CONFIG_DIR"] = dir }
            else { env.removeValue(forKey: "CLAUDE_CONFIG_DIR") }
            FileHandle.standardError.write(Data(
                "→ \(group.name): \(plan.account.label)\n".utf8))
            execvp(plan.executable, [plan.executable] + plan.arguments, env)
            fail("não foi possível executar \(plan.executable)")
        } catch {
            fail("\(error)")
        }
    }

    /// Uma volta da rotação em todos os grupos, para um agente periódico.
    static func rotate() {
        guard let config = loadConfig() else { return }
        let keychain = SecurityCLIKeychain()
        let engine = RotationEngine(keychain: keychain)
        let usage = GroupUsageReader(usageDir: RouterPaths().usageDir).usageByAccount(config)
        for group in config.groups {
            engine.mirrorActive(in: group, config: config)
            guard let target = engine.rotationTarget(for: group, config: config, usage: usage)
            else { continue }
            try? engine.activate(target, in: group, config: config)
        }
    }

    static func fail(_ message: String) -> Never {
        FileHandle.standardError.write(Data("router: \(message)\n".utf8))
        exit(1)
    }
}

/// Reimplementa `execvp` com ambiente explícito (a libc `execvpe` não existe no
/// Darwin). Procura no PATH, monta o `environ` e troca a imagem do processo.
@discardableResult
func execvp(_ file: String, _ argv: [String], _ environment: [String: String]) -> Int32 {
    let path = resolveInPath(file, environment: environment) ?? file
    var cArgs: [UnsafeMutablePointer<CChar>?] = argv.map { strdup($0) }
    cArgs.append(nil)
    var cEnv: [UnsafeMutablePointer<CChar>?] = environment.map { strdup("\($0)=\($1)") }
    cEnv.append(nil)
    return execve(path, &cArgs, &cEnv)
}

/// O caminho absoluto deste próprio binário `router`, para apontar a status line.
func selfExecutablePath() -> String? {
    if let path = Bundle.main.executablePath,
       FileManager.default.isExecutableFile(atPath: path) { return path }
    let arg0 = CommandLine.arguments.first ?? "router"
    if arg0.contains("/") {
        let resolved = URL(fileURLWithPath: arg0).standardizedFileURL.path
        if FileManager.default.isExecutableFile(atPath: resolved) { return resolved }
    }
    return resolveInPath(arg0, environment: ProcessInfo.processInfo.environment)
}

func resolveInPath(_ file: String, environment: [String: String]) -> String? {
    if file.contains("/") { return file }
    let path = environment["PATH"] ?? "/usr/bin:/bin:/usr/local/bin:/opt/homebrew/bin"
    for dir in path.split(separator: ":") {
        let candidate = "\(dir)/\(file)"
        if FileManager.default.isExecutableFile(atPath: candidate) { return candidate }
    }
    return nil
}

// MARK: - Diagnóstico

/// Confere a instalação e nomeia o que está torto, em vez de deixar o usuário
/// descobrir por um sintoma que não parece com a causa.
///
/// Existe porque os modos de falha daqui são **silenciosos**: a função de shell
/// apontando para um `.app` que mudou de nome não dá erro nenhum — `claude
/// trabalho` simplesmente abre no `~/.claude`, na conta errada. Nenhum dos
/// sinais visíveis aponta para o caminho absoluto gravado num arquivo gerado.
enum Doctor {
    static func descreve(_ status: LiveSession.Status) -> String {
        switch status {
        case .busy: "trabalhando"
        case .waiting: "esperando você"
        case .idle: "ociosa"
        case .shell: "shell"
        case .other(let cru): cru.isEmpty ? "?" : cru
        }
    }

    static func run() -> Bool {
        var ok = true
        func diga(_ bom: Bool, _ texto: String) {
            print("\(bom ? "  ok  " : "  !!  ")\(texto)")
            if !bom { ok = false }
        }

        let paths = RouterPaths()
        print("router doctor")
        print("  base: \(paths.base.path)")

        // 1. O binário que a integração DEVERIA citar é este que está rodando.
        guard let eu = selfExecutablePath() else {
            diga(false, "não sei o meu próprio caminho")
            return false
        }
        print("  binário: \(eu)")

        // 2. A configuração.
        guard let config = Launcher.loadConfig() else {
            diga(false, "sem config.json — crie um grupo no app")
            return false
        }
        diga(true, "config: \(config.groups.count) grupo(s), \(config.accounts.count) conta(s)")

        // 3. A função de shell: existe, e cita ESTE binário?
        let shell = paths.base.appending(path: "shell.sh")
        let script = (try? String(contentsOf: shell, encoding: .utf8)) ?? ""
        if script.isEmpty {
            diga(false, "shell.sh ausente — Grupos → Integração com o terminal → Ativar")
        } else {
            diga(script.contains(eu),
                 script.contains(eu)
                 ? "shell.sh aponta para este binário"
                 : "shell.sh aponta para OUTRO binário (app renomeado/movido) — reinstale a integração")
        }

        // 4. A linha no profile do shell.
        let zshrc = FileManager.default.homeDirectoryForCurrentUser
            .appending(path: ".zshrc").resolvingSymlinksInPath()
        let profile = (try? String(contentsOf: zshrc, encoding: .utf8)) ?? ""
        diga(profile.contains(shell.path), "~/.zshrc dá source no shell.sh")

        // 5. O sensor, perfil a perfil. Sem ele a conta não tem número e o
        //    rodízio decide às cegas.
        for group in config.groups {
            let stale = ShellIntegration.statusLineIsStale(routerPath: eu, in: group.configDir)
            diga(!stale, "sensor no grupo \(group.name): \(stale ? "ausente ou apontando para outro binário" : "instalado")")
        }

        // 6. Quem serve cada grupo, e há quanto tempo foi medida.
        let engine = RotationEngine(keychain: SecurityCLIKeychain())
        let uso = GroupUsageReader(usageDir: paths.usageDir).detailByAccount(config)
        for group in config.groups {
            guard let ativa = engine.activeAccount(in: group, config: config) else {
                diga(false, "grupo \(group.name): nenhuma conta ativa")
                continue
            }
            if let u = uso[ativa.id] {
                let idade = Int(Date().timeIntervalSince(u.sampledAt) / 60)
                let quem = u.origin == .probe ? "sondado" : "sensor"
                diga(idade < 720,
                     "grupo \(group.name): \(ativa.label) — \(UsagePercent.text(u.fraction)) (\(quem), \(idade) min)")
            } else {
                diga(true, "grupo \(group.name): \(ativa.label) — sem amostra ainda (pronta)")
            }
        }

        // 7. As sessões vivas, por grupo. É onde o modo de falha silencioso
        //    aparece: sessão que devia estar num grupo e subiu no perfil padrão
        //    fica do lado errado da conta.
        for group in config.groups {
            let vivas = SessionRegistry.liveSessions(in: group.configDir)
            guard !vivas.isEmpty else { continue }
            let conta = engine.activeAccount(in: group, config: config)?.label ?? "?"
            print("  --    \(vivas.count) sessão(ões) em \(group.name) → \(conta)")
            for s in vivas.prefix(8) {
                print("          pid \(s.pid)  \(s.label)  [\(descreve(s.status))]")
            }
            if vivas.count > 8 { print("          … e mais \(vivas.count - 8)") }
        }

        // 8. Conta ativa em mais de um grupo: duas cópias de um refresh token
        //    que gira, que é a falha que já matou conta aqui.
        var vistas: [UUID: String] = [:]
        for group in config.groups {
            guard let ativa = engine.activeAccount(in: group, config: config) else { continue }
            if let outro = vistas[ativa.id] {
                diga(false, "\(ativa.label) está ativa em DOIS grupos (\(outro) e \(group.name)) — risco de matar a credencial")
            }
            vistas[ativa.id] = group.name
        }

        print(ok ? "\ntudo certo." : "\nhá problemas acima.")
        return ok
    }
}

// MARK: - Sonda ativa

/// `router measure [grupo]` — mede as contas perguntando ao binário oficial.
///
/// O sensor passivo só enxerga quem está servindo, e só as janelas de 5h e 7
/// dias. Esta sonda cobre as duas lacunas: mede **conta ociosa** (que antes
/// aparecia como "pronta", sem número nenhum) e traz o limite **por modelo**,
/// que é o que estoura primeiro.
///
/// Custa um processo Node por conta e uma requisição de verdade em cada uma,
/// então é comando, não laço.
enum Measure {
    static func run(_ argv: [String]) -> Bool {
        guard let config = Launcher.loadConfig() else {
            FileHandle.standardError.write(Data("router: configuração não encontrada\n".utf8))
            return false
        }
        guard let probe = ClaudeUsageProbe.system() else {
            FileHandle.standardError.write(Data(
                "router: binário `claude` não encontrado — a sonda precisa dele\n".utf8))
            return false
        }

        let launcher = SessionLauncher(keychain: SecurityCLIKeychain())
        let engine = RotationEngine(keychain: SecurityCLIKeychain())
        let alvo = argv.first.flatMap { launcher.group(named: $0, in: config) }
        if let nome = argv.first, alvo == nil {
            FileHandle.standardError.write(Data("router: grupo desconhecido: \(nome)\n".utf8))
            return false
        }
        let grupos = alvo.map { [$0] } ?? config.groups
        let contas = grupos.flatMap { config.accounts(in: $0) }
            .reduce(into: [Account]()) { acc, c in if !acc.contains(where: { $0.id == c.id }) { acc.append(c) } }

        guard !contas.isEmpty else {
            print("nenhuma conta para medir.")
            return true
        }

        let dir = RouterPaths().usageDir
        var falhas = 0
        for conta in contas {
            // A regra que não pode ser quebrada: conta ativa é sondada pelo
            // perfil do GRUPO, nunca pela casa. Ver `probeConfigDir`.
            let perfil = engine.probeConfigDir(for: conta, config: config)
            do {
                let leitura = try probe.read(configDir: perfil)
                let agora = Date()
                let amostra = GroupUsageSample(
                    configDirRaw: perfil.raw, email: conta.identity.email,
                    fiveHourPercent: leitura.session, fiveHourResetsAt: leitura.sessionResetsAt,
                    sevenDayPercent: leitura.weeklyAll, sevenDayResetsAt: leitura.weeklyAllResetsAt,
                    sampledAt: agora,
                    models: ModelUsage(windows: leitura.models, sampledAt: agora),
                    origin: .probe)
                try GroupUsageStore.write(amostra, forEmail: conta.identity.email, in: dir)
                print("  \(conta.label): \(descreve(leitura))")
            } catch ClaudeUsageProbe.ProbeError.notSignedIn {
                print("  \(conta.label): sem login neste perfil — use Relogar no app")
                falhas += 1
            } catch {
                print("  \(conta.label): falhou (\(error))")
                falhas += 1
            }
        }
        return falhas == 0
    }

    static func descreve(_ r: ClaudeUsageProbe.Reading) -> String {
        var partes: [String] = []
        if let s = r.session { partes.append("5h \(UsagePercent.text(s))") }
        if let w = r.weeklyAll { partes.append("7d \(UsagePercent.text(w))") }
        for m in r.models { partes.append("\(m.name) \(UsagePercent.text(m.percent))") }
        return partes.isEmpty ? "sem janelas" : partes.joined(separator: "  ")
    }
}
