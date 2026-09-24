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
    #expect(ModelID(raw: "opus") == .opus5)
    #expect(ModelID(raw: "sonnet") == .sonnet5)
    #expect(ModelID(raw: "haiku") == .haiku45)
}

@Test func unknownModelKeepsItsRawName() {
    #expect(ModelID(raw: "claude-opus-9") == .unknown("claude-opus-9"))
    #expect(ModelID(raw: "<synthetic>") == .unknown("<synthetic>"))
}
