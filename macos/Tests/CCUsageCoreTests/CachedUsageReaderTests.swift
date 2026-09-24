import Foundation
import Testing
@testable import CCUsageCore

/// Estrutura real de `~/.claude.json`, reduzida ao que importa. O bloco
/// `utilization` é a cópia gravada da resposta de `/api/oauth/usage`, e desde
/// versões recentes do Claude Code traz o `limits[]` junto.
private let cacheShape = """
{
  "numStartups": 13512,
  "cachedUsageUtilization": {
    "fetchedAtMs": 1787010875105,
    "accountUuid": "00000000-0000-4000-8000-000000000000",
    "utilization": {
      "five_hour": { "utilization": 22, "resets_at": "2026-08-18T02:20:00.007553+00:00" },
      "seven_day": { "utilization": 19, "resets_at": "2026-08-23T07:00:00.007577+00:00" },
      "seven_day_opus": null,
      "limits": [
        { "kind": "session", "percent": 22, "severity": "normal",
          "resets_at": "2026-08-18T02:20:00.007553+00:00", "is_active": true },
        { "kind": "weekly_all", "percent": 19, "severity": "normal",
          "resets_at": "2026-08-23T07:00:00.007577+00:00", "is_active": false }
      ]
    }
  }
}
"""

private func writeTemp(_ contents: String) -> URL {
    let url = URL(fileURLWithPath: NSTemporaryDirectory())
        .appending(path: "cctc-cache-\(UUID().uuidString).json")
    try! contents.write(to: url, atomically: true, encoding: .utf8)
    return url
}

@Suite struct CachedUsageReaderTests {
    @Test func readsWindowsAndTheFetchTimestamp() {
        let url = writeTemp(cacheShape)
        defer { try? FileManager.default.removeItem(at: url) }

        let report = CachedUsageReader.read(from: url)!
        #expect(report.session?.fraction == 0.22)
        #expect(report.weeklyAll?.fraction == 0.19)
        // A idade do cache é parte do dado: ele só se move quando o Claude Code roda.
        #expect(report.fetchedAt == Date(timeIntervalSince1970: 1787010875.105))
    }

    @Test func readsTheCachedLimitsArrayNotOnlyTheTopLevelKeys() {
        let url = writeTemp(cacheShape)
        defer { try? FileManager.default.removeItem(at: url) }
        #expect(CachedUsageReader.read(from: url)?.session?.isActive == true)
    }

    @Test func cacheWithoutLimitsFallsBackToTopLevelKeys() {
        let json = """
        { "cachedUsageUtilization": { "fetchedAtMs": 1787010875105,
            "utilization": { "five_hour": { "utilization": 40, "resets_at": null } } } }
        """
        let url = writeTemp(json)
        defer { try? FileManager.default.removeItem(at: url) }
        #expect(CachedUsageReader.read(from: url)?.session?.fraction == 0.40)
    }

    @Test func missingFileYieldsNoReading() {
        let absent = URL(fileURLWithPath: NSTemporaryDirectory())
            .appending(path: "cctc-absent-\(UUID().uuidString).json")
        #expect(CachedUsageReader.read(from: absent) == nil)
    }

    @Test func malformedFileYieldsNoReading() {
        // Arquivo corrompido cai na próxima fonte em vez de derrubar o app.
        let url = writeTemp("{ isto não é json")
        defer { try? FileManager.default.removeItem(at: url) }
        #expect(CachedUsageReader.read(from: url) == nil)
    }

    @Test func fileWithoutTheCacheKeyYieldsNoReading() {
        let url = writeTemp(#"{ "numStartups": 3 }"#)
        defer { try? FileManager.default.removeItem(at: url) }
        #expect(CachedUsageReader.read(from: url) == nil)
    }

    @Test func cacheWithoutFetchTimestampYieldsNoReading() {
        // Sem carimbo não dá para dizer a idade, e a idade é o que decide se este
        // dado pode ser mostrado sem aviso.
        let json = #"{ "cachedUsageUtilization": { "utilization": { "five_hour": { "utilization": 5 } } } }"#
        let url = writeTemp(json)
        defer { try? FileManager.default.removeItem(at: url) }
        #expect(CachedUsageReader.read(from: url) == nil)
    }
}
