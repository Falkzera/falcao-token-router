import SwiftUI
import CCUsageCore

/// As contas de cada grupo, com quem serve o grupo e quanto sobrou.
///
/// Existe porque o resto do painel mede **o perfil onde o app roda** — e isso
/// não diz qual conta atende os outros grupos, nem quanto sobrou nelas. É a
/// resposta para "em qual conta eu estou?", que o `/status` do Claude Code dá
/// só para a sessão de agora.
///
/// Até 18/09/2026 esta view tinha um segundo caminho, o "panorama": uma leitura
/// do `cswap`/`teamclaude` via `panorama.py`, de antes dos grupos existirem. Ele
/// saiu — com grupos criados nunca era desenhado, e o store que o alimentava
/// lançava um `python3` a cada 60s para um resultado que ninguém lia.

/// As larguras da tabela de contas, definidas UMA vez.
///
/// O cabeçalho e as linhas precisam concordar sobre onde cada coluna começa.
/// Enquanto cada view escolhia a sua largura, a linha sem amostra ("pronta")
/// tinha geometria diferente da linha com número e as colunas dançavam de conta
/// para conta — o painel virava uma lista de números soltos, que é o defeito
/// que esta tabela existe para resolver.
enum AccountsLayout {
    static let dot: CGFloat = 6
    static let bar: CGFloat = 40
    static let number: CGFloat = 34
    /// Respiro entre a barra e os números, e entre os dois números.
    static let gap: CGFloat = 8
    /// O bloco inteiro da direita (barra + as duas janelas). A linha sem
    /// amostra ocupa exatamente isto, para "pronta" cair SOB as colunas.
    static var readings: CGFloat { bar + number * 2 + gap * 2 }
}

/// A forma é uma TABELA, não uma lista de linhas independentes: o rótulo da
/// janela ("5h", "7d") sobe para o cabeçalho, os números descem em colunas de
/// largura fixa, e a conta que está servindo o grupo ganha fundo próprio. A
/// pergunta que traz o usuário aqui é "em qual conta eu estou, e quanto sobrou
/// nas outras?" — as duas se respondem varrendo uma coluna, sem ler cada linha.
struct AccountsSection: View {
    @Bindable var store: RouterConfigStore
    @Binding var selection: UUID?

    var body: some View {
        VStack(alignment: .leading, spacing: 10) {
            VStack(alignment: .leading, spacing: 6) {
                header
                Divider().opacity(0.5)
            }
            ForEach(store.config.groups) { group in
                let accounts = store.config.accounts(in: group)
                if !accounts.isEmpty {
                    VStack(alignment: .leading, spacing: 1) {
                        HStack(spacing: 5) {
                            Text(verbatim: group.name)
                                .font(.caption2.weight(.medium))
                                .foregroundStyle(.secondary)
                            SessionsBadge(sessions: store.liveSessions[group.id] ?? [])
                        }
                        .padding(.horizontal, 6)
                        .padding(.bottom, 2)
                        ForEach(accounts) { account in
                            RouterAccountRow(
                                label: account.label,
                                isActive: store.activeByGroup[group.id] == account.id,
                                usage: store.usageDetail[account.id],
                                sampledAt: store.usageSampledAt[account.id],
                                isSelected: selection == account.id,
                                // Clicar de novo na mesma conta volta para a
                                // sessão: sem isso o painel entra no modo conta
                                // e não tem como sair pela própria tabela.
                                onTap: { selection = selection == account.id ? nil : account.id })
                        }
                    }
                }
            }
        }
        .onAppear { store.refreshUsage() }
    }

    /// O título e os nomes das colunas na MESMA linha: o rótulo da janela é
    /// informação de coluna, não de linha, e dito uma vez ele para de disputar
    /// espaço com o nome da conta.
    private var header: some View {
        HStack(spacing: 0) {
            Text("panel.section.accounts")
                .font(.caption2.weight(.semibold))
                .foregroundStyle(.secondary)
            Spacer(minLength: 8)
            HStack(spacing: AccountsLayout.gap) {
                Color.clear.frame(width: AccountsLayout.bar, height: 1)
                Text("panel.accounts.window.fiveHour")
                    .frame(width: AccountsLayout.number, alignment: .trailing)
                Text("panel.accounts.window.sevenDay")
                    .frame(width: AccountsLayout.number, alignment: .trailing)
            }
            .font(.caption2)
            .foregroundStyle(.tertiary)
        }
        .padding(.horizontal, 6)
        .help(String(localized: "panel.accounts.columns.help"))
    }
}

private struct RouterAccountRow: View {
    let label: String
    let isActive: Bool
    /// O uso COM procedência: sem saber a janela, a linha mostraria um número
    /// solto que o usuário lê como sendo o das 5 horas — e quase sempre é o
    /// semanal, porque o painel mostra o maior dos dois.
    let usage: AccountUsage?
    let sampledAt: Date?
    let isSelected: Bool
    let onTap: () -> Void

    /// Amostra passiva com mais de 1h já pode descrever outra realidade — o
    /// mesmo limiar da linha de procedência do painel. Esmaece, sem esconder.
    private var isStale: Bool {
        sampledAt.map { Date().timeIntervalSince($0) > UsageAge.stale } ?? false
    }

    /// Meio dia sem medir é outra ordem de problema: em conta compartilhada, o
    /// número já não descreve o que os colegas gastaram. Esmaecer não basta —
    /// aqui entra marca explícita, porque o risco é acreditar num valor otimista.
    private var isVeryStale: Bool {
        sampledAt.map { Date().timeIntervalSince($0) > UsageAge.veryStale } ?? false
    }

    /// O quadro inteiro no tooltip: as duas janelas do sensor, a do modelo
    /// quando alguém sondou, e a idade de cada origem.
    private var usageHelp: String { AccountHelp.text(for: usage) }

    /// Selecionada manda no fundo: enquanto o detalhamento embaixo é dela, a
    /// linha precisa dizer de quem ele fala — mais forte que a marca de ativa,
    /// que continua valendo para todas as outras.
    private var background: AnyShapeStyle {
        if isSelected { return AnyShapeStyle(.selection.opacity(0.5)) }
        return isActive ? AnyShapeStyle(.quaternary) : AnyShapeStyle(.clear)
    }

    private var staleHelp: String {
        String(format: String(localized: usage?.origin == .probe
                              ? "panel.accounts.stale.help.probe.format"
                              : "panel.accounts.stale.help.sensor.format"),
               Format.duration(Date().timeIntervalSince(sampledAt ?? Date())))
    }

    var body: some View {
        HStack(spacing: 0) {
            HStack(spacing: 6) {
                // A conta ativa é a informação mais procurada da seção; o ponto
                // cheio encontra o olho antes do negrito, e o fundo da linha
                // confirma sem depender de cor (o ponto usa a cor de severidade,
                // que num dia calmo é quase o cinza do inativo).
                Circle()
                    .fill(isActive ? UsageColor.bar(usage?.fraction ?? 0) : .clear)
                    .strokeBorder(isActive ? .clear : Color.secondary.opacity(0.35),
                                  lineWidth: 1)
                    .frame(width: AccountsLayout.dot, height: AccountsLayout.dot)
                Text(verbatim: label)
                    .font(.caption).fontWeight(isActive ? .semibold : .regular)
                    .lineLimit(1).truncationMode(.middle)
                if case .model = usage?.window, let modelo = usage?.model {
                    ModelBadge(window: modelo)
                }
                if isVeryStale {
                    Image(systemName: "clock.badge.exclamationmark")
                        .font(.caption2).foregroundStyle(.tertiary)
                        .help(staleHelp)
                }
            }
            Spacer(minLength: 8)
            HStack(spacing: AccountsLayout.gap) {
                if let usage {
                    // A barra desenha a janela de 5h, a coluna que ela encosta.
                    // Desenhava a que DECIDE (quase sempre a semanal) e virava
                    // uma barra cheia ao lado de um "—" ou de um "2%": o olho
                    // casa o gráfico com o número vizinho, e ali ele mentia.
                    MiniBar(fraction: usage.fiveHour)
                        .opacity(isStale ? 0.45 : 1)
                    WindowReading(label: "panel.accounts.window.fiveHour",
                                  fraction: usage.fiveHour,
                                  isBound: usage.window == .fiveHour,
                                  isStale: isStale,
                                  layout: .column)
                    WindowReading(label: "panel.accounts.window.sevenDay",
                                  fraction: usage.sevenDay,
                                  isBound: usage.window == .sevenDay,
                                  isStale: isStale,
                                  layout: .column)
                } else {
                    // Sem amostra a conta está PRONTA (presumida fresca, entra no
                    // rodízio sozinha) — um traço leria como "quebrada". Ocupa a
                    // largura das três colunas para não desalinhar as de cima.
                    Text("panel.accounts.ready")
                        .font(.caption2).foregroundStyle(.tertiary)
                        .frame(width: AccountsLayout.readings, alignment: .trailing)
                }
            }
        }
        .padding(.vertical, 3)
        .padding(.horizontal, 6)
        .background(
            RoundedRectangle(cornerRadius: 6, style: .continuous)
                .fill(background))
        // A linha inteira é o alvo do clique: o número tem 34 pontos, e o vazio
        // entre o nome e a coluna é a maior parte da linha — sem `contentShape`
        // ele não conta, e o clique parece falhar justamente onde se mira.
        .contentShape(.rect)
        .onTapGesture(perform: onTap)
        // O tooltip é da LINHA inteira, não de cada número: a pergunta ("por que
        // 66% se a status line diz 1%?") é sobre a conta, e mirar um alvo de
        // 34 pontos para obtê-la era pedir precisão que ninguém tem com o mouse.
        .help(usageHelp)
    }
}

/// O tooltip de uma linha de conta, num lugar só — o painel e a tela de Grupos
/// precisam contar a mesma história.
///
/// Duas origens, duas idades. As janelas de 5h e 7d vêm do sensor passivo, que
/// lê a cada mensagem; a do modelo vem da sonda, que roda quando alguém pede.
/// Dizer uma idade só para as duas seria afirmar que o número do Fable é tão
/// fresco quanto o das 5 horas — e é justamente o do Fable que trava a conta.
enum AccountHelp {
    static func text(for usage: AccountUsage?) -> String {
        guard let usage else { return String(localized: "panel.accounts.ready.help.probe") }
        let ausente = String(localized: "panel.accounts.window.absent")
        var linhas = [String(
            format: String(localized: "panel.accounts.usage.help.format"),
            usage.fiveHour.map(UsagePercent.text) ?? ausente,
            usage.sevenDay.map(UsagePercent.text) ?? ausente,
            Format.duration(Date().timeIntervalSince(usage.sampledAt)))]
        // QUEM mediu, e não só quando. As duas fontes são oficiais e dizem
        // coisas diferentes: o sensor é o gasto que a conta viu ao atender uma
        // requisição; a sonda é uma consulta de um instante, e é a única que
        // serve para conta ociosa.
        linhas.append(String(localized: usage.origin == .probe
                             ? "panel.accounts.origin.probe"
                             : "panel.accounts.origin.sensor"))
        if let modelo = usage.model, let quando = usage.modelSampledAt {
            linhas.append(String(
                format: String(localized: "panel.accounts.model.help.format"),
                modelo.name, UsagePercent.text(modelo.percent),
                Format.duration(Date().timeIntervalSince(quando))))
        }
        return linhas.joined(separator: "\n\n")
    }
}

/// A marca de que quem manda no número é uma janela POR MODELO.
///
/// Sem ela a linha mentiria por omissão: com o modelo mandando, nem a coluna de
/// 5h nem a de 7d ficam destacadas, e o usuário veria duas folgas confortáveis
/// sem nada explicando por que a conta vai trocar. O nome vem cru do `/usage`
/// (`Fable`, `Opus`) — o app não mantém lista de modelos.
struct ModelBadge: View {
    let window: ClaudeUsageProbe.ModelWindow

    var body: some View {
        Text(verbatim: "\(window.name) \(UsagePercent.text(window.percent))")
            .font(.caption2.monospacedDigit())
            .padding(.horizontal, 5).padding(.vertical, 1)
            .background(UsageColor.bar(window.percent).opacity(0.22), in: .capsule)
            .foregroundStyle(UsageColor.bar(window.percent))
    }
}

/// Quantas sessões do Claude Code estão vivas num grupo, e quantas delas estão
/// trabalhando ou esperando o usuário.
///
/// Responde de relance a pergunta que o `/status` do Claude Code não responde —
/// "em qual conta eu estou?" — porque o registro de sessões é por perfil, e o
/// perfil é o grupo. É também onde o modo de falha silencioso aparece: sessão
/// que você abriu esperando um grupo e subiu no perfil padrão aparece contada
/// do lado errado.
struct SessionsBadge: View {
    let sessions: [LiveSession]

    private var engajadas: Int { sessions.count { $0.status.isEngaged } }

    var body: some View {
        if !sessions.isEmpty {
            Label {
                Text(verbatim: "\(sessions.count)")
                    .font(.caption2.monospacedDigit())
            } icon: {
                // Ponto cheio quando alguma sessão está de fato trabalhando ou
                // esperando resposta; vazio quando estão todas ociosas. É a
                // diferença entre "há sessões abertas" e "há trabalho em curso",
                // e ela decide se trocar a conta agora se faz sentir.
                Image(systemName: engajadas > 0 ? "circle.fill" : "circle")
                    .font(.system(size: 5))
            }
            .foregroundStyle(engajadas > 0 ? AnyShapeStyle(UsageColor.calm)
                                           : AnyShapeStyle(.tertiary))
            .help(String(format: String(localized: engajadas > 0
                                        ? "groups.sessions.help.active.format"
                                        : "groups.sessions.help.idle.format"),
                         sessions.count, engajadas))
        }
    }
}

/// Os dois limiares de idade da amostra, num lugar só — o painel e a tela de
/// grupos precisam concordar sobre o que é "velho".
enum UsageAge {
    /// Acima disto o número esmaece: pode já descrever outra realidade.
    static let stale: TimeInterval = 3600
    /// Acima disto ganha marca explícita: meio dia sem medir, em conta
    /// compartilhada, é tempo de sobra para o valor ficar otimista.
    static let veryStale: TimeInterval = 12 * 3600
}

private struct MiniBar: View {
    /// A janela de 5h. `nil` quando não há medida válida: a trilha fica VAZIA,
    /// sem barra nenhuma — o traço da coluna já diz que não se sabe, e uma
    /// barra de largura mínima ali leria como "quase zero", que é afirmação.
    let fraction: Double?

    var body: some View {
        GeometryReader { geo in
            ZStack(alignment: .leading) {
                Capsule().fill(Color.secondary.opacity(0.18))
                if let fraction {
                    Capsule()
                        .fill(UsageColor.bar(fraction))
                        .frame(width: max(2, geo.size.width * min(1, max(0, fraction))))
                }
            }
        }
        .frame(width: AccountsLayout.bar, height: 5)
    }
}
