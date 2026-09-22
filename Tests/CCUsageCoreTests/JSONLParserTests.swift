import Foundation
import Testing
@testable import CCUsageCore

private func line(_ json: String) -> Data { Data(json.utf8) }

private let assistantLine = """
{"type":"assistant","timestamp":"2026-07-13T10:08:12.767Z","requestId":"req_1",\
"message":{"id":"msg_1","model":"claude-opus-5","usage":{"input_tokens":2,\
"output_tokens":792,"cache_creation_input_tokens":26186,"cache_read_input_tokens":19059,\
"speed":"standard","cache_creation":{"ephemeral_1h_input_tokens":26186,\
"ephemeral_5m_input_tokens":0}}}}
"""

@Test func prefilterRejectsLinesWithoutUsage() {
    #expect(JSONLParser.mayContainUsage(line(#"{"type":"user","message":{}}"#)) == false)
    #expect(JSONLParser.mayContainUsage(line(assistantLine)) == true)
}

@Test func parsesAllFiveTokenDimensions() {
    let e = JSONLParser.event(from: line(assistantLine))!
    #expect(e.model == .opus5)
    #expect(e.input == 2)
    #expect(e.output == 792)
    #expect(e.cacheWrite1h == 26186)
    #expect(e.cacheWrite5m == 0)
    #expect(e.cacheRead == 19059)
    #expect(e.isFast == false)
    #expect(e.dedupeKey == "msg_1:req_1")
}

@Test func detectsFastMode() {
    let fast = assistantLine.replacingOccurrences(of: #""speed":"standard""#,
                                                  with: #""speed":"fast""#)
    #expect(JSONLParser.event(from: line(fast))!.isFast == true)
}

@Test func fallsBackToFiveMinuteTTLWhenSplitIsAbsent() {
    let noSplit = """
    {"type":"assistant","timestamp":"2026-07-13T10:08:12.767Z","requestId":"req_2",\
    "message":{"id":"msg_2","model":"claude-opus-5","usage":{"input_tokens":1,\
    "output_tokens":1,"cache_creation_input_tokens":500}}}
    """
    let e = JSONLParser.event(from: line(noSplit))!
    #expect(e.cacheWrite5m == 500)
    #expect(e.cacheWrite1h == 0)
}

@Test func skipsSyntheticModel() {
    let synthetic = assistantLine.replacingOccurrences(of: #""claude-opus-5""#,
                                                       with: #""<synthetic>""#)
    #expect(JSONLParser.event(from: line(synthetic)) == nil)
}

@Test func skipsNonAssistantAndMalformedLines() {
    let user = #"{"type":"user","timestamp":"2026-07-13T10:08:12.767Z","message":{"usage":{}}}"#
    #expect(JSONLParser.event(from: line(user)) == nil)
    #expect(JSONLParser.event(from: line(#"{"type":"assistant","usage" broken"#)) == nil)
    #expect(JSONLParser.event(from: line("")) == nil)
}

@Test func parsesTimestampWithoutFractionalSeconds() {
    let plain = assistantLine.replacingOccurrences(of: "10:08:12.767Z", with: "10:08:12Z")
    #expect(JSONLParser.event(from: line(plain)) != nil)
}
