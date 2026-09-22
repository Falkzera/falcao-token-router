/// Um modelo do Claude, agrupado por faixa de preço.
///
/// Os casos `4x` colapsam gerações que compartilham exatamente a mesma tabela
/// de preços — separá-las não mudaria nenhum número.
public enum ModelID: Hashable, Sendable, Codable {
    case opus5
    case opus4x      // Opus 4.5 / 4.6 / 4.7 / 4.8 — todos $5/$25
    case sonnet5
    case sonnet4x    // Sonnet 4.5 / 4.6 — ambos $3/$15
    case haiku45
    case fable5      // inclui Mythos 5 / Mythos Preview
    case unknown(String)

    /// Resolve o valor cru de `message.model` no JSONL.
    ///
    /// Aliases nus (`"opus"`, `"sonnet"`, `"haiku"`) aparecem quando um subagente
    /// é configurado por tier em vez de por ID. Resolvem para a geração corrente
    /// da família, que é o que o Claude Code entende por eles.
    public init(raw: String) {
        if raw == "claude-fable-5" || raw == "claude-mythos-5" || raw == "claude-mythos-preview" {
            self = .fable5
        } else if raw == "claude-opus-5" || raw == "opus" {
            self = .opus5
        } else if raw.hasPrefix("claude-opus-4") {
            self = .opus4x
        } else if raw == "claude-sonnet-5" || raw == "sonnet" {
            self = .sonnet5
        } else if raw.hasPrefix("claude-sonnet-4") {
            self = .sonnet4x
        } else if raw.hasPrefix("claude-haiku-4-5") || raw == "haiku" {
            self = .haiku45
        } else {
            self = .unknown(raw)
        }
    }

    /// Nome cru quando o modelo não foi reconhecido — para a UI poder dizer qual é.
    public var unknownName: String? {
        if case .unknown(let name) = self { return name }
        return nil
    }
}
