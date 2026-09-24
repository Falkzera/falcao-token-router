import Foundation

/// Onde tudo do produto mora no disco.
///
/// Um espaço só, em `Application Support`, com o `bundleID` como raiz. É estável
/// e independente do `CFBundleIdentifier` do `.app`, para a configuração
/// sobreviver a uma renomeação do binário.
public struct RouterPaths: Sendable {
    /// Namespace de armazenamento. Não muda quando o app é renomeado.
    ///
    /// **Diverge do `CFBundleIdentifier` de propósito, e não pode ser
    /// "corrigido".** Em 2026-09-18 o produto virou `falcao-token-router` e o
    /// bundle passou a ser `com.synqo.falcao-token-router`; este valor ficou
    /// como estava. O motivo é a credencial: a casa de cada conta é
    /// `<base>/accounts/<uuid>`, e o Claude Code nomeia o item de chaveiro do
    /// perfil como `Claude Code-credentials-<sha256(caminho)[:8]>`. Mexer na
    /// base muda o caminho, muda o hash, e TODA credencial fica inalcançável de
    /// uma vez — com os itens antigos órfãos no chaveiro e nenhuma mensagem que
    /// explique. Renomear aqui exige migrar diretórios e reescrever cada item de
    /// chaveiro antes de apagar o antigo; até que alguém queira pagar esse
    /// preço, o nome de pasta é histórico e fica.
    public static let bundleID = "com.synqo.falcao-router"

    public let base: URL

    public init(bundleID: String = RouterPaths.bundleID,
                appSupport: URL? = nil) {
        // `ROUTER_APP_SUPPORT` redireciona a raiz — usado em teste e para a CLI e
        // o app apontarem para o mesmo lugar sob outro id.
        let root = appSupport
            ?? ProcessInfo.processInfo.environment["ROUTER_APP_SUPPORT"].map(URL.init(fileURLWithPath:))
            ?? FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask).first!
        self.base = root.appending(path: bundleID)
    }

    /// O `RouterConfig` inteiro, num JSON.
    public var configFile: URL { base.appending(path: "config.json") }

    /// A casa de cada conta — o perfil onde ela faz `auth login` e onde a
    /// credencial-mãe vive no chaveiro. Uma por conta, isolada.
    public func accountHome(_ id: UUID) -> ConfigDir {
        .dedicated(base.appending(path: "accounts").appending(path: id.uuidString).path)
    }

    /// O perfil onde as sessões de um grupo rodam. O grupo padrão usa `~/.claude`
    /// (sem variável de ambiente); os demais, um perfil dedicado.
    public func groupConfigDir(_ id: UUID, isDefault: Bool,
                               home: String = NSHomeDirectory()) -> ConfigDir {
        isDefault
            ? .standard(home: home)
            : .dedicated(base.appending(path: "groups").appending(path: id.uuidString).path)
    }

    /// Onde o sensor grava as amostras de uso.
    public var usageDir: URL {
        GroupUsageStore.directory(bundleID: RouterPaths.bundleID,
                                  base: base.deletingLastPathComponent())
    }
}
