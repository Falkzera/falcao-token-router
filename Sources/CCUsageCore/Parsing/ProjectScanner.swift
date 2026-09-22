import Foundation

public struct ProjectScanner: Sendable {
    public let root: URL

    public init(root: URL = FileManager.default.homeDirectoryForCurrentUser
                                .appending(path: ".claude/projects")) {
        self.root = root
    }

    /// Arquivos `.jsonl` cujo `mtime` é igual ou posterior a `since`.
    ///
    /// Descartar por mtime é **correto**, não só otimização: o log é append-only,
    /// então nenhum arquivo pode conter um evento mais novo que seu próprio mtime.
    public func files(modifiedSince since: Date) throws -> [URL] {
        let fm = FileManager.default
        guard fm.fileExists(atPath: root.path) else { return [] }
        guard let walker = fm.enumerator(
            at: root,
            includingPropertiesForKeys: [.contentModificationDateKey, .isRegularFileKey],
            options: [.skipsHiddenFiles]
        ) else { return [] }

        var result: [URL] = []
        for case let url as URL in walker {
            guard url.pathExtension == "jsonl" else { continue }
            let values = try? url.resourceValues(forKeys: [.contentModificationDateKey,
                                                           .isRegularFileKey])
            guard values?.isRegularFile == true else { continue }
            guard let mtime = values?.contentModificationDate, mtime >= since else { continue }
            result.append(url)
        }
        return result
    }

    /// Lê apenas o que ainda não foi lido e devolve os eventos novos, já
    /// deduplicados, junto com o cache atualizado.
    public func ingest(
        since: Date,
        cache: ParseCache
    ) throws -> (events: [UsageEvent], cache: ParseCache) {
        var cache = cache
        var events: [UsageEvent] = []
        var seen = Set<String>()

        for url in try files(modifiedSince: since) {
            let key = url.path
            let values = try? url.resourceValues(forKeys: [.fileSizeKey,
                                                           .contentModificationDateKey])
            guard let size = values?.fileSize.map(UInt64.init),
                  let mtime = values?.contentModificationDate
            else { continue }

            var offset = cache.files[key]?.byteOffset ?? 0
            // Arquivo encolheu → foi truncado ou reescrito; reparseia do zero.
            if size < offset { offset = 0 }
            guard size > offset else {
                cache.files[key] = FileState(size: size, mtime: mtime, byteOffset: offset)
                continue
            }

            guard let handle = try? FileHandle(forReadingFrom: url) else { continue }
            defer { try? handle.close() }
            try handle.seek(toOffset: offset)
            guard let data = try handle.readToEnd() else { continue }

            var consumed = offset
            let lines = data.split(separator: UInt8(ascii: "\n"),
                                   omittingEmptySubsequences: false).dropLast()
            for lineBytes in lines {
                consumed += UInt64(lineBytes.count) + 1  // +1 pelo \n consumido
                guard let event = JSONLParser.event(from: Data(lineBytes)) else { continue }
                guard seen.insert(event.dedupeKey).inserted else { continue }
                events.append(event)
            }

            // `consumed` para na última quebra de linha: uma linha parcial no
            // fim do arquivo (o Claude Code ainda escrevendo) é relida inteira
            // na próxima passada em vez de ser parseada pela metade.
            cache.files[key] = FileState(size: size, mtime: mtime, byteOffset: consumed)
        }

        return (events, cache)
    }
}
