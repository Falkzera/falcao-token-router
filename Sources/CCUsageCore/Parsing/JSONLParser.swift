import Foundation

public enum JSONLParser {
    // MARK: - Pré-filtro

    private static let usageMarker = Data(#""usage""#.utf8)

    /// Portão barato em bytes: só linhas contendo `"usage"` podem gerar evento.
    /// A esmagadora maioria das linhas é user/tool/system e nunca chega ao
    /// `JSONDecoder` — é o que torna a varredura de 252 MB viável.
    public static func mayContainUsage(_ line: Data) -> Bool {
        line.range(of: usageMarker) != nil
    }

    // MARK: - Decode

    // JSONDecoder não é Sendable, mas decodificar com uma instância que nunca é
    // mutada é seguro para uso concorrente.
    nonisolated(unsafe) private static let decoder = JSONDecoder()

    private static let isoWithFraction = Date.ISO8601FormatStyle(includingFractionalSeconds: true)
    private static let isoPlain = Date.ISO8601FormatStyle(includingFractionalSeconds: false)

    static func parseTimestamp(_ raw: String) -> Date? {
        if let d = try? isoWithFraction.parse(raw) { return d }
        return try? isoPlain.parse(raw)
    }

    private struct RawLine: Decodable {
        let type: String?
        let timestamp: String?
        let requestId: String?
        let message: RawMessage?
    }

    private struct RawMessage: Decodable {
        let id: String?
        let model: String?
        let usage: RawUsage?
    }

    private struct RawUsage: Decodable {
        let inputTokens: UInt32?
        let outputTokens: UInt32?
        let cacheCreationInputTokens: UInt32?
        let cacheReadInputTokens: UInt32?
        let speed: String?
        let cacheCreation: RawCacheCreation?

        enum CodingKeys: String, CodingKey {
            case inputTokens = "input_tokens"
            case outputTokens = "output_tokens"
            case cacheCreationInputTokens = "cache_creation_input_tokens"
            case cacheReadInputTokens = "cache_read_input_tokens"
            case speed
            case cacheCreation = "cache_creation"
        }
    }

    private struct RawCacheCreation: Decodable {
        let ephemeral1h: UInt32?
        let ephemeral5m: UInt32?

        enum CodingKeys: String, CodingKey {
            case ephemeral1h = "ephemeral_1h_input_tokens"
            case ephemeral5m = "ephemeral_5m_input_tokens"
        }
    }

    /// `nil` para qualquer linha que não seja uma mensagem assistant com uso real.
    /// Linha malformada é descartada silenciosamente — um log corrompido não
    /// pode derrubar a varredura inteira.
    public static func event(from line: Data) -> UsageEvent? {
        guard mayContainUsage(line),
              let raw = try? decoder.decode(RawLine.self, from: line),
              raw.type == "assistant",
              let message = raw.message,
              let usage = message.usage,
              let modelRaw = message.model,
              modelRaw != "<synthetic>",
              let messageID = message.id,
              let requestID = raw.requestId,
              let timestampRaw = raw.timestamp,
              let timestamp = parseTimestamp(timestampRaw)
        else { return nil }

        // Versões antigas do Claude Code não gravam o split por TTL; nesse caso
        // o total cai no bucket de 5m, que é o TTL padrão.
        let write5m: UInt32
        let write1h: UInt32
        if let split = usage.cacheCreation {
            write5m = split.ephemeral5m ?? 0
            write1h = split.ephemeral1h ?? 0
        } else {
            write5m = usage.cacheCreationInputTokens ?? 0
            write1h = 0
        }

        return UsageEvent(
            timestamp: timestamp,
            model: ModelID(raw: modelRaw),
            isFast: usage.speed == "fast",
            input: usage.inputTokens ?? 0,
            output: usage.outputTokens ?? 0,
            cacheWrite5m: write5m,
            cacheWrite1h: write1h,
            cacheRead: usage.cacheReadInputTokens ?? 0,
            dedupeKey: "\(messageID):\(requestID)"
        )
    }
}
