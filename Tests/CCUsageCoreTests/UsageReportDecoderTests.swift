import Foundation
import Testing
@testable import CCUsageCore

/// Resposta real de `GET /api/oauth/usage`, capturada de uma conta Max 5×.
/// Preservada com os percentuais originais: são eles que dão sentido aos testes
/// de janela por modelo. Não contém credencial, e-mail nem identificador.
let liveResponse = """
{
  "five_hour":  { "utilization": 6.0,  "resets_at": "2026-08-18T19:20:00.510586+00:00" },
  "seven_day":  { "utilization": 25.0, "resets_at": "2026-08-23T07:00:00.510608+00:00" },
  "seven_day_opus": null,
  "seven_day_sonnet": null,
  "tangelo": null,
  "nimbus_quill": { "utilization": 0.0, "resets_at": null },
  "limits": [
    { "kind": "session", "group": "session", "percent": 6, "severity": "normal",
      "resets_at": "2026-08-18T19:20:00.510586+00:00", "scope": null, "is_active": false },
    { "kind": "weekly_all", "group": "weekly", "percent": 25, "severity": "normal",
      "resets_at": "2026-08-23T07:00:00.510608+00:00", "scope": null, "is_active": true },
    { "kind": "weekly_scoped", "group": "weekly", "percent": 0, "severity": "normal",
      "resets_at": null, "is_active": false,
      "scope": { "model": { "id": null, "display_name": "Fable" } } }
  ]
}
"""

func dictionary(_ json: String) -> [String: Any] {
    try! JSONSerialization.jsonObject(with: Data(json.utf8)) as! [String: Any]
}

let fixedNow = Date(timeIntervalSince1970: 1_787_000_000)

@Suite struct UsageReportDecoderTests {
    @Test func decodesTheThreeLimitKindsFromTheLiveResponse() {
        let report = UsageReportDecoder.decode(dictionary(liveResponse), fetchedAt: fixedNow)!

        #expect(report.session?.fraction == 0.06)
        #expect(report.session?.isActive == false)
        #expect(report.weeklyAll?.fraction == 0.25)
        #expect(report.weeklyAll?.isActive == true)
        #expect(report.weeklyScoped.count == 1)
        #expect(report.weeklyScoped.first?.modelName == "Fable")
        #expect(report.fetchedAt == fixedNow)
    }

    @Test func readsLimitsAndIgnoresTheTopLevelCodenames() {
        // `tangelo`, `nimbus_quill` e companhia são codinomes internos que giram.
        // Nenhum pode virar janela: só `limits[]` define o que existe.
        let report = UsageReportDecoder.decode(dictionary(liveResponse), fetchedAt: fixedNow)!
        #expect(report.limits.count == 3)
    }

    @Test func parsesResetTimestampWithSixFractionalDigits() {
        // O payload traz microssegundos e offset "+00:00" — formatos que o parser
        // ISO padrão do Swift recusa se levados ao pé da letra.
        let expected = try! Date.ISO8601FormatStyle(includingFractionalSeconds: false)
            .parse("2026-08-18T19:20:00Z")
        let report = UsageReportDecoder.decode(dictionary(liveResponse), fetchedAt: fixedNow)!
        #expect(report.session?.resetsAt == expected)
    }

    @Test func nullResetTimeStaysNil() {
        let report = UsageReportDecoder.decode(dictionary(liveResponse), fetchedAt: fixedNow)!
        #expect(report.weeklyScoped.first?.resetsAt == nil)
    }

    @Test func unknownKindIsPreservedNotDiscarded() {
        // O que aparece aqui é o que a próxima versão precisa suportar.
        let json = """
        { "limits": [ { "kind": "monthly_pilot", "percent": 5, "severity": "normal",
                        "is_active": false } ] }
        """
        let report = UsageReportDecoder.decode(dictionary(json), fetchedAt: fixedNow)!
        #expect(report.limits.first?.kind == .other("monthly_pilot"))
        #expect(report.session == nil)
        #expect(report.weeklyAll == nil)
    }

    @Test func unknownSeverityIsPreservedNotDiscarded() {
        let json = """
        { "limits": [ { "kind": "session", "percent": 91, "severity": "screaming",
                        "is_active": true } ] }
        """
        let report = UsageReportDecoder.decode(dictionary(json), fetchedAt: fixedNow)!
        #expect(report.session?.severity == .other("screaming"))
    }

    @Test func firstEntryOfEachKindWins() {
        // Uma segunda entrada de mesmo kind é formato novo, não correção da
        // primeira — e formato novo deve ser ignorado, não obedecido.
        let json = """
        { "limits": [ { "kind": "session", "percent": 10, "severity": "normal", "is_active": true },
                      { "kind": "session", "percent": 90, "severity": "normal", "is_active": true } ] }
        """
        let report = UsageReportDecoder.decode(dictionary(json), fetchedAt: fixedNow)!
        #expect(report.session?.fraction == 0.10)
    }

    @Test func scopedWindowsKeepPayloadOrder() {
        let json = """
        { "limits": [
            { "kind": "weekly_scoped", "percent": 1, "severity": "normal", "is_active": false,
              "scope": { "model": { "display_name": "Opus" } } },
            { "kind": "weekly_scoped", "percent": 2, "severity": "normal", "is_active": false,
              "scope": { "model": { "display_name": "Sonnet" } } } ] }
        """
        let report = UsageReportDecoder.decode(dictionary(json), fetchedAt: fixedNow)!
        #expect(report.weeklyScoped.map(\.modelName) == ["Opus", "Sonnet"])
    }

    @Test func fallsBackToTopLevelKeysWhenLimitsIsAbsent() {
        // Claude Code antigo grava o cache sem `limits`. A janela ainda é legível.
        let json = """
        { "five_hour": { "utilization": 22, "resets_at": "2026-08-18T02:20:00.007553+00:00" },
          "seven_day": { "utilization": 19, "resets_at": "2026-08-23T07:00:00.007577+00:00" },
          "seven_day_opus": null }
        """
        let report = UsageReportDecoder.decode(dictionary(json), fetchedAt: fixedNow)!
        #expect(report.session?.fraction == 0.22)
        #expect(report.weeklyAll?.fraction == 0.19)
        // Sem `limits`, não há janela por modelo a recuperar: as chaves de topo
        // para modelo vêm null e não são contrato.
        #expect(report.weeklyScoped.isEmpty)
    }

    @Test func nullTopLevelWindowsBecomeNothingNotZero() {
        let json = #"{ "five_hour": null, "seven_day": null }"#
        #expect(UsageReportDecoder.decode(dictionary(json), fetchedAt: fixedNow) == nil)
    }

    @Test func payloadWithNoUsableWindowYieldsNoReport() {
        #expect(UsageReportDecoder.decode(dictionary(#"{ "numStartups": 3 }"#),
                                          fetchedAt: fixedNow) == nil)
    }

    @Test func entryMissingPercentIsSkippedNotZeroed() {
        let json = """
        { "limits": [ { "kind": "session", "severity": "normal", "is_active": true },
                      { "kind": "weekly_all", "percent": 30, "severity": "normal", "is_active": true } ] }
        """
        let report = UsageReportDecoder.decode(dictionary(json), fetchedAt: fixedNow)!
        #expect(report.session == nil)
        #expect(report.weeklyAll?.fraction == 0.30)
    }
}
