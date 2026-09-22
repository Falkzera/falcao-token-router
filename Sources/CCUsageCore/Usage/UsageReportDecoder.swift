import Foundation

/// Decodifica o payload de uso da Anthropic, venha ele da rede ou do cache.
///
/// **`limits[]` é o contrato.** As chaves de topo (`five_hour`, `seven_day`,
/// `tangelo`, `nimbus_quill`, `iguana_necktie`, `amber_ladder`…) são codinomes
/// internos, não documentados, que giram a cada ciclo de produto. `limits[]` é
/// auto-descritivo: `kind` e `scope` dizem o que cada entrada é, e é ele que
/// carrega as janelas por modelo com nome de exibição.
///
/// As chaves de topo entram só como fallback para versões do Claude Code que
/// gravaram o cache antes de `limits` existir.
///
/// Endpoint interno e indocumentado: toda falha devolve `nil`, e quem chama cai
/// para a próxima fonte. Nunca lança, nunca vira zero.
public enum UsageReportDecoder {
    public static func decode(_ root: [String: Any], fetchedAt: Date) -> UsageReport? {
        if let raw = root["limits"] as? [[String: Any]] {
            let limits = raw.compactMap(limit(from:))
            if !limits.isEmpty { return UsageReport(limits: limits, fetchedAt: fetchedAt) }
        }
        let legacy = legacyLimits(from: root)
        guard !legacy.isEmpty else { return nil }
        return UsageReport(limits: legacy, fetchedAt: fetchedAt)
    }

    /// Entrada sem `kind` ou sem `percent` é descartada em vez de virar janela
    /// zerada — o mesmo princípio que rege modelo sem preço no `PricingTable`.
    private static func limit(from dict: [String: Any]) -> UsageReport.Limit? {
        guard let kindRaw = dict["kind"] as? String,
              let percent = dict["percent"] as? Double
        else { return nil }

        let model = (dict["scope"] as? [String: Any])?["model"] as? [String: Any]
        return UsageReport.Limit(
            kind: .init(raw: kindRaw),
            fraction: percent / 100,
            severity: .init(raw: dict["severity"] as? String ?? ""),
            resetsAt: (dict["resets_at"] as? String).flatMap(parseTimestamp),
            modelName: model?["display_name"] as? String,
            isActive: dict["is_active"] as? Bool ?? false)
    }

    /// Sem `limits`, recupera só as duas janelas genéricas. As chaves de topo
    /// por modelo (`seven_day_opus`, `seven_day_sonnet`) ficam de fora de
    /// propósito: vieram `null` na conta real, e apostar nesses nomes é
    /// exatamente o que ler `limits[]` evita.
    private static func legacyLimits(from root: [String: Any]) -> [UsageReport.Limit] {
        let pairs: [(String, UsageReport.Limit.Kind)] = [
            ("five_hour", .session), ("seven_day", .weeklyAll),
        ]
        return pairs.compactMap { key, kind in
            guard let dict = root[key] as? [String: Any],
                  let utilization = dict["utilization"] as? Double
            else { return nil }
            return UsageReport.Limit(
                kind: kind, fraction: utilization / 100, severity: .normal,
                resetsAt: (dict["resets_at"] as? String).flatMap(parseTimestamp),
                modelName: nil, isActive: false)
        }
    }

    /// O payload traz microssegundos e offset explícito
    /// (`2026-08-18T19:20:00.510586+00:00`). O parser ISO do Swift aceita no
    /// máximo milissegundos, então a parte fracionária é descartada — precisão
    /// sub-segundo não significa nada para um horário de reset.
    static func parseTimestamp(_ raw: String) -> Date? {
        var cleaned = raw
        if let dot = raw.firstIndex(of: "."),
           let offsetStart = raw[dot...].firstIndex(where: { $0 == "+" || $0 == "-" || $0 == "Z" }) {
            cleaned = String(raw[..<dot]) + String(raw[offsetStart...])
        }
        let plain = Date.ISO8601FormatStyle(includingFractionalSeconds: false)
        if let date = try? plain.parse(cleaned) { return date }
        return try? plain.parse(cleaned.replacingOccurrences(of: "+00:00", with: "Z"))
    }
}
