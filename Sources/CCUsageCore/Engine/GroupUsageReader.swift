import Foundation

/// Traduz as amostras que o sensor grava em "quanto cada conta usou".
///
/// Fonte única: os arquivos por conta em `usageDir`, escritos pela status line a
/// partir do `rate_limits` do Claude Code. Nenhuma chamada de rede aqui.
///
/// **Limite considerado:** o maior entre a janela de 5h, a de 7 dias e a mais
/// apertada POR MODELO — é qualquer um dos três que, ao bater o limiar, deve
/// disparar a troca.
///
/// O por modelo entrou em 18/09/2026 e é o que fechou a lacuna da v1: ele não
/// chega no `rate_limits` da status line, e é justamente o que estoura primeiro
/// (uma conta travou com `5h 91%` / `7d 79%` e `Fable 100%`). Ele vem da sonda
/// ativa (`ClaudeUsageProbe`), que roda sob demanda; conta que nunca foi sondada
/// simplesmente não tem esse número, e aí a decisão é como antes.
public struct GroupUsageReader: Sendable {
    private let usageDir: URL

    public init(usageDir: URL) {
        self.usageDir = usageDir
    }

    /// A amostra mais recente de cada conta, por e-mail.
    public func samplesByEmail() -> [String: GroupUsageSample] {
        var byEmail: [String: GroupUsageSample] = [:]
        for sample in GroupUsageStore.readAll(in: usageDir) {
            guard let email = sample.email else { continue }
            if let existing = byEmail[email], existing.sampledAt >= sample.sampledAt { continue }
            byEmail[email] = sample
        }
        return byEmail
    }

    /// A amostra mais recente de cada conta do config, pelo id da conta.
    public func samplesByAccount(_ config: RouterConfig) -> [UUID: GroupUsageSample] {
        let byEmail = samplesByEmail()
        var result: [UUID: GroupUsageSample] = [:]
        for account in config.accounts {
            if let sample = byEmail[account.identity.email] { result[account.id] = sample }
        }
        return result
    }

    /// O uso de cada conta **com a procedência do número**, para a UI poder
    /// dizer de qual janela ele veio. Contas sem nenhuma janela válida não
    /// aparecem — o motor as trata como desconhecidas (presumidas frescas).
    ///
    /// Janela cujo reset já passou é descartada: a amostra envelheceu além do
    /// próprio limite que media, e mantê-la deixaria uma conta "cheia" para
    /// sempre aos olhos da rotação.
    public func detailByAccount(_ config: RouterConfig, now: Date = Date()) -> [UUID: AccountUsage] {
        var result: [UUID: AccountUsage] = [:]
        for (id, sample) in samplesByAccount(config) {
            let fiveValid = sample.fiveHourResetsAt.map { $0 > now } ?? true
            let sevenValid = sample.sevenDayResetsAt.map { $0 > now } ?? true
            let five = fiveValid ? sample.fiveHourPercent : nil
            let seven = sevenValid ? sample.sevenDayPercent : nil

            // A janela por modelo mais apertada que ainda vale. Mesmo descarte
            // por reset vencido das outras duas.
            let model = sample.models?.binding(now: now)

            // Candidatas, e a que manda é a maior. No empate vale a de horizonte
            // mais longo: as duas limitam igual, mas a que leva dias para
            // aliviar é a que descreve melhor a situação. Daí a ordem —
            // `max(by:)` devolve o ÚLTIMO dos empatados, então a mais longa vem
            // depois.
            let candidatas: [(fraction: Double, window: AccountUsage.Window)] =
                [five.map { ($0, AccountUsage.Window.fiveHour) },
                 seven.map { ($0, AccountUsage.Window.sevenDay) },
                 model.map { ($0.percent, AccountUsage.Window.model($0.name)) }]
                .compactMap { $0 }
            guard let bound = candidatas.max(by: { $0.fraction <= $1.fraction })
            else { continue }

            result[id] = AccountUsage(
                fraction: bound.fraction, window: bound.window,
                fiveHour: five, fiveHourResetsAt: five == nil ? nil : sample.fiveHourResetsAt,
                sevenDay: seven, sevenDayResetsAt: seven == nil ? nil : sample.sevenDayResetsAt,
                model: model, modelSampledAt: model == nil ? nil : sample.models?.sampledAt,
                origin: sample.origin, sampledAt: sample.sampledAt)
        }
        return result
    }

    /// Uso 0–1 por conta (id), para a decisão de rotação — a mesma fração de
    /// `detailByAccount`, sem a procedência que só a UI precisa.
    public func usageByAccount(_ config: RouterConfig, now: Date = Date()) -> [UUID: Double] {
        detailByAccount(config, now: now).mapValues(\.fraction)
    }
}

/// O uso que decide, junto com a **procedência** do número.
///
/// Existe porque o painel mostrava só a fração, e o usuário não tinha como
/// saber de qual janela ela vinha: ver "66%" no painel ao lado de uma status
/// line escrita `5h 1%  7d 66%` parece contradição, e não é — é o maior dos
/// dois, que é exatamente o que dispara a troca. Carregar a janela vencedora e
/// as duas medidas deixa a UI responder isso sem o usuário ter que adivinhar.
public struct AccountUsage: Sendable, Equatable {
    /// Qual janela está mandando no número. `model` carrega o nome cru que o
    /// `/usage` imprime (`Fable`, `Opus`), porque é o que a UI mostra e o app
    /// não mantém lista de modelos.
    public enum Window: Sendable, Equatable {
        case fiveHour, sevenDay, model(String)
    }

    /// O maior entre as janelas válidas — o número que a rotação compara com o
    /// limiar do grupo.
    public let fraction: Double
    /// De qual janela `fraction` veio.
    public let window: Window
    /// As duas janelas ainda válidas, para a UI mostrar o quadro inteiro. O
    /// reset acompanha cada uma: é o que o detalhamento por conta usa para dizer
    /// "reseta 13:20 · em 4h 6m" sem inventar nada.
    public let fiveHour: Double?
    public let fiveHourResetsAt: Date?
    public let sevenDay: Double?
    public let sevenDayResetsAt: Date?
    /// A janela por modelo mais apertada, quando a conta já foi sondada.
    public let model: ClaudeUsageProbe.ModelWindow?
    /// Quando a SONDA rodou — idade própria, porque ela roda sob demanda e as
    /// outras duas a cada mensagem. Sem isto, um Fable de ontem apareceria com
    /// a idade do 5h de agora.
    public let modelSampledAt: Date?
    /// Quem mediu as janelas de 5h e 7 dias: o sensor (a requisição que a conta
    /// atendeu) ou a sonda (uma consulta feita de propósito). A UI precisa saber
    /// para não atribuir ao sensor um número que a sonda buscou — a frase que
    /// acompanhava dizia "pela sessão da própria conta", e conta ociosa nunca
    /// serviu sessão nenhuma.
    public let origin: UsageOrigin
    public let sampledAt: Date

    public init(fraction: Double, window: Window,
                fiveHour: Double?, fiveHourResetsAt: Date?,
                sevenDay: Double?, sevenDayResetsAt: Date?,
                model: ClaudeUsageProbe.ModelWindow? = nil, modelSampledAt: Date? = nil,
                origin: UsageOrigin = .sensor, sampledAt: Date) {
        self.fraction = fraction
        self.window = window
        self.fiveHour = fiveHour
        self.fiveHourResetsAt = fiveHourResetsAt
        self.sevenDay = sevenDay
        self.sevenDayResetsAt = sevenDayResetsAt
        self.model = model
        self.modelSampledAt = modelSampledAt
        self.origin = origin
        self.sampledAt = sampledAt
    }
}
