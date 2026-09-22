import Foundation
import Testing
@testable import CCUsageCore

@Suite("SessionLauncher")
struct SessionLauncherTests {
    private func fixture() -> (SessionLauncher, RouterConfig, AccountGroup, Account, Account) {
        let kc = FakeKeychain()
        let adapter = FakeAdapter()
        let a = Account(provider: .anthropic,
                        identity: AccountIdentity(email: "a@k.com", organizationName: "K",
                                                  rateLimitTier: nil, raw: ["emailAddress": .string("a@k.com")]),
                        home: .dedicated("/tmp/.claude-a"))
        let b = Account(provider: .anthropic,
                        identity: AccountIdentity(email: "b@k.com", organizationName: "K",
                                                  rateLimitTier: nil, raw: ["emailAddress": .string("b@k.com")]),
                        home: .dedicated("/tmp/.claude-b"))
        try? kc.write("ca", service: adapter.keychainService(forConfigDir: a.home))
        try? kc.write("cb", service: adapter.keychainService(forConfigDir: b.home))
        let group = AccountGroup(name: "Trabalho", accountIDs: [a.id, b.id],
                                 configDir: .standard(home: "/Users/exemplo"), thresholdPercent: 90)
        let config = RouterConfig(accounts: [a, b], groups: [group])
        let launcher = SessionLauncher(keychain: kc, adapters: [adapter])
        return (launcher, config, group, a, b)
    }

    @Test("acha grupo sem diferenciar caixa nem espaços")
    func findsGroup() {
        let (launcher, config, _, _, _) = fixture()
        #expect(launcher.group(named: " trabalho ", in: config)?.name == "Trabalho")
        #expect(launcher.group(named: "pessoal", in: config) == nil)
    }

    @Test("grupo vazio é erro nomeado")
    func emptyGroup() {
        let kc = FakeKeychain()
        let launcher = SessionLauncher(keychain: kc, adapters: [FakeAdapter()])
        let group = AccountGroup(name: "g", accountIDs: [],
                                 configDir: .standard(home: "/Users/exemplo"))
        let config = RouterConfig(accounts: [], groups: [group])
        #expect(throws: SessionLauncher.LaunchError.emptyGroup("g")) {
            _ = try launcher.prepare(group: group, config: config, usage: [:], arguments: [])
        }
    }

    @Test("começa pela primeira conta quando nenhuma ativou ainda")
    func firstOnColdStart() throws {
        let (launcher, config, group, a, _) = fixture()
        let plan = try launcher.prepare(group: group, config: config, usage: [:], arguments: [])
        #expect(plan.account.id == a.id)
        // grupo padrão não exporta CLAUDE_CONFIG_DIR
        #expect(plan.configDirEnv == nil)
        #expect(plan.executable == "claude")
    }

    @Test("mantém a conta ativa enquanto ela tem folga")
    func keepsActiveWithHeadroom() throws {
        let (launcher, config, group, a, _) = fixture()
        _ = try launcher.prepare(group: group, config: config, usage: [:], arguments: [])  // ativa 'a'
        let plan = try launcher.prepare(group: group, config: config,
                                        usage: [a.id: 0.5], arguments: [])
        #expect(plan.account.id == a.id)
    }

    @Test("pula para a próxima com folga quando a ativa estourou")
    func rotatesOnLaunch() throws {
        let (launcher, config, group, a, b) = fixture()
        _ = try launcher.prepare(group: group, config: config, usage: [:], arguments: [])  // ativa 'a'
        let plan = try launcher.prepare(group: group, config: config,
                                        usage: [a.id: 0.95, b.id: 0.1], arguments: ["--resume"])
        #expect(plan.account.id == b.id)
        #expect(plan.arguments == ["--resume"])
    }

    @Test("grupo dedicado exporta CLAUDE_CONFIG_DIR")
    func dedicatedExportsEnv() throws {
        let kc = FakeKeychain(); let adapter = FakeAdapter()
        let a = Account(provider: .anthropic,
                        identity: AccountIdentity(email: "a@k.com", organizationName: nil,
                                                  rateLimitTier: nil, raw: [:]),
                        home: .dedicated("/tmp/.claude-a"))
        try? kc.write("ca", service: adapter.keychainService(forConfigDir: a.home))
        let group = AccountGroup(name: "pessoal", accountIDs: [a.id],
                                 configDir: .dedicated("/tmp/groups/pessoal"))
        let config = RouterConfig(accounts: [a], groups: [group])
        let launcher = SessionLauncher(keychain: kc, adapters: [adapter])
        let plan = try launcher.prepare(group: group, config: config, usage: [:], arguments: [])
        #expect(plan.configDirEnv == "/tmp/groups/pessoal")
    }
}

@Suite("ShellIntegration")
struct ShellIntegrationTests {
    @Test("statusLine aponta para o sensor")
    func statusLineShape() {
        let cmd = ShellIntegration.statusLineCommand(routerPath: "/Applications/X.app/router")
        #expect((cmd["command"] as? String)?.contains("statusline") == true)
        #expect((cmd["command"] as? String)?.contains("/Applications/X.app/router") == true)
    }

    @Test("instalar a statusLine preserva as outras chaves")
    func preservesSettings() throws {
        var written: Data?
        let existing = try JSONSerialization.data(withJSONObject: [
            "model": "claude-fable-5[1m]", "theme": "dark",
        ])
        try ShellIntegration.installStatusLine(
            routerPath: "/x/router",
            into: .dedicated("/tmp/g"),
            readFile: { _ in existing },
            writeFile: { data, _ in written = data })

        let root = try JSONSerialization.jsonObject(with: #require(written)) as! [String: Any]
        #expect(root["model"] as? String == "claude-fable-5[1m]")
        #expect(root["theme"] as? String == "dark")
        #expect((root["statusLine"] as? [String: Any])?["command"] != nil)
    }

    /// O caso que motivou a checagem: o `.app` foi renomeado de
    /// `ClaudeTokenCounter` para `FalcaoTokenRouter` em 18/09/2026 e as três
    /// peças da integração continuaram apontando para o caminho morto.
    @Test("statusLine de outro binário é obsoleta; a do binário atual não é")
    func statusLineStaleness() throws {
        let velho = try JSONSerialization.data(withJSONObject: [
            "statusLine": ["type": "command", "padding": 0,
                           "command": "'/Applications/ClaudeTokenCounter.app/Contents/MacOS/router' statusline"],
        ])
        #expect(ShellIntegration.statusLineIsStale(
            routerPath: "/Applications/FalcaoTokenRouter.app/Contents/MacOS/router",
            in: .dedicated("/tmp/g"), readFile: { _ in velho }))

        let novo = try JSONSerialization.data(withJSONObject: [
            "statusLine": ["type": "command", "padding": 0,
                           "command": "'/Applications/FalcaoTokenRouter.app/Contents/MacOS/router' statusline"],
        ])
        #expect(!ShellIntegration.statusLineIsStale(
            routerPath: "/Applications/FalcaoTokenRouter.app/Contents/MacOS/router",
            in: .dedicated("/tmp/g"), readFile: { _ in novo }))
    }

    /// Perfil sem `settings.json` nenhum conta como obsoleto: o sensor não está
    /// plantado ali, que é o mesmo resultado prático de apontar para o lugar
    /// errado — a conta fica sem medição e o rodízio decide às cegas.
    @Test("perfil sem statusLine conta como obsoleto")
    func missingStatusLineIsStale() {
        #expect(ShellIntegration.statusLineIsStale(
            routerPath: "/x/router", in: .dedicated("/tmp/g"), readFile: { _ in nil }))
    }

    @Test("a função de shell embute o caminho do binário")
    func shellFunctionHasPath() {
        let fn = ShellIntegration.shellFunction(routerPath: "/Applications/Falcão Router.app/router")
        #expect(fn.contains("claude()"))
        #expect(fn.contains("is-group"))
        #expect(fn.contains("launch"))
        #expect(fn.contains("Falcão Router.app/router"))
    }
}

@Suite("ShellIntegration profile")
struct ShellProfileTests {
    private func tempProfile(_ contents: String = "") -> URL {
        let url = URL(fileURLWithPath: NSTemporaryDirectory())
            .appending(path: "zshrc-\(UUID().uuidString)")
        try? contents.write(to: url, atomically: true, encoding: .utf8)
        return url
    }

    @Test("adiciona a linha uma vez e não duplica")
    func idempotent() throws {
        let profile = tempProfile("# meu zshrc\nexport PATH=/usr/bin\n")
        let script = "/Applications/X.app/Contents/MacOS/router"
        let line = "source \"\(script)/shell.sh\""

        let first = try ShellIntegration.ensureInProfile(
            sourceLine: line, scriptPath: "\(script)/shell.sh", profileURL: profile)
        #expect(first == .added)

        let second = try ShellIntegration.ensureInProfile(
            sourceLine: line, scriptPath: "\(script)/shell.sh", profileURL: profile)
        #expect(second == .alreadyPresent)

        let content = try String(contentsOf: profile, encoding: .utf8)
        // O conteúdo original ficou; a linha entrou exatamente uma vez.
        #expect(content.contains("# meu zshrc"))
        #expect(content.components(separatedBy: "shell.sh\"").count - 1 == 1)
    }

    @Test("cria o arquivo se ele não existe")
    func createsIfMissing() throws {
        let profile = URL(fileURLWithPath: NSTemporaryDirectory())
            .appending(path: "zshrc-missing-\(UUID().uuidString)")
        let out = try ShellIntegration.ensureInProfile(
            sourceLine: "source \"/x/shell.sh\"", scriptPath: "/x/shell.sh", profileURL: profile)
        #expect(out == .added)
        #expect(FileManager.default.fileExists(atPath: profile.path))
        try? FileManager.default.removeItem(at: profile)
    }
}

@Suite("ProfileSharing")
struct ProfileSharingTests {
    private func makeHome() -> (home: ConfigDir, base: URL) {
        let base = URL(fileURLWithPath: NSTemporaryDirectory())
            .appending(path: "share-\(UUID().uuidString)")
        let homeDir = base.appending(path: ".claude")
        let fm = FileManager.default
        try? fm.createDirectory(at: homeDir.appending(path: "projects"), withIntermediateDirectories: true)
        try? fm.createDirectory(at: homeDir.appending(path: "skills"), withIntermediateDirectories: true)
        try? "x".write(to: homeDir.appending(path: "history.jsonl"), atomically: true, encoding: .utf8)
        try? "y".write(to: homeDir.appending(path: "CLAUDE.md"), atomically: true, encoding: .utf8)
        return (ConfigDir.dedicated(homeDir.path), base)
    }

    @Test("linka projects e history quando o histórico é compartilhado")
    func linksHistory() throws {
        let (home, base) = makeHome()
        let group = ConfigDir.dedicated(base.appending(path: "grupo").path)
        ProfileSharing.link(into: group, home: home, shareHistory: true)

        let fm = FileManager.default
        for name in ["projects", "history.jsonl", "skills", "CLAUDE.md"] {
            let link = group.url.appending(path: name)
            #expect(fm.fileExists(atPath: link.path), "faltou \(name)")
            let isLink = (try? link.resourceValues(forKeys: [.isSymbolicLinkKey]))?.isSymbolicLink
            #expect(isLink == true, "\(name) devia ser symlink")
        }
        try? fm.removeItem(at: base)
    }

    @Test("sem histórico compartilhado, projects não é linkado")
    func skipsHistoryWhenOff() throws {
        let (home, base) = makeHome()
        let group = ConfigDir.dedicated(base.appending(path: "grupo").path)
        ProfileSharing.link(into: group, home: home, shareHistory: false)

        let fm = FileManager.default
        #expect(!fm.fileExists(atPath: group.url.appending(path: "projects").path))
        // mas skills (não é histórico) segue vindo
        #expect(fm.fileExists(atPath: group.url.appending(path: "skills").path))
        try? fm.removeItem(at: base)
    }

    @Test("não mexe no grupo padrão (seria linkar para si mesmo)")
    func skipsDefault() {
        ProfileSharing.link(into: .standard(home: "/tmp/whatever-xyz"), shareHistory: true)
        // Não cria nada em ~/.claude real; só não deve crashar.
        #expect(Bool(true))
    }
}

@Suite("ensureInProfile — integridade do ~/.zshrc")
struct ProfileIntegrityTests {
    /// Um `~/.zshrc` que não é UTF-8 válido. Acontece de verdade: um alias com
    /// acento salvo em latin-1, um arquivo tocado por um editor antigo.
    @Test("profile ilegível não pode ser sobrescrito")
    func doesNotDestroyUnreadableProfile() throws {
        let url = URL(fileURLWithPath: NSTemporaryDirectory())
            .appending(path: "zshrc-bin-\(UUID().uuidString)")
        // `alias café='echo oi'` com o "é" em latin-1 (0xE9) — byte inválido em UTF-8.
        var bytes = Data("alias caf".utf8)
        bytes.append(0xE9)
        bytes.append(contentsOf: Data("='echo oi'\nexport PATH=/meu/bin:$PATH\n".utf8))
        try bytes.write(to: url)
        let antes = try Data(contentsOf: url)

        _ = try? ShellIntegration.ensureInProfile(
            sourceLine: "source /x/shell.sh", scriptPath: "/x/shell.sh", profileURL: url)

        let depois = try Data(contentsOf: url)
        // O conteúdo original tem de continuar lá. Sobrescrever é perda de dados.
        #expect(depois.count >= antes.count,
                "o ~/.zshrc encolheu: o conteúdo do usuário foi destruído")
        #expect(depois.starts(with: antes),
                "o conteúdo original não foi preservado")
    }
}
