import Foundation
import Observation

/// Fachada observável: mantém os eventos em memória, reage a mudanças no disco
/// e publica um `UsageSnapshot`. É o único tipo que a UI conhece.
@MainActor
@Observable
public final class UsageStore {
    public private(set) var snapshot: UsageSnapshot

    /// Chamado a cada snapshot publicado. Existe para os alertas, e o store
    /// segue sem saber que notificação existe — ele entrega o fato e pronto.
    ///
    /// `@ObservationIgnored` porque é colaborador, não estado observável:
    /// sem isso, atribuir o gancho invalidaria views que leem o store.
    @ObservationIgnored public var onSnapshot: (@MainActor (UsageSnapshot) -> Void)?
    public private(set) var isLoading = false

    /// Teto manual das settings; `nil` usa a calibração automática.
    public var ceilingOverride: Ceilings? {
        didSet { rebuild() }
    }

    /// Como ler o número ao vivo — a amostra recente do sensor. `nil` quando não
    /// há amostra recente, que não é erro (ver `UsageSourcePolicy`). Injetável
    /// para os testes.
    public typealias LiveFetch = @Sendable (Date) async -> UsageReport?

    /// Liga a busca ao vivo. Ligar dispara uma busca imediata.
    ///
    /// O `lastLive = nil` importa no **religar**, não no desligar: com o toggle
    /// desligado a política já ignora o resultado ao vivo, e a UI volta ao cache
    /// de qualquer jeito. Sem o descarte, religar faria o `rebuild()` síncrono
    /// logo abaixo mostrar o número ao vivo *antigo*, com procedência `.live` —
    /// e `age(at:)` devolve `nil` para `.live` por desenho, então o painel diria
    /// "ao vivo" sem nenhum jeito de o usuário perceber que o dado é velho. É a
    /// mesma mentira que esta mudança existe para eliminar.
    public var liveUsageEnabled: Bool {
        didSet {
            guard liveUsageEnabled != oldValue else { return }
            lastLive = nil
            rebuild()
            if liveUsageEnabled { Task { await refreshLive() } }
        }
    }

    private let scanner: ProjectScanner
    private let cacheURL: URL
    private let cachedUsageURL: URL
    private let lookback: TimeInterval
    private let fetchLive: LiveFetch
    private var lastLive: UsageReport?
    private var lastLiveAttempt: Date?
    private var liveTicker: Task<Void, Never>?
    private var isFetchingLive = false

    private var events: [UsageEvent] = []
    private var seenKeys = Set<String>()
    private var cache = ParseCache()
    private var watcher: FSWatcher?
    private var ticker: Task<Void, Never>?

    /// A fonte "ao vivo" é o **sensor**, não uma chamada de API própria.
    ///
    /// O app nunca fala com `api.anthropic.com` — os termos reservam o token
    /// OAuth para o cliente oficial. Em vez disso, lê a amostra que a status line
    /// do Claude Code grava por conta (o `rate_limits` que ele mesmo recebe), e
    /// só a considera "ao vivo" se for recente; senão, o painel cai para o cache
    /// de `~/.claude.json` e mostra a idade, sem fingir frescor.
    ///
    /// Isso lê apenas arquivos locais do próprio usuário: nenhum token nosso sai
    /// da máquina, nenhuma requisição é feita.
    nonisolated static let liveFreshness: TimeInterval = 3600

    public static let defaultLiveFetch: LiveFetch = { now in
        let dir = ConfigDir.standard()
        guard let email = AnthropicAdapter().identity(inConfigDir: dir)?.email,
              let sample = GroupUsageStore.read(
                forEmail: email, in: RouterPaths().usageDir),
              now.timeIntervalSince(sample.sampledAt) < liveFreshness
        else { return nil }
        return sample.asUsageReport()
    }

    public init(
        scanner: ProjectScanner = ProjectScanner(),
        cacheURL: URL = ParseCache.defaultURL,
        cachedUsageURL: URL = CachedUsageReader.defaultURL,
        lookback: TimeInterval = 90 * 24 * 60 * 60,
        liveUsageEnabled: Bool = false,
        fetchLive: @escaping LiveFetch = UsageStore.defaultLiveFetch
    ) {
        self.scanner = scanner
        self.cacheURL = cacheURL
        self.cachedUsageURL = cachedUsageURL
        self.lookback = lookback
        self.liveUsageEnabled = liveUsageEnabled
        self.fetchLive = fetchLive
        self.snapshot = .empty(at: Date())
    }

    /// Lê o delta do disco, funde com o que já está em memória e reconstrói o snapshot.
    public func refresh() async {
        isLoading = true
        defer { isLoading = false }

        let scanner = self.scanner
        let cache = self.cache
        let since = Date().addingTimeInterval(-lookback)

        let result: (events: [UsageEvent], cache: ParseCache)
        do {
            result = try await Task.detached(priority: .utility) {
                try scanner.ingest(since: since, cache: cache)
            }.value
        } catch {
            // Disco indisponível ou permissão negada: mantém o último snapshot bom.
            return
        }

        self.cache = result.cache
        for event in result.events where seenKeys.insert(event.dedupeKey).inserted {
            events.append(event)
        }

        // Descarta o que já saiu da janela de interesse, senão o arquivo de
        // cache cresce sem limite.
        let horizon = Date().addingTimeInterval(-lookback)
        if events.contains(where: { $0.timestamp < horizon }) {
            events.removeAll { $0.timestamp < horizon }
            seenKeys = Set(events.map(\.dedupeKey))
        }

        self.cache.events = events
        try? self.cache.save(to: cacheURL)
        rebuild()
    }

    /// Lê o número ao vivo.
    ///
    /// Uma leitura por vez. O `await` abaixo suspende, e sem esta guarda duas
    /// invocações se atropelam: a mais lenta termina por último e uma leitura
    /// velha sobrescreve a mais nova.
    public func refreshLive() async {
        guard liveUsageEnabled, !isFetchingLive else { return }
        isFetchingLive = true
        defer { isFetchingLive = false }
        let now = Date()
        lastLiveAttempt = now
        lastLive = await fetchLive(now)
        rebuild()
    }

    /// O painel abriu. Vale uma leitura fora de hora: o número na tela não
    /// precisa esperar o próximo tique de cinco minutos.
    ///
    /// Represado em 30s porque abrir e fechar o menu é gesto barato e repetido,
    /// e reler o disco a cada um não traz número novo.
    public func panelDidOpen() {
        if let lastLiveAttempt, Date().timeIntervalSince(lastLiveAttempt) < 30 { return }
        Task { await refreshLive() }
    }

    public func start() {
        cache = ParseCache.load(from: cacheURL)
        // Restaura o histórico antes de tocar no disco: o painel abre com dados
        // completos em vez de esperar os segundos da varredura.
        events = cache.events
        seenKeys = Set(events.map(\.dedupeKey))
        rebuild()

        Task { await refresh() }

        watcher = FSWatcher(url: scanner.root) { [weak self] in
            Task { @MainActor in await self?.refresh() }
        }
        watcher?.start()

        // O bloco de 5h continua correndo mesmo sem escrita nova no disco:
        // o tempo até o reset precisa avançar sozinho.
        ticker = Task { [weak self] in
            while !Task.isCancelled {
                try? await Task.sleep(for: .seconds(30))
                await MainActor.run { self?.rebuild() }
            }
        }

        // Cadência fixa por ora. A política adaptativa (bateria, pressão
        // térmica, ociosidade) encaixa aqui trocando a constante por uma
        // função, sem mexer no resto.
        liveTicker = Task { [weak self] in
            while !Task.isCancelled {
                await self?.refreshLive()
                try? await Task.sleep(for: .seconds(300))
            }
        }
    }

    public func stop() {
        watcher?.stop()
        watcher = nil
        ticker?.cancel()
        ticker = nil
        liveTicker?.cancel()
        liveTicker = nil
    }

    private func rebuild() {
        let now = Date()
        // Relido a cada reconstrução: o arquivo é pequeno e o cache se move
        // sozinho enquanto o Claude Code roda.
        let cached = CachedUsageReader.read(from: cachedUsageURL)
        let (official, status) = UsageSourcePolicy.select(
            liveEnabled: liveUsageEnabled, live: lastLive, cached: cached, now: now)
        snapshot = SnapshotBuilder.build(
            from: events, now: now, calendar: .current,
            override: ceilingOverride, official: official, status: status)
        onSnapshot?(snapshot)
    }
}
