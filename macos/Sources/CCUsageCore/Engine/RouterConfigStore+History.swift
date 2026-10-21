import Foundation

/// Uma conta a medir, com o perfil já decidido pelo motor.
public struct ProbeTarget: Sendable, Equatable {
    public let accountID: UUID
    public let email: String
    public let dir: ConfigDir
}

/// Um grupo na medição automática: as contas dele que venceram, na ordem da
/// lista. A marca "medindo" do grupo acompanha o plano.
public struct MeasurePlan: Sendable, Equatable {
    public let groupID: UUID
    public let targets: [ProbeTarget]
}

/// O histórico e a medição de 5 em 5 horas (21/10/2026). Arquivo à parte do
/// `RouterConfigStore.swift` pela régua de 600 linhas de produção.
extension RouterConfigStore {
    /// As contas de um grupo com o perfil por onde medir cada uma — decidido
    /// AQUI, com o config na mão: conta ativa vai pelo perfil do grupo, nunca
    /// pela casa (ver `probeConfigDir` — sondar a casa de uma conta ativa
    /// derruba a sessão viva). O botão e a agenda passam por aqui.
    public func probeTargets(in group: AccountGroup) -> [ProbeTarget] {
        config.accounts(in: group).map {
            ProbeTarget(accountID: $0.id, email: $0.identity.email,
                        dir: engine.probeConfigDir(for: $0, config: config))
        }
    }

    /// A medição automática: por grupo, na ordem da lista, um plano só com as
    /// contas VENCIDAS (`MeasureSchedule`). Cada conta vai uma vez — a que está
    /// em dois grupos, no primeiro —, e grupo sem conta vencida fica de fora.
    public func scheduledMeasurePlans(now: Date = Date()) -> [MeasurePlan] {
        let history = paths.historyDir
        var planned = Set<UUID>()
        var plans: [MeasurePlan] = []
        for group in config.groups {
            let targets = probeTargets(in: group).filter { target in
                !planned.contains(target.accountID) && MeasureSchedule.isDue(
                    lastMeasured: UsageHistory.lastSampledAt(forEmail: target.email, in: history),
                    lastAttempt: measureAttempts[target.accountID], now: now)
            }
            guard !targets.isEmpty else { continue }
            planned.formUnion(targets.map(\.accountID))
            plans.append(MeasurePlan(groupID: group.id, targets: targets))
        }
        return plans
    }

    /// Uma olhada na agenda: mede, grupo a grupo, as contas vencidas. Chamada
    /// pelo laço do app (180 s) a partir da 2ª volta, com a chave dos Ajustes
    /// ligada.
    ///
    /// Silenciosa: falha não vira `lastError` — ele é da última ação DO
    /// USUÁRIO, e uma conta deslogada repetiria o aviso a cada 5 horas; a conta
    /// que não respondeu vira um buraco no gráfico. Espera medição em curso (o
    /// botão) e login aberto.
    public func runScheduledMeasure(now: Date = Date()) async {
        guard measuringGroup == nil, !signingIn else { return }
        let plans = scheduledMeasurePlans(now: now)
        guard !plans.isEmpty else { return }
        // A tentativa conta antes de medir: sucesso ou falha, a conta só volta
        // ao plano em 5 horas.
        for plan in plans {
            for target in plan.targets { measureAttempts[target.accountID] = now }
        }
        let usageDir = paths.usageDir
        let historyDir = paths.historyDir
        let scratchBase = paths.base
        for plan in plans {
            // A marca passa de grupo em grupo sem se apagar no meio: o botão
            // não começa outra medição entre dois grupos.
            measuringGroup = plan.groupID
            let targets = plan.targets
            await Task.detached(priority: .utility) {
                guard let probe = ClaudeUsageProbe.system(scratchBase: scratchBase) else { return }
                for target in targets {
                    guard let reading = try? probe.read(configDir: target.dir) else { continue }
                    try? ProbeRecord.write(reading, email: target.email, configDir: target.dir,
                                           usageDir: usageDir, historyDir: historyDir)
                }
            }.value
            refreshUsage()
        }
        measuringGroup = nil
    }

    /// "Apagar o histórico…" (Ajustes): os pontos de todas as contas saem do
    /// disco. As medições continuam: cada conta ganha um novo primeiro ponto na
    /// próxima. Devolve quantos arquivos saíram.
    @discardableResult
    public func clearHistory() throws -> Int {
        let removed = try UsageHistory.clear(in: paths.historyDir)
        refreshHistory()
        return removed
    }

    /// Relê o histórico de cada conta e publica.
    public func refreshHistory() {
        let dir = paths.historyDir
        var byAccount: [UUID: [UsageHistoryPoint]] = [:]
        for account in config.accounts {
            byAccount[account.id] = UsageHistory.read(forEmail: account.identity.email, in: dir)
        }
        historyByAccount = byAccount
    }
}
