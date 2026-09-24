import Foundation

/// Um grupo de contas que rotacionam entre si.
///
/// É o coração do que o produto faz e o `cswap` não faz: cada grupo tem o seu
/// perfil e a sua lista ordenada, então vários grupos rotacionam em paralelo —
/// trabalho, pessoal, faculdade — sem um pisar no outro. O usuário cria, nomeia,
/// ordena e liga/desliga cada um pela UI.
public struct AccountGroup: Sendable, Equatable, Codable, Identifiable {
    public let id: UUID
    public var name: String
    public let provider: Provider

    /// As contas na **ordem de preferência** que o usuário definiu: a primeira
    /// disponível é a escolhida. Arrastar na UI reescreve esta ordem.
    public var accountIDs: [UUID]

    /// O perfil onde as sessões deste grupo rodam. Um único grupo pode ser o
    /// padrão (`~/.claude`, sem variável de ambiente); os demais têm perfil
    /// dedicado. Configurável — qual grupo é o padrão é escolha do usuário.
    public var configDir: ConfigDir

    /// A partir de quanto de uso trocar de conta. Por grupo, porque um grupo
    /// compartilhado com colegas pode querer folga maior que um pessoal.
    public var thresholdPercent: Double

    /// Rotação automática ligada. Desligado, o grupo ainda existe e troca na
    /// mão, mas não sozinho.
    public var autoRotate: Bool

    public init(id: UUID = UUID(), name: String, provider: Provider = .anthropic,
                accountIDs: [UUID] = [], configDir: ConfigDir,
                thresholdPercent: Double = 90, autoRotate: Bool = true) {
        self.id = id
        self.name = name
        self.provider = provider
        self.accountIDs = accountIDs
        self.configDir = configDir
        self.thresholdPercent = thresholdPercent
        self.autoRotate = autoRotate
    }
}

/// A configuração inteira do app, persistida como um JSON só.
///
/// Tudo que o usuário monta pela UI vive aqui: as contas, os grupos, a ordem, os
/// limiares, e as preferências que na primeira versão foram perguntas de
/// setup — qual grupo é o padrão, se o histórico é compartilhado. Nenhuma delas
/// é constante de código; o que o app traz de fábrica é só o valor inicial.
public struct RouterConfig: Sendable, Equatable, Codable {
    public var accounts: [Account]
    public var groups: [AccountGroup]

    /// Um só histórico de conversas entre os grupos (`--resume` enxerga tudo),
    /// ou cada grupo com o seu. Configurável; padrão de fábrica é compartilhado.
    public var shareHistory: Bool

    /// Versão do formato, para migrar sem perder configuração de quem já usava.
    public var version: Int

    public static let currentVersion = 1

    public init(accounts: [Account] = [], groups: [AccountGroup] = [],
                shareHistory: Bool = true, version: Int = RouterConfig.currentVersion) {
        self.accounts = accounts
        self.groups = groups
        self.shareHistory = shareHistory
        self.version = version
    }

    public func account(_ id: UUID) -> Account? { accounts.first { $0.id == id } }

    /// As contas de um grupo, na ordem de preferência, ignorando IDs órfãos.
    public func accounts(in group: AccountGroup) -> [Account] {
        group.accountIDs.compactMap { id in accounts.first { $0.id == id } }
    }

    /// O grupo marcado como padrão — o que roda em `~/.claude`. No máximo um.
    public var defaultGroup: AccountGroup? { groups.first { $0.configDir.isDefault } }
}
