import Foundation

/// Observa um diretório e chama `onChange` com debounce.
///
/// O Claude Code escreve continuamente durante uma sessão; sem o debounce o app
/// reparsearia a cada token emitido.
public final class FSWatcher: @unchecked Sendable {
    private let url: URL
    private let debounce: TimeInterval
    private let onChange: @Sendable () -> Void
    private let queue = DispatchQueue(label: "com.synqo.falcao-token-router.fswatcher")

    private var source: DispatchSourceFileSystemObject?
    private var descriptor: CInt = -1
    private var pending: DispatchWorkItem?

    public init(url: URL, debounce: TimeInterval = 2.0,
                onChange: @escaping @Sendable () -> Void) {
        self.url = url
        self.debounce = debounce
        self.onChange = onChange
    }

    public func start() {
        stop()
        let fd = open(url.path, O_EVTONLY)
        guard fd >= 0 else { return }
        descriptor = fd

        let source = DispatchSource.makeFileSystemObjectSource(
            fileDescriptor: fd, eventMask: [.write, .extend, .attrib], queue: queue)
        source.setEventHandler { [weak self] in self?.scheduleCallback() }
        source.setCancelHandler { close(fd) }
        source.resume()
        self.source = source
    }

    public func stop() {
        pending?.cancel()
        pending = nil
        source?.cancel()
        source = nil
        descriptor = -1
    }

    private func scheduleCallback() {
        pending?.cancel()
        let work = DispatchWorkItem { [onChange] in onChange() }
        pending = work
        queue.asyncAfter(deadline: .now() + debounce, execute: work)
    }

    deinit { stop() }
}
