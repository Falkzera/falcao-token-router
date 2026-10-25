/// Um modelo do Claude, agrupado por faixa de preço.
///
/// Os casos `4x` colapsam gerações que compartilham exatamente a mesma tabela
/// de preços — separá-las não mudaria nenhum número.
public enum ModelID: Hashable, Sendable, Codable {
    case opus55
    case opus5
    case opus4x      // Opus 4.5 / 4.6 / 4.7 / 4.8 — todos $5/$25
    case sonnet55
    case sonnet5
    case sonnet4x    // Sonnet 4.5 / 4.6 — ambos $3/$15
    case haiku45
    case fable51     // inclui Mythos 5.1
    case fable5      // inclui Mythos 5 / Mythos Preview
    case unknown(String)

    /// Resolve o valor cru de `message.model` no JSONL.
    ///
    /// Aliases nus (`"opus"`, `"sonnet"`, `"haiku"`) aparecem quando um subagente
    /// é configurado por tier em vez de por ID. Resolvem para a geração corrente
    /// da família, que é o que o Claude Code entende por eles.
    ///
    /// Um sufixo entre colchetes (`claude-opus-5-5[1m]`, a janela de 1M) não muda
    /// o preço e sai antes de casar; sem isso o modelo padrão do Claude Code caía
    /// em `.unknown` e o total do medidor ficava "parcial".
    public init(raw: String) {
        let id = raw.firstIndex(of: "[").map { String(raw[..<$0]) } ?? raw
        switch id {
        case "claude-fable-5-1", "claude-mythos-5-1": self = .fable51
        case "claude-fable-5", "claude-mythos-5", "claude-mythos-preview": self = .fable5
        case "claude-opus-5-5", "opus": self = .opus55
        case "claude-opus-5": self = .opus5
        case "claude-sonnet-5-5", "sonnet": self = .sonnet55
        case "claude-sonnet-5": self = .sonnet5
        case "haiku": self = .haiku45
        default:
            if id.hasPrefix("claude-opus-4") { self = .opus4x }
            else if id.hasPrefix("claude-sonnet-4") { self = .sonnet4x }
            else if id.hasPrefix("claude-haiku-4-5") { self = .haiku45 }
            else { self = .unknown(raw) }
        }
    }

    /// O mesmo modelo, resolvido de novo pela tabela de hoje. Um evento lido
    /// antes de o modelo dele entrar aqui fica no cache como `.unknown`, e o
    /// cache não relê bytes já lidos — sem isto, o histórico de um modelo novo
    /// continuaria sem preço depois da atualização que o reconhece.
    public var resolved: ModelID {
        if case .unknown(let raw) = self { return ModelID(raw: raw) }
        return self
    }

    /// Nome cru quando o modelo não foi reconhecido — para a UI poder dizer qual é.
    public var unknownName: String? {
        if case .unknown(let name) = self { return name }
        return nil
    }
}
