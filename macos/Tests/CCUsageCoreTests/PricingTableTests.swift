import Foundation
import Testing
@testable import CCUsageCore

private func date(_ s: String) -> Date {
    try! Date.ISO8601FormatStyle(includingFractionalSeconds: false).parse(s)
}

private func event(
    model: ModelID, at: Date, isFast: Bool = false,
    input: UInt32 = 0, output: UInt32 = 0,
    w5m: UInt32 = 0, w1h: UInt32 = 0, read: UInt32 = 0
) -> UsageEvent {
    UsageEvent(timestamp: at, model: model, isFast: isFast,
               input: input, output: output,
               cacheWrite5m: w5m, cacheWrite1h: w1h, cacheRead: read,
               dedupeKey: "k")
}

@Test func cacheDimensionsDeriveFromInput() {
    let r = PricingTable.rates(for: .opus5, at: date("2026-08-17T12:00:00Z"), isFast: false)!
    #expect(r.input == 5)
    #expect(r.output == 25)
    #expect(r.cacheWrite5m == Decimal(string: "6.25"))
    #expect(r.cacheWrite1h == 10)
    #expect(r.cacheRead == Decimal(string: "0.5"))
}

/// A tabela previa o Sonnet 5 voltando a $3/$15 em 01/09/2026; a oficial de
/// 25/09 o lista a $2/$10. Desde setembro o total saía 50% acima, sem marca.
@Test("Sonnet 5 custa $2/$10, antes e depois de setembro")
func sonnet5StaysAtTwoAndTen() {
    for day in ["2026-08-20T00:00:00Z", "2026-09-25T00:00:00Z"] {
        let r = PricingTable.rates(for: .sonnet5, at: date(day), isFast: false)!
        #expect(r.input == 2)
        #expect(r.output == 10)
    }
}

@Test("a geração atual tem preço: Opus 5.5, Sonnet 5.5, Fable 5.1")
func currentGenerationIsPriced() {
    let d = date("2026-10-01T00:00:00Z")
    let opus = PricingTable.rates(for: .opus55, at: d, isFast: false)!
    #expect(opus.input == 4 && opus.output == 20)
    #expect(opus.cacheRead == Decimal(string: "0.2"))
    let opusFast = PricingTable.rates(for: .opus55, at: d, isFast: true)!
    #expect(opusFast.input == 8 && opusFast.output == 40)
    let sonnet = PricingTable.rates(for: .sonnet55, at: d, isFast: false)!
    #expect(sonnet.input == 2 && sonnet.output == 10)
    #expect(sonnet.cacheRead == Decimal(string: "0.2"))
    let fable = PricingTable.rates(for: .fable51, at: d, isFast: false)!
    #expect(fable.input == 10 && fable.output == 50)
    #expect(fable.cacheRead == Decimal(string: "0.25"))
}

/// O cache do medidor guarda o evento já resolvido; um modelo que esta tabela
/// passou a conhecer continuava `.unknown` no histórico, sem preço.
@Test("evento guardado como desconhecido ganha preço quando o modelo entra na tabela")
func aCachedUnknownModelIsResolvedAgain() {
    let e = event(model: .unknown("claude-opus-5-5"), at: date("2026-10-01T00:00:00Z"),
                  input: 1_000_000)
    #expect(PricingTable.cost(of: e) == 4)
}

@Test func fastModeDoublesOpus5() {
    let d = date("2026-08-17T12:00:00Z")
    let standard = PricingTable.rates(for: .opus5, at: d, isFast: false)!
    let fast = PricingTable.rates(for: .opus5, at: d, isFast: true)!
    #expect(fast.input == standard.input * 2)
    #expect(fast.output == standard.output * 2)
}

@Test func unknownModelHasNoPrice() {
    #expect(PricingTable.rates(for: .unknown("claude-opus-9"),
                               at: date("2026-08-17T12:00:00Z"), isFast: false) == nil)
    #expect(PricingTable.cost(of: event(model: .unknown("x"),
                                        at: date("2026-08-17T12:00:00Z"), input: 1000)) == nil)
}

@Test func costSumsAllFiveDimensions() {
    // Opus 5 standard: 1M input=$5, 1M output=$25, 1M w5m=$6.25, 1M w1h=$10, 1M read=$0.50
    let e = event(model: .opus5, at: date("2026-08-17T12:00:00Z"),
                  input: 1_000_000, output: 1_000_000,
                  w5m: 1_000_000, w1h: 1_000_000, read: 1_000_000)
    #expect(PricingTable.cost(of: e) == Decimal(string: "46.75"))
}
