import Testing
@testable import CCUsageCore

@Test func resolvesCanonicalIDs() {
    #expect(ModelID(raw: "claude-opus-5") == .opus5)
    #expect(ModelID(raw: "claude-opus-4-8") == .opus4x)
    #expect(ModelID(raw: "claude-sonnet-5") == .sonnet5)
    #expect(ModelID(raw: "claude-sonnet-4-5-20250929") == .sonnet4x)
    #expect(ModelID(raw: "claude-haiku-4-5-20251001") == .haiku45)
    #expect(ModelID(raw: "claude-fable-5") == .fable5)
}

@Test func resolvesBareAliases() {
    // A geração corrente da família — o que o Claude Code entende pelo alias.
    #expect(ModelID(raw: "opus") == .opus55)
    #expect(ModelID(raw: "sonnet") == .sonnet55)
    #expect(ModelID(raw: "haiku") == .haiku45)
}

@Test("a geração atual é reconhecida")
func resolvesTheCurrentGeneration() {
    #expect(ModelID(raw: "claude-opus-5-5") == .opus55)
    #expect(ModelID(raw: "claude-sonnet-5-5") == .sonnet55)
    #expect(ModelID(raw: "claude-fable-5-1") == .fable51)
    #expect(ModelID(raw: "claude-mythos-5-1") == .fable51)
}

/// O modelo padrão do Claude Code aparece com a janela no nome
/// (`claude-opus-5-5[1m]`), e caía em `.unknown`: o total ficava parcial.
@Test("o sufixo entre colchetes não muda o modelo")
func bracketSuffixIsIgnored() {
    #expect(ModelID(raw: "claude-opus-5-5[1m]") == .opus55)
    #expect(ModelID(raw: "claude-sonnet-4-6[1m]") == .sonnet4x)
}

@Test func unknownModelKeepsItsRawName() {
    #expect(ModelID(raw: "claude-opus-9") == .unknown("claude-opus-9"))
    #expect(ModelID(raw: "<synthetic>") == .unknown("<synthetic>"))
}
