import Foundation
import Testing
@testable import CCUsageCore

private func makeLine(id: String, request: String, output: UInt32) -> String {
    """
    {"type":"assistant","timestamp":"2026-08-17T10:00:00.000Z","requestId":"\(request)",\
    "message":{"id":"\(id)","model":"claude-opus-5","usage":{"input_tokens":1,\
    "output_tokens":\(output),"cache_read_input_tokens":0}}}
    """
}

private func makeTempRoot() throws -> URL {
    let root = URL(fileURLWithPath: NSTemporaryDirectory())
        .appending(path: "cctc-\(UUID().uuidString)")
    try FileManager.default.createDirectory(at: root.appending(path: "projA"),
                                            withIntermediateDirectories: true)
    return root
}

@Test func ingestsEventsFromNestedJSONLFiles() throws {
    let root = try makeTempRoot()
    defer { try? FileManager.default.removeItem(at: root) }
    let file = root.appending(path: "projA/session.jsonl")
    try (makeLine(id: "m1", request: "r1", output: 10) + "\n")
        .write(to: file, atomically: true, encoding: .utf8)

    let scanner = ProjectScanner(root: root)
    let result = try scanner.ingest(since: .distantPast, cache: ParseCache())
    #expect(result.events.count == 1)
    #expect(result.events[0].output == 10)
}

@Test func dedupesIdenticalMessageAcrossFiles() throws {
    let root = try makeTempRoot()
    defer { try? FileManager.default.removeItem(at: root) }
    let same = makeLine(id: "m1", request: "r1", output: 10) + "\n"
    try same.write(to: root.appending(path: "projA/a.jsonl"), atomically: true, encoding: .utf8)
    try same.write(to: root.appending(path: "projA/b.jsonl"), atomically: true, encoding: .utf8)

    let result = try ProjectScanner(root: root).ingest(since: .distantPast, cache: ParseCache())
    #expect(result.events.count == 1)
}

@Test func secondIngestReadsOnlyTheAppendedDelta() throws {
    let root = try makeTempRoot()
    defer { try? FileManager.default.removeItem(at: root) }
    let file = root.appending(path: "projA/session.jsonl")
    try (makeLine(id: "m1", request: "r1", output: 10) + "\n")
        .write(to: file, atomically: true, encoding: .utf8)

    let scanner = ProjectScanner(root: root)
    let first = try scanner.ingest(since: .distantPast, cache: ParseCache())
    #expect(first.events.count == 1)

    let handle = try FileHandle(forWritingTo: file)
    try handle.seekToEnd()
    try handle.write(contentsOf: Data((makeLine(id: "m2", request: "r2", output: 20) + "\n").utf8))
    try handle.close()

    let second = try scanner.ingest(since: .distantPast, cache: first.cache)
    #expect(second.events.count == 1)
    #expect(second.events[0].output == 20)
}

@Test func truncatedFileTriggersFullReparse() throws {
    let root = try makeTempRoot()
    defer { try? FileManager.default.removeItem(at: root) }
    let file = root.appending(path: "projA/session.jsonl")
    let twoLines = makeLine(id: "m1", request: "r1", output: 10) + "\n"
        + makeLine(id: "m2", request: "r2", output: 20) + "\n"
    try twoLines.write(to: file, atomically: true, encoding: .utf8)

    let scanner = ProjectScanner(root: root)
    let first = try scanner.ingest(since: .distantPast, cache: ParseCache())
    #expect(first.events.count == 2)

    try (makeLine(id: "m3", request: "r3", output: 30) + "\n")
        .write(to: file, atomically: true, encoding: .utf8)

    let second = try scanner.ingest(since: .distantPast, cache: first.cache)
    #expect(second.events.count == 1)
    #expect(second.events[0].output == 30)
}

@Test func missingRootYieldsNoEvents() throws {
    let missing = URL(fileURLWithPath: NSTemporaryDirectory())
        .appending(path: "cctc-absent-\(UUID().uuidString)")
    let result = try ProjectScanner(root: missing).ingest(since: .distantPast, cache: ParseCache())
    #expect(result.events.isEmpty)
}
