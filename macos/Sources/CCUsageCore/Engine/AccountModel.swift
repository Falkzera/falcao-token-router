import Foundation

/// A identidade de uma conta, lida do `.claude.json` que o Claude Code grava —
/// nunca da rede.
///
/// É o que se copia entre perfis para a tela do provedor mostrar a conta certa.
/// Guarda o dicionário `oauthAccount` inteiro (opaco) porque é ele que o Claude
/// Code lê, e reescrevê-lo por partes só criaria formas de divergir do que o
/// provedor espera; os campos abaixo são os que a UI usa, extraídos por cima.
public struct AccountIdentity: Sendable, Equatable, Codable {
    public let email: String
    public let organizationName: String?
    /// `default_claude_max_5x`, `default_claude_max_20x`, etc. — o que decide o
    /// plano sem chamada nenhuma.
    public let rateLimitTier: String?
    /// O `oauthAccount` cru, para regravar idêntico no perfil de destino.
    public let raw: [String: JSONValue]

    public init(email: String, organizationName: String?,
                rateLimitTier: String?, raw: [String: JSONValue]) {
        self.email = email
        self.organizationName = organizationName
        self.rateLimitTier = rateLimitTier
        self.raw = raw
    }

    /// Nome curto: o que cabe numa linha de lista.
    public var shortName: String { String(email.prefix(while: { $0 != "@" })) }
}

/// Uma conta que o usuário adicionou a um grupo.
///
/// A credencial em si **nunca** vive aqui — vive no chaveiro, no item do perfil
/// de origem da conta (a "casa"), escrita pelo próprio Claude Code no login. O
/// que se guarda no arquivo de configuração é só o suficiente para achar essa
/// credencial e mostrar a conta: identidade e o caminho da casa.
public struct Account: Sendable, Equatable, Codable, Identifiable {
    public let id: UUID
    public let provider: Provider
    /// Identidade capturada no momento em que a conta foi adicionada. Serve de
    /// rótulo estável mesmo antes da primeira ativação.
    public var identity: AccountIdentity
    /// O perfil onde esta conta fez `auth login` — onde a credencial-mãe mora
    /// no chaveiro. Cada conta tem a sua, isolada das demais.
    public let home: ConfigDir
    /// Apelido opcional que o usuário deu ("trabalho principal", "faculdade").
    public var nickname: String?

    public init(id: UUID = UUID(), provider: Provider, identity: AccountIdentity,
                home: ConfigDir, nickname: String? = nil) {
        self.id = id
        self.provider = provider
        self.identity = identity
        self.home = home
        self.nickname = nickname
    }

    /// O que mostrar: o apelido se houver, senão o nome curto do e-mail.
    public var label: String { nickname ?? identity.shortName }
}

/// Um valor JSON preservado tal e qual, para regravar o `oauthAccount` sem
/// perder campos que este app não conhece — e não vai fingir conhecer.
public enum JSONValue: Sendable, Equatable, Codable {
    case string(String)
    case number(Double)
    case bool(Bool)
    case null
    case array([JSONValue])
    case object([String: JSONValue])

    public init(from decoder: any Decoder) throws {
        let c = try decoder.singleValueContainer()
        if c.decodeNil() { self = .null }
        else if let b = try? c.decode(Bool.self) { self = .bool(b) }
        else if let n = try? c.decode(Double.self) { self = .number(n) }
        else if let s = try? c.decode(String.self) { self = .string(s) }
        else if let a = try? c.decode([JSONValue].self) { self = .array(a) }
        else if let o = try? c.decode([String: JSONValue].self) { self = .object(o) }
        else { self = .null }
    }

    public func encode(to encoder: any Encoder) throws {
        var c = encoder.singleValueContainer()
        switch self {
        case .string(let s): try c.encode(s)
        case .number(let n): try c.encode(n)
        case .bool(let b): try c.encode(b)
        case .null: try c.encodeNil()
        case .array(let a): try c.encode(a)
        case .object(let o): try c.encode(o)
        }
    }

    /// Converte de/para o `Any` que `JSONSerialization` produz, para ler e
    /// escrever o `.claude.json` sem impor um schema à parte que é opaca.
    public init(any value: Any) {
        switch value {
        case let s as String: self = .string(s)
        case let b as Bool: self = .bool(b)
        case let n as NSNumber:
            // NSNumber não distingue bool de número; o caso bool já foi acima.
            self = .number(n.doubleValue)
        case let a as [Any]: self = .array(a.map(JSONValue.init(any:)))
        case let o as [String: Any]:
            self = .object(o.mapValues(JSONValue.init(any:)))
        default: self = .null
        }
    }

    public var anyValue: Any {
        switch self {
        case .string(let s): s
        case .number(let n): n
        case .bool(let b): b
        case .null: NSNull()
        case .array(let a): a.map(\.anyValue)
        case .object(let o): o.mapValues(\.anyValue)
        }
    }

    public var stringValue: String? { if case .string(let s) = self { return s }; return nil }
}
