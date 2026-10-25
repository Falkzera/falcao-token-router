import Foundation

/// As contas do store: a casa de um login novo, o desfecho do login e do
/// relogin, e a remoção (que passa o grupo da conta ativa para a próxima).
/// Arquivo à parte do `RouterConfigStore.swift` pela régua de 600 linhas de
/// produção — um motivo de mudança por arquivo.
extension RouterConfigStore {
    // MARK: - Contas

    /// Reserva a casa de uma conta nova (o perfil onde ela fará login), sem
    /// lançar nada. Quem conduz o login (o app, in-app) usa este caminho; o
    /// resultado é confirmado depois por `finishPendingLogin`.
    public func newAccountHome() -> (home: ConfigDir, accountID: UUID) {
        let id = UUID()
        let home = paths.accountHome(id)
        try? FileManager.default.createDirectory(at: home.url, withIntermediateDirectories: true)
        return (home, id)
    }

    /// Desfaz a casa reservada de um login que não virou conta — cancelado,
    /// fechado, falho, ou que trouxe uma conta que já está no grupo.
    ///
    /// O `claude auth login` grava a credencial na casa ANTES de alguém saber de
    /// quem ela é. Até 10/2026 nada a apagava: cada "Tentar de novo" depois de
    /// uma duplicada deixava mais um refresh token vivo no chaveiro, que o app
    /// não mostrava nem sabia remover.
    ///
    /// Só apaga o que é seguro: casa de uma conta que NÃO está no registro, e
    /// dentro de `<base>/accounts/`. Relogin reusa a casa de uma conta que
    /// existe, e por isso nunca passa daqui.
    public func discardPendingHome(_ home: ConfigDir, accountID: UUID) {
        guard config.account(accountID) == nil,
              home.url.standardizedFileURL.path == paths.accountHome(accountID).url
                .standardizedFileURL.path
        else { return }
        keychain.delete(service: engine.keychainService(for: home, provider: .anthropic))
        try? FileManager.default.removeItem(at: home.url)
    }

    /// Apaga as casas que nenhuma conta do registro cita — sobras de logins
    /// interrompidos (o app fechado no meio, ou anteriores à limpeza do
    /// `discardPendingHome`). Roda na subida do app, quando nenhum login está
    /// em andamento, e só com o `config.json` lido inteiro: com o registro vazio
    /// por falha de leitura, tudo pareceria órfão.
    public func discardOrphanHomes() {
        guard configLoadedFromDisk else { return }
        let accountsDir = paths.base.appending(path: "accounts")
        guard let entries = try? FileManager.default.contentsOfDirectory(
            at: accountsDir, includingPropertiesForKeys: nil) else { return }
        let referenced = Set(config.accounts.map { $0.home.url.standardizedFileURL.path })
        for entry in entries {
            guard let id = UUID(uuidString: entry.lastPathComponent),
                  !referenced.contains(entry.standardizedFileURL.path)
            else { continue }
            discardPendingHome(paths.accountHome(id), accountID: id)
        }
    }

    /// O resultado de checar um login pendente.
    public enum LoginOutcome: Sendable, Equatable {
        /// Ainda não terminou; continue observando.
        case pending
        /// Entrou uma conta nova; foi adicionada ao grupo.
        case added(Account)
        /// O login trouxe uma conta que já está no grupo — quase sempre porque o
        /// navegador ainda estava logado nela. O e-mail diz qual.
        case duplicate(email: String)
    }

    /// Confere se um login pendente terminou. Distingue "ainda não" de "veio a
    /// mesma conta de novo", que é o caso comum quando o navegador não pediu para
    /// escolher a conta.
    @discardableResult
    public func finishPendingLogin(home: ConfigDir, accountID: UUID,
                                   into groupID: UUID, provider: Provider = .anthropic)
        -> LoginOutcome {
        guard let identity = login.loginResult(inHome: home, keychain: keychain) else {
            return .pending
        }
        // A mesma conta já está neste grupo: rejeita e diz o e-mail, para a UI
        // explicar (provavelmente a sessão do navegador não trocou).
        let inGroup = config.groups.first { $0.id == groupID }?.accountIDs ?? []
        if config.accounts.contains(where: {
            $0.identity.email == identity.email && inGroup.contains($0.id)
        }) {
            return .duplicate(email: identity.email)
        }
        let account = Account(id: accountID, provider: provider,
                              identity: identity, home: home)
        config.accounts.append(account)
        if let i = config.groups.firstIndex(where: { $0.id == groupID }) {
            config.groups[i].accountIDs.append(account.id)
        }
        save()
        return .added(account)
    }

    /// O resultado de checar um relogin pendente.
    public enum ReloginOutcome: Sendable, Equatable {
        /// Ainda não terminou; continue observando.
        case pending
        /// A credencial nova entrou, na mesma conta. Se ela estava ativa num
        /// grupo, o item do grupo já recebeu a credencial nova.
        case renewed(Account)
        /// O navegador logou OUTRA conta. Nada do registro muda; a UI explica
        /// e oferece tentar de novo.
        case wrongAccount(expected: String, got: String)
    }

    /// Confere se o relogin de uma conta existente terminou. Relogin reusa a
    /// MESMA casa (o `claude auth login` sobrescreve credencial e identidade),
    /// então sucesso aqui significa: casa com identidade + credencial de novo.
    @discardableResult
    public func finishRelogin(accountID: UUID) -> ReloginOutcome {
        guard let i = config.accounts.firstIndex(where: { $0.id == accountID }),
              let identity = login.loginResult(inHome: config.accounts[i].home,
                                               keychain: keychain)
        else { return .pending }

        let expected = config.accounts[i].identity.email
        guard identity.email == expected else {
            // O `claude auth login` já gravou a credencial da OUTRA conta na casa
            // desta. Deixada lá, ela ficava com o nome desta: "Usar" serviria a
            // outra conta sob este nome, e o sensor carimbaria o consumo no
            // e-mail errado (issue #12). Credencial e identidade estranhas
            // saem; a conta volta a pedir relogin — a credencial que havia ali
            // era a morta que motivou este.
            engine.resetProfile(config.accounts[i].home, provider: config.accounts[i].provider)
            reloginInFlight.remove(accountID)
            return .wrongAccount(expected: expected, got: identity.email)
        }
        reloginInFlight.remove(accountID)
        config.accounts[i].identity = identity  // tier/organização podem ter mudado
        save()
        signedOut[accountID] = nil
        // Conta ativa num grupo: o item do grupo guarda a credencial MORTA que
        // motivou o relogin — a nova precisa ir por cima, senão a próxima
        // sessão continua no "Login expired".
        for group in config.groups
        where engine.activeAccount(in: group, config: config)?.id == accountID {
            engine.pushHomeToGroup(config.accounts[i], in: group)
        }
        refreshUsage()
        return .renewed(config.accounts[i])
    }

    /// Marca o início e o fim de um relogin, para a rotação não espelhar a conta
    /// no meio dele (ver `reloginInFlight`). Idempotentes.
    public func beginRelogin(_ accountID: UUID) { reloginInFlight.insert(accountID) }
    public func endRelogin(_ accountID: UUID) { reloginInFlight.remove(accountID) }

    /// Remove a conta do registro **e apaga a credencial dela**.
    ///
    /// Até 18/09/2026 só o registro saía: o item de chaveiro e a pasta da casa
    /// ficavam para trás, com um refresh token vivo, para sempre. Num produto
    /// pago isso é promessa quebrada — "remover a conta" tem de remover a conta.
    ///
    /// Se ela serve um grupo agora, o grupo passa para a próxima conta (ver
    /// `handOver`) — a troca a quente de sempre, então a sessão viva segue, só
    /// que atendida pela próxima. Até 22/10/2026 o item do grupo ficava
    /// intocado: o grupo seguia servido pela conta removida, e a tela o
    /// mostrava sem conta em uso.
    ///
    /// Sem próxima (era a única, ou nenhuma pode assumir), o item do GRUPO
    /// segue intocado: pode haver sessão viva atendida por ele agora, e
    /// derrubar o trabalho de alguém não é consequência aceitável de arrumar a
    /// lista.
    public func removeAccount(_ accountID: UUID) {
        guard let account = config.account(accountID) else { return }
        let service = engine.homeKeychainService(for: account)
        // Apurado ANTES de a conta sair do registro: depois, a identidade no
        // perfil do grupo não casa com conta nenhuma.
        let served = config.groups
            .filter { engine.activeAccount(in: $0, config: config)?.id == accountID }
            .map(\.id)

        config.accounts.removeAll { $0.id == accountID }
        for i in config.groups.indices {
            config.groups[i].accountIDs.removeAll { $0 == accountID }
        }
        save()
        signedOut[accountID] = nil

        for groupID in served { handOver(groupID) }

        keychain.delete(service: service)
        try? FileManager.default.removeItem(at: account.home.url)
        refreshUsage()
    }

    /// Passa o grupo cuja conta ativa acabou de ser removida para a primeira
    /// de `RotationEngine.successors` que aceitar a ativação — quem não pode
    /// (sem credencial na casa, ativa noutro grupo) é pulada. A marcada sem
    /// login também: a credencial da casa dela a sonda já viu morta, e passar o
    /// grupo para ela derrubaria as sessões abertas que a removida ainda
    /// atende. Sem ninguém, o item do grupo fica como está. Com a conta já
    /// fora do registro, o `activate` não espelha a que sai: a casa dela vai
    /// ser apagada. Se outra conta do registro já está no grupo (a CLI rodou a
    /// rotação no meio), fica a escolha dela.
    private func handOver(_ groupID: UUID) {
        guard let group = config.groups.first(where: { $0.id == groupID }),
              engine.activeAccount(in: group, config: config) == nil
        else { return }
        for candidate in engine.successors(for: group, config: config, usage: usageSnapshot)
        where signedOut[candidate.id] == nil {
            if (try? engine.activate(candidate, in: group, config: config)) != nil { return }
        }
    }
}
