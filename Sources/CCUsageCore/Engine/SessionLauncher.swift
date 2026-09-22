import Foundation

/// Prepara o lançamento de uma sessão num grupo: escolhe a conta certa, ativa-a,
/// e devolve o que o processo precisa para subir. O `exec` em si fica na CLI —
/// aqui é só a decisão, para ser testável sem lançar nada.
public struct SessionLauncher: Sendable {
    /// O que a CLI precisa para subir a sessão no grupo.
    public struct LaunchPlan: Sendable, Equatable {
        /// O executável do provedor (`claude`).
        public let executable: String
        /// Argumentos do lançamento (os que o usuário passou depois do grupo).
        public let arguments: [String]
        /// O valor de `CLAUDE_CONFIG_DIR`, ou `nil` no grupo padrão (que não
        /// exporta a variável — setá-la sobe a sessão deslogada).
        public let configDirEnv: String?
        /// A conta que ficou ativa para esta sessão.
        public let account: Account
    }

    public enum LaunchError: Error, Equatable {
        case unknownGroup(String)
        case emptyGroup(String)
        case noUsableAccount(String)
    }

    private let engine: RotationEngine
    private let adapter: any ProviderAdapter

    public init(keychain: any KeychainStore,
                adapters: [any ProviderAdapter] = [AnthropicAdapter()]) {
        self.engine = RotationEngine(keychain: keychain, adapters: adapters)
        self.adapter = adapters.first ?? AnthropicAdapter()
    }

    /// Acha um grupo pelo nome, sem diferenciar maiúsculas nem espaços nas pontas.
    public func group(named name: String, in config: RouterConfig) -> AccountGroup? {
        let wanted = name.lowercased().trimmingCharacters(in: .whitespaces)
        return config.groups.first {
            $0.name.lowercased().trimmingCharacters(in: .whitespaces) == wanted
        }
    }

    /// Escolhe a conta que vai servir a sessão e a ativa.
    ///
    /// Ordem da decisão: mantém a conta ativa se ela ainda tem folga; senão vai
    /// para a próxima com folga na ordem de preferência; senão, para a primeira
    /// que ao menos tenha credencial — é melhor subir numa conta cheia do que
    /// recusar o lançamento (o usuário pode trocar de modelo, ou só ler).
    public func prepare(group: AccountGroup, config: RouterConfig,
                        usage: [UUID: Double], arguments: [String]) throws -> LaunchPlan {
        let accounts = config.accounts(in: group)
        guard !accounts.isEmpty else { throw LaunchError.emptyGroup(group.name) }

        let active = engine.activeAccount(in: group, config: config)
        let threshold = group.thresholdPercent / 100

        let chosen: Account
        if let active, let used = usage[active.id], used < threshold {
            chosen = active  // ativa ainda tem folga
        } else if let next = engine.nextAccount(for: group, config: config, usage: usage) {
            chosen = next    // a melhor com folga
        } else if let active {
            chosen = active  // ninguém com folga; fica na atual
        } else {
            chosen = accounts.first!  // nunca ativou nenhuma; começa pela primeira
        }

        do {
            try engine.activate(chosen, in: group, config: config)
        } catch {
            throw LaunchError.noUsableAccount(group.name)
        }

        return LaunchPlan(
            executable: adapter.launchCommand().executable,
            arguments: arguments,
            configDirEnv: group.configDir.environmentValue,
            account: chosen)
    }
}
