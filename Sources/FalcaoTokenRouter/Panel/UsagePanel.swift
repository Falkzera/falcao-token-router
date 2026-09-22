import AppKit
import SwiftUI
import CCUsageCore

struct UsagePanel: View {
    @Environment(\.openWindow) private var openWindow
    let snapshot: UsageSnapshot
    let plan: Plan
    /// `true` quando a busca ao vivo está desligada — o único estado em que
    /// ligar é uma ação disponível.
    let canEnableLive: Bool
    let onEnableLive: () -> Void
    /// Os grupos do usuário e o que o sensor mediu de cada conta. Separado do
    /// `snapshot` de propósito: aquele mede o perfil onde o app roda, este
    /// responde "quais contas existem e quanto sobrou em cada uma".
    let router: RouterConfigStore
    /// Em qual aba a janela deve abrir. O rodapé tem duas portas para a MESMA
    /// janela — manter os dois rótulos, e não um só, é de propósito: "Grupos" e
    /// "Ajustes" são o que o usuário procura, e obrigá-lo a abrir uma para
    /// achar a outra é o que a janela única veio desfazer.
    let navigation: HomeNavigation

    /// A conta que o usuário tocou na tabela. Enquanto houver uma, o cartão de
    /// baixo fala DELA em vez da sessão do perfil medido — que é a resposta para
    /// "e as outras contas, como estão?", impossível de ver antes sem trocar de
    /// conta só para medir.
    @ViewState private var selectedAccount: UUID?

    var body: some View {
        GlassEffectContainer {
            VStack(alignment: .leading, spacing: 14) {
                // Mesmo cartão das outras seções: sem ele, CONTAS flutuava sobre
                // o fundo do painel enquanto Sessão e Valor tinham superfície
                // própria, e a régua separando as duas metades era um traço.
                AccountsSection(store: router, selection: $selectedAccount)
                    .padding(12)
                    .glassEffect(.regular, in: .rect(cornerRadius: 16))
                detailSection
                valueSection
                if !snapshot.unknownModels.isEmpty { unknownModelsNotice }
                footer
            }
            .padding(16)
            .frame(width: 330)
        }
    }

    // MARK: - Sessão / conta selecionada

    /// O cartão de baixo tem dois assuntos possíveis. Sem seleção fala da sessão
    /// medida (números ricos, do medidor local); com seleção fala da conta
    /// tocada, e aí só existe o que o sensor passivo colheu.
    @ViewBuilder
    private var detailSection: some View {
        if let id = selectedAccount, let account = router.config.account(id) {
            accountSection(account)
        } else {
            sessionSection
        }
    }

    /// O detalhamento de UMA conta, do sensor.
    ///
    /// Deliberadamente mais pobre que a sessão: não há tokens/min nem valor aqui
    /// porque isso vem do JSONL do perfil local, e de outra conta não temos —
    /// inventar as linhas com o número do perfil medido seria atribuir a ela um
    /// consumo que não é dela.
    private func accountSection(_ account: Account) -> some View {
        let usage = router.usageDetail[account.id]
        return VStack(alignment: .leading, spacing: 10) {
            HStack(spacing: 6) {
                sectionTitle("panel.section.account")
                Text(verbatim: "· \(account.label)")
                    .font(.caption2).foregroundStyle(.tertiary)
                    .lineLimit(1)
                Spacer(minLength: 6)
                Button(action: { selectedAccount = nil }) {
                    Image(systemName: "xmark.circle.fill")
                }
                .buttonStyle(.plain)
                .font(.caption2)
                .foregroundStyle(.tertiary)
                .help(String(localized: "panel.account.back"))
            }

            if let usage {
                sensorGauge(title: String(localized: "panel.gauge.current"),
                            fraction: usage.fiveHour, resetsAt: usage.fiveHourResetsAt)
                sensorGauge(title: String(localized: "panel.gauge.weekly"),
                            fraction: usage.sevenDay, resetsAt: usage.sevenDayResetsAt)
                // Rótulo E ícone acompanham a origem: a antena é o sensor (uma
                // requisição que a conta atendeu), o medidor é a sonda (uma
                // consulta feita de propósito). Dizer "pela sessão da própria
                // conta" sobre um número sondado era falso justamente nas contas
                // ociosas, que sessão nenhuma serviu.
                Label(String(format: String(localized: usage.origin == .probe
                                            ? "panel.account.sampled.probe.format"
                                            : "panel.account.sampled.sensor.format"),
                             Format.duration(Date().timeIntervalSince(usage.sampledAt))),
                      systemImage: usage.origin == .probe
                          ? "gauge.with.dots.needle.bottom.50percent"
                          : "dot.radiowaves.left.and.right")
                    .font(.caption2)
                    .foregroundStyle(.tertiary)
                // A janela POR MODELO vem da sonda, não do sensor, e por isso
                // fica embaixo com carimbo próprio: ela pode ser de ontem
                // enquanto as duas de cima são de agora. Só aparece quando
                // alguém sondou — inventar um "0%" aqui seria afirmar folga
                // num limite que ninguém mediu.
                if let modelo = usage.model, let quando = usage.modelSampledAt {
                    Divider().opacity(0.4)
                    sensorGauge(title: modelo.name, fraction: modelo.percent,
                                resetsAt: modelo.resetsAt)
                    Label(String(format: String(localized: "panel.account.probed.format"),
                                 Format.duration(Date().timeIntervalSince(quando))),
                          systemImage: "gauge.with.dots.needle.bottom.50percent")
                        .font(.caption2)
                        .foregroundStyle(.tertiary)
                }
            } else {
                Text("panel.accounts.ready.help.probe")
                    .font(.caption2)
                    .foregroundStyle(.secondary)
            }
        }
        .padding(12)
        .glassEffect(.regular, in: .rect(cornerRadius: 16))
    }

    /// O medidor de uma janela do sensor. Sem `UsageSnapshot.Gauge`: aquele traz
    /// procedência, saturação e teto calibrado, que só existem para o perfil
    /// medido. Aqui há uma fração e um reset, e é só isso que se promete.
    private func sensorGauge(title: String, fraction: Double?, resetsAt: Date?) -> some View {
        VStack(alignment: .leading, spacing: 4) {
            HStack {
                Text(verbatim: title).font(.callout)
                Spacer()
                if let fraction {
                    Text(Format.percent(fraction))
                        .font(.callout.monospacedDigit())
                        .foregroundStyle(.primary)
                } else {
                    Text("panel.accounts.window.absent")
                        .font(.callout)
                        .foregroundStyle(.tertiary)
                }
            }
            ProgressView(value: fraction ?? 0)
                .tint(UsageColor.bar(fraction ?? 0))
                .opacity(fraction == nil ? 0.3 : 1)
            Text(fraction == nil
                 ? String(localized: "panel.account.window.expired")
                 : resetText(resetsAt))
                .font(.caption)
                .foregroundStyle(.secondary)
        }
    }

    /// "reseta 13:20 · em 4h 6m", com o tempo restante contado de agora — a
    /// amostra pode ser de horas atrás, mas o RESET é uma hora do relógio e não
    /// envelhece junto com ela.
    private func resetText(_ resetsAt: Date?) -> String {
        guard let resetsAt else { return "" }
        let base = String(format: String(localized: "panel.reset.format"),
                          Format.clockTime(resetsAt))
        let remaining = resetsAt.timeIntervalSinceNow
        guard remaining > 0 else { return base }
        return String(format: String(localized: "panel.reset.remaining.format"),
                      base, Format.duration(remaining))
    }

    /// De qual perfil são os números da sessão. O medidor lê o perfil PADRÃO
    /// (`~/.claude`); num mundo de grupos, dizer isso evita "82% de quem?".
    private var measuredProfileLabel: String? {
        guard !router.config.groups.isEmpty else { return nil }
        if let group = router.config.defaultGroup {
            let account = router.activeByGroup[group.id]
                .flatMap { router.config.account($0) }?.label
            return account.map {
                String(format: String(localized: "panel.session.profile.format"),
                       group.name, $0)
            } ?? group.name
        }
        return String(localized: "panel.session.profile.plain")
    }

    private var sessionSection: some View {
        VStack(alignment: .leading, spacing: 10) {
            HStack(spacing: 6) {
                sectionTitle("panel.section.session")
                if let label = measuredProfileLabel {
                    Text(verbatim: "· \(label)")
                        .font(.caption2).foregroundStyle(.tertiary)
                        .lineLimit(1)
                }
            }

            gauge(title: String(localized: "panel.gauge.current"),
                  gauge: snapshot.session,
                  detail: resetDetail(snapshot.session))

            if let weekly = snapshot.weekly {
                gauge(title: String(localized: "panel.gauge.weekly"), gauge: weekly, detail: resetDetail(weekly))
            } else {
                paceRow(snapshot.weeklyPace)
            }

            // Só as que dizem algo. Uma linha "Fable 0%" permanente é ruído: a
            // janela existe no payload mas não informa nada.
            ForEach(snapshot.scopedWeekly.filter { $0.gauge.isActive || $0.gauge.rawFraction > 0 },
                    id: \.self) { scoped in
                gauge(title: scoped.modelName,
                      gauge: scoped.gauge,
                      detail: resetDetail(scoped.gauge))
            }

            if let rate = snapshot.burnRatePerMinute {
                Text(String(format: String(localized: "panel.burnRate.format"),
                            Format.tokens(UInt64(rate))))
                    .font(.caption)
                    .foregroundStyle(.secondary)
            }

            provenanceRow
        }
        .padding(12)
        .glassEffect(.regular, in: .rect(cornerRadius: 16))
    }

    /// De onde vieram os números. Substitui o rótulo binário anterior, que só
    /// sabia dizer "oficial" ou "estimado" — e chamava de oficial um cache que
    /// podia estar treze horas atrasado.
    private var provenanceRow: some View {
        let (text, icon, isWarning) = provenance
        return HStack(spacing: 6) {
            Label(text, systemImage: icon)
                .font(.caption2)
                .foregroundStyle(isWarning
                                 ? AnyShapeStyle(UsageColor.warning)
                                 : AnyShapeStyle(HierarchicalShapeStyle.tertiary))
            if offersLiveUsage {
                Button("panel.enableLive", action: onEnableLive)
                    .buttonStyle(.link)
                    .font(.caption2)
            }
        }
    }

    /// A linha diz há muito tempo o que está errado; até aqui ela não fazia nada
    /// a respeito. O botão aparece só onde o problema é remediável por clique.
    ///
    /// Credencial expirada não entra: a saída é rodar o Claude Code. Sem conexão
    /// também não: a saída é esperar. Oferecer um botão nesses estados seria
    /// prometer conserto que ele não faz.
    private var offersLiveUsage: Bool {
        guard canEnableLive else { return false }
        switch snapshot.sourceStatus {
        case .cached, .derivedOnly: return true
        case .live, .credentialExpired, .liveUnavailable: return false
        }
    }

    private var provenance: (String, String, Bool) {
        switch snapshot.sourceStatus {
        case .live:
            return (String(localized: "panel.provenance.live"), "bolt.horizontal.circle", false)
        case let .cached(age):
            // Uma hora é 20% de uma janela de 5h. Cache mais velho que isso já
            // pode estar descrevendo uma sessão que resetou — foi exatamente o
            // estado que mostrava 35% quando o valor real era 6%.
            return age < 3600
                ? (String(format: String(localized: "panel.provenance.cached.format"),
                          Format.duration(age)), "clock", false)
                : (String(format: String(localized: "panel.provenance.stale.format"),
                          Format.duration(age)), "exclamationmark.triangle", true)
        case let .credentialExpired(age):
            return (String(format: String(localized: "panel.provenance.expired.format"),
                           Format.duration(age)), "exclamationmark.triangle", true)
        case let .liveUnavailable(age):
            return (String(format: String(localized: "panel.provenance.offline.format"),
                           Format.duration(age)), "wifi.slash", true)
        case .derivedOnly:
            return (String(localized: "panel.provenance.derived"), "info.circle", false)
        }
    }

    private func resetDetail(_ gauge: UsageSnapshot.Gauge) -> String {
        var text: String
        if let resetsAt = gauge.resetsAt {
            text = String(format: String(localized: "panel.reset.format"),
                          Format.clockTime(resetsAt))
            if let remaining = gauge.timeRemaining(at: snapshot.generatedAt) {
                text = String(format: String(localized: "panel.reset.remaining.format"),
                              text, Format.duration(remaining))
            }
        } else {
            text = gauge.isOfficial ? "" : String(localized: "panel.reset.noSession")
        }

        // Um snapshot pode ter procedências mistas: a semanal vem do relatório
        // oficial e a sessão cai no derivado quando o payload não traz a janela
        // de 5h — cache antigo com `five_hour: null`, que é o estado justamente
        // quando o cache está mais velho. A linha de procedência é uma só, do
        // snapshot inteiro, então sem esta marca o medidor derivado apareceria
        // sob "ao vivo", prometendo um frescor que ele não tem. O código
        // anterior estampava a idade por medidor e não tinha esse buraco; a
        // linha única é melhor, mas precisa disto para não mentir.
        if gauge.provenance == .derived, snapshot.sourceStatus != .derivedOnly {
            text = text.isEmpty
                ? String(localized: "panel.provenance.derived")
                : String(format: String(localized: "panel.reset.estimatedSuffix.format"), text)
        }
        return text
    }

    /// Sem barra, deliberadamente: barra implica um teto, e aqui não há teto a
    /// prometer — só a comparação com o ritmo habitual.
    private func paceRow(_ pace: Pace) -> some View {
        VStack(alignment: .leading, spacing: 2) {
            HStack {
                Text("panel.gauge.weekly").font(.callout)
                Spacer()
                if let multiple = pace.multiple {
                    Text(String(format: "%.1f×", multiple))
                        .font(.callout.monospacedDigit())
                        .foregroundStyle(multiple >= 2 ? UsageColor.warning : .primary)
                } else {
                    Text(verbatim: "—").font(.callout).foregroundStyle(.secondary)
                }
            }
            Text(pace.multiple == nil
                 ? String(format: String(localized: "panel.pace.recent.format"),
                          Format.tokens(pace.tokens))
                 : String(format: String(localized: "panel.pace.typical.format"),
                          Format.tokens(pace.tokens), Format.tokens(pace.typical)))
                .font(.caption)
                .foregroundStyle(.secondary)
        }
    }

    /// A barra usa a fração saturada; o texto usa a bruta, para não esconder
    /// que o consumo passou do maior já observado.
    private func gauge(title: String, gauge g: UsageSnapshot.Gauge, detail: String) -> some View {
        let fraction = g.fraction
        return VStack(alignment: .leading, spacing: 4) {
            HStack {
                Text(title).font(.callout)
                Spacer()
                Text(Format.percent(g.rawFraction))
                    .font(.callout.monospacedDigit())
                    .foregroundStyle(g.rawFraction > 1 ? UsageColor.critical : .primary)
            }
            ProgressView(value: fraction)
                .tint(UsageColor.bar(fraction))
            if !detail.isEmpty {
                Text(detail).font(.caption).foregroundStyle(.secondary)
            }
        }
    }

    // MARK: - Valor

    private var valueSection: some View {
        VStack(alignment: .leading, spacing: 10) {
            sectionTitle("panel.section.value")
            HStack(alignment: .top, spacing: 0) {
                column("panel.column.today", snapshot.today)
                column("panel.column.week", snapshot.week)
                column("panel.column.month", snapshot.month)
            }
            returnRow
        }
        .padding(12)
        .glassEffect(.regular, in: .rect(cornerRadius: 16))
    }

    /// Quanto o valor equivalente de API do mês cobre a mensalidade. Some
    /// enquanto não há consumo: "0×" no dia 1 não informa nada.
    @ViewBuilder
    private var returnRow: some View {
        if let multiple = plan.returnMultiple(forMonthly: snapshot.month.money) {
            Divider()
            HStack(spacing: 4) {
                // `verbatim`: os dois pedaços já vêm localizados, e o
                // separador não é texto a traduzir. Sem isto o SwiftUI procura
                // uma chave "%@ · %@" que não existe.
                Text(verbatim: "\(plan.label) · \(Format.planPrice(plan))")
                    .font(.caption)
                    .foregroundStyle(.secondary)
                Spacer()
                // Herda o "+" do total parcial: um piso não pode ser
                // apresentado como número exato.
                Text(String(format: "%.1f×", multiple)
                     + (snapshot.month.money.isPartial ? "+" : ""))
                    .font(.caption.weight(.semibold).monospacedDigit())
                    .foregroundStyle(multiple >= 1 ? UsageColor.calm : .secondary)
                Text("panel.return.suffix")
                    .font(.caption)
                    .foregroundStyle(.secondary)
            }
        }
    }

    private func column(_ title: LocalizedStringKey, _ totals: Totals) -> some View {
        VStack(alignment: .leading, spacing: 2) {
            Text(title).font(.caption2).foregroundStyle(.secondary)
            Text(Format.tokens(totals.tokens)).font(.callout.monospacedDigit())
            Text(Format.money(totals.money))
                .font(.callout.weight(.medium).monospacedDigit())
        }
        .frame(maxWidth: .infinity, alignment: .leading)
    }

    // MARK: - Rodapé

    private func sectionTitle(_ text: LocalizedStringKey) -> some View {
        Text(text)
            .font(.caption2.weight(.semibold))
            .foregroundStyle(.secondary)
    }

    private var unknownModelsNotice: some View {
        Label(String(format: String(localized: "panel.unknownModels.format"),
                     snapshot.unknownModels.sorted().joined(separator: ", ")),
              systemImage: "exclamationmark.triangle")
            .font(.caption2)
            .foregroundStyle(.orange)
    }

    /// Um botão do rodapé: escolhe a aba e abre a janela única.
    private func openTab(_ tab: HomeTab,
                         label: LocalizedStringKey,
                         icon: String) -> some View {
        Button {
            navigation.tab = tab
            openWindow(id: HomeWindowID.value)
            NSApp.activate()
        } label: {
            Label(label, systemImage: icon)
        }
        .buttonStyle(.plain)
        .font(.caption)
        .foregroundStyle(.secondary)
    }

    private var footer: some View {
        HStack {
            // As duas portas da MESMA janela, cada uma na sua aba. O
            // `SettingsLink` saiu com a cena `Settings`: sem ela o link não
            // teria o que abrir.
            //
            // A ativação vai no clique, não no `onAppear` da janela. O app é
            // `LSUIElement`: não tem Dock e nunca se ativa sozinho, e a janela
            // é criada uma vez e reaproveitada — `onAppear` não dispararia da
            // segunda vez em diante, que é justamente quando o usuário já está
            // confuso e clicando de novo. Medido: a janela existia com
            // `onscreen=false`, e ativar o app a trouxe para a frente sem tocar
            // em mais nada.
            openTab(.groups, label: "panel.groups", icon: "rectangle.stack")
            openTab(.meter, label: "panel.settings", icon: "gearshape")
            Spacer()
            Button("panel.quit") { NSApplication.shared.terminate(nil) }
                .buttonStyle(.plain)
                .font(.caption)
                .foregroundStyle(.secondary)
        }
    }
}
