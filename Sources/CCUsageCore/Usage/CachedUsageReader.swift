import Foundation

/// Lê `cachedUsageUtilization` de `~/.claude.json`.
///
/// É a cópia que o Claude Code grava da resposta de `/api/oauth/usage`. Serve
/// como fallback quando a busca ao vivo está desligada ou falhou — mas **só se
/// move quando o Claude Code roda**, e por isso o `fetchedAt` é obrigatório:
/// sem ele não dá para dizer a idade, e a idade é o que decide se este dado
/// pode ser exibido sem aviso.
///
/// Campo interno e indocumentado: toda falha devolve `nil` e o app segue para a
/// próxima fonte, em vez de quebrar.
public enum CachedUsageReader {
    public static var defaultURL: URL {
        FileManager.default.homeDirectoryForCurrentUser.appending(path: ".claude.json")
    }

    public static func read(from url: URL = defaultURL) -> UsageReport? {
        guard let data = try? Data(contentsOf: url),
              let root = try? JSONSerialization.jsonObject(with: data) as? [String: Any],
              let cache = root["cachedUsageUtilization"] as? [String: Any],
              let fetchedAtMs = cache["fetchedAtMs"] as? Double,
              let windows = cache["utilization"] as? [String: Any]
        else { return nil }

        return UsageReportDecoder.decode(
            windows, fetchedAt: Date(timeIntervalSince1970: fetchedAtMs / 1000))
    }
}
