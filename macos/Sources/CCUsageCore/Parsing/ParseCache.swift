import Foundation

public struct FileState: Codable, Sendable, Equatable {
    public var size: UInt64
    public var mtime: Date
    /// Offset em bytes até onde o arquivo já foi parseado.
    public var byteOffset: UInt64

    public init(size: UInt64, mtime: Date, byteOffset: UInt64) {
        self.size = size
        self.mtime = mtime
        self.byteOffset = byteOffset
    }
}

/// Estado persistido entre execuções: até onde cada arquivo foi lido **e** os
/// eventos que essa leitura produziu.
///
/// Guardar só os offsets seria pior que não guardar nada: no segundo launch o
/// `ingest` devolveria zero eventos (tudo "já lido") e o app acordaria sem
/// histórico nenhum, com os tetos caindo no piso e todos os gauges saturados.
public struct ParseCache: Codable, Sendable, Equatable {
    public var files: [String: FileState]
    public var events: [UsageEvent]

    public init(files: [String: FileState] = [:], events: [UsageEvent] = []) {
        self.files = files
        self.events = events
    }

    public static func load(from url: URL) -> ParseCache {
        // Cache corrompido ou ausente não é erro: descarta e reparseia tudo.
        guard let data = try? Data(contentsOf: url),
              let cache = try? JSONDecoder().decode(ParseCache.self, from: data)
        else { return ParseCache() }
        return cache
    }

    public func save(to url: URL) throws {
        try FileManager.default.createDirectory(
            at: url.deletingLastPathComponent(), withIntermediateDirectories: true)
        try JSONEncoder().encode(self).write(to: url, options: .atomic)
    }

    /// `~/Library/Application Support/FalcaoTokenRouter/cache.json`
    public static var defaultURL: URL {
        FileManager.default
            .homeDirectoryForCurrentUser
            .appending(path: "Library/Application Support/FalcaoTokenRouter/cache.json")
    }
}
