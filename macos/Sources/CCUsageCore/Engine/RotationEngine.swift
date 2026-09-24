import Foundation

/// O ato de trocar a conta ativa de um grupo, e as regras que impedem os erros
/// que já mataram contas neste setup.
///
/// A troca em si é uma cópia: o segredo da **casa** da conta (o item de chaveiro
/// onde ela fez login) vai para o item do **grupo**, e a identidade é gravada no
/// `.claude.json` do grupo. A sessão viva relê o item de chaveiro na requisição
/// seguinte e passa a atender pela conta nova — sem reiniciar, sem `--resume`.
///
/// Duas regras são o que separa isto do rodízio caseiro que falhou:
///
/// - **Espelhar antes de trocar.** Enquanto uma conta está ativa num grupo, é o
///   item do grupo que o Claude Code renova; a casa fica para trás. Antes de
///   ativar outra conta, o token fresco do grupo é copiado de volta para a casa
///   da conta que sai. Assim a casa é sempre a verdade quando a conta está
///   ociosa, e nunca se ativa uma cópia vencida.
///
/// - **Uma conta, um lugar.** A mesma conta em dois grupos ativos seria duas
///   cópias de um refresh token que gira — a falha silenciosa que derrubou
///   contas aqui. O motor recusa ativar uma conta que já está ativa noutro grupo.
public struct RotationEngine: Sendable {
    public enum RotationError: Error, Equatable {
        /// A conta não tem credencial legível na casa. Saída: relogar.
        case noCredential(accountID: UUID)
        /// A conta já serve outro grupo agora. Ativar aqui duplicaria o token.
        case accountBusyElsewhere(accountID: UUID, groupID: UUID)
        /// Falha ao escrever chaveiro ou `.claude.json`.
        case writeFailed(String)
    }

    private let keychain: any KeychainStore
    private let adapters: [Provider: any ProviderAdapter]

    public init(keychain: any KeychainStore,
                adapters: [any ProviderAdapter] = [AnthropicAdapter()]) {
        self.keychain = keychain
        self.adapters = Dictionary(uniqueKeysWithValues: adapters.map { ($0.provider, $0) })
    }

    private func adapter(for provider: Provider) -> any ProviderAdapter {
        adapters[provider] ?? AnthropicAdapter()
    }

    /// A conta que serve um grupo agora, comparando a identidade gravada no
    /// perfil do grupo com as contas do grupo. `nil` se o grupo está vazio ou
    /// não tem identidade legível.
    public func activeAccount(in group: AccountGroup, config: RouterConfig) -> Account? {
        let adapter = adapter(for: group.provider)
        guard let identity = adapter.identity(inConfigDir: group.configDir) else { return nil }
        return config.accounts(in: group).first { $0.identity.email == identity.email }
    }

    /// Ativa uma conta no grupo: espelha a que sai, recusa duplicação, copia o
    /// segredo para o item do grupo e grava a identidade.
    ///
    /// Idempotente: reativar a conta que já está ativa não faz nada além de
    /// garantir que o item do grupo está em dia.
    @discardableResult
    public func activate(_ account: Account, in group: AccountGroup,
                         config: RouterConfig) throws -> Account {
        let adapter = adapter(for: group.provider)
        let groupService = adapter.keychainService(forConfigDir: group.configDir)

        // Já é a ativa deste grupo? Então o item do GRUPO é a cópia viva — é
        // nele que o Claude Code renova, e o refresh token GIRA a cada
        // renovação. Copiar a casa por cima mataria o token girado (o "Login
        // expired" de 26/ago). O movimento certo é o inverso: vivo → casa.
        if let current = activeAccount(in: group, config: config), current.id == account.id {
            mirrorGroupToHome(account, groupService: groupService)
            return account
        }

        // Recusa duplicação: a conta não pode estar ativa em OUTRO grupo.
        for other in config.groups where other.id != group.id {
            if let busy = activeAccount(in: other, config: config), busy.id == account.id {
                throw RotationError.accountBusyElsewhere(accountID: account.id, groupID: other.id)
            }
        }

        // Espelha a conta que sai: o token fresco do grupo volta para a casa
        // dela, para nunca ativarmos uma cópia vencida mais tarde.
        if let leaving = activeAccount(in: group, config: config), leaving.id != account.id {
            mirrorGroupToHome(leaving, groupService: groupService)
        }

        // Copia o segredo da casa da conta para o item do grupo.
        let homeService = adapter.keychainService(forConfigDir: account.home)
        guard let secret = keychain.read(service: homeService) else {
            throw RotationError.noCredential(accountID: account.id)
        }
        do {
            try keychain.write(secret, service: groupService)
            try adapter.writeIdentity(account.identity, toConfigDir: group.configDir)
        } catch {
            throw RotationError.writeFailed("\(error)")
        }
        return account
    }

    /// Copia o segredo vivo do item do grupo de volta para a casa da conta.
    ///
    /// Chamado antes de cada troca, e também num ciclo periódico enquanto a conta
    /// está ativa, para que a casa nunca fique muito atrás do token que gira.
    public func mirrorGroupToHome(_ account: Account, groupService: String) {
        let adapter = adapter(for: account.provider)
        let homeService = adapter.keychainService(forConfigDir: account.home)
        guard let fresh = keychain.read(service: groupService) else { return }
        // Só escreve se mudou, para não reabrir o item à toa.
        if keychain.read(service: homeService) != fresh {
            try? keychain.write(fresh, service: homeService)
        }
    }

    /// Depois de um RELOGIN de conta ativa: a casa tem a credencial nova e o
    /// item do grupo guarda a morta que motivou o relogin — o único caso em que
    /// casa→grupo com a conta já ativa é o movimento certo. (O `activate`
    /// normal faz o inverso de propósito.)
    public func pushHomeToGroup(_ account: Account, in group: AccountGroup) {
        let adapter = adapter(for: group.provider)
        guard let secret = keychain.read(
            service: adapter.keychainService(forConfigDir: account.home)) else { return }
        let groupService = adapter.keychainService(forConfigDir: group.configDir)
        try? keychain.write(secret, service: groupService)
        try? adapter.writeIdentity(account.identity, toConfigDir: group.configDir)
    }

    /// O ciclo periódico do espelhamento: o token vivo do grupo volta para a
    /// casa da conta ativa. Sem isto a casa apodrece — o refresh token dela é
    /// invalidado na primeira renovação que o Claude Code faz no item do grupo.
    public func mirrorActive(in group: AccountGroup, config: RouterConfig) {
        guard let active = activeAccount(in: group, config: config) else { return }
        let service = adapter(for: group.provider).keychainService(forConfigDir: group.configDir)
        mirrorGroupToHome(active, groupService: service)
    }

    /// O item de chaveiro onde mora a credencial-mãe de uma conta.
    ///
    /// Pelo **provedor da conta**, e não por um adapter fixo: o store precisava
    /// disso para apagar a credencial ao remover a conta, e chamar
    /// `AnthropicAdapter()` direto ali fazia o nome do item divergir do que o
    /// motor usa em qualquer outro provedor — apagando nada, em silêncio.
    public func homeKeychainService(for account: Account) -> String {
        adapter(for: account.provider).keychainService(forConfigDir: account.home)
    }

    /// Por qual perfil sondar esta conta — e **nunca** pela casa de uma conta
    /// ativa.
    ///
    /// A regra não é higiene, é a mesma que separa este motor do rodízio caseiro
    /// que matou contas. Enquanto a conta serve um grupo, quem tem a credencial
    /// viva é o item do GRUPO — é nele que o Claude Code renova, e o refresh
    /// token **gira** a cada renovação. A casa fica com uma cópia atrás.
    ///
    /// Sondar a casa de uma conta ativa faria o `claude` tentar renovar com o
    /// refresh token velho. Se ele ainda valer, a renovação gira a cadeia e
    /// **invalida a cópia do grupo** — a sessão viva do usuário cai em "Login
    /// expired", do nada, no meio do trabalho. É o episódio de 26/ago com outro
    /// gatilho.
    ///
    /// Conta ociosa é o caso seguro e o mais útil: a casa é a única cópia, e
    /// sondá-la ainda renova o token dela, que é o contrário de apodrecer.
    public func probeConfigDir(for account: Account, config: RouterConfig) -> ConfigDir {
        for group in config.groups
        where activeAccount(in: group, config: config)?.id == account.id {
            return group.configDir
        }
        return account.home
    }

    /// A próxima conta que o grupo deve usar, dado o uso de cada uma.
    ///
    /// Regra: a primeira conta na ordem de preferência cujo uso conhecido está
    /// **abaixo do limiar** — ou sem medição nenhuma, presumida fresca. Se
    /// nenhuma qualifica, devolve `nil`, e quem chama mantém a atual (fail-safe:
    /// pior é "não trocou", nunca "travou").
    public func nextAccount(for group: AccountGroup, config: RouterConfig,
                            usage: [UUID: Double]) -> Account? {
        let threshold = group.thresholdPercent / 100
        return config.accounts(in: group).first { account in
            // Sem amostra = presumida fresca. No sensor passivo, conta nunca
            // usada não tem medição — exigir amostra criava um deadlock (só
            // mede quem serve; só serve quem é escolhida). Se a presunção
            // errar, a primeira mensagem dela mede e o laço seguinte corrige.
            usage[account.id].map { $0 < threshold } ?? true
        }
    }

    /// Decide se um grupo deve trocar agora, e para qual conta.
    ///
    /// Com histerese: só troca se a conta ativa passou do limiar **e** existe um
    /// destino melhor. Sem a ativa passar do limiar, fica onde está mesmo que
    /// outra conta esteja mais folgada — trocar por pouco só reconstrói cache de
    /// contexto à toa.
    public func rotationTarget(for group: AccountGroup, config: RouterConfig,
                               usage: [UUID: Double]) -> Account? {
        guard group.autoRotate else { return nil }
        let threshold = group.thresholdPercent / 100
        let active = activeAccount(in: group, config: config)

        if let active, let used = usage[active.id], used < threshold {
            return nil  // ativa ainda tem folga; não mexe
        }
        guard let target = nextAccount(for: group, config: config, usage: usage) else {
            return nil  // ninguém qualifica; mantém a atual
        }
        return target.id == active?.id ? nil : target
    }
}
