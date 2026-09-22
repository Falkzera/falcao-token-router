import Foundation

/// Uma mensagem assistant com uso de token, extraída de uma linha do JSONL.
public struct UsageEvent: Sendable, Equatable, Codable {
    public let timestamp: Date
    public let model: ModelID
    /// `message.usage.speed == "fast"` — fast mode custa o dobro no Opus 5.
    public let isFast: Bool
    public let input: UInt32
    public let output: UInt32
    public let cacheWrite5m: UInt32
    public let cacheWrite1h: UInt32
    public let cacheRead: UInt32
    /// `"\(message.id):\(requestId)"` — resume e fork reescrevem a mesma
    /// mensagem em arquivos diferentes; sem isso os tokens contam em dobro.
    public let dedupeKey: String

    public init(
        timestamp: Date, model: ModelID, isFast: Bool,
        input: UInt32, output: UInt32,
        cacheWrite5m: UInt32, cacheWrite1h: UInt32, cacheRead: UInt32,
        dedupeKey: String
    ) {
        self.timestamp = timestamp
        self.model = model
        self.isFast = isFast
        self.input = input
        self.output = output
        self.cacheWrite5m = cacheWrite5m
        self.cacheWrite1h = cacheWrite1h
        self.cacheRead = cacheRead
        self.dedupeKey = dedupeKey
    }

    public var totalTokens: UInt64 {
        UInt64(input) + UInt64(output)
            + UInt64(cacheWrite5m) + UInt64(cacheWrite1h) + UInt64(cacheRead)
    }
}
